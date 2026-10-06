//! JSON Schema for the functions and declared types of a built program.
//!
//! <para>A host that hands an NX function to a system speaking JSON Schema — a language model's tool
//! interface, an MCP client — asks the program artifact for the function's argument and result
//! schemas here. The artifact is the one place that has documentation, alias targets, applied type
//! arguments and inferred result types together; an NX IR image has none of them.</para>
//!
//! <para>Every schema describes the canonical JSON value encoding and nothing wider, so a value that
//! is valid against an input schema is accepted at a runtime's boundary, and a value a runtime
//! returns is valid against the output schema. A type with no JSON form is reported as a
//! diagnostic, never approximated.</para>

use crate::diagnostics::{diagnostics_to_api_with_sources, text_range_to_span, LineIndex};
use crate::{NxDiagnostic, NxSeverity, NxTextSpan, ProgramArtifact};
use nx_diagnostics::{Diagnostic, Label, TextSpan};
use nx_hir::ast::{self, Expr, Literal, Occurrence, TypeRef, UnOp};
use nx_hir::{
    DeclaringOrigin, EffectiveField, Function, Item, LocalDefinitionId, LoweredModule, Name,
    PreparedModule, PreparedNamespace, QualifiedExprRef, RecordDef, RecordField, UnionCaseDef,
    UnionDef,
};
use nx_types::{ModuleArtifact, Primitive, Type};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserialize, Serialize, Serializer};

/// The JSON Schema dialect every document declares.
pub const JSON_SCHEMA_DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

/// A JSON value whose objects keep their keys in the order they were written.
///
/// <para>Schema documents are compared as text across SDKs, and a property list reads in
/// declaration order, so key order is part of the answer rather than an accident of a map.</para>
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<SchemaValue>),
    Object(Vec<(String, SchemaValue)>),
}

impl SchemaValue {
    fn object(entries: Vec<(&str, SchemaValue)>) -> Self {
        SchemaValue::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
        )
    }

    fn string(text: impl Into<String>) -> Self {
        SchemaValue::String(text.into())
    }

    /// Appends `key` to an object; anything else is left as it is.
    fn push(&mut self, key: &str, value: SchemaValue) {
        if let SchemaValue::Object(entries) = self {
            entries.push((key.to_string(), value));
        }
    }
}

impl Serialize for SchemaValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            SchemaValue::Null => serializer.serialize_unit(),
            SchemaValue::Bool(value) => serializer.serialize_bool(*value),
            SchemaValue::Int(value) => serializer.serialize_i64(*value),
            SchemaValue::Float(value) => serializer.serialize_f64(*value),
            SchemaValue::String(value) => serializer.serialize_str(value),
            SchemaValue::Array(items) => {
                let mut seq = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            SchemaValue::Object(entries) => {
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (key, value) in entries {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
        }
    }
}

/// A declaration named by the identity of its module and its declared name.
///
/// <para>This is the pair a canonical `Function` record carries, so a host holding one asks for its
/// schema with no translation. A reference with no module names the program's entry module.</para>
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeclarationRef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    pub name: String,
}

impl DeclarationRef {
    /// A reference to `name` in the entry module.
    pub fn entry(name: impl Into<String>) -> Self {
        Self {
            module: None,
            name: name.into(),
        }
    }

    /// A reference to `name` in the module `module`.
    pub fn in_module(module: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            module: Some(module.into()),
            name: name.into(),
        }
    }
}

/// A declaration the export found, by module identity and name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclarationName {
    pub module: String,
    pub name: String,
}

/// Options for a function's schema.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FunctionSchemaOptions {
    /// Types whose parameters the host fills in itself. A parameter declared with one of them, or
    /// with a record that extends one, is left out of the input schema.
    #[serde(default)]
    pub host_supplied_types: Vec<DeclarationRef>,
}

/// Which way a value crosses the host boundary, which decides where `$type` is asked for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SchemaDirection {
    /// A value a host supplies to a runtime: a function's arguments.
    Input,
    /// A value a runtime returns to a host: a function's result.
    #[default]
    Output,
}

/// Options for a declared type's schema.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TypeSchemaOptions {
    #[serde(default)]
    pub direction: SchemaDirection,
}

/// The schema of one function: its arguments, its result and its documentation.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionSchema {
    pub module: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub parameters: Vec<ParameterSchema>,
    /// Absent when a parameter's type has no JSON form; the diagnostics say which.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<SchemaValue>,
    /// Absent when the result type has no JSON form; the diagnostics say why.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<SchemaValue>,
    /// The result type in NX spelling, declared or inferred.
    pub result_type: String,
    /// The whole declaration's span in `module`'s source, when the artifact holds that source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declaration: Option<NxTextSpan>,
    pub diagnostics: Vec<NxDiagnostic>,
}

/// One declared parameter of a function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterSchema {
    pub name: String,
    /// The declared type in NX spelling.
    #[serde(rename = "type")]
    pub ty: String,
    /// Whether a caller must supply it: it has neither the `?` mark nor a default.
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The record or union the parameter is declared with, ignoring its `?` mark.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_ref: Option<DeclarationName>,
    /// The listed host-supplied type this parameter matched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_supplied: Option<DeclarationName>,
    /// The listed host-supplied types this parameter's type holds without being one: under an
    /// occurrence, as a field, in a union case, through an alias or as a type argument. In the
    /// order the host listed them. Such a parameter is still in the input schema.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub host_supplied_within: Vec<DeclarationName>,
}

/// The schema of one declared type.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeSchema {
    pub module: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Absent when the type has no JSON form; the diagnostics say why.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<SchemaValue>,
    pub diagnostics: Vec<NxDiagnostic>,
}

/// The code of the diagnostic for a type with no JSON form.
pub const SCHEMA_INEXPRESSIBLE_TYPE: &str = "schema-inexpressible-type";
/// The code of the diagnostic for two shapes at one site that share a `$type`.
pub const SCHEMA_AMBIGUOUS_DISCRIMINATOR: &str = "schema-ambiguous-discriminator";
/// The code of the diagnostic for a reference that names nothing in the program.
pub const SCHEMA_UNKNOWN_DECLARATION: &str = "schema-unknown-declaration";

/// A schema query as the SDKs send it: `{ reference: { module?, name }, options? }`.
///
/// <para>A field the query does not define is refused rather than ignored, so a misspelled option
/// fails loudly instead of describing a different schema.</para>
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SchemaRequest<T> {
    pub reference: DeclarationRef,
    #[serde(default)]
    pub options: T,
}

/// Why a schema query sent as JSON was not answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaQueryError {
    /// The request is not a schema query; the message says why. A caller bug.
    Request(String),
    /// The reference names nothing in the program: a `schema-unknown-declaration` diagnostic.
    Declaration(Vec<NxDiagnostic>),
}

/// Answers a function schema query sent as JSON with the answer as JSON text.
///
/// <para>Every SDK answers through this, so the answers are the same text in each.</para>
pub fn program_artifact_function_schema_json(
    artifact: &ProgramArtifact,
    request: &str,
) -> Result<String, SchemaQueryError> {
    let request: SchemaRequest<FunctionSchemaOptions> = parse_schema_request(request)?;
    let answer = program_artifact_function_schema(artifact, &request.reference, &request.options)
        .map_err(SchemaQueryError::Declaration)?;
    answer_json(&answer)
}

/// Answers a type schema query sent as JSON with the answer as JSON text.
pub fn program_artifact_type_schema_json(
    artifact: &ProgramArtifact,
    request: &str,
) -> Result<String, SchemaQueryError> {
    let request: SchemaRequest<TypeSchemaOptions> = parse_schema_request(request)?;
    let answer = program_artifact_type_schema(artifact, &request.reference, &request.options)
        .map_err(SchemaQueryError::Declaration)?;
    answer_json(&answer)
}

fn parse_schema_request<T: Default + for<'de> Deserialize<'de>>(
    request: &str,
) -> Result<SchemaRequest<T>, SchemaQueryError> {
    serde_json::from_str(request)
        .map_err(|error| SchemaQueryError::Request(format!("Schema request is not valid: {error}")))
}

fn answer_json<T: Serialize>(answer: &T) -> Result<String, SchemaQueryError> {
    serde_json::to_string(answer).map_err(|error| {
        SchemaQueryError::Request(format!("Schema answer could not be serialized: {error}"))
    })
}

/// Answers with the schema of the function `reference` names.
///
/// <para>A type with no JSON form is answered as data: the affected schema is absent and the
/// diagnostics say why. A reference that names no function fails with a
/// `schema-unknown-declaration` diagnostic. The program is not evaluated and the artifact is not
/// changed.</para>
pub fn program_artifact_function_schema(
    artifact: &ProgramArtifact,
    reference: &DeclarationRef,
    options: &FunctionSchemaOptions,
) -> Result<FunctionSchema, Vec<NxDiagnostic>> {
    let program = Program::new(artifact);
    let (module, function) = program.find_function(reference)?;
    let identity = program.modules[module].identity;
    // A type the host lists twice is one listed type, at its first place.
    let mut listed_once = FxHashSet::default();
    let host_supplied = options
        .host_supplied_types
        .iter()
        .filter_map(|reference| program.find_record(reference))
        .filter(|addr| listed_once.insert(*addr))
        .collect::<Vec<_>>();

    let mut input = Document::new(&program, SchemaDirection::Input);
    let mut properties = Vec::new();
    let mut required = Vec::new();
    let mut parameters = Vec::with_capacity(function.params.len());
    for param in &function.params {
        let shape = program.resolve(module, &param.ty, &TypeArgs::NONE);
        let host = program.host_supplied_match(module, &param.ty, &host_supplied);
        // A parameter the host fills in is not looked into: nothing of it is put to a caller.
        let within = match host {
            Some(_) => Vec::new(),
            None => program.host_supplied_within(&shape, &host_supplied),
        };
        parameters.push(ParameterSchema {
            name: param.name.to_string(),
            ty: ast::spell_type_ref(&param.ty),
            required: !param.is_omissible(),
            description: param.doc.as_ref().map(nx_hir::Doc::markdown),
            type_ref: program.declared_type_name(module, &param.ty),
            host_supplied: host.clone(),
            host_supplied_within: within,
        });
        if host.is_some() {
            continue;
        }

        let site = Site {
            member: format!("parameter `{}`", param.name),
            plain: param.name.to_string(),
            label: Some((identity.to_string(), param.span)),
        };
        let mut path = vec![param.name.to_string()];
        let mut schema = input.mention(&shape, &site, &mut path);
        if let Some(doc) = &param.doc {
            schema.push("description", SchemaValue::string(doc.markdown()));
        }
        if let Some(default) = param
            .default
            .and_then(|default| program.literal_default(module, default, &shape))
        {
            schema.push("default", default);
        }
        if !param.is_omissible() {
            required.push(param.name.to_string());
        }
        properties.push((param.name.to_string(), schema));
    }
    let input_root = closed_object(properties, required);
    let (input_schema, input_problems) = input.finish(input_root);

    let (result_shape, result_type) = match &function.return_type {
        Some(ty) => (
            program.resolve(module, ty, &TypeArgs::NONE),
            ast::spell_type_ref(ty),
        ),
        None => match program.inferred_result(module, function) {
            Some(ty) => (program.shape_of_checked(&ty), ty.to_string()),
            None => (
                Shape::NoJsonForm(NoJsonForm::UnresolvedResult),
                "?".to_string(),
            ),
        },
    };
    let result_span = program.modules[module]
        .lowered
        .annotation_span(function.span)
        .unwrap_or(function.span);
    let result_site = Site {
        member: format!("the result of `{}`", function.name),
        plain: result_type.clone(),
        label: Some((identity.to_string(), result_span)),
    };
    let mut output = Document::new(&program, SchemaDirection::Output);
    let mut path = vec![result_type.clone()];
    let output_root = output.mention(&result_shape, &result_site, &mut path);
    let (output_schema, output_problems) = output.finish(output_root);

    let declaration = artifact
        .source_text(identity)
        .map(|source| text_range_to_span(function.span, source, &LineIndex::new(source)));
    Ok(FunctionSchema {
        module: identity.to_string(),
        name: function.name.to_string(),
        description: function.doc.as_ref().map(nx_hir::Doc::markdown),
        summary: function.doc.as_ref().map(nx_hir::Doc::markdown_summary),
        parameters,
        input_schema,
        output_schema,
        result_type,
        declaration,
        diagnostics: problems_to_api(artifact, input_problems.into_iter().chain(output_problems)),
    })
}

/// Answers with the schema of the declared type `reference` names, written for `options`'
/// direction.
///
/// <para>The type may be a record, an action, a union, a type alias, a derived `<Target>.Update`
/// record or a derived `<Target>.Property` union. A component is answered as a type with no JSON
/// form. A reference that names no type fails with a `schema-unknown-declaration`
/// diagnostic.</para>
pub fn program_artifact_type_schema(
    artifact: &ProgramArtifact,
    reference: &DeclarationRef,
    options: &TypeSchemaOptions,
) -> Result<TypeSchema, Vec<NxDiagnostic>> {
    let program = Program::new(artifact);
    let (addr, item) = program.find_type(reference)?;
    let identity = program.modules[addr.module].identity;
    let shape = Shape::Declared(Declared {
        addr,
        args: Vec::new(),
    });
    let site = Site::named(item.name().to_string(), identity, item.span());
    let mut document = Document::new(&program, options.direction);
    let mut path = vec![item.name().to_string()];
    let root = document.mention(&shape, &site, &mut path);
    let (schema, problems) = document.finish(root);
    Ok(TypeSchema {
        module: identity.to_string(),
        name: item.name().to_string(),
        description: item.doc().map(nx_hir::Doc::markdown),
        summary: item.doc().map(nx_hir::Doc::markdown_summary),
        schema,
        diagnostics: problems_to_api(artifact, problems),
    })
}

/// The identities of the modules that declare a concrete subtype of an abstract record a function
/// takes or returns, at any depth, in the program's module order. Only functions of the modules the
/// entry reaches through its imports count: a function of any other module is never linked from
/// the entry, so a runtime cannot call it. The subtypes themselves may be declared in any module.
///
/// <para>A value a host supplies at such a site may name any of those subtypes by `$type`, and the
/// export's schemas list every one of them. A runtime can resolve one only from a module it linked,
/// and it links the modules images reference, which need not include a module that only declares a
/// subtype. The entry image lists these modules in its module table so that a program linked from
/// its entry resolves every subtype its schemas admit.</para>
pub fn program_artifact_boundary_subtype_modules(artifact: &ProgramArtifact) -> Vec<String> {
    let program = Program::new(artifact);
    let callable = program.entry_import_closure();
    let mut reach = BoundaryReach::default();
    for (module, entry) in program.modules.iter().enumerate() {
        if !callable.contains(&module) {
            continue;
        }
        for item in entry.lowered.items() {
            let Item::Function(function) = item else {
                continue;
            };
            for param in &function.params {
                let shape = program.resolve(module, &param.ty, &TypeArgs::NONE);
                reach.visit(&program, &shape);
            }
            let result = match &function.return_type {
                Some(ty) => program.resolve(module, ty, &TypeArgs::NONE),
                None => match program.inferred_result(module, function) {
                    Some(ty) => program.shape_of_checked(&ty),
                    None => continue,
                },
            };
            reach.visit(&program, &result);
        }
    }

    let mut modules = reach
        .abstracts
        .iter()
        .flat_map(|abstract_record| program.descendants(*abstract_record))
        .map(|descendant| match descendant {
            Descendant::Record(addr) | Descendant::Case(addr, _) => addr.module,
        })
        .collect::<Vec<_>>();
    modules.sort_unstable();
    modules.dedup();
    modules
        .into_iter()
        .map(|module| program.modules[module].identity.to_string())
        .collect()
}

/// The abstract records reachable from the types a walk visited.
#[derive(Default)]
struct BoundaryReach {
    visited: FxHashSet<DefId>,
    abstracts: Vec<DeclAddr>,
}

impl BoundaryReach {
    /// Visits every declaration a value of `shape` may hold, through fields, cases, aliases and the
    /// shapes that extend an abstract record.
    fn visit(&mut self, program: &Program<'_>, shape: &Shape) {
        match shape {
            Shape::Seq(item, _) => self.visit(program, item),
            Shape::Declared(declared) => self.visit_declared(program, declared),
            Shape::Case { union, case } => self.visit_case(program, *union, case),
            Shape::Primitive(_) | Shape::Object | Shape::Parameter | Shape::NoJsonForm(_) => {}
        }
    }

    /// Visits a declaration once, whatever its type arguments. An argument reaches the fields
    /// typed by its parameter and nothing else, so visiting each argument on its own finds what an
    /// instantiation reaches, and a record that applies itself to a growing argument, as
    /// `inner?:<Box T=<Box T=T />/>` does, still ends.
    fn visit_declared(&mut self, program: &Program<'_>, declared: &Declared) {
        for (_, arg) in &declared.args {
            self.visit(program, arg);
        }
        let declaration = Declared {
            addr: declared.addr,
            args: Vec::new(),
        };
        if !self.visited.insert(DefId::Declared(declaration)) {
            return;
        }
        let module = declared.addr.module;
        match program.item(declared.addr) {
            Some(Item::TypeAlias(alias)) => {
                let target = program.resolve(module, &alias.ty, &TypeArgs::NONE);
                self.visit(program, &target);
            }
            Some(Item::Record(record)) if record.is_abstract => {
                self.abstracts.push(declared.addr);
                for descendant in program.descendants(declared.addr) {
                    match descendant {
                        Descendant::Record(addr) => self.visit_declared(
                            program,
                            &Declared {
                                addr,
                                args: Vec::new(),
                            },
                        ),
                        Descendant::Case(union, case) => self.visit_case(program, union, &case),
                    }
                }
            }
            Some(Item::Record(record)) => {
                // The arguments were visited above, so a parameter here stands for nothing more.
                let bindings = record
                    .type_params
                    .iter()
                    .map(|param| (param.name.clone(), None))
                    .collect::<Vec<_>>();
                let args = TypeArgs {
                    bindings: &bindings,
                };
                for field in program.record_fields(module, record) {
                    let Some(field_module) = program
                        .by_identity
                        .get(field.module_identity.as_str())
                        .copied()
                    else {
                        continue;
                    };
                    let shape = program.resolve(field_module, &field.ty, &args);
                    self.visit(program, &shape);
                }
            }
            Some(Item::Union(union)) => {
                for case in &union.cases {
                    self.visit_case(program, declared.addr, &case.name);
                }
            }
            _ => {}
        }
    }

    fn visit_case(&mut self, program: &Program<'_>, union_addr: DeclAddr, case_name: &Name) {
        if !self
            .visited
            .insert(DefId::Case(union_addr, case_name.clone()))
        {
            return;
        }
        let Some(Item::Union(union)) = program.item(union_addr) else {
            return;
        };
        let Some(case) = union.cases.iter().find(|case| case.name == *case_name) else {
            return;
        };
        for field in program.case_fields(union_addr.module, union, case) {
            let Some(field_module) = program
                .by_identity
                .get(field.module_identity.as_str())
                .copied()
            else {
                continue;
            };
            let shape = program.resolve(field_module, &field.ty, &TypeArgs::NONE);
            self.visit(program, &shape);
        }
    }
}

/// One module of the program, with its checked artifact.
struct Module<'a> {
    identity: &'a str,
    artifact: &'a ModuleArtifact,
    lowered: &'a LoweredModule,
}

/// The modules of a program artifact, addressable by identity.
///
/// <para>The program's module order is each linked library's modules, libraries in the artifact's
/// order (the prelude first), then its root modules in the order the source provider gave them:
/// what a module builds on comes before it. A site typed by an abstract record lists the shapes
/// extending it in this order.</para>
struct Program<'a> {
    artifact: &'a ProgramArtifact,
    modules: Vec<Module<'a>>,
    by_identity: FxHashMap<&'a str, usize>,
}

/// Where one declaration is: a module of the program and the declaration's place in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct DeclAddr {
    module: usize,
    definition: LocalDefinitionId,
}

impl<'a> Program<'a> {
    fn new(artifact: &'a ProgramArtifact) -> Self {
        let mut program = Self {
            artifact,
            modules: Vec::new(),
            by_identity: FxHashMap::default(),
        };
        let candidates = artifact
            .libraries
            .iter()
            .flat_map(|library| library.modules.iter())
            .chain(artifact.root_modules.iter());
        for module in candidates {
            let Some(lowered) = module.lowered_module.as_deref() else {
                continue;
            };
            let identity = module.file_name.as_str();
            if program.by_identity.contains_key(identity) {
                continue;
            }
            program.by_identity.insert(identity, program.modules.len());
            program.modules.push(Module {
                identity,
                artifact: module,
                lowered,
            });
        }
        program
    }

    /// The modules the entry reaches through its imports, directly or transitively, the entry
    /// included. A module that has no resolved counterpart contributes none of its own imports.
    fn entry_import_closure(&self) -> FxHashSet<usize> {
        let resolved = &self.artifact.resolved_program;
        let mut reached = FxHashSet::default();
        let mut pending = self
            .by_identity
            .get(self.artifact.entry_identity.as_str())
            .copied()
            .into_iter()
            .collect::<Vec<_>>();
        while let Some(module) = pending.pop() {
            if !reached.insert(module) {
                continue;
            }
            let Some(resolved_module) =
                resolved.module_by_prepared_identity(self.modules[module].identity)
            else {
                continue;
            };
            let imported = resolved
                .imported_items(resolved_module.id)
                .into_iter()
                .flat_map(|items| items.values())
                .filter_map(|item| resolved.module(item.module_id))
                .filter_map(|target| {
                    self.by_identity
                        .get(target.prepared_module_identity().as_str())
                        .copied()
                });
            pending.extend(imported);
        }
        reached
    }

    fn prepared(&self, module: usize) -> Option<&'a PreparedModule> {
        self.modules[module].artifact.prepared_module.as_deref()
    }

    fn item(&self, addr: DeclAddr) -> Option<&'a Item> {
        self.modules[addr.module]
            .lowered
            .item_by_definition(addr.definition)
    }

    fn origin(&self, addr: DeclAddr) -> DeclaringOrigin {
        DeclaringOrigin::new(self.modules[addr.module].identity, addr.definition)
    }

    fn addr_of(&self, origin: &DeclaringOrigin) -> Option<DeclAddr> {
        Some(DeclAddr {
            module: *self.by_identity.get(origin.module_identity())?,
            definition: origin.definition_id(),
        })
    }

    fn module_named(&self, reference: &DeclarationRef) -> Result<usize, Vec<NxDiagnostic>> {
        let identity = reference
            .module
            .as_deref()
            .unwrap_or(self.artifact.entry_identity.as_str());
        self.by_identity.get(identity).copied().ok_or_else(|| {
            unknown_declaration(format!(
                "The program has no module '{identity}', so it declares no '{}' there",
                reference.name
            ))
        })
    }

    fn find_function(
        &self,
        reference: &DeclarationRef,
    ) -> Result<(usize, &'a Function), Vec<NxDiagnostic>> {
        let module = self.module_named(reference)?;
        self.modules[module]
            .lowered
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Function(function) if function.name.as_str() == reference.name => {
                    Some((module, function))
                }
                _ => None,
            })
            .ok_or_else(|| {
                unknown_declaration(format!(
                    "Module '{}' declares no function '{}'",
                    self.modules[module].identity, reference.name
                ))
            })
    }

    fn find_type(
        &self,
        reference: &DeclarationRef,
    ) -> Result<(DeclAddr, &'a Item), Vec<NxDiagnostic>> {
        let module = self.module_named(reference)?;
        self.modules[module]
            .lowered
            .items()
            .iter()
            .enumerate()
            .find(|(_, item)| {
                item.name().as_str() == reference.name
                    && matches!(
                        item,
                        Item::Record(_) | Item::Union(_) | Item::TypeAlias(_) | Item::Component(_)
                    )
            })
            .map(|(index, item)| {
                (
                    DeclAddr {
                        module,
                        definition: LocalDefinitionId::new(index as u32),
                    },
                    item,
                )
            })
            .ok_or_else(|| {
                unknown_declaration(format!(
                    "Module '{}' declares no type '{}'",
                    self.modules[module].identity, reference.name
                ))
            })
    }

    /// The record a host-supplied type names, or `None` when the program declares no such record.
    fn find_record(&self, reference: &DeclarationRef) -> Option<DeclAddr> {
        let (addr, item) = self.find_type(reference).ok()?;
        matches!(item, Item::Record(_)).then_some(addr)
    }

    /// The declaration a type name written in `module` reaches, in that module's namespace.
    fn resolve_name(&self, module: usize, name: &Name) -> Option<DeclAddr> {
        let Some(prepared) = self.prepared(module) else {
            let (definition, _) = self.modules[module]
                .lowered
                .find_item_with_definition(name.as_str())?;
            return Some(DeclAddr { module, definition });
        };
        [PreparedNamespace::Type, PreparedNamespace::Element]
            .into_iter()
            .find_map(|namespace| prepared.resolve_binding(namespace, name))
            .and_then(|binding| {
                self.addr_of(&DeclaringOrigin::new(
                    binding.module_identity(prepared.module_identity()),
                    binding.definition_id(),
                ))
            })
    }

    /// The type a reference written in `module` denotes, with `args` standing in for the type
    /// parameters of the record whose field wrote it.
    fn resolve(&self, module: usize, ty: &TypeRef, args: &TypeArgs<'_>) -> Shape {
        match ty {
            TypeRef::Name(name) => {
                if let Some(bound) = args.get(name) {
                    return bound;
                }
                if let Some(shape) = primitive_shape(name.as_str()) {
                    return shape;
                }
                if nx_syntax::BUILTIN_TYPE_NAMES.contains(&name.as_str()) {
                    return Shape::NoJsonForm(NoJsonForm::Markup(name.to_string()));
                }
                match self.resolve_name(module, name) {
                    Some(addr) => Shape::Declared(Declared {
                        addr,
                        args: Vec::new(),
                    }),
                    None => Shape::NoJsonForm(NoJsonForm::Unresolved(name.to_string())),
                }
            }
            TypeRef::Applied { name, args: given } => match self.resolve_name(module, name) {
                Some(addr) => Shape::Declared(Declared {
                    addr,
                    args: given
                        .iter()
                        .map(|(param, arg)| (param.clone(), self.resolve(module, arg, args)))
                        .collect(),
                }),
                None => Shape::NoJsonForm(NoJsonForm::Unresolved(ast::spell_type_ref(ty))),
            },
            TypeRef::Seq { inner, occ } => {
                Shape::Seq(Box::new(self.resolve(module, inner, args)), *occ)
            }
            TypeRef::Function { .. } => {
                Shape::NoJsonForm(NoJsonForm::FunctionType(ast::spell_type_ref(ty)))
            }
            TypeRef::AnyFunction { .. } => {
                Shape::NoJsonForm(NoJsonForm::FunctionReference(ast::spell_type_ref(ty)))
            }
        }
    }

    /// The type a checked type denotes, for a result the checker inferred.
    fn shape_of_checked(&self, ty: &Type) -> Shape {
        match ty {
            Type::Primitive(Primitive::Never) => Shape::NoJsonForm(NoJsonForm::UnresolvedResult),
            Type::Primitive(primitive) => Shape::Primitive(*primitive),
            Type::Seq { item, occ } => Shape::Seq(Box::new(self.shape_of_checked(item)), *occ),
            Type::Function { .. } => Shape::NoJsonForm(NoJsonForm::FunctionType(ty.to_string())),
            Type::AnyFunction { .. } => {
                Shape::NoJsonForm(NoJsonForm::FunctionReference(ty.to_string()))
            }
            Type::Named(named) => match named.origin().and_then(|origin| self.addr_of(origin)) {
                Some(addr) => Shape::Declared(Declared {
                    addr,
                    args: named
                        .args()
                        .iter()
                        .map(|(param, arg)| (param.clone(), self.shape_of_checked(arg)))
                        .collect(),
                }),
                None if nx_types::is_object_type(ty) => Shape::Object,
                // A nominal type that reaches no declaration is markup: an element such as `div`,
                // or the built-in `Element`.
                None => Shape::NoJsonForm(NoJsonForm::Markup(ty.to_string())),
            },
            Type::Union(union) => match union.origin().and_then(|origin| self.addr_of(origin)) {
                Some(addr) => Shape::Declared(Declared {
                    addr,
                    args: Vec::new(),
                }),
                None => Shape::NoJsonForm(NoJsonForm::Unresolved(ty.to_string())),
            },
            Type::UnionCase(case) => match case.origin().and_then(|origin| self.addr_of(origin)) {
                Some(union) => Shape::Case {
                    union,
                    case: case.case.clone(),
                },
                None => Shape::NoJsonForm(NoJsonForm::Unresolved(ty.to_string())),
            },
            Type::Parameter(_) => Shape::Object,
            Type::Variable(_) | Type::ContextualName(_) | Type::Unknown | Type::Error => {
                Shape::NoJsonForm(NoJsonForm::UnresolvedResult)
            }
        }
    }

    /// The result type the checker gave a function that declares none.
    fn inferred_result(&self, module: usize, function: &Function) -> Option<Type> {
        let env = &self.modules[module].artifact.type_env;
        env.lookup(&function.name)
            .and_then(Type::function_parts)
            .map(|(_, result)| result.clone())
            .or_else(|| env.get_expr_type(function.body).cloned())
    }

    /// The record or union a parameter is declared with, apart from its `?` mark: the one it
    /// names, or the one an alias it names denotes.
    fn declared_type_name(&self, module: usize, ty: &TypeRef) -> Option<DeclarationName> {
        let addr = self.denoted_addr(module, ty)?;
        match self.item(addr)? {
            item @ (Item::Record(_) | Item::Union(_)) => Some(DeclarationName {
                module: self.modules[addr.module].identity.to_string(),
                name: item.name().to_string(),
            }),
            _ => None,
        }
    }

    fn declared_addr(&self, module: usize, ty: &TypeRef) -> Option<DeclAddr> {
        match ty {
            TypeRef::Name(name) | TypeRef::Applied { name, .. } => self.resolve_name(module, name),
            _ => None,
        }
    }

    /// The declaration a type written in `module` denotes: the one it names, or, when that is a
    /// type alias, the one the alias's target names, through any number of aliases. An alias of
    /// anything but a name, an occurrence for one, denotes no declaration.
    fn denoted_addr(&self, module: usize, ty: &TypeRef) -> Option<DeclAddr> {
        let mut addr = self.declared_addr(module, ty)?;
        for _ in 0..MAX_ALIAS_DEPTH {
            let Some(Item::TypeAlias(alias)) = self.item(addr) else {
                return Some(addr);
            };
            addr = self.declared_addr(addr.module, &alias.ty)?;
        }
        // Aliases that go round in a circle denote nothing.
        None
    }

    /// The listed host-supplied type a parameter declared `ty` in `module` is, or extends.
    fn host_supplied_match(
        &self,
        module: usize,
        ty: &TypeRef,
        listed: &[DeclAddr],
    ) -> Option<DeclarationName> {
        if listed.is_empty() {
            return None;
        }
        let addr = self.denoted_addr(module, ty)?;
        let Item::Record(record) = self.item(addr)? else {
            return None;
        };
        // A record that is two of the listed types is filled in as the first of them.
        let place = *self.listed_record(addr, record, listed).first()?;
        self.declaration_name(listed[place])
    }

    fn declaration_name(&self, addr: DeclAddr) -> Option<DeclarationName> {
        Some(DeclarationName {
            module: self.modules[addr.module].identity.to_string(),
            name: self.item(addr)?.name().to_string(),
        })
    }

    /// Which of the `listed` types the record at `addr` is, or extends: their places in the list.
    /// A record is more than one of them when one listed type extends another.
    fn listed_record(&self, addr: DeclAddr, record: &RecordDef, listed: &[DeclAddr]) -> Vec<usize> {
        let shape = self
            .prepared(addr.module)
            .and_then(|prepared| nx_hir::effective_record_shape(prepared, record).ok());
        listed
            .iter()
            .enumerate()
            .filter(|(_, candidate)| {
                **candidate == addr
                    || shape
                        .as_ref()
                        .is_some_and(|shape| self.descends_from_listed(shape, **candidate))
            })
            .map(|(place, _)| place)
            .collect()
    }

    /// Which of the `listed` types the base of the union declared in `module` is, or extends:
    /// their places in the list. A union's cases carry its base's fields, so a value of the union
    /// is one of the base.
    fn listed_union_base(
        &self,
        module: usize,
        union: &UnionDef,
        listed: &[DeclAddr],
    ) -> Vec<usize> {
        let shape = union.base.as_ref().and_then(|base| {
            nx_hir::effective_record_shape_for_name(self.prepared(module)?, base)
                .ok()
                .flatten()
        });
        let Some(shape) = shape else {
            return Vec::new();
        };
        listed
            .iter()
            .enumerate()
            .filter(|(_, candidate)| self.descends_from_listed(&shape, **candidate))
            .map(|(place, _)| place)
            .collect()
    }

    fn descends_from_listed(
        &self,
        shape: &nx_hir::EffectiveRecordShape,
        candidate: DeclAddr,
    ) -> bool {
        let Some(item) = self.item(candidate) else {
            return false;
        };
        shape.descends_from(item.name(), Some(&self.origin(candidate)))
    }

    /// The `listed` host-supplied types that a value of `shape` holds, in the order they are
    /// listed and each once: a listed type, or a record that extends one, reached at any depth
    /// under an occurrence, as a field of a record or of a union case, through a type alias or as
    /// a type argument.
    ///
    /// <para>This is a walk of its own and not a by-product of writing the schema. The schema
    /// writer describes a declaration once and refers to it afterwards, so a second parameter of
    /// one record would never be looked into.</para>
    fn host_supplied_within(&self, shape: &Shape, listed: &[DeclAddr]) -> Vec<DeclarationName> {
        if listed.is_empty() {
            return Vec::new();
        }
        let mut walk = HeldWalk {
            program: self,
            listed,
            held: vec![false; listed.len()],
            entered: FxHashSet::default(),
        };
        walk.shape(shape);
        listed
            .iter()
            .zip(walk.held)
            .filter(|(_, held)| *held)
            .filter_map(|(addr, _)| self.declaration_name(*addr))
            .collect()
    }

    /// The effective fields of `record`, declared in `module`: inherited fields first.
    fn record_fields(&self, module: usize, record: &RecordDef) -> Vec<EffectiveField> {
        self.prepared(module)
            .and_then(|prepared| nx_hir::effective_record_shape(prepared, record).ok())
            .map(|shape| shape.fields)
            .unwrap_or_else(|| {
                record
                    .properties
                    .iter()
                    .cloned()
                    .map(|field| {
                        EffectiveField::from_record_field(field, self.modules[module].identity)
                    })
                    .collect()
            })
    }

    /// The fields one case of a union declared in `module` carries: the union base's, then its own.
    fn case_fields(
        &self,
        module: usize,
        union: &UnionDef,
        case: &UnionCaseDef,
    ) -> Vec<EffectiveField> {
        let mut fields = union
            .base
            .as_ref()
            .and_then(|base| {
                let prepared = self.prepared(module)?;
                nx_hir::effective_record_shape_for_name(prepared, base)
                    .ok()
                    .flatten()
            })
            .map(|shape| shape.fields)
            .unwrap_or_default();
        fields.extend(case.fields.iter().map(|field| {
            EffectiveField::from_record_field(
                RecordField {
                    name: field.name.clone(),
                    ty: field.ty.clone(),
                    is_content: field.is_content,
                    optional: field.optional,
                    default: field.default,
                    doc: field.doc.clone(),
                    span: field.span,
                },
                self.modules[module].identity,
            )
        }));
        fields
    }

    /// Every shape a value at a site typed by the abstract record at `target` may take, in the
    /// program's module order and declaration order within a module.
    fn descendants(&self, target: DeclAddr) -> Vec<Descendant> {
        let origin = self.origin(target);
        let name = match self.item(target) {
            Some(item) => item.name().clone(),
            None => return Vec::new(),
        };
        let mut found = Vec::new();
        for (module, entry) in self.modules.iter().enumerate() {
            let Some(prepared) = self.prepared(module) else {
                continue;
            };
            for (index, item) in entry.lowered.items().iter().enumerate() {
                let definition = LocalDefinitionId::new(index as u32);
                match item {
                    Item::Record(record)
                        if !record.is_abstract && record.update_target().is_none() =>
                    {
                        let extends = nx_hir::effective_record_shape(prepared, record)
                            .is_ok_and(|shape| shape.descends_from(&name, Some(&origin)));
                        if extends {
                            found.push(Descendant::Record(DeclAddr { module, definition }));
                        }
                    }
                    Item::Union(union) => {
                        let Some(base) = union.base.as_ref() else {
                            continue;
                        };
                        let extends = nx_hir::effective_record_shape_for_name(prepared, base)
                            .ok()
                            .flatten()
                            .is_some_and(|shape| shape.descends_from(&name, Some(&origin)));
                        if extends {
                            for case in &union.cases {
                                found.push(Descendant::Case(
                                    DeclAddr { module, definition },
                                    case.name.clone(),
                                ));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        found
    }

    /// The JSON value a default written as `default` in `module` stands for at a site of `shape`,
    /// when it is a literal or names a constant case of the site's union.
    fn literal_default(
        &self,
        module: usize,
        default: nx_hir::ExprId,
        shape: &Shape,
    ) -> Option<SchemaValue> {
        let lowered = self.modules[module].lowered;
        match lowered.expr(default) {
            Expr::Literal(literal) => Some(literal_value(literal, false)),
            Expr::UnaryOp {
                op: UnOp::Neg,
                expr,
                ..
            } => match lowered.expr(*expr) {
                Expr::Literal(literal @ (Literal::Int(_) | Literal::Int32(_)))
                | Expr::Literal(literal @ (Literal::Float(_) | Literal::Float32(_))) => {
                    Some(literal_value(literal, true))
                }
                _ => None,
            },
            // `Status.active` is a constant case only when `Status` names the site's union;
            // `c.paused` reads a field, whatever its name.
            Expr::Member { base, member, .. } => match lowered.expr(*base) {
                Expr::Ident(union_name) => {
                    let (union, _) = self.site_union(shape)?;
                    (self.resolve_name(module, union_name) == Some(union))
                        .then(|| self.constant_case_default(shape, member))
                        .flatten()
                }
                _ => None,
            },
            Expr::ResolvedUnionCase { case, .. } => self.constant_case_default(shape, case),
            _ => None,
        }
    }

    fn qualified_default(&self, default: &QualifiedExprRef, shape: &Shape) -> Option<SchemaValue> {
        let module = *self.by_identity.get(default.module_identity.as_str())?;
        self.literal_default(module, default.expr_id, shape)
    }

    /// The union a site of `shape` is typed by, through aliases.
    fn site_union(&self, shape: &Shape) -> Option<(DeclAddr, &'a UnionDef)> {
        let Shape::Declared(declared) = self.through_aliases(shape) else {
            return None;
        };
        match self.item(declared.addr)? {
            Item::Union(union) => Some((declared.addr, union)),
            _ => None,
        }
    }

    fn constant_case_default(&self, shape: &Shape, case: &Name) -> Option<SchemaValue> {
        let (_, union) = self.site_union(shape)?;
        union
            .cases
            .iter()
            .any(|candidate| candidate.name == *case && union.is_constant_case(candidate))
            .then(|| SchemaValue::string(case.as_str()))
    }

    /// `shape` with every type alias it names replaced by the alias's target.
    fn through_aliases(&self, shape: &Shape) -> Shape {
        let mut current = shape.clone();
        for _ in 0..MAX_ALIAS_DEPTH {
            let Shape::Declared(declared) = &current else {
                break;
            };
            let Some(Item::TypeAlias(alias)) = self.item(declared.addr) else {
                break;
            };
            current = self.resolve(declared.addr.module, &alias.ty, &TypeArgs::NONE);
        }
        current
    }

    /// The `$defs` key a declaration takes when nothing else has taken it.
    fn natural_key(&self, declared: &Declared) -> String {
        let Some(item) = self.item(declared.addr) else {
            return "unknown".to_string();
        };
        let mut key = item.name().to_string();
        if let Item::Record(record) = item {
            for param in &record.type_params {
                if let Some((_, arg)) = declared.args.iter().find(|(name, _)| *name == param.name) {
                    key.push('_');
                    key.push_str(&self.argument_key(arg));
                }
            }
        }
        key
    }

    fn argument_key(&self, shape: &Shape) -> String {
        match shape {
            Shape::Primitive(primitive) => primitive.as_str().to_string(),
            Shape::Object | Shape::Parameter => "object".to_string(),
            Shape::Seq(item, _) => self.argument_key(item),
            Shape::Declared(declared) => self.natural_key(declared),
            Shape::Case { union, case } => match self.item(*union) {
                Some(item) => format!("{}.{}", item.name(), case),
                None => case.to_string(),
            },
            Shape::NoJsonForm(_) => "unknown".to_string(),
        }
    }
}

/// How deep an alias chain is followed before the walk gives up on it. The checker rejects alias
/// cycles, so this only bounds a program that failed to check.
const MAX_ALIAS_DEPTH: usize = 64;

/// A type as the export reads it: every name resolved to the declaration it reaches.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Shape {
    Primitive(Primitive),
    /// `object`, the top type.
    Object,
    Seq(Box<Shape>, Occurrence),
    Declared(Declared),
    /// One case of a union, which only an inferred result names.
    Case {
        union: DeclAddr,
        case: Name,
    },
    /// A type parameter of a generic record named with no arguments; a runtime reads it as
    /// `object`.
    Parameter,
    NoJsonForm(NoJsonForm),
}

/// A record, union, alias or component, with the type arguments an applied type gave it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Declared {
    addr: DeclAddr,
    args: Vec<(Name, Shape)>,
}

/// A type that has no JSON form whatever it is mentioned in.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NoJsonForm {
    /// A function type, spelled as written.
    FunctionType(String),
    /// The function reference type `<function ... />: R`, spelled as written.
    FunctionReference(String),
    /// A name that reaches no declaration; analysis has already reported it.
    Unresolved(String),
    /// Markup: an element, or the built-in `Element`, spelled as the checker shows it.
    Markup(String),
    /// A result type the checker could not infer.
    UnresolvedResult,
}

/// One walk over a type for the listed host-supplied types it holds.
struct HeldWalk<'w, 'p, 'a> {
    program: &'p Program<'a>,
    listed: &'w [DeclAddr],
    /// Which of `listed` were found, by place.
    held: Vec<bool>,
    /// The declarations and union cases already looked into, so a type that holds itself ends.
    entered: FxHashSet<(DeclAddr, Option<Name>)>,
}

impl HeldWalk<'_, '_, '_> {
    /// Looks into `shape`. Every form a type can take is named here, with no catch-all: a form
    /// added to the language does not compile until this says whether it can hold another type.
    /// One that wraps or narrows another type is looked through.
    fn shape(&mut self, shape: &Shape) {
        match shape {
            // These hold no declared type: a primitive, the open `object`, an unbound type
            // parameter, which a runtime reads as `object`, and a type with no JSON form, which
            // no caller can supply.
            Shape::Primitive(_) | Shape::Object | Shape::Parameter | Shape::NoJsonForm(_) => {}
            Shape::Seq(item, _) => self.shape(item),
            Shape::Case { union, case } => self.case(*union, case),
            Shape::Declared(declared) => {
                // A type argument is looked into where it is written. The declaration is then
                // looked into once, with its parameters unbound, however it is applied: that ends
                // on a record that applies itself to an ever larger argument, and misses nothing,
                // since a field typed by a parameter holds only what the argument does.
                for (_, argument) in &declared.args {
                    self.shape(argument);
                }
                self.declaration(declared.addr);
            }
        }
    }

    fn declaration(&mut self, addr: DeclAddr) {
        if !self.entered.insert((addr, None)) {
            return;
        }
        let program = self.program;
        let Some(item) = program.item(addr) else {
            return;
        };
        match item {
            Item::TypeAlias(alias) => {
                let target = program.resolve(addr.module, &alias.ty, &TypeArgs::NONE);
                self.shape(&target);
            }
            Item::Record(record) => {
                for place in program.listed_record(addr, record, self.listed) {
                    self.held[place] = true;
                }
                if record.is_abstract {
                    // A value at an abstract record is one of the shapes that extend it.
                    for descendant in program.descendants(addr) {
                        match descendant {
                            Descendant::Record(record) => self.declaration(record),
                            Descendant::Case(union, case) => self.case(union, &case),
                        }
                    }
                }
                let bindings = record
                    .type_params
                    .iter()
                    .map(|param| (param.name.clone(), None))
                    .collect::<Vec<_>>();
                let fields = program.record_fields(addr.module, record);
                self.fields(&fields, &bindings);
            }
            Item::Union(union) => {
                for place in program.listed_union_base(addr.module, union, self.listed) {
                    self.held[place] = true;
                }
                for case in &union.cases {
                    self.case(addr, &case.name);
                }
            }
            // None of these is a type a parameter's value can have.
            Item::Component(_) | Item::Function(_) | Item::Value(_) => {}
        }
    }

    fn case(&mut self, union_addr: DeclAddr, case_name: &Name) {
        if !self.entered.insert((union_addr, Some(case_name.clone()))) {
            return;
        }
        let program = self.program;
        let Some(Item::Union(union)) = program.item(union_addr) else {
            return;
        };
        for place in program.listed_union_base(union_addr.module, union, self.listed) {
            self.held[place] = true;
        }
        if let Some(case) = union.cases.iter().find(|case| case.name == *case_name) {
            let fields = program.case_fields(union_addr.module, union, case);
            self.fields(&fields, &[]);
        }
    }

    fn fields(&mut self, fields: &[EffectiveField], bindings: &[(Name, Option<Shape>)]) {
        for field in fields {
            let Some(module) = self
                .program
                .by_identity
                .get(field.module_identity.as_str())
                .copied()
            else {
                continue;
            };
            let shape = self
                .program
                .resolve(module, &field.ty, &TypeArgs { bindings });
            self.shape(&shape);
        }
    }
}

/// What stands in for the type parameters of the record whose field is being resolved.
struct TypeArgs<'s> {
    bindings: &'s [(Name, Option<Shape>)],
}

impl TypeArgs<'_> {
    const NONE: TypeArgs<'static> = TypeArgs { bindings: &[] };

    /// The shape a type parameter named `name` stands for: its argument, or `object`-like
    /// openness when the record was named with no arguments.
    fn get(&self, name: &Name) -> Option<Shape> {
        self.bindings
            .iter()
            .find(|(param, _)| param == name)
            .map(|(_, arg)| arg.clone().unwrap_or(Shape::Parameter))
    }
}

/// One shape a value at an abstract-record site may take.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Descendant {
    Record(DeclAddr),
    Case(DeclAddr, Name),
}

/// The member a diagnostic is about and where it was declared.
#[derive(Debug, Clone)]
struct Site {
    /// The member as a sentence names it: ``parameter `teamSize` ``, `` `Row.template` `` or
    /// ``the result of `findPlans` ``.
    member: String,
    /// The member as a path names it, `Row.template`, to tell whether the path says more.
    plain: String,
    label: Option<(String, TextSpan)>,
}

impl Site {
    /// A record field, a payload case field or a declared type, named `plain`.
    fn named(plain: String, module: &str, span: TextSpan) -> Self {
        Self {
            member: format!("`{plain}`"),
            plain,
            label: Some((module.to_string(), span)),
        }
    }

    /// The member at the start of a sentence.
    fn member_label(&self) -> String {
        let mut chars = self.member.chars();
        match chars.next() {
            Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
            None => String::new(),
        }
    }

    /// `` (reached through `Catalog.rows.template`)`` when the path from the schema's root says
    /// more than the member does.
    fn path_note(&self, path: &[String]) -> String {
        let joined = path.join(".");
        if path.len() < 2 || joined == self.plain {
            String::new()
        } else {
            format!(" (reached through `{joined}`)")
        }
    }
}

/// A diagnostic the walk found, before it is placed in the program's sources.
#[derive(Debug, Clone, PartialEq)]
struct Problem {
    code: &'static str,
    message: String,
    help: Option<String>,
    label: Option<(String, TextSpan)>,
}

/// What a `$defs` entry is keyed by: one declaration with one set of type arguments, or one
/// payload case of a union.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum DefId {
    Declared(Declared),
    Case(DeclAddr, Name),
}

struct DefEntry {
    key: String,
    /// Whether the entry is an object that must carry and require `$type`.
    needs_type: bool,
    body: DefBody,
}

enum DefBody {
    /// Reserved at first mention, so a type that mentions itself refers to the entry being built.
    Pending,
    Object(ObjectDef),
    Schema(SchemaValue),
}

/// A record or payload case, kept apart from its JSON until every mention has decided whether it
/// carries `$type`.
struct ObjectDef {
    discriminator: String,
    properties: Vec<(String, SchemaValue)>,
    required: Vec<String>,
    description: Option<String>,
}

/// One schema document being written: its `$defs` table and the problems found on the way.
struct Document<'p, 'a> {
    program: &'p Program<'a>,
    direction: SchemaDirection,
    defs: Vec<DefEntry>,
    def_index: FxHashMap<DefId, usize>,
    keys: FxHashSet<String>,
    problems: Vec<Problem>,
    alias_depth: usize,
}

impl<'p, 'a> Document<'p, 'a> {
    fn new(program: &'p Program<'a>, direction: SchemaDirection) -> Self {
        Self {
            program,
            direction,
            defs: Vec::new(),
            def_index: FxHashMap::default(),
            keys: FxHashSet::default(),
            problems: Vec::new(),
            alias_depth: 0,
        }
    }

    /// The document with `root` at its root, or nothing when a mention had no JSON form; the
    /// problems either way.
    fn finish(self, root: SchemaValue) -> (Option<SchemaValue>, Vec<Problem>) {
        if !self.problems.is_empty() {
            return (None, self.problems);
        }
        let mut entries = vec![(
            "$schema".to_string(),
            SchemaValue::string(JSON_SCHEMA_DIALECT),
        )];
        if let SchemaValue::Object(root) = root {
            entries.extend(root);
        }
        if !self.defs.is_empty() {
            let defs = self
                .defs
                .into_iter()
                .map(|entry| (entry.key, render_def(entry.needs_type, entry.body)))
                .collect();
            entries.push(("$defs".to_string(), SchemaValue::Object(defs)));
        }
        (Some(SchemaValue::Object(entries)), self.problems)
    }

    fn report(&mut self, code: &'static str, message: String, help: Option<&str>, site: &Site) {
        let problem = Problem {
            code,
            message,
            help: help.map(str::to_string),
            label: site.label.clone(),
        };
        if !self.problems.contains(&problem) {
            self.problems.push(problem);
        }
    }

    /// The schema of a value of `shape` at a site whose own type is `shape`.
    fn mention(&mut self, shape: &Shape, site: &Site, path: &mut Vec<String>) -> SchemaValue {
        match shape {
            Shape::Primitive(primitive) => primitive_schema(*primitive),
            Shape::Object | Shape::Parameter => any_value(),
            Shape::Seq(item, occ) => {
                let item = self.mention(item, site, path);
                if occ.admits_many() {
                    let mut array = SchemaValue::object(vec![
                        ("type", SchemaValue::string("array")),
                        ("items", item),
                    ]);
                    if !occ.admits_zero() {
                        array.push("minItems", SchemaValue::Int(1));
                    }
                    array
                } else {
                    nullable(item)
                }
            }
            Shape::NoJsonForm(form) => {
                self.no_json_form(form, site, path);
                SchemaValue::Object(Vec::new())
            }
            Shape::Case { union, case } => self.case_mention(*union, case, site, path),
            Shape::Declared(declared) => self.declared_mention(declared, site, path),
        }
    }

    fn no_json_form(&mut self, form: &NoJsonForm, site: &Site, path: &[String]) {
        let through = site.path_note(path);
        match form {
            NoJsonForm::FunctionType(spelling) => self.report(
                SCHEMA_INEXPRESSIBLE_TYPE,
                format!(
                    "{} has the function type `{spelling}`, which has no JSON form{through}",
                    site.member_label()
                ),
                Some("A function value names code; a host supplies it, not a JSON value."),
                site,
            ),
            NoJsonForm::FunctionReference(spelling) => self.report(
                SCHEMA_INEXPRESSIBLE_TYPE,
                format!(
                    "{} has the function reference type `{spelling}`, which has no JSON form{through}",
                    site.member_label()
                ),
                Some("A function value names code; a host supplies it, not a JSON value."),
                site,
            ),
            NoJsonForm::Unresolved(spelling) => self.report(
                SCHEMA_INEXPRESSIBLE_TYPE,
                format!(
                    "{} names the type `{spelling}`, which does not resolve, so it has no JSON form{through}",
                    site.member_label()
                ),
                None,
                site,
            ),
            NoJsonForm::Markup(spelling) => self.report(
                SCHEMA_INEXPRESSIBLE_TYPE,
                format!(
                    "{} is markup, `{spelling}`, which has no JSON form{through}",
                    site.member_label()
                ),
                Some("Markup is rendered by a host, not supplied or read as a JSON value."),
                site,
            ),
            NoJsonForm::UnresolvedResult => self.report(
                SCHEMA_INEXPRESSIBLE_TYPE,
                format!(
                    "The type of {} could not be inferred, so it has no JSON form",
                    site.member
                ),
                Some("Annotate the result type."),
                site,
            ),
        }
    }

    fn declared_mention(
        &mut self,
        declared: &Declared,
        site: &Site,
        path: &mut Vec<String>,
    ) -> SchemaValue {
        let Some(item) = self.program.item(declared.addr) else {
            self.no_json_form(&NoJsonForm::Unresolved("?".to_string()), site, path);
            return SchemaValue::Object(Vec::new());
        };
        match item {
            Item::TypeAlias(alias) => {
                if self.alias_depth >= MAX_ALIAS_DEPTH {
                    self.no_json_form(&NoJsonForm::Unresolved(alias.name.to_string()), site, path);
                    return SchemaValue::Object(Vec::new());
                }
                let target = self
                    .program
                    .resolve(declared.addr.module, &alias.ty, &TypeArgs::NONE);
                self.alias_depth += 1;
                let schema = self.mention(&target, site, path);
                self.alias_depth -= 1;
                schema
            }
            Item::Record(record) if record.is_abstract => {
                self.abstract_mention(declared.addr, record, site, path)
            }
            Item::Record(_) => {
                let index = self.record_def(declared, path);
                if self.direction == SchemaDirection::Output {
                    self.defs[index].needs_type = true;
                }
                self.reference(index)
            }
            Item::Union(union) => {
                let index = self.union_def(declared.addr, union, path);
                self.reference(index)
            }
            Item::Component(component) => {
                let through = site.path_note(path);
                self.report(
                    SCHEMA_INEXPRESSIBLE_TYPE,
                    format!(
                        "{} is typed by the component `{}`, which has no JSON form{through}",
                        site.member_label(),
                        component.name
                    ),
                    None,
                    site,
                );
                SchemaValue::Object(Vec::new())
            }
            Item::Function(_) | Item::Value(_) => {
                self.no_json_form(&NoJsonForm::Unresolved(item.name().to_string()), site, path);
                SchemaValue::Object(Vec::new())
            }
        }
    }

    fn case_mention(
        &mut self,
        union_addr: DeclAddr,
        case_name: &Name,
        site: &Site,
        path: &mut Vec<String>,
    ) -> SchemaValue {
        let Some(Item::Union(union)) = self.program.item(union_addr) else {
            self.no_json_form(&NoJsonForm::Unresolved(case_name.to_string()), site, path);
            return SchemaValue::Object(Vec::new());
        };
        let Some(case) = union.cases.iter().find(|case| case.name == *case_name) else {
            self.no_json_form(&NoJsonForm::Unresolved(case_name.to_string()), site, path);
            return SchemaValue::Object(Vec::new());
        };
        if union.is_constant_case(case) {
            return string_enum(vec![case.name.to_string()]);
        }
        let index = self.case_def(union_addr, union, case, path);
        self.reference(index)
    }

    /// `anyOf` every shape that extends the abstract record at `addr`, each requiring `$type`.
    fn abstract_mention(
        &mut self,
        addr: DeclAddr,
        record: &RecordDef,
        site: &Site,
        path: &mut Vec<String>,
    ) -> SchemaValue {
        let descendants = self.program.descendants(addr);
        if descendants.is_empty() {
            let through = site.path_note(path);
            self.report(
                SCHEMA_INEXPRESSIBLE_TYPE,
                format!(
                    "{} is typed by the abstract record `{}`, which nothing in the program extends, so no value has it{through}",
                    site.member_label(),
                    record.name
                ),
                None,
                site,
            );
            return SchemaValue::Object(Vec::new());
        }

        let discriminators = descendants
            .iter()
            .map(|descendant| self.discriminator(descendant))
            .collect::<Vec<_>>();
        if let Some(shared) = discriminators
            .iter()
            .enumerate()
            .find(|(index, name)| discriminators[..*index].contains(name))
            .map(|(_, name)| name.clone())
        {
            self.report(
                SCHEMA_AMBIGUOUS_DISCRIMINATOR,
                format!(
                    "{} is typed by the abstract record `{}`, and two shapes in the program that extend it share the discriminator `{shared}`",
                    site.member_label(),
                    record.name
                ),
                Some("A runtime refuses such a value as ambiguous; rename one of the records."),
                site,
            );
            return SchemaValue::Object(Vec::new());
        }

        let mut branches = Vec::with_capacity(descendants.len());
        for (descendant, discriminator) in descendants.into_iter().zip(discriminators) {
            // The path names the shape it went through: `Tool(FunctionTool).function`.
            let segment = path.pop().unwrap_or_default();
            path.push(format!("{segment}({discriminator})"));
            let index = match &descendant {
                Descendant::Record(addr) => self.record_def(
                    &Declared {
                        addr: *addr,
                        args: Vec::new(),
                    },
                    path,
                ),
                Descendant::Case(union_addr, case_name) => {
                    let case = match self.program.item(*union_addr) {
                        Some(Item::Union(union)) => union
                            .cases
                            .iter()
                            .find(|case| case.name == *case_name)
                            .map(|case| (union, case)),
                        _ => None,
                    };
                    match case {
                        Some((union, case)) => self.case_def(*union_addr, union, case, path),
                        None => {
                            path.pop();
                            path.push(segment);
                            continue;
                        }
                    }
                }
            };
            path.pop();
            path.push(segment);
            self.defs[index].needs_type = true;
            branches.push(self.reference(index));
        }
        SchemaValue::object(vec![("anyOf", SchemaValue::Array(branches))])
    }

    fn discriminator(&self, descendant: &Descendant) -> String {
        match descendant {
            Descendant::Record(addr) => self
                .program
                .item(*addr)
                .map(|item| item.name().to_string())
                .unwrap_or_default(),
            Descendant::Case(union, case) => match self.program.item(*union) {
                Some(item) => format!("{}.{case}", item.name()),
                None => case.to_string(),
            },
        }
    }

    fn reference(&self, index: usize) -> SchemaValue {
        SchemaValue::object(vec![(
            "$ref",
            SchemaValue::string(format!("#/$defs/{}", self.defs[index].key)),
        )])
    }

    /// The index of the entry `id` has, reserving one keyed `natural` when it has none yet.
    /// `true` with the index means the entry is new and its body still has to be built.
    fn entry(&mut self, id: DefId, natural: &str) -> (usize, bool) {
        if let Some(index) = self.def_index.get(&id) {
            return (*index, false);
        }
        let natural = def_key(natural);
        let mut key = natural.clone();
        let mut suffix = 2;
        while self.keys.contains(&key) {
            key = format!("{natural}_{suffix}");
            suffix += 1;
        }
        self.keys.insert(key.clone());
        let index = self.defs.len();
        self.defs.push(DefEntry {
            key,
            needs_type: false,
            body: DefBody::Pending,
        });
        self.def_index.insert(id, index);
        (index, true)
    }

    /// The entry of a concrete record, an action or an update record, with its arguments.
    fn record_def(&mut self, declared: &Declared, path: &mut Vec<String>) -> usize {
        // A record that applies itself to a growing argument, `inner?:<Box T=<Box T=T />/>`, or
        // grows it through another record, `inner?:<Box T=<Pair L=T R=T />/>`, would need one entry
        // per depth. Once its arguments apply the record twice, or nest deeper than any written type
        // plausibly does, it is described as the record named with no arguments, whose parameter
        // fields take the `object` schema: what a runtime checks there, since NX IR erases a
        // record's type arguments. The depth bound alone guarantees the walk ends, since a program
        // has finitely many shapes of bounded depth.
        let unapplied;
        let declared = if declared
            .args
            .iter()
            .map(|(_, arg)| applications_of(declared.addr, arg))
            .sum::<usize>()
            >= 2
            || declared
                .args
                .iter()
                .any(|(_, arg)| argument_depth(arg) > MAX_TYPE_ARGUMENT_DEPTH)
        {
            unapplied = Declared {
                addr: declared.addr,
                args: Vec::new(),
            };
            &unapplied
        } else {
            declared
        };
        let natural = self.program.natural_key(declared);
        let (index, new) = self.entry(DefId::Declared(declared.clone()), &natural);
        if !new {
            return index;
        }
        let Some(Item::Record(record)) = self.program.item(declared.addr) else {
            self.defs[index].body = DefBody::Schema(SchemaValue::Object(Vec::new()));
            return index;
        };
        let module = declared.addr.module;
        let bindings = record
            .type_params
            .iter()
            .map(|param| {
                let arg = declared
                    .args
                    .iter()
                    .find(|(name, _)| *name == param.name)
                    .map(|(_, arg)| arg.clone());
                (param.name.clone(), arg)
            })
            .collect::<Vec<_>>();
        let fields = self.program.record_fields(module, record);
        let update = record.update_target().is_some();
        let object = self.object_def(
            record.name.as_str(),
            &fields,
            &TypeArgs {
                bindings: &bindings,
            },
            update,
            record.doc.as_ref().map(nx_hir::Doc::markdown),
            path,
        );
        self.defs[index].body = DefBody::Object(object);
        index
    }

    /// The entry of a union: a string `enum` of its constant cases, or `anyOf` those and a
    /// reference to each payload case.
    fn union_def(&mut self, addr: DeclAddr, union: &UnionDef, path: &mut Vec<String>) -> usize {
        let id = DefId::Declared(Declared {
            addr,
            args: Vec::new(),
        });
        let (index, new) = self.entry(id, union.name.as_str());
        if !new {
            return index;
        }
        let constants = union
            .cases
            .iter()
            .filter(|case| union.is_constant_case(case))
            .map(|case| case.name.to_string())
            .collect::<Vec<_>>();
        let mut schema = if constants.len() == union.cases.len() {
            string_enum(constants)
        } else {
            let mut branches = Vec::new();
            if !constants.is_empty() {
                branches.push(string_enum(constants));
            }
            for case in union
                .cases
                .iter()
                .filter(|case| !union.is_constant_case(case))
            {
                let case_index = self.case_def(addr, union, case, path);
                branches.push(self.reference(case_index));
            }
            SchemaValue::object(vec![("anyOf", SchemaValue::Array(branches))])
        };
        if let Some(description) = union_description(union) {
            schema.push("description", SchemaValue::string(description));
        }
        self.defs[index].body = DefBody::Schema(schema);
        index
    }

    /// The entry of one payload case, `<Union>.<case>`, which always carries `$type`.
    fn case_def(
        &mut self,
        union_addr: DeclAddr,
        union: &UnionDef,
        case: &UnionCaseDef,
        path: &mut Vec<String>,
    ) -> usize {
        let discriminator = format!("{}.{}", union.name, case.name);
        let (index, new) = self.entry(
            DefId::Case(union_addr, case.name.clone()),
            discriminator.as_str(),
        );
        self.defs[index].needs_type = true;
        if !new {
            return index;
        }
        let fields = self.program.case_fields(union_addr.module, union, case);
        let object = self.object_def(
            &discriminator,
            &fields,
            &TypeArgs::NONE,
            false,
            case.doc.as_ref().map(nx_hir::Doc::markdown),
            path,
        );
        self.defs[index].body = DefBody::Object(object);
        index
    }

    /// The object a record, update record or payload case named `owner` describes, before its
    /// `$type` is decided.
    fn object_def(
        &mut self,
        owner: &str,
        fields: &[EffectiveField],
        args: &TypeArgs<'_>,
        update: bool,
        description: Option<String>,
        path: &mut Vec<String>,
    ) -> ObjectDef {
        let mut properties = Vec::with_capacity(fields.len());
        let mut required = Vec::new();
        for field in fields {
            let module = self
                .program
                .by_identity
                .get(field.module_identity.as_str())
                .copied();
            let shape = match module {
                Some(module) => self.program.resolve(module, &field.ty, args),
                None => Shape::NoJsonForm(NoJsonForm::Unresolved(ast::spell_type_ref(&field.ty))),
            };
            let site = Site::named(
                format!("{owner}.{}", field.name),
                &field.module_identity,
                field.span,
            );
            path.push(field.name.to_string());
            let mut schema = self.mention(&shape, &site, path);
            path.pop();
            if update && field.optional {
                schema = nullable(schema);
            }
            if let Some(doc) = &field.doc {
                schema.push("description", SchemaValue::string(doc.markdown()));
            }
            if !update {
                if let Some(default) = field
                    .default
                    .as_ref()
                    .and_then(|default| self.program.qualified_default(default, &shape))
                {
                    schema.push("default", default);
                }
            }
            let is_required = match (update, self.direction) {
                (true, _) => false,
                (false, SchemaDirection::Input) => field.is_required,
                (false, SchemaDirection::Output) => !field.optional,
            };
            if is_required {
                required.push(field.name.to_string());
            }
            properties.push((field.name.to_string(), schema));
        }
        ObjectDef {
            discriminator: owner.to_string(),
            properties,
            required,
            description,
        }
    }
}

/// How deeply an instantiation's type arguments may nest before it is described as the record named
/// with no arguments: `<Page T=<Page T=<Page T=Plan/>/>/>` is three.
const MAX_TYPE_ARGUMENT_DEPTH: usize = 4;

/// How deeply `shape` nests applied types: `Plan` is one, `<Page T=Plan/>` two, `int` none.
fn argument_depth(shape: &Shape) -> usize {
    match shape {
        Shape::Seq(item, _) => argument_depth(item),
        Shape::Declared(declared) => {
            1 + declared
                .args
                .iter()
                .map(|(_, arg)| argument_depth(arg))
                .max()
                .unwrap_or(0)
        }
        _ => 0,
    }
}

/// How many times `shape` applies the declaration at `addr`, at any depth.
fn applications_of(addr: DeclAddr, shape: &Shape) -> usize {
    match shape {
        Shape::Seq(item, _) => applications_of(addr, item),
        Shape::Declared(declared) => {
            usize::from(declared.addr == addr)
                + declared
                    .args
                    .iter()
                    .map(|(_, arg)| applications_of(addr, arg))
                    .sum::<usize>()
        }
        _ => 0,
    }
}

fn render_def(needs_type: bool, body: DefBody) -> SchemaValue {
    match body {
        DefBody::Pending => SchemaValue::Object(Vec::new()),
        DefBody::Schema(schema) => schema,
        DefBody::Object(object) => {
            let mut properties = Vec::with_capacity(object.properties.len() + 1);
            let mut required = Vec::with_capacity(object.required.len() + 1);
            if needs_type {
                properties.push((
                    "$type".to_string(),
                    SchemaValue::object(vec![(
                        "const",
                        SchemaValue::string(object.discriminator.clone()),
                    )]),
                ));
                required.push("$type".to_string());
            }
            properties.extend(object.properties);
            required.extend(object.required);
            let mut schema = closed_object(properties, required);
            if let Some(description) = object.description {
                schema.push("description", SchemaValue::string(description));
            }
            schema
        }
    }
}

/// `{ "type": "object", "properties": …, "required": …, "additionalProperties": false }`, with no
/// `required` when nothing is.
fn closed_object(properties: Vec<(String, SchemaValue)>, required: Vec<String>) -> SchemaValue {
    let mut schema = SchemaValue::object(vec![
        ("type", SchemaValue::string("object")),
        ("properties", SchemaValue::Object(properties)),
    ]);
    if !required.is_empty() {
        schema.push(
            "required",
            SchemaValue::Array(required.into_iter().map(SchemaValue::String).collect()),
        );
    }
    schema.push("additionalProperties", SchemaValue::Bool(false));
    schema
}

/// What `object` admits: any JSON value but `null`, which a runtime reads as no value and refuses
/// where exactly one value is expected. Arrays and objects are taken whole, opaquely.
fn any_value() -> SchemaValue {
    SchemaValue::object(vec![(
        "not",
        SchemaValue::object(vec![("type", SchemaValue::string("null"))]),
    )])
}

fn nullable(schema: SchemaValue) -> SchemaValue {
    SchemaValue::object(vec![(
        "anyOf",
        SchemaValue::Array(vec![
            schema,
            SchemaValue::object(vec![("type", SchemaValue::string("null"))]),
        ]),
    )])
}

fn string_enum(cases: Vec<String>) -> SchemaValue {
    SchemaValue::object(vec![
        ("type", SchemaValue::string("string")),
        (
            "enum",
            SchemaValue::Array(cases.into_iter().map(SchemaValue::String).collect()),
        ),
    ])
}

fn primitive_shape(name: &str) -> Option<Shape> {
    Some(match name {
        "object" => Shape::Object,
        "string" => Shape::Primitive(Primitive::String),
        "boolean" => Shape::Primitive(Primitive::Boolean),
        "int" => Shape::Primitive(Primitive::Int),
        "int32" => Shape::Primitive(Primitive::Int32),
        "int64" => Shape::Primitive(Primitive::Int64),
        "float32" => Shape::Primitive(Primitive::Float32),
        "float64" => Shape::Primitive(Primitive::Float64),
        _ => return None,
    })
}

fn primitive_schema(primitive: Primitive) -> SchemaValue {
    let ty = |name: &str| SchemaValue::object(vec![("type", SchemaValue::string(name))]);
    match primitive {
        Primitive::String => ty("string"),
        Primitive::Boolean => ty("boolean"),
        Primitive::Int | Primitive::Int64 => ty("integer"),
        Primitive::Int32 => SchemaValue::object(vec![
            ("type", SchemaValue::string("integer")),
            ("minimum", SchemaValue::Int(i64::from(i32::MIN))),
            ("maximum", SchemaValue::Int(i64::from(i32::MAX))),
        ]),
        Primitive::Float32 | Primitive::Float64 => ty("number"),
        // No value has the bottom type; it never reaches here from a declared type.
        Primitive::Never => SchemaValue::object(vec![("not", SchemaValue::Object(Vec::new()))]),
    }
}

fn literal_value(literal: &Literal, negate: bool) -> SchemaValue {
    let sign = if negate { -1 } else { 1 };
    match literal {
        Literal::String(value) => SchemaValue::string(value.as_str()),
        Literal::Int(value) => SchemaValue::Int(value * sign),
        Literal::Int32(value) => SchemaValue::Int(i64::from(*value) * sign),
        Literal::Float(value) | Literal::Float32(value) => {
            SchemaValue::Float(value.0 * f64::from(sign as i32))
        }
        Literal::Boolean(value) => SchemaValue::Bool(*value),
    }
}

/// A `$defs` key holds only letters, digits, `_`, `.` and `-`.
fn def_key(natural: &str) -> String {
    natural
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// A union's documentation, followed by a list of its documented constant cases, since JSON Schema
/// has no place for a description per `enum` value.
fn union_description(union: &UnionDef) -> Option<String> {
    let own = union.doc.as_ref().map(nx_hir::Doc::markdown);
    let cases = union
        .cases
        .iter()
        .filter(|case| union.is_constant_case(case))
        .filter_map(|case| {
            case.doc
                .as_ref()
                .map(|doc| format!("- `{}`: {}", case.name, doc.markdown_summary()))
        })
        .collect::<Vec<_>>();
    if cases.is_empty() {
        return own;
    }
    let list = cases.join("\n");
    Some(match own {
        Some(own) => format!("{own}\n\n{list}"),
        None => list,
    })
}

fn unknown_declaration(message: String) -> Vec<NxDiagnostic> {
    vec![NxDiagnostic {
        severity: NxSeverity::Error,
        code: Some(SCHEMA_UNKNOWN_DECLARATION.to_string()),
        message,
        labels: Vec::new(),
        help: None,
        note: None,
    }]
}

fn problems_to_api(
    artifact: &ProgramArtifact,
    problems: impl IntoIterator<Item = Problem>,
) -> Vec<NxDiagnostic> {
    let mut seen = Vec::<Problem>::new();
    let mut diagnostics = Vec::new();
    for problem in problems {
        if seen.contains(&problem) {
            continue;
        }
        let mut builder = Diagnostic::error(problem.code).with_message(problem.message.clone());
        if let Some((file, span)) = &problem.label {
            builder = builder.with_label(Label::primary(file.clone(), *span));
        }
        if let Some(help) = &problem.help {
            builder = builder.with_help(help.clone());
        }
        diagnostics.push(builder.build());
        seen.push(problem);
    }
    diagnostics_to_api_with_sources(&diagnostics, "", &artifact.source_map)
}
