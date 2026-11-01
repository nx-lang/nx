//! Structural source-test expectations, not an evaluator.
use nx_hir::Name;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int32(i32),
    Int(i64),
    Float32(f32),
    Float(f64),
    String(SmolStr),
    Boolean(bool),
    Array(Vec<Value>),
    UnionCase {
        union: Name,
        case: SmolStr,
    },
    Record {
        type_name: Name,
        fields: FxHashMap<SmolStr, Value>,
    },
    Function {
        module: SmolStr,
        name: SmolStr,
    },
    ActionHandler {
        action_name: Name,
        token: Option<SmolStr>,
    },
}
impl Value {
    pub fn is_empty_value(&self) -> bool {
        matches!(self, Self::Array(v) if v.is_empty())
    }
    pub fn empty() -> Self {
        Self::Array(Vec::new())
    }
}

pub struct IrEvaluator {
    source: String,
    pub artifact: nx_api::ProgramArtifact,
}
impl IrEvaluator {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
            artifact: nx_api::build_program_artifact_from_source(
                source,
                "test.nx",
                &nx_api::ProgramBuildContext::empty(),
            )
            .unwrap(),
        }
    }
    pub fn instantiate_record_defaults(
        &self,
        _module: &nx_hir::LoweredModule,
        name: &str,
    ) -> Result<Value, String> {
        let source = format!("{}\nlet retirementConstruct() = <{name} />", self.source);
        let evaluator = Self::new(&source);
        evaluator.execute_function(_module, "retirementConstruct", vec![])
    }
    pub fn execute_function(
        &self,
        _module: &nx_hir::LoweredModule,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        let program = super::link(&self.artifact, &self.artifact.entry_identity)
            .map_err(|e| format!("{e:?}"))?;
        let args = args.iter().map(to_host).collect::<Vec<_>>();
        let value = program
            .evaluate_function_value(name, &args, &Default::default())
            .map_err(|e| e.to_string())?;
        let result_type = self.artifact.root_modules[0]
            .type_env
            .lookup(&Name::new(name))
            .and_then(nx_types::Type::function_parts)
            .map(|(_, result)| result.to_string());
        let mut result = from_view(value.view());
        normalize_number(&mut result, result_type.as_deref().unwrap_or(""));
        normalize_record_numbers(&mut result, &self.artifact);
        Ok(result)
    }
}
pub fn to_host(v: &Value) -> nx_value::NxValue {
    use nx_value::NxValue as N;
    match v {
        Value::Int(v) => N::Int(*v),
        Value::Int32(v) => N::Int(*v as i64),
        Value::Float(v) => N::Float(*v),
        Value::Float32(v) => N::Float(*v as f64),
        Value::Boolean(v) => N::Bool(*v),
        Value::String(v) => N::String(v.to_string()),
        Value::Array(v) => N::Array(v.iter().map(to_host).collect()),
        Value::Record { type_name, fields } => N::Record {
            type_name: Some(type_name.to_string()),
            properties: fields
                .iter()
                .map(|(k, v)| (k.to_string(), to_host(v)))
                .collect(),
        },
        Value::UnionCase { case, .. } => N::String(case.to_string()),
        Value::Function { module, name } => N::Record {
            type_name: Some("Function".into()),
            properties: std::collections::BTreeMap::from([
                ("module".into(), N::String(module.to_string())),
                ("name".into(), N::String(name.to_string())),
            ]),
        },
        Value::ActionHandler { .. } => panic!("host input cannot contain a handler body"),
    }
}
pub fn from_view(v: nx_api::ir_runtime::ValueView<'_>) -> Value {
    use nx_api::ir_runtime::ValueKind as K;
    match v.kind() {
        K::Int(v) => Value::Int(v),
        K::Float(v) => Value::Float(v),
        K::Boolean(v) => Value::Boolean(v),
        K::String(v) => Value::String(v.into()),
        K::Sequence(v) => Value::Array(v.map(from_view).collect()),
        K::Record { type_name, fields } => Value::Record {
            type_name: Name::new(type_name.unwrap_or("object")),
            fields: fields.map(|(k, v)| (k.into(), from_view(v))).collect(),
        },
        K::Case { union, case } => Value::UnionCase {
            union: Name::new(union),
            case: case.into(),
        },
        K::Function { module, name } => Value::Function {
            module: module.into(),
            name: name.into(),
        },
        K::ActionHandler { action } => Value::ActionHandler {
            action_name: Name::new(action),
            token: None,
        },
    }
}

fn normalize_number(value: &mut Value, ty: &str) {
    let ty = ty.trim_end_matches(['?', '*', '+']);
    match value {
        Value::Array(items) => items.iter_mut().for_each(|v| normalize_number(v, ty)),
        Value::Int(v) if ty == "float64" => *value = Value::Float(*v as f64),
        Value::Int(v) if ty == "float32" => *value = Value::Float32(*v as f32),
        Value::Float(v) if ty == "float32" => *value = Value::Float32(*v as f32),
        _ => {}
    }
}

fn normalize_record_numbers(value: &mut Value, artifact: &nx_api::ProgramArtifact) {
    match value {
        Value::Array(items) => items
            .iter_mut()
            .for_each(|v| normalize_record_numbers(v, artifact)),
        Value::Record { type_name, fields } => {
            for module in artifact.resolved_program.modules() {
                for item in module.lowered_module.items() {
                    if let nx_hir::Item::Record(record) = item {
                        if record.name == *type_name {
                            for field in &record.properties {
                                if let Some(value) = fields.get_mut(field.name.as_str()) {
                                    normalize_number(
                                        value,
                                        &nx_hir::ast::spell_type_ref(&field.ty),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            fields
                .values_mut()
                .for_each(|v| normalize_record_numbers(v, artifact));
        }
        _ => {}
    }
}

#[derive(Debug)]
pub struct InitValue {
    pub rendered: Value,
    pub state_snapshot: nx_api::ir_runtime::ComponentInstance,
}
#[derive(Debug)]
pub struct DispatchValue {
    pub rendered: Value,
    pub effects: Vec<Value>,
    pub state_snapshot: nx_api::ir_runtime::ComponentInstance,
}
#[derive(Debug)]
pub struct RenderValue {
    pub rendered: Value,
}
fn fields_of(value: Value) -> std::collections::BTreeMap<String, nx_value::NxValue> {
    match to_host(&value) {
        nx_value::NxValue::Record { properties, .. } => properties,
        nx_value::NxValue::Array(v) if v.is_empty() => Default::default(),
        other => panic!("expected props/state record, got {other:?}"),
    }
}
impl IrEvaluator {
    fn linked(&self) -> nx_api::ir_runtime::Program {
        super::link(&self.artifact, &self.artifact.entry_identity).unwrap()
    }
    pub fn initialize_component(
        &self,
        _module: &nx_hir::LoweredModule,
        name: &str,
        props: Value,
    ) -> Result<InitValue, String> {
        let result = self
            .linked()
            .initialize_component(
                name,
                &fields_of(props),
                &Default::default(),
                &Default::default(),
            )
            .map_err(|e| e.to_string())?;
        Ok(InitValue {
            rendered: from_host(&result.rendered),
            state_snapshot: result.instance,
        })
    }
    pub fn evaluate_component(
        &self,
        _module: &nx_hir::LoweredModule,
        name: &str,
        props: Value,
        state: Value,
    ) -> Result<RenderValue, String> {
        let result = self
            .linked()
            .evaluate_component(
                name,
                &fields_of(props),
                &fields_of(state),
                &Default::default(),
            )
            .map_err(|e| e.to_string())?;
        Ok(RenderValue {
            rendered: from_host(&result),
        })
    }
    pub fn dispatch_component_actions(
        &self,
        _module: &nx_hir::LoweredModule,
        instance: &nx_api::ir_runtime::ComponentInstance,
        batch: Vec<Value>,
    ) -> Result<DispatchValue, String> {
        let result = self
            .linked()
            .dispatch_component_actions(
                instance,
                &batch.iter().map(to_host).collect::<Vec<_>>(),
                &Default::default(),
            )
            .map_err(|e| e.to_string())?;
        Ok(DispatchValue {
            rendered: from_host(&result.rendered),
            effects: result.effects.iter().map(from_host).collect(),
            state_snapshot: result.instance,
        })
    }
}
fn from_host(value: &nx_value::NxValue) -> Value {
    use nx_value::NxValue as N;
    match value {
        N::Int(v) => Value::Int(*v),
        N::Int32(v) => Value::Int(*v as i64),
        N::Float(v) => Value::Float(*v),
        N::Float32(v) => Value::Float(*v as f64),
        N::Bool(v) => Value::Boolean(*v),
        N::String(v) => Value::String(v.as_str().into()),
        N::Null => Value::empty(),
        N::Array(v) => Value::Array(v.iter().map(from_host).collect()),
        N::Record {
            type_name,
            properties,
        } => {
            let text = |key| match properties.get(key) {
                Some(N::String(v)) => v.as_str(),
                _ => "",
            };
            match type_name.as_deref() {
                Some("Function") => Value::Function {
                    module: text("module").into(),
                    name: text("name").into(),
                },
                Some("ActionHandler") => Value::ActionHandler {
                    action_name: Name::new(text("action")),
                    token: properties.get("token").and_then(|v| {
                        if let N::String(v) = v {
                            Some(v.as_str().into())
                        } else {
                            None
                        }
                    }),
                },
                _ => Value::Record {
                    type_name: Name::new(type_name.as_deref().unwrap_or("object")),
                    fields: properties
                        .iter()
                        .map(|(k, v)| (k.as_str().into(), from_host(v)))
                        .collect(),
                },
            }
        }
    }
}
