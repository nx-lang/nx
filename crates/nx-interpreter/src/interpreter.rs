//! Core interpreter implementation for executing NX HIR.

use crate::context::{ExecutionContext, ResourceLimits};
use crate::error::{RuntimeError, RuntimeErrorKind};
use crate::resolved_program::{ResolvedItemKind, ResolvedProgram, RuntimeModuleId};
use crate::value::Value;
use la_arena::RawIdx;
use nx_hir::{
    ast, effective_record_shape_for_name,
    resolve_record_definition as resolve_hir_record_definition, ElementId, ExprId, Function, Item,
    LoweredModule, Name, PreparedBinding, PreparedBindingOrigin, PreparedBindingTarget,
    PreparedItemKind, PreparedModule, RecordKind,
};
use nx_types::{
    common_supertype, is_object_type, resolve_type_ref_with, resolve_type_ref_with_seen,
    type_satisfies_expected, Type,
};
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;

const COMPONENT_SNAPSHOT_VERSION: u32 = 1;

/// Tree-walking interpreter for NX HIR
#[derive(Debug)]
pub struct Interpreter {
    program: Option<ResolvedProgram>,
    runtime_prepared_cache: RefCell<FxHashMap<RuntimeModuleId, Arc<PreparedModule>>>,
}

/// Result of component initialization.
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentInitResult {
    /// Rendered component body value
    pub rendered: Value,
    /// Opaque serialized component state snapshot owned by the host
    pub state_snapshot: Vec<u8>,
}

/// Result of component action dispatch.
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentDispatchResult {
    /// Effect actions returned by bound handlers in dispatch order
    pub effects: Vec<Value>,
    /// Opaque serialized component state snapshot owned by the host
    pub state_snapshot: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct SerializedComponentSnapshot {
    version: u32,
    program_fingerprint: Option<u64>,
    component_module_id: u32,
    component: String,
    props: BTreeMap<String, SerializedValue>,
    state: BTreeMap<String, SerializedValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum SerializedValue {
    Int32(i32),
    Int(i64),
    Float32(f32),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
    Array(Vec<SerializedValue>),
    EnumVariant {
        type_name: String,
        variant: String,
    },
    Record {
        type_name: String,
        fields: BTreeMap<String, SerializedValue>,
    },
    ActionHandler {
        module_id: u32,
        component: String,
        emit: String,
        action_name: String,
        body: u32,
        captured: BTreeMap<String, SerializedValue>,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct DecodedComponentSnapshot {
    component_module_id: RuntimeModuleId,
    component: Name,
    props: FxHashMap<SmolStr, Value>,
    state: FxHashMap<SmolStr, Value>,
}

impl Interpreter {
    /// Create a new interpreter
    pub fn new() -> Self {
        Self {
            program: None,
            runtime_prepared_cache: RefCell::new(FxHashMap::default()),
        }
    }

    /// Create a new interpreter bound to a resolved program.
    pub fn from_resolved_program(program: ResolvedProgram) -> Self {
        Self {
            program: Some(program),
            runtime_prepared_cache: RefCell::new(FxHashMap::default()),
        }
    }

    /// Execute a function by name with the given arguments
    ///
    /// Uses default resource limits (recursion: 1000, operations: 1M).
    /// For custom limits, use [`execute_function_with_limits`](Self::execute_function_with_limits).
    ///
    /// # Arguments
    /// * `module` - The HIR module containing the function
    /// * `function_name` - Name of the function to execute
    /// * `args` - Arguments to pass to the function (must match parameter count)
    ///
    /// # Returns
    /// The result value or a runtime error
    ///
    /// # Errors
    /// Returns `RuntimeError` if:
    /// - Function not found in module
    /// - Parameter count mismatch
    /// - Runtime error occurs during execution (division by zero, type mismatch, etc.)
    /// - Resource limits exceeded (recursion depth or operation count)
    ///
    /// # Example
    /// ```ignore
    /// use nx_interpreter::{Interpreter, Value};
    ///
    /// let interpreter = Interpreter::new();
    /// let result = interpreter.execute_function(
    ///     &module,
    ///     "add",
    ///     vec![Value::Int(5), Value::Int(3)],
    /// )?;
    /// assert_eq!(result, Value::Int(8));
    /// ```
    pub fn execute_function(
        &self,
        module: &LoweredModule,
        function_name: &str,
        args: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        self.execute_function_with_limits(module, function_name, args, ResourceLimits::default())
    }

    /// Invoke a lowered component action handler with a concrete action value.
    pub fn invoke_action_handler(
        &self,
        module: &LoweredModule,
        handler: &Value,
        action: Value,
    ) -> Result<Vec<Value>, RuntimeError> {
        self.invoke_action_handler_with_limits(module, handler, action, ResourceLimits::default())
    }

    /// Initialize a named component instance from input props.
    pub fn initialize_component(
        &self,
        module: &LoweredModule,
        component_name: &str,
        props: Value,
    ) -> Result<ComponentInitResult, RuntimeError> {
        self.initialize_component_with_limits(
            module,
            component_name,
            props,
            ResourceLimits::default(),
        )
    }

    /// Dispatch a batch of actions against an opaque component state snapshot.
    pub fn dispatch_component_actions(
        &self,
        module: &LoweredModule,
        state_snapshot: &[u8],
        actions: Vec<Value>,
    ) -> Result<ComponentDispatchResult, RuntimeError> {
        self.dispatch_component_actions_with_limits(
            module,
            state_snapshot,
            actions,
            ResourceLimits::default(),
        )
    }

    /// Execute a resolved-program entrypoint function.
    pub fn execute_resolved_program_function(
        &self,
        function_name: &str,
        args: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        self.execute_resolved_program_function_with_limits(
            function_name,
            args,
            ResourceLimits::default(),
        )
    }

    /// Execute a resolved-program entrypoint function with custom limits.
    pub fn execute_resolved_program_function_with_limits(
        &self,
        function_name: &str,
        args: Vec<Value>,
        limits: ResourceLimits,
    ) -> Result<Value, RuntimeError> {
        let program = self.program.as_ref().ok_or_else(|| {
            RuntimeError::new(RuntimeErrorKind::FunctionNotFound {
                name: SmolStr::new(function_name),
            })
        })?;
        let entry = program.entry_function(function_name).ok_or_else(|| {
            RuntimeError::new(RuntimeErrorKind::FunctionNotFound {
                name: SmolStr::new(function_name),
            })
        })?;
        let module = program.module(entry.module_id).ok_or_else(|| {
            RuntimeError::new(RuntimeErrorKind::FunctionNotFound {
                name: SmolStr::new(function_name),
            })
        })?;
        let item = module
            .lowered_module
            .item_by_definition(entry.definition_id)
            .ok_or_else(|| {
                RuntimeError::new(RuntimeErrorKind::FunctionNotFound {
                    name: SmolStr::new(function_name),
                })
            })?;
        let target_name = match item {
            Item::Function(function) => function.name.as_str(),
            _ => function_name,
        };

        self.execute_function_with_limits(module.lowered_module.as_ref(), target_name, args, limits)
    }

    /// Initialize a resolved-program component entrypoint.
    pub fn initialize_resolved_component(
        &self,
        component_name: &str,
        props: Value,
    ) -> Result<ComponentInitResult, RuntimeError> {
        self.initialize_resolved_component_with_limits(
            component_name,
            props,
            ResourceLimits::default(),
        )
    }

    /// Initialize a resolved-program component entrypoint with custom limits.
    pub fn initialize_resolved_component_with_limits(
        &self,
        component_name: &str,
        props: Value,
        limits: ResourceLimits,
    ) -> Result<ComponentInitResult, RuntimeError> {
        let program = self.program.as_ref().ok_or_else(|| {
            RuntimeError::new(RuntimeErrorKind::ComponentNotFound {
                name: SmolStr::new(component_name),
            })
        })?;
        let entry = program.entry_component(component_name).ok_or_else(|| {
            RuntimeError::new(RuntimeErrorKind::ComponentNotFound {
                name: SmolStr::new(component_name),
            })
        })?;
        let module = program.module(entry.module_id).ok_or_else(|| {
            RuntimeError::new(RuntimeErrorKind::ComponentNotFound {
                name: SmolStr::new(component_name),
            })
        })?;
        let item = module
            .lowered_module
            .item_by_definition(entry.definition_id)
            .ok_or_else(|| {
                RuntimeError::new(RuntimeErrorKind::ComponentNotFound {
                    name: SmolStr::new(component_name),
                })
            })?;
        let target_name = match item {
            Item::Component(component) => component.name.as_str(),
            _ => component_name,
        };

        self.initialize_component_with_limits(
            module.lowered_module.as_ref(),
            target_name,
            props,
            limits,
        )
    }

    /// Dispatch actions against a resolved-program snapshot.
    pub fn dispatch_resolved_component_actions(
        &self,
        state_snapshot: &[u8],
        actions: Vec<Value>,
    ) -> Result<ComponentDispatchResult, RuntimeError> {
        self.dispatch_resolved_component_actions_with_limits(
            state_snapshot,
            actions,
            ResourceLimits::default(),
        )
    }

    /// Dispatch actions against a resolved-program snapshot with custom limits.
    pub fn dispatch_resolved_component_actions_with_limits(
        &self,
        state_snapshot: &[u8],
        actions: Vec<Value>,
        limits: ResourceLimits,
    ) -> Result<ComponentDispatchResult, RuntimeError> {
        let module = self.program_root_module()?;
        self.dispatch_component_actions_with_limits(module, state_snapshot, actions, limits)
    }

    fn program_root_module(&self) -> Result<&LoweredModule, RuntimeError> {
        let program = self.program.as_ref().ok_or_else(|| {
            RuntimeError::new(RuntimeErrorKind::InvalidComponentStateSnapshot {
                reason: "resolved program runtime is not available".to_string(),
            })
        })?;
        let root_module_id = program.root_modules().first().copied().ok_or_else(|| {
            RuntimeError::new(RuntimeErrorKind::InvalidComponentStateSnapshot {
                reason: "resolved program does not contain any root modules".to_string(),
            })
        })?;
        let module = program.module(root_module_id).ok_or_else(|| {
            RuntimeError::new(RuntimeErrorKind::InvalidComponentStateSnapshot {
                reason: format!(
                    "resolved program is missing root module {}",
                    root_module_id.as_u32()
                ),
            })
        })?;

        Ok(module.lowered_module.as_ref())
    }

    fn current_module_id(&self, module: &LoweredModule) -> Option<RuntimeModuleId> {
        self.program.as_ref().and_then(|program| {
            program
                .modules()
                .iter()
                .find(|candidate| std::ptr::eq(candidate.lowered_module.as_ref(), module))
                .map(|candidate| candidate.id)
        })
    }

    fn require_current_module_id(
        &self,
        module: &LoweredModule,
        operation: &str,
    ) -> Result<RuntimeModuleId, RuntimeError> {
        self.current_module_id(module).ok_or_else(|| {
            RuntimeError::new(RuntimeErrorKind::ResolvedProgramRequired {
                operation: operation.to_string(),
            })
        })
    }

    fn module_for_id<'a>(
        &'a self,
        fallback_module: &'a LoweredModule,
        module_id: RuntimeModuleId,
    ) -> Result<&'a LoweredModule, RuntimeError> {
        if let Some(program) = self.program.as_ref() {
            let resolved_module = program.module(module_id).ok_or_else(|| {
                RuntimeError::new(RuntimeErrorKind::InvalidComponentStateSnapshot {
                    reason: format!(
                        "snapshot references unknown module id '{}'",
                        module_id.as_u32()
                    ),
                })
            })?;
            Ok(resolved_module.lowered_module.as_ref())
        } else {
            Ok(fallback_module)
        }
    }

    fn runtime_prepared_module(&self, module: &LoweredModule) -> Arc<PreparedModule> {
        let module_identity = self
            .program
            .as_ref()
            .and_then(|program| {
                self.current_module_id(module).and_then(|module_id| {
                    program
                        .module(module_id)
                        .map(|resolved_module| resolved_module.identity.clone())
                })
            })
            .unwrap_or_else(|| "<runtime>".to_string());

        let Some(program) = self.program.as_ref() else {
            return Arc::new(PreparedModule::standalone(module_identity, module.clone()));
        };
        let Some(module_id) = self.current_module_id(module) else {
            return Arc::new(PreparedModule::standalone(module_identity, module.clone()));
        };

        if let Some(prepared) = self.runtime_prepared_cache.borrow().get(&module_id) {
            return Arc::clone(prepared);
        };

        let mut prepared = PreparedModule::standalone(module_identity, module.clone());
        let Some(visible_items) = program.imported_items(module_id) else {
            let prepared = Arc::new(prepared);
            self.runtime_prepared_cache
                .borrow_mut()
                .insert(module_id, Arc::clone(&prepared));
            return prepared;
        };

        for (visible_name, item_ref) in visible_items {
            let Some(target_module) = program.module(item_ref.module_id) else {
                continue;
            };
            let Some(kind) = runtime_prepared_item_kind(item_ref.kind) else {
                continue;
            };

            prepared.add_peer_module(
                target_module.identity.clone(),
                target_module.lowered_module.clone(),
            );

            for namespace in kind.namespaces() {
                prepared.insert_binding(PreparedBinding {
                    visible_name: Name::new(visible_name),
                    namespace: *namespace,
                    kind,
                    origin: PreparedBindingOrigin::Peer {
                        module_identity: target_module.identity.clone(),
                    },
                    target: PreparedBindingTarget::Peer {
                        module_identity: target_module.identity.clone(),
                        definition_id: item_ref.definition_id,
                    },
                });
            }
        }

        let prepared = Arc::new(prepared);
        self.runtime_prepared_cache
            .borrow_mut()
            .insert(module_id, Arc::clone(&prepared));
        prepared
    }

    fn resolve_item<'a>(
        &'a self,
        module: &'a LoweredModule,
        name: &str,
    ) -> Option<(&'a LoweredModule, &'a Item)> {
        if let Some(program) = self.program.as_ref() {
            let module_id = self.current_module_id(module)?;
            if let Some(item_ref) = program.local_item(module_id, name) {
                let target_module = program.module(item_ref.module_id)?;
                let item = target_module
                    .lowered_module
                    .item_by_definition(item_ref.definition_id)?;
                return Some((target_module.lowered_module.as_ref(), item));
            }

            if let Some(item_ref) = program.imported_item(module_id, name) {
                let target_module = program.module(item_ref.module_id)?;
                let item = target_module
                    .lowered_module
                    .item_by_definition(item_ref.definition_id)?;
                return Some((target_module.lowered_module.as_ref(), item));
            }

            return None;
        }

        module.find_item(name).map(|item| (module, item))
    }

    /// Execute a function with custom resource limits
    ///
    /// # Arguments
    /// * `module` - The HIR module containing the function
    /// * `function_name` - Name of the function to execute
    /// * `args` - Arguments to pass to the function
    /// * `limits` - Custom resource limits (recursion depth, operation count)
    ///
    /// # Returns
    /// The result value or a runtime error
    ///
    /// # Errors
    /// Returns `RuntimeError` if:
    /// - Function not found in module
    /// - Parameter count mismatch
    /// - Runtime error occurs during execution
    /// - Resource limits exceeded
    ///
    /// # Example
    /// ```ignore
    /// let limits = ResourceLimits {
    ///     recursion_limit: 100,
    ///     operation_limit: 10_000,
    /// };
    /// let result = interpreter.execute_function_with_limits(
    ///     &module,
    ///     "factorial",
    ///     vec![Value::Int(5)],
    ///     limits,
    /// )?;
    /// ```
    pub fn execute_function_with_limits(
        &self,
        module: &LoweredModule,
        function_name: &str,
        args: Vec<Value>,
        limits: ResourceLimits,
    ) -> Result<Value, RuntimeError> {
        // T011: Find function in module
        let function = self.find_function(module, function_name)?;

        // T011: Validate parameter count
        if function.params.len() != args.len() {
            return Err(RuntimeError::new(
                RuntimeErrorKind::ParameterCountMismatch {
                    expected: function.params.len(),
                    actual: args.len(),
                    function: SmolStr::new(function_name),
                },
            ));
        }

        // T011: Create execution context
        let mut ctx = ExecutionContext::with_limits(limits);
        self.bind_top_level_values(module, &mut ctx)?;

        let coerced_args =
            self.coerce_arguments_for_params(module, args, &function.params, "function call")?;

        // T012: Bind parameters to argument values
        for (param, arg) in function.params.iter().zip(coerced_args.iter()) {
            ctx.define_variable(SmolStr::new(param.name.as_str()), arg.clone());
        }

        // Execute the function body
        let result = self.eval_expr(module, &mut ctx, function.body)?;
        if let Some(return_type) = function.return_type.as_ref() {
            self.coerce_value_to_type(
                module,
                result,
                return_type,
                &format!("return value for '{}'", function.name.as_str()),
            )
        } else {
            Ok(result)
        }
    }

    /// Invoke a lowered component action handler with custom resource limits.
    pub fn invoke_action_handler_with_limits(
        &self,
        module: &LoweredModule,
        handler: &Value,
        action: Value,
        limits: ResourceLimits,
    ) -> Result<Vec<Value>, RuntimeError> {
        let (handler_module_id, component, emit, action_name, body, captured) = match handler {
            Value::ActionHandler {
                module_id,
                component,
                emit,
                action_name,
                body,
                captured,
            } => (*module_id, component, emit, action_name, *body, captured),
            other => {
                return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                    expected: "action handler".to_string(),
                    actual: other.type_name().to_string(),
                    operation: "action handler invocation".to_string(),
                }))
            }
        };
        let handler_module = self.module_for_id(module, handler_module_id)?;

        let action =
            self.validate_handler_input(handler_module, action, action_name, component, emit)?;

        let mut ctx = ExecutionContext::with_limits(limits);
        self.bind_top_level_values(handler_module, &mut ctx)?;
        for (name, value) in captured {
            ctx.define_variable(name.clone(), value.clone());
        }
        ctx.define_variable(SmolStr::new("action"), action);

        let result = self.eval_expr(handler_module, &mut ctx, body)?;
        self.normalize_handler_result(handler_module, result, component, emit)
    }

    /// Initialize a named component instance from input props with custom resource limits.
    pub fn initialize_component_with_limits(
        &self,
        module: &LoweredModule,
        component_name: &str,
        props: Value,
        limits: ResourceLimits,
    ) -> Result<ComponentInitResult, RuntimeError> {
        let component = self.find_component(module, component_name)?;
        let mut ctx = ExecutionContext::with_limits(limits);
        self.bind_top_level_values(module, &mut ctx)?;
        let normalized_props =
            self.normalize_component_props(module, &mut ctx, component, props)?;
        let normalized_state =
            self.materialize_component_state(module, &mut ctx, component, FxHashMap::default())?;
        let rendered = self.eval_expr(module, &mut ctx, component.body)?;
        let component_module_id =
            self.require_current_module_id(module, "component state snapshot creation")?;
        let state_snapshot = self.encode_component_snapshot(
            component_module_id,
            &component.name,
            &normalized_props,
            &normalized_state,
        )?;

        Ok(ComponentInitResult {
            rendered,
            state_snapshot,
        })
    }

    /// Dispatch a batch of actions against an opaque component state snapshot with custom limits.
    pub fn dispatch_component_actions_with_limits(
        &self,
        module: &LoweredModule,
        state_snapshot: &[u8],
        actions: Vec<Value>,
        limits: ResourceLimits,
    ) -> Result<ComponentDispatchResult, RuntimeError> {
        let decoded_snapshot = self.decode_component_snapshot(module, state_snapshot)?;
        let component_module = self.module_for_id(module, decoded_snapshot.component_module_id)?;
        let component =
            self.find_component(component_module, decoded_snapshot.component.as_str())?;
        let mut effects = Vec::new();

        for action in actions {
            let emit = self.validate_component_action(component, &action)?;
            let handler_name = Self::component_handler_prop_name(emit.name.as_str());

            if let Some(handler) = decoded_snapshot.props.get(handler_name.as_str()) {
                match handler {
                    Value::ActionHandler { .. } => {
                        effects.extend(self.invoke_action_handler_with_limits(
                            component_module,
                            handler,
                            action,
                            limits,
                        )?);
                    }
                    _ => {
                        return Err(RuntimeError::new(
                            RuntimeErrorKind::InvalidComponentStateSnapshot {
                                reason: format!(
                                    "handler prop '{}' is not an action handler",
                                    handler_name
                                ),
                            },
                        ));
                    }
                }
            }
        }

        let next_state_snapshot = self.encode_component_snapshot(
            decoded_snapshot.component_module_id,
            &decoded_snapshot.component,
            &decoded_snapshot.props,
            &decoded_snapshot.state,
        )?;

        Ok(ComponentDispatchResult {
            effects,
            state_snapshot: next_state_snapshot,
        })
    }

    /// Find a function by name in the module
    fn find_function<'a>(
        &'a self,
        module: &'a LoweredModule,
        name: &str,
    ) -> Result<&'a Function, RuntimeError> {
        match self.resolve_item(module, name) {
            Some((_, nx_hir::Item::Function(func))) => Ok(func),
            _ => Err(RuntimeError::new(RuntimeErrorKind::FunctionNotFound {
                name: SmolStr::new(name),
            })),
        }
    }

    fn find_component<'a>(
        &'a self,
        module: &'a LoweredModule,
        name: &str,
    ) -> Result<&'a nx_hir::Component, RuntimeError> {
        match self.resolve_item(module, name) {
            Some((_, Item::Component(component))) => Ok(component),
            _ => Err(RuntimeError::new(RuntimeErrorKind::ComponentNotFound {
                name: SmolStr::new(name),
            })),
        }
    }

    fn bind_top_level_values(
        &self,
        module: &LoweredModule,
        ctx: &mut ExecutionContext,
    ) -> Result<(), RuntimeError> {
        for item in module.items() {
            if let Item::Value(value) = item {
                let mut evaluated = self.eval_expr(module, ctx, value.value)?;
                if let Some(ty) = value.ty.as_ref() {
                    evaluated = self.coerce_value_to_type(
                        module,
                        evaluated,
                        ty,
                        &format!("initializer for '{}'", value.name.as_str()),
                    )?;
                }
                ctx.define_variable(SmolStr::new(value.name.as_str()), evaluated);
            }
        }

        Ok(())
    }

    fn flattened_expr_name(&self, module: &LoweredModule, expr_id: ExprId) -> Option<String> {
        match module.expr(expr_id) {
            ast::Expr::Ident(name) => Some(name.as_str().to_string()),
            ast::Expr::Member { base, member, .. } => {
                let mut name = self.flattened_expr_name(module, *base)?;
                name.push('.');
                name.push_str(member.as_str());
                Some(name)
            }
            _ => None,
        }
    }

    fn component_handler_prop_name(emit_name: &str) -> SmolStr {
        SmolStr::new(format!("on{}", emit_name))
    }

    fn input_fields_from_value(
        &self,
        value: Value,
        operation: &str,
    ) -> Result<FxHashMap<SmolStr, Value>, RuntimeError> {
        match value {
            Value::Null => Ok(FxHashMap::default()),
            Value::Record { fields, .. } => Ok(fields),
            other => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                expected: "record".to_string(),
                actual: other.type_name().to_string(),
                operation: operation.to_string(),
            })),
        }
    }

    fn missing_required_component_field_error(
        &self,
        component_name: &Name,
        field_name: &Name,
        phase: &str,
    ) -> RuntimeError {
        RuntimeError::new(RuntimeErrorKind::MissingRequiredComponentField {
            component: SmolStr::new(component_name.as_str()),
            field: SmolStr::new(field_name.as_str()),
            phase: phase.to_string(),
        })
    }

    fn materialize_component_fields(
        &self,
        module: &LoweredModule,
        ctx: &mut ExecutionContext,
        component: &nx_hir::Component,
        fields: &[nx_hir::RecordField],
        overrides: &mut FxHashMap<SmolStr, Value>,
        phase: &str,
    ) -> Result<FxHashMap<SmolStr, Value>, RuntimeError> {
        let mut normalized = FxHashMap::default();

        for field in fields {
            let value = if let Some(value) = overrides.remove(field.name.as_str()) {
                value
            } else if let Some(default_expr) = field.default {
                self.eval_expr(module, ctx, default_expr)?
            } else if matches!(&field.ty, ast::TypeRef::Nullable(_)) {
                Value::Null
            } else {
                return Err(self.missing_required_component_field_error(
                    &component.name,
                    &field.name,
                    phase,
                ));
            };

            let value = self.coerce_value_to_type(
                module,
                value,
                &field.ty,
                &format!(
                    "component {} '{}.{}'",
                    phase,
                    component.name.as_str(),
                    field.name.as_str()
                ),
            )?;
            ctx.define_variable(SmolStr::new(field.name.as_str()), value.clone());
            normalized.insert(SmolStr::new(field.name.as_str()), value);
        }

        Ok(normalized)
    }

    fn normalize_component_props(
        &self,
        module: &LoweredModule,
        ctx: &mut ExecutionContext,
        component: &nx_hir::Component,
        props: Value,
    ) -> Result<FxHashMap<SmolStr, Value>, RuntimeError> {
        let mut overrides = self.input_fields_from_value(
            props,
            &format!("component props for '{}'", component.name.as_str()),
        )?;
        let mut normalized = self.materialize_component_fields(
            module,
            ctx,
            component,
            &component.props,
            &mut overrides,
            "prop initialization",
        )?;

        for (name, value) in overrides {
            if let Some(emit) = component
                .emits
                .iter()
                .find(|emit| Self::component_handler_prop_name(emit.name.as_str()) == name)
            {
                match &value {
                    Value::ActionHandler {
                        component: handler_component,
                        emit: handler_emit,
                        action_name,
                        ..
                    } if handler_component == &component.name
                        && handler_emit == &emit.name
                        && action_name == &emit.action_name => {}
                    Value::ActionHandler { .. } => {
                        return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                            expected: format!(
                                "handler for {}.{}",
                                component.name.as_str(),
                                emit.name.as_str()
                            ),
                            actual: "different action handler".to_string(),
                            operation: "component prop initialization".to_string(),
                        }));
                    }
                    other => {
                        return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                            expected: "action_handler".to_string(),
                            actual: other.type_name().to_string(),
                            operation: "component prop initialization".to_string(),
                        }));
                    }
                }

                normalized.insert(name, value);
            } else {
                return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                    expected: "declared component prop".to_string(),
                    actual: format!("unknown prop '{}'", name),
                    operation: format!(
                        "component prop initialization for '{}'",
                        component.name.as_str()
                    ),
                }));
            }
        }

        Ok(normalized)
    }

    fn materialize_component_state(
        &self,
        module: &LoweredModule,
        ctx: &mut ExecutionContext,
        component: &nx_hir::Component,
        mut overrides: FxHashMap<SmolStr, Value>,
    ) -> Result<FxHashMap<SmolStr, Value>, RuntimeError> {
        self.materialize_component_fields(
            module,
            ctx,
            component,
            &component.state,
            &mut overrides,
            "state initialization",
        )
    }

    fn validate_component_action<'a>(
        &self,
        component: &'a nx_hir::Component,
        action: &Value,
    ) -> Result<&'a nx_hir::ComponentEmit, RuntimeError> {
        let Value::Record { type_name, .. } = action else {
            return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                expected: "action".to_string(),
                actual: action.type_name().to_string(),
                operation: format!("component dispatch for '{}'", component.name.as_str()),
            }));
        };

        component
            .emits
            .iter()
            .find(|emit| emit.action_name == *type_name)
            .ok_or_else(|| {
                RuntimeError::new(RuntimeErrorKind::UnsupportedComponentAction {
                    component: SmolStr::new(component.name.as_str()),
                    action: SmolStr::new(type_name.as_str()),
                })
            })
    }

    fn encode_component_snapshot(
        &self,
        component_module_id: RuntimeModuleId,
        component_name: &Name,
        props: &FxHashMap<SmolStr, Value>,
        state: &FxHashMap<SmolStr, Value>,
    ) -> Result<Vec<u8>, RuntimeError> {
        let snapshot = SerializedComponentSnapshot {
            version: COMPONENT_SNAPSHOT_VERSION,
            program_fingerprint: self.program.as_ref().map(|program| program.fingerprint),
            component_module_id: component_module_id.as_u32(),
            component: component_name.as_str().to_string(),
            props: props
                .iter()
                .map(|(name, value)| (name.to_string(), Self::serialize_runtime_value(value)))
                .collect(),
            state: state
                .iter()
                .map(|(name, value)| (name.to_string(), Self::serialize_runtime_value(value)))
                .collect(),
        };

        rmp_serde::to_vec_named(&snapshot).map_err(|e| {
            RuntimeError::new(RuntimeErrorKind::InvalidComponentStateSnapshot {
                reason: format!("snapshot serialization failed: {e}"),
            })
        })
    }

    fn decode_component_snapshot(
        &self,
        module: &LoweredModule,
        bytes: &[u8],
    ) -> Result<DecodedComponentSnapshot, RuntimeError> {
        let snapshot: SerializedComponentSnapshot = rmp_serde::from_slice(bytes).map_err(|e| {
            RuntimeError::new(RuntimeErrorKind::InvalidComponentStateSnapshot {
                reason: format!("snapshot deserialization failed: {e}"),
            })
        })?;

        if snapshot.version != COMPONENT_SNAPSHOT_VERSION {
            return Err(RuntimeError::new(
                RuntimeErrorKind::InvalidComponentStateSnapshot {
                    reason: format!(
                        "unsupported snapshot version {}, expected {}",
                        snapshot.version, COMPONENT_SNAPSHOT_VERSION
                    ),
                },
            ));
        }

        match self.program.as_ref() {
            Some(program) => {
                if snapshot.program_fingerprint != Some(program.fingerprint) {
                    return Err(RuntimeError::new(
                        RuntimeErrorKind::InvalidComponentStateSnapshot {
                            reason: format!(
                                "snapshot fingerprint {:?} does not match program fingerprint {}",
                                snapshot.program_fingerprint, program.fingerprint
                            ),
                        },
                    ));
                }
            }
            None if snapshot.program_fingerprint.is_some() => {
                return Err(RuntimeError::new(
                    RuntimeErrorKind::InvalidComponentStateSnapshot {
                        reason: format!(
                            "snapshot fingerprint {:?} requires a resolved program runtime",
                            snapshot.program_fingerprint
                        ),
                    },
                ));
            }
            None => {}
        }

        let props = snapshot
            .props
            .into_iter()
            .map(|(name, value)| {
                Ok((
                    SmolStr::new(name.as_str()),
                    self.deserialize_runtime_value(module, value)?,
                ))
            })
            .collect::<Result<FxHashMap<_, _>, RuntimeError>>()?;
        let state = snapshot
            .state
            .into_iter()
            .map(|(name, value)| {
                Ok((
                    SmolStr::new(name.as_str()),
                    self.deserialize_runtime_value(module, value)?,
                ))
            })
            .collect::<Result<FxHashMap<_, _>, RuntimeError>>()?;

        Ok(DecodedComponentSnapshot {
            component_module_id: RuntimeModuleId::new(snapshot.component_module_id),
            component: Name::new(&snapshot.component),
            props,
            state,
        })
    }

    fn serialize_runtime_value(value: &Value) -> SerializedValue {
        match value {
            Value::Int32(value) => SerializedValue::Int32(*value),
            Value::Int(value) => SerializedValue::Int(*value),
            Value::Float32(value) => SerializedValue::Float32(*value),
            Value::Float(value) => SerializedValue::Float(*value),
            Value::String(value) => SerializedValue::String(value.to_string()),
            Value::Boolean(value) => SerializedValue::Boolean(*value),
            Value::Null => SerializedValue::Null,
            Value::Array(values) => {
                SerializedValue::Array(values.iter().map(Self::serialize_runtime_value).collect())
            }
            Value::EnumVariant { type_name, variant } => SerializedValue::EnumVariant {
                type_name: type_name.as_str().to_string(),
                variant: variant.to_string(),
            },
            Value::Record { type_name, fields } => SerializedValue::Record {
                type_name: type_name.as_str().to_string(),
                fields: fields
                    .iter()
                    .map(|(name, value)| (name.to_string(), Self::serialize_runtime_value(value)))
                    .collect(),
            },
            Value::ActionHandler {
                module_id,
                component,
                emit,
                action_name,
                body,
                captured,
            } => SerializedValue::ActionHandler {
                module_id: module_id.as_u32(),
                component: component.as_str().to_string(),
                emit: emit.as_str().to_string(),
                action_name: action_name.as_str().to_string(),
                body: body.into_raw().into_u32(),
                captured: captured
                    .iter()
                    .map(|(name, value)| (name.to_string(), Self::serialize_runtime_value(value)))
                    .collect(),
            },
        }
    }

    fn deserialize_runtime_value(
        &self,
        module: &LoweredModule,
        value: SerializedValue,
    ) -> Result<Value, RuntimeError> {
        match value {
            SerializedValue::Int32(value) => Ok(Value::Int32(value)),
            SerializedValue::Int(value) => Ok(Value::Int(value)),
            SerializedValue::Float32(value) => Ok(Value::Float32(value)),
            SerializedValue::Float(value) => Ok(Value::Float(value)),
            SerializedValue::String(value) => Ok(Value::String(SmolStr::new(value.as_str()))),
            SerializedValue::Boolean(value) => Ok(Value::Boolean(value)),
            SerializedValue::Null => Ok(Value::Null),
            SerializedValue::Array(values) => Ok(Value::Array(
                values
                    .into_iter()
                    .map(|value| self.deserialize_runtime_value(module, value))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            SerializedValue::EnumVariant { type_name, variant } => Ok(Value::EnumVariant {
                type_name: Name::new(&type_name),
                variant: SmolStr::new(variant.as_str()),
            }),
            SerializedValue::Record { type_name, fields } => Ok(Value::Record {
                type_name: Name::new(&type_name),
                fields: fields
                    .into_iter()
                    .map(|(name, value)| {
                        Ok((
                            SmolStr::new(name.as_str()),
                            self.deserialize_runtime_value(module, value)?,
                        ))
                    })
                    .collect::<Result<FxHashMap<_, _>, RuntimeError>>()?,
            }),
            SerializedValue::ActionHandler {
                module_id,
                component,
                emit,
                action_name,
                body,
                captured,
            } => {
                let handler_module_id = RuntimeModuleId::new(module_id);
                let handler_module = self.module_for_id(module, handler_module_id)?;

                if usize::try_from(body)
                    .ok()
                    .filter(|body| *body < handler_module.expr_count())
                    .is_none()
                {
                    return Err(RuntimeError::new(
                        RuntimeErrorKind::InvalidComponentStateSnapshot {
                            reason: format!(
                                "handler body expression id '{}' is out of bounds",
                                body
                            ),
                        },
                    ));
                }

                Ok(Value::ActionHandler {
                    module_id: handler_module_id,
                    component: Name::new(&component),
                    emit: Name::new(&emit),
                    action_name: Name::new(&action_name),
                    body: ExprId::from_raw(RawIdx::from_u32(body)),
                    captured: captured
                        .into_iter()
                        .map(|(name, value)| {
                            Ok((
                                SmolStr::new(name.as_str()),
                                self.deserialize_runtime_value(module, value)?,
                            ))
                        })
                        .collect::<Result<FxHashMap<_, _>, RuntimeError>>()?,
                })
            }
        }
    }

    /// Evaluate an expression (T013 - skeleton)
    fn eval_expr(
        &self,
        module: &LoweredModule,
        ctx: &mut ExecutionContext,
        expr_id: ExprId,
    ) -> Result<Value, RuntimeError> {
        // Check operation limit
        ctx.check_operation_limit()?;

        let expr = module.expr(expr_id);
        match expr {
            ast::Expr::Literal(lit) => self.eval_literal(lit),
            ast::Expr::Ident(name) => self.eval_ident(ctx, name),
            ast::Expr::Block { stmts, expr, .. } => {
                self.eval_block(module, ctx, stmts, expr.as_ref())
            }
            ast::Expr::BinaryOp { lhs, op, rhs, .. } => {
                self.eval_binary_op(module, ctx, *lhs, *op, *rhs)
            }
            ast::Expr::UnaryOp { op, expr, .. } => self.eval_unary_op(module, ctx, *op, *expr),
            ast::Expr::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => self.eval_if(module, ctx, *condition, *then_branch, *else_branch),
            ast::Expr::Let {
                name, value, body, ..
            } => self.eval_let(module, ctx, name, *value, *body),
            ast::Expr::Call { func, args, .. } => self.eval_call(module, ctx, *func, args),
            ast::Expr::For {
                item,
                index,
                iterable,
                body,
                ..
            } => self.eval_for(module, ctx, item, index.as_ref(), *iterable, *body),
            ast::Expr::Array { elements, .. } => {
                let mut values = Vec::with_capacity(elements.len());
                for elem_expr in elements {
                    values.push(self.eval_expr(module, ctx, *elem_expr)?);
                }
                Ok(Value::Array(values))
            }
            ast::Expr::ActionHandler {
                component,
                emit,
                action_name,
                body,
                ..
            } => self.eval_action_handler_expr(module, ctx, component, emit, action_name, *body),
            ast::Expr::Element { element, .. } => self.eval_element_expr(module, ctx, *element),
            ast::Expr::RecordLiteral {
                record, properties, ..
            } => self.eval_record_literal(module, ctx, record, properties),
            ast::Expr::Member { base, member, .. } => self.eval_member(module, ctx, *base, member),
            _ => {
                // Other expression types not yet implemented
                Ok(Value::Null)
            }
        }
    }

    /// Evaluate a literal expression (T015 - placeholder)
    fn eval_literal(&self, lit: &ast::Literal) -> Result<Value, RuntimeError> {
        let value = match lit {
            ast::Literal::Int(n) => Value::Int(*n),
            ast::Literal::Float(f) => Value::Float(f.0),
            ast::Literal::String(s) => Value::String(s.clone()),
            ast::Literal::Boolean(b) => Value::Boolean(*b),
            ast::Literal::Null => Value::Null,
        };
        Ok(value)
    }

    fn eval_action_handler_expr(
        &self,
        module: &LoweredModule,
        ctx: &ExecutionContext,
        component: &Name,
        emit: &Name,
        action_name: &Name,
        body: ExprId,
    ) -> Result<Value, RuntimeError> {
        let mut captured = ctx.snapshot_visible_variables();
        captured.remove("action");
        let module_id = self.require_current_module_id(module, "action handler creation")?;

        Ok(Value::ActionHandler {
            module_id,
            component: component.clone(),
            emit: emit.clone(),
            action_name: action_name.clone(),
            body,
            captured,
        })
    }

    /// Evaluate an identifier (T016 - placeholder)
    fn eval_ident(&self, ctx: &ExecutionContext, name: &Name) -> Result<Value, RuntimeError> {
        ctx.lookup_variable(name.as_str())
    }

    /// Evaluate a block expression (T014 - placeholder)
    fn eval_block(
        &self,
        module: &LoweredModule,
        ctx: &mut ExecutionContext,
        stmts: &[ast::Stmt],
        final_expr: Option<&ExprId>,
    ) -> Result<Value, RuntimeError> {
        ctx.push_scope();

        // Execute statements
        for stmt in stmts {
            self.eval_stmt(module, ctx, stmt)?;
        }

        // Evaluate final expression or return null
        let result = if let Some(expr_id) = final_expr {
            self.eval_expr(module, ctx, *expr_id)?
        } else {
            Value::Null
        };

        ctx.pop_scope();
        Ok(result)
    }

    /// Evaluate a statement (T014 - placeholder)
    fn eval_stmt(
        &self,
        module: &LoweredModule,
        ctx: &mut ExecutionContext,
        stmt: &ast::Stmt,
    ) -> Result<(), RuntimeError> {
        match stmt {
            ast::Stmt::Let { name, init, .. } => {
                let value = self.eval_expr(module, ctx, *init)?;
                ctx.define_variable(SmolStr::new(name.as_str()), value);
                Ok(())
            }
            ast::Stmt::Expr(expr_id, _) => {
                self.eval_expr(module, ctx, *expr_id)?;
                Ok(())
            }
        }
    }

    /// Evaluate a binary operation (T017, T036, T038)
    fn eval_binary_op(
        &self,
        module: &LoweredModule,
        ctx: &mut ExecutionContext,
        lhs: ExprId,
        op: ast::BinOp,
        rhs: ExprId,
    ) -> Result<Value, RuntimeError> {
        // Handle short-circuit operators specially - don't evaluate rhs eagerly
        match op {
            ast::BinOp::And => {
                let lhs_val = self.eval_expr(module, ctx, lhs)?;
                match lhs_val {
                    Value::Boolean(false) => Ok(Value::Boolean(false)),
                    Value::Boolean(true) => {
                        let rhs_val = self.eval_expr(module, ctx, rhs)?;
                        match rhs_val {
                            Value::Boolean(b) => Ok(Value::Boolean(b)),
                            v => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                                expected: "bool".to_string(),
                                actual: v.type_name().to_string(),
                                operation: "logical and".to_string(),
                            })),
                        }
                    }
                    v => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                        expected: "bool".to_string(),
                        actual: v.type_name().to_string(),
                        operation: "logical and".to_string(),
                    })),
                }
            }
            ast::BinOp::Or => {
                let lhs_val = self.eval_expr(module, ctx, lhs)?;
                match lhs_val {
                    Value::Boolean(true) => Ok(Value::Boolean(true)),
                    Value::Boolean(false) => {
                        let rhs_val = self.eval_expr(module, ctx, rhs)?;
                        match rhs_val {
                            Value::Boolean(b) => Ok(Value::Boolean(b)),
                            v => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                                expected: "bool".to_string(),
                                actual: v.type_name().to_string(),
                                operation: "logical or".to_string(),
                            })),
                        }
                    }
                    v => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                        expected: "bool".to_string(),
                        actual: v.type_name().to_string(),
                        operation: "logical or".to_string(),
                    })),
                }
            }
            // All other operators evaluate both sides eagerly
            _ => {
                let lhs_val = self.eval_expr(module, ctx, lhs)?;
                let rhs_val = self.eval_expr(module, ctx, rhs)?;

                match op {
                    // Arithmetic operators
                    ast::BinOp::Add
                    | ast::BinOp::Sub
                    | ast::BinOp::Mul
                    | ast::BinOp::Div
                    | ast::BinOp::Mod
                    | ast::BinOp::Concat => {
                        crate::eval::arithmetic::eval_arithmetic_op(lhs_val, op, rhs_val)
                    }

                    // Comparison operators (T036)
                    ast::BinOp::Eq
                    | ast::BinOp::Ne
                    | ast::BinOp::Lt
                    | ast::BinOp::Le
                    | ast::BinOp::Gt
                    | ast::BinOp::Ge => {
                        crate::eval::logical::eval_comparison_op(lhs_val, op, rhs_val)
                    }

                    // And/Or already handled above
                    ast::BinOp::And | ast::BinOp::Or => unreachable!(),
                }
            }
        }
    }

    /// Evaluate a unary operation (T038)
    fn eval_unary_op(
        &self,
        module: &LoweredModule,
        ctx: &mut ExecutionContext,
        op: ast::UnOp,
        expr: ExprId,
    ) -> Result<Value, RuntimeError> {
        let operand = self.eval_expr(module, ctx, expr)?;

        match op {
            ast::UnOp::Not => crate::eval::logical::eval_logical_unary(op, operand),
            ast::UnOp::Neg => {
                // Arithmetic negation
                match operand {
                    Value::Int32(n) => Ok(Value::Int32(-n)),
                    Value::Int(n) => Ok(Value::Int(-n)),
                    Value::Float32(f) => Ok(Value::Float32(-f)),
                    Value::Float(f) => Ok(Value::Float(-f)),
                    v => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                        expected: "number".to_string(),
                        actual: v.type_name().to_string(),
                        operation: "negation".to_string(),
                    })),
                }
            }
        }
    }

    /// Evaluate an if expression (T037)
    fn eval_if(
        &self,
        module: &LoweredModule,
        ctx: &mut ExecutionContext,
        condition: ExprId,
        then_branch: ExprId,
        else_branch: Option<ExprId>,
    ) -> Result<Value, RuntimeError> {
        // Evaluate condition
        let condition_value = self.eval_expr(module, ctx, condition)?;

        // Condition must be a bool
        let condition_bool = match condition_value {
            Value::Boolean(b) => b,
            v => {
                return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                    expected: "bool".to_string(),
                    actual: v.type_name().to_string(),
                    operation: "if condition".to_string(),
                }))
            }
        };

        // Execute appropriate branch
        if condition_bool {
            self.eval_expr(module, ctx, then_branch)
        } else if let Some(else_expr) = else_branch {
            self.eval_expr(module, ctx, else_expr)
        } else {
            Ok(Value::Null)
        }
    }

    /// Evaluate a let binding expression
    ///
    /// Evaluates the value expression once, binds it to the name in a new scope,
    /// then evaluates the body expression with that binding.
    fn eval_let(
        &self,
        module: &LoweredModule,
        ctx: &mut ExecutionContext,
        name: &Name,
        value: ExprId,
        body: ExprId,
    ) -> Result<Value, RuntimeError> {
        // Evaluate the value expression once
        let val = self.eval_expr(module, ctx, value)?;

        // Create a new scope with the binding
        ctx.push_scope();
        ctx.define_variable(name.as_str().into(), val);

        // Evaluate the body
        let result = self.eval_expr(module, ctx, body);

        // Pop the scope
        ctx.pop_scope();

        result
    }

    /// Evaluate a function call expression (T053)
    fn eval_call(
        &self,
        module: &LoweredModule,
        ctx: &mut ExecutionContext,
        func_expr: ExprId,
        args: &[ExprId],
    ) -> Result<Value, RuntimeError> {
        let func_name = self.flattened_expr_name(module, func_expr).ok_or_else(|| {
            RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                expected: "function name".to_string(),
                actual: "complex expression".to_string(),
                operation: "function call".to_string(),
            })
        })?;

        let mut arg_values = Vec::with_capacity(args.len());
        for arg_expr in args {
            arg_values.push(self.eval_expr(module, ctx, *arg_expr)?);
        }

        match self.resolve_item(module, func_name.as_str()) {
            Some((target_module, Item::Function(function))) => self.eval_function_call(
                target_module,
                ctx,
                func_name.as_str(),
                function,
                arg_values,
            ),
            Some((target_module, Item::Record(record_def))) => self.eval_record_constructor_call(
                target_module,
                ctx,
                func_name.as_str(),
                record_def,
                arg_values,
            ),
            Some((target_module, Item::TypeAlias(_))) => {
                if let Some(record_def) = self.resolve_record_definition(module, func_name.as_str())
                {
                    self.eval_record_constructor_call(
                        target_module,
                        ctx,
                        func_name.as_str(),
                        &record_def,
                        arg_values,
                    )
                } else {
                    Err(RuntimeError::new(RuntimeErrorKind::FunctionNotFound {
                        name: SmolStr::new(func_name.as_str()),
                    }))
                }
            }
            _ => Err(RuntimeError::new(RuntimeErrorKind::FunctionNotFound {
                name: SmolStr::new(func_name.as_str()),
            })),
        }
    }

    fn eval_function_call(
        &self,
        module: &LoweredModule,
        ctx: &mut ExecutionContext,
        func_name: &str,
        function: &Function,
        arg_values: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        if function.params.len() != arg_values.len() {
            return Err(RuntimeError::new(
                RuntimeErrorKind::ParameterCountMismatch {
                    expected: function.params.len(),
                    actual: arg_values.len(),
                    function: SmolStr::new(func_name),
                },
            ));
        }

        let call_frame = crate::error::CallFrame::new(SmolStr::new(func_name), None);
        ctx.push_call_frame(call_frame)?;

        let coerced_args = self.coerce_arguments_for_params(
            module,
            arg_values,
            &function.params,
            "function call",
        )?;

        ctx.push_scope();
        for (param, arg) in function.params.iter().zip(coerced_args.iter()) {
            ctx.define_variable(SmolStr::new(param.name.as_str()), arg.clone());
        }

        let result = self
            .eval_expr(module, ctx, function.body)
            .and_then(|value| {
                if let Some(return_type) = function.return_type.as_ref() {
                    self.coerce_value_to_type(
                        module,
                        value,
                        return_type,
                        &format!("return value for '{}'", function.name.as_str()),
                    )
                } else {
                    Ok(value)
                }
            });

        ctx.pop_scope();
        ctx.pop_call_frame();

        result
    }

    fn eval_record_constructor_call(
        &self,
        module: &LoweredModule,
        ctx: &mut ExecutionContext,
        func_name: &str,
        record_def: &nx_hir::RecordDef,
        arg_values: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        let record_shape = self.effective_record_shape(module, &record_def.name)?;

        if arg_values.len() > record_shape.fields.len() {
            return Err(RuntimeError::new(
                RuntimeErrorKind::ParameterCountMismatch {
                    expected: record_shape.fields.len(),
                    actual: arg_values.len(),
                    function: SmolStr::new(func_name),
                },
            ));
        }

        let mut overrides = FxHashMap::default();
        for (field, value) in record_shape.fields.iter().zip(arg_values.into_iter()) {
            let value = self.coerce_value_to_type(
                module,
                value,
                &field.ty,
                &format!("record field '{}'", field.name.as_str()),
            )?;
            overrides.insert(SmolStr::new(field.name.as_str()), value);
        }

        self.build_record_value_from_shape(module, ctx, record_shape, overrides, None)
    }

    fn eval_element_expr(
        &self,
        module: &LoweredModule,
        ctx: &mut ExecutionContext,
        element_id: ElementId,
    ) -> Result<Value, RuntimeError> {
        let element = module.element(element_id);
        let tag_name = element.tag.as_str();

        let mut fields = FxHashMap::default();
        for prop in &element.properties {
            let value = self.eval_expr(module, ctx, prop.value)?;
            fields.insert(SmolStr::new(prop.key.as_str()), value);
        }

        let content_values = self.eval_content_expressions(module, ctx, &element.content)?;
        let normalized_content = self.normalize_content_values(content_values);

        if let Some((target_module, Item::Function(function))) = self.resolve_item(module, tag_name)
        {
            self.inject_element_content_field(
                &mut fields,
                normalized_content,
                function.content_param().map(|param| param.name.as_str()),
                "function without a declared content parameter",
                "element function call",
            )?;

            if fields.len() != function.params.len() {
                return Err(RuntimeError::new(
                    RuntimeErrorKind::ParameterCountMismatch {
                        expected: function.params.len(),
                        actual: fields.len(),
                        function: SmolStr::new(tag_name),
                    },
                ));
            }

            let mut arg_values = Vec::with_capacity(function.params.len());
            for param in &function.params {
                match fields.remove(param.name.as_str()) {
                    Some(value) => {
                        arg_values.push(self.coerce_value_to_type(
                            target_module,
                            value,
                            &param.ty,
                            &format!("parameter '{}'", param.name.as_str()),
                        )?);
                    }
                    None => {
                        return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                            expected: format!("argument '{}'", param.name.as_str()),
                            actual: "missing".to_string(),
                            operation: "element function call".to_string(),
                        }))
                    }
                }
            }

            if !fields.is_empty() {
                return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                    expected: "known parameters".to_string(),
                    actual: "unknown property".to_string(),
                    operation: "element function call".to_string(),
                }));
            }

            return self.eval_function_call(target_module, ctx, tag_name, function, arg_values);
        }

        if let Some((target_module, Item::Component(component))) =
            self.resolve_item(module, tag_name)
        {
            self.inject_element_content_field(
                &mut fields,
                normalized_content,
                component.content_prop().map(|field| field.name.as_str()),
                "component without a declared content prop",
                "element component call",
            )?;

            ctx.push_scope();
            let normalized_props = self.normalize_component_props(
                target_module,
                ctx,
                component,
                Value::Record {
                    type_name: component.name.clone(),
                    fields,
                },
            );
            ctx.pop_scope();
            let normalized_props = normalized_props?;

            return Ok(Value::Record {
                type_name: component.name.clone(),
                fields: normalized_props,
            });
        }

        if let Some(record_def) = self.resolve_record_definition(module, tag_name) {
            let record_shape = self.effective_record_shape(module, &record_def.name)?;
            self.inject_element_content_field(
                &mut fields,
                normalized_content,
                record_shape
                    .content_property()
                    .map(|field| field.name.as_str()),
                "record without a declared content field",
                "element record call",
            )?;

            return self.build_record_value_from_shape(module, ctx, record_shape, fields, None);
        }

        self.inject_element_content_field(
            &mut fields,
            normalized_content,
            Some("content"),
            "intrinsic element content channel",
            "element intrinsic call",
        )?;

        Ok(Value::Record {
            type_name: element.tag.clone(),
            fields,
        })
    }

    fn inject_element_content_field(
        &self,
        fields: &mut FxHashMap<SmolStr, Value>,
        normalized_content: Option<Value>,
        content_property_name: Option<&str>,
        missing_content_expected: &str,
        operation: &str,
    ) -> Result<(), RuntimeError> {
        let Some(content_value) = normalized_content else {
            return Ok(());
        };

        let Some(content_property_name) = content_property_name else {
            return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                expected: missing_content_expected.to_string(),
                actual: "element body content".to_string(),
                operation: operation.to_string(),
            }));
        };

        if fields.contains_key(content_property_name) {
            return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                expected: format!(
                    "content for '{}' passed either as a named property or as element body",
                    content_property_name
                ),
                actual: format!(
                    "both a '{}' property and element body content",
                    content_property_name
                ),
                operation: operation.to_string(),
            }));
        }

        fields.insert(SmolStr::new(content_property_name), content_value);

        Ok(())
    }

    fn eval_content_expressions(
        &self,
        module: &LoweredModule,
        ctx: &mut ExecutionContext,
        content_exprs: &[ExprId],
    ) -> Result<Vec<Value>, RuntimeError> {
        let mut values = Vec::new();
        for content_expr in content_exprs {
            let value = self.eval_expr(module, ctx, *content_expr)?;
            match value {
                // Content arrays represent sibling body-content results from multi-item braces and
                // element-producing control flow, so splice them into the parent content list.
                Value::Array(items) => values.extend(items),
                other => values.push(other),
            }
        }
        Ok(values)
    }

    fn normalize_content_values(&self, content_values: Vec<Value>) -> Option<Value> {
        match content_values.len() {
            0 => None,
            1 => content_values.into_iter().next(),
            _ => Some(Value::Array(content_values)),
        }
    }

    fn coerce_arguments_for_params(
        &self,
        module: &LoweredModule,
        arg_values: Vec<Value>,
        params: &[nx_hir::Param],
        operation: &str,
    ) -> Result<Vec<Value>, RuntimeError> {
        let mut coerced = Vec::with_capacity(arg_values.len());
        for (param, value) in params.iter().zip(arg_values.into_iter()) {
            coerced.push(self.coerce_value_to_type(
                module,
                value,
                &param.ty,
                &format!("{} parameter '{}'", operation, param.name.as_str()),
            )?);
        }
        Ok(coerced)
    }

    fn coerce_value_to_type(
        &self,
        module: &LoweredModule,
        value: Value,
        expected: &ast::TypeRef,
        operation: &str,
    ) -> Result<Value, RuntimeError> {
        let expected_ty = self.runtime_type_from_type_ref(module, expected);
        self.coerce_value_to_resolved_type(module, value, &expected_ty, operation)
    }

    fn coerce_value_to_resolved_type(
        &self,
        module: &LoweredModule,
        value: Value,
        expected: &Type,
        operation: &str,
    ) -> Result<Value, RuntimeError> {
        if let Type::Nullable(expected_inner) = expected {
            return match value {
                Value::Null => Ok(Value::Null),
                other => {
                    self.coerce_value_to_resolved_type(module, other, expected_inner, operation)
                }
            };
        }

        if let Type::Array(expected_item) = expected {
            return match value {
                Value::Array(values) => {
                    let mut coerced = Vec::with_capacity(values.len());
                    for item in values {
                        coerced.push(self.coerce_value_to_resolved_type(
                            module,
                            item,
                            expected_item,
                            operation,
                        )?);
                    }
                    Ok(Value::Array(coerced))
                }
                other => Ok(Value::Array(vec![self.coerce_value_to_resolved_type(
                    module,
                    other,
                    expected_item,
                    operation,
                )?])),
            };
        }

        // Ordinary record-typed parameters preserve the caller-supplied object shape. Typed record
        // construction is reserved for explicit external input boundaries such as handler
        // invocation so defaults and required-field validation do not run on every function call.
        match value {
            Value::Record { type_name, fields } => {
                if self.record_value_matches_expected_type(module, &type_name, expected) {
                    Ok(Value::Record { type_name, fields })
                } else {
                    Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                        expected: expected.to_string(),
                        actual: type_name.as_str().to_string(),
                        operation: operation.to_string(),
                    }))
                }
            }
            other => self.coerce_non_record_value(other, expected, operation),
        }
    }

    fn coerce_non_record_value(
        &self,
        value: Value,
        expected: &Type,
        operation: &str,
    ) -> Result<Value, RuntimeError> {
        if matches!(value, Value::Array(_)) && !is_object_type(expected) {
            let actual_ty = self.runtime_type_of_value(&value);
            return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                expected: expected.to_string(),
                actual: format!("list {}", actual_ty),
                operation: operation.to_string(),
            }));
        }

        if self.value_matches_expected_type(&value, expected) {
            Ok(value)
        } else {
            let actual_ty = self.runtime_type_of_value(&value);
            Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                expected: expected.to_string(),
                actual: actual_ty.to_string(),
                operation: operation.to_string(),
            }))
        }
    }

    fn value_matches_expected_type(&self, value: &Value, expected: &Type) -> bool {
        if matches!(value, Value::Null) {
            return matches!(expected, Type::Nullable(_)) || is_object_type(expected);
        }

        let actual_ty = self.runtime_type_of_value(value);
        type_satisfies_expected(&actual_ty, expected)
    }

    fn runtime_type_of_value(&self, value: &Value) -> Type {
        match value {
            Value::Int32(_) => Type::i32(),
            Value::Int(_) => Type::int(),
            Value::Float32(_) => Type::f32(),
            Value::Float(_) => Type::float(),
            Value::String(_) => Type::string(),
            Value::Boolean(_) => Type::bool(),
            Value::Null => Type::nullable(Type::named("object")),
            Value::Array(values) => {
                if values.is_empty() {
                    Type::array(Type::named("object"))
                } else {
                    let mut current = self.runtime_type_of_value(&values[0]);
                    for value in values.iter().skip(1) {
                        let value_ty = self.runtime_type_of_value(value);
                        current = common_supertype(&current, &value_ty);
                    }
                    Type::array(current)
                }
            }
            Value::EnumVariant { type_name, .. } => Type::named(type_name.clone()),
            Value::Record { type_name, .. } => Type::named(type_name.clone()),
            // Handlers are opaque runtime callback objects rather than first-class typed functions.
            Value::ActionHandler { .. } => Type::named("action_handler"),
        }
    }

    fn runtime_type_from_type_ref(&self, module: &LoweredModule, type_ref: &ast::TypeRef) -> Type {
        resolve_type_ref_with(type_ref, &mut |name, seen| {
            self.resolve_runtime_named_type(module, name, seen)
        })
    }

    fn resolve_runtime_named_type(
        &self,
        module: &LoweredModule,
        name: &Name,
        seen: &mut FxHashSet<Name>,
    ) -> Type {
        if !seen.insert(name.clone()) {
            return Type::named(name.clone());
        }

        let resolved = match self.resolve_item(module, name.as_str()) {
            Some((target_module, Item::TypeAlias(alias))) => {
                resolve_type_ref_with_seen(&alias.ty, seen, &mut |nested_name, nested_seen| {
                    self.resolve_runtime_named_type(target_module, nested_name, nested_seen)
                })
            }
            _ => Type::named(name.clone()),
        };

        seen.remove(name);
        resolved
    }

    fn eval_member(
        &self,
        module: &LoweredModule,
        ctx: &mut ExecutionContext,
        base_expr: ExprId,
        member: &Name,
    ) -> Result<Value, RuntimeError> {
        if let Some(mut qualified_name) = self.flattened_expr_name(module, base_expr) {
            qualified_name.push('.');
            qualified_name.push_str(member.as_str());
            if let Some(value) = ctx.try_lookup_variable(&qualified_name) {
                return Ok(value);
            }
        }

        if let ast::Expr::Ident(base_name) = module.expr(base_expr) {
            // Prefer runtime value if variable exists
            if let Some(var_value) = ctx.try_lookup_variable(base_name.as_str()) {
                return self.project_member(var_value, member, Some(base_name.as_str()));
            }

            if let Some(enum_def) = self.resolve_enum_definition(module, base_name) {
                if enum_def
                    .members
                    .iter()
                    .any(|m| m.name.as_str() == member.as_str())
                {
                    return Ok(Value::EnumVariant {
                        type_name: enum_def.name.clone(),
                        variant: SmolStr::new(member.as_str()),
                    });
                } else {
                    return Err(RuntimeError::new(RuntimeErrorKind::EnumMemberNotFound {
                        enum_name: SmolStr::new(enum_def.name.as_str()),
                        member: SmolStr::new(member.as_str()),
                    }));
                }
            } else {
                return Err(RuntimeError::new(RuntimeErrorKind::EnumNotFound {
                    name: SmolStr::new(base_name.as_str()),
                }));
            }
        }

        if let Some(base_name) = self.flattened_expr_name(module, base_expr) {
            let base_name = Name::new(&base_name);
            if let Some(enum_def) = self.resolve_enum_definition(module, &base_name) {
                if enum_def
                    .members
                    .iter()
                    .any(|m| m.name.as_str() == member.as_str())
                {
                    return Ok(Value::EnumVariant {
                        type_name: enum_def.name.clone(),
                        variant: SmolStr::new(member.as_str()),
                    });
                }

                return Err(RuntimeError::new(RuntimeErrorKind::EnumMemberNotFound {
                    enum_name: SmolStr::new(enum_def.name.as_str()),
                    member: SmolStr::new(member.as_str()),
                }));
            }
        }

        let base_value = self.eval_expr(module, ctx, base_expr)?;
        self.project_member(base_value, member, None)
    }

    fn project_member(
        &self,
        base_value: Value,
        member: &Name,
        record_label: Option<&str>,
    ) -> Result<Value, RuntimeError> {
        match base_value {
            Value::Record { fields, .. } => {
                if let Some(value) = fields.get(member.as_str()) {
                    Ok(value.clone())
                } else {
                    let name = record_label.unwrap_or("record");
                    Err(RuntimeError::new(RuntimeErrorKind::RecordFieldNotFound {
                        record: SmolStr::new(name),
                        field: SmolStr::new(member.as_str()),
                    }))
                }
            }
            Value::EnumVariant { .. } => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                expected: "record".to_string(),
                actual: "enum".to_string(),
                operation: format!("member access .{}", member.as_str()),
            })),
            other => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                expected: "record".to_string(),
                actual: other.type_name().to_string(),
                operation: format!("member access .{}", member.as_str()),
            })),
        }
    }

    /// Instantiate a record value from its definition, applying default values.
    pub fn instantiate_record_defaults(
        &self,
        module: &LoweredModule,
        record_name: &str,
    ) -> Result<Value, RuntimeError> {
        let mut ctx = ExecutionContext::new();
        self.build_record_value(module, &mut ctx, record_name, FxHashMap::default())
    }

    /// Evaluate a for loop expression
    fn eval_for(
        &self,
        module: &LoweredModule,
        ctx: &mut ExecutionContext,
        item: &Name,
        index: Option<&Name>,
        iterable_expr: ExprId,
        body_expr: ExprId,
    ) -> Result<Value, RuntimeError> {
        // Evaluate the iterable expression
        let iterable_value = self.eval_expr(module, ctx, iterable_expr)?;

        // Extract array elements
        let elements = match iterable_value {
            Value::Array(ref arr) => arr.clone(),
            _ => {
                return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                    expected: "array".to_string(),
                    actual: iterable_value.type_name().to_string(),
                    operation: "for loop iteration".to_string(),
                }))
            }
        };

        // Collect results from each iteration
        let mut results = Vec::with_capacity(elements.len());

        // Iterate over elements
        for (idx, element) in elements.iter().enumerate() {
            // Create new scope for loop variables
            ctx.push_scope();

            // Bind item variable
            ctx.define_variable(SmolStr::new(item.as_str()), element.clone());

            // Bind index variable if present
            if let Some(index_name) = index {
                ctx.define_variable(SmolStr::new(index_name.as_str()), Value::Int(idx as i64));
            }

            // Evaluate body
            let result = self.eval_expr(module, ctx, body_expr)?;
            results.push(result);

            // Pop scope
            ctx.pop_scope();
        }

        // Return array of results
        Ok(Value::Array(results))
    }

    fn resolve_enum_definition<'a>(
        &'a self,
        module: &'a LoweredModule,
        name: &Name,
    ) -> Option<&'a nx_hir::EnumDef> {
        self.resolve_enum_definition_inner(module, name, &mut FxHashSet::default())
    }

    fn resolve_record_definition(
        &self,
        module: &LoweredModule,
        name: &str,
    ) -> Option<nx_hir::RecordDef> {
        let (target_module, item) = self.resolve_item(module, name)?;
        let prepared = self.runtime_prepared_module(target_module);
        match item {
            Item::Record(record_def) => Some(record_def.clone()),
            Item::TypeAlias(alias) => resolve_hir_record_definition(prepared.as_ref(), &alias.name),
            _ => None,
        }
    }

    fn resolve_enum_definition_inner<'a>(
        &'a self,
        module: &'a LoweredModule,
        name: &Name,
        seen: &mut FxHashSet<SmolStr>,
    ) -> Option<&'a nx_hir::EnumDef> {
        let key = SmolStr::new(name.as_str());
        if !seen.insert(key.clone()) {
            return None;
        }

        let result = match self.resolve_item(module, name.as_str()) {
            Some((_, nx_hir::Item::Enum(enum_def))) => Some(enum_def),
            Some((target_module, nx_hir::Item::TypeAlias(alias))) => match &alias.ty {
                ast::TypeRef::Name(target) => {
                    self.resolve_enum_definition_inner(target_module, target, seen)
                }
                _ => None,
            },
            _ => None,
        };

        seen.remove(&key);
        result
    }

    fn effective_record_shape(
        &self,
        module: &LoweredModule,
        name: &Name,
    ) -> Result<nx_hir::EffectiveRecordShape, RuntimeError> {
        let target_module = self
            .resolve_item(module, name.as_str())
            .map(|(target_module, _)| target_module)
            .unwrap_or(module);
        let prepared = self.runtime_prepared_module(target_module);
        let shape = effective_record_shape_for_name(prepared.as_ref(), name)
            .map_err(|error| self.record_resolution_runtime_error(error))?;

        shape.ok_or_else(|| {
            RuntimeError::new(RuntimeErrorKind::RecordTypeNotFound {
                name: SmolStr::new(name.as_str()),
            })
        })
    }

    fn record_resolution_runtime_error(
        &self,
        error: nx_hir::RecordResolutionError,
    ) -> RuntimeError {
        RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "valid record definition".to_string(),
            actual: error.message(),
            operation: "record resolution".to_string(),
        })
    }

    fn record_type_satisfies_expected(
        &self,
        module: &LoweredModule,
        actual: &Name,
        expected: &Name,
    ) -> bool {
        if actual == expected {
            return true;
        }

        self.effective_record_shape(module, actual)
            .map(|shape| shape.ancestors.iter().any(|ancestor| ancestor == expected))
            .unwrap_or(false)
    }

    fn record_value_matches_expected_type(
        &self,
        module: &LoweredModule,
        actual_type_name: &Name,
        expected: &Type,
    ) -> bool {
        let actual = Type::named(actual_type_name.clone());
        type_satisfies_expected(&actual, expected)
            || match expected {
                Type::Named(expected_name) => {
                    self.record_type_satisfies_expected(module, actual_type_name, expected_name)
                }
                Type::Nullable(expected_inner) => self.record_value_matches_expected_type(
                    module,
                    actual_type_name,
                    expected_inner,
                ),
                _ => false,
            }
    }

    fn abstract_record_instantiation_error(
        &self,
        record_name: &Name,
        operation: &str,
    ) -> RuntimeError {
        RuntimeError::new(RuntimeErrorKind::AbstractRecordInstantiation {
            record: SmolStr::new(record_name.as_str()),
            operation: operation.to_string(),
        })
    }

    // Construct a typed record object from an externally supplied record-shaped value. This is
    // intentionally scoped to explicit input boundaries rather than general parameter coercion.
    fn construct_external_record_value(
        &self,
        module: &LoweredModule,
        actual_type_name: &Name,
        fields: FxHashMap<SmolStr, Value>,
        expected_type_name: &Name,
        operation: &str,
    ) -> Result<Value, RuntimeError> {
        let expected_def = self.resolve_record_definition(module, expected_type_name.as_str());
        let actual_def = self.resolve_record_definition(module, actual_type_name.as_str());
        let (Some(expected_def), Some(actual_def)) = (expected_def, actual_def) else {
            return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                expected: expected_type_name.as_str().to_string(),
                actual: actual_type_name.as_str().to_string(),
                operation: operation.to_string(),
            }));
        };

        if !self.record_type_satisfies_expected(module, &actual_def.name, &expected_def.name) {
            return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                expected: expected_type_name.as_str().to_string(),
                actual: actual_type_name.as_str().to_string(),
                operation: operation.to_string(),
            }));
        }

        let mut ctx = ExecutionContext::new();
        let actual_shape = self.effective_record_shape(module, &actual_def.name)?;
        self.build_record_value_from_shape(module, &mut ctx, actual_shape, fields, Some(operation))
    }

    fn missing_required_record_field_error(
        &self,
        record_name: &Name,
        field_name: &Name,
        operation: &str,
    ) -> RuntimeError {
        RuntimeError::new(RuntimeErrorKind::MissingRequiredRecordField {
            record: SmolStr::new(record_name.as_str()),
            field: SmolStr::new(field_name.as_str()),
            operation: operation.to_string(),
        })
    }

    fn validate_handler_input(
        &self,
        module: &LoweredModule,
        action: Value,
        expected_action_name: &Name,
        component: &Name,
        emit: &Name,
    ) -> Result<Value, RuntimeError> {
        let operation = format!(
            "action handler input for {}.{}",
            component.as_str(),
            emit.as_str()
        );
        let expected_def = self
            .resolve_record_definition(module, expected_action_name.as_str())
            .ok_or_else(|| {
                RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                    expected: expected_action_name.as_str().to_string(),
                    actual: "unknown".to_string(),
                    operation: operation.clone(),
                })
            })?;
        if expected_def.kind != RecordKind::Action {
            return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                expected: expected_action_name.as_str().to_string(),
                actual: "non-action record".to_string(),
                operation,
            }));
        }

        // Handler invocation constructs the typed action record before executing the handler body,
        // so defaults, nullable fields, and required-field diagnostics are applied at the input
        // boundary.
        match action {
            Value::Record { type_name, fields } => self.construct_external_record_value(
                module,
                &type_name,
                fields,
                expected_action_name,
                &operation,
            ),
            other => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                expected: expected_action_name.as_str().to_string(),
                actual: other.type_name().to_string(),
                operation,
            })),
        }
    }

    fn normalize_handler_result(
        &self,
        module: &LoweredModule,
        result: Value,
        component: &Name,
        emit: &Name,
    ) -> Result<Vec<Value>, RuntimeError> {
        match result {
            Value::Record { .. } => {
                self.ensure_action_result_value(module, &result, component, emit)?;
                Ok(vec![result])
            }
            Value::Array(values) => {
                if values.is_empty() {
                    return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                        expected: "one or more actions".to_string(),
                        actual: "empty array".to_string(),
                        operation: format!(
                            "action handler result for {}.{}",
                            component.as_str(),
                            emit.as_str()
                        ),
                    }));
                }

                for value in &values {
                    self.ensure_action_result_value(module, value, component, emit)?;
                }

                Ok(values)
            }
            other => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                expected: "action or action array".to_string(),
                actual: other.type_name().to_string(),
                operation: format!(
                    "action handler result for {}.{}",
                    component.as_str(),
                    emit.as_str()
                ),
            })),
        }
    }

    fn ensure_action_result_value(
        &self,
        module: &LoweredModule,
        value: &Value,
        component: &Name,
        emit: &Name,
    ) -> Result<(), RuntimeError> {
        match value {
            Value::Record { type_name, .. } => {
                let is_action = self
                    .resolve_record_definition(module, type_name.as_str())
                    .map(|record| record.kind == RecordKind::Action)
                    .unwrap_or(false);

                if is_action {
                    Ok(())
                } else {
                    Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                        expected: "action".to_string(),
                        actual: type_name.as_str().to_string(),
                        operation: format!(
                            "action handler result for {}.{}",
                            component.as_str(),
                            emit.as_str()
                        ),
                    }))
                }
            }
            other => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                expected: "action".to_string(),
                actual: other.type_name().to_string(),
                operation: format!(
                    "action handler result for {}.{}",
                    component.as_str(),
                    emit.as_str()
                ),
            })),
        }
    }
}

fn runtime_prepared_item_kind(kind: ResolvedItemKind) -> Option<PreparedItemKind> {
    match kind {
        ResolvedItemKind::Function => Some(PreparedItemKind::Function),
        ResolvedItemKind::Value => Some(PreparedItemKind::Value),
        ResolvedItemKind::Component => Some(PreparedItemKind::Component),
        ResolvedItemKind::TypeAlias => Some(PreparedItemKind::TypeAlias),
        ResolvedItemKind::Enum => Some(PreparedItemKind::Enum),
        ResolvedItemKind::Record => Some(PreparedItemKind::Record),
    }
}

impl Interpreter {
    fn eval_record_literal(
        &self,
        module: &LoweredModule,
        ctx: &mut ExecutionContext,
        record: &Name,
        properties: &[(Name, ExprId)],
    ) -> Result<Value, RuntimeError> {
        let mut overrides = FxHashMap::default();
        for (name, expr_id) in properties {
            let value = self.eval_expr(module, ctx, *expr_id)?;
            overrides.insert(SmolStr::new(name.as_str()), value);
        }

        self.build_record_value(module, ctx, record.as_str(), overrides)
    }

    fn build_record_value(
        &self,
        module: &LoweredModule,
        ctx: &mut ExecutionContext,
        record_name: &str,
        overrides: FxHashMap<SmolStr, Value>,
    ) -> Result<Value, RuntimeError> {
        let record_shape = self.effective_record_shape(module, &Name::new(record_name))?;
        self.build_record_value_from_shape(module, ctx, record_shape, overrides, None)
    }

    fn build_record_value_from_shape(
        &self,
        module: &LoweredModule,
        ctx: &mut ExecutionContext,
        record_shape: nx_hir::EffectiveRecordShape,
        mut overrides: FxHashMap<SmolStr, Value>,
        missing_operation: Option<&str>,
    ) -> Result<Value, RuntimeError> {
        let record_def = &record_shape.record;

        if record_def.is_abstract {
            return Err(self.abstract_record_instantiation_error(
                &record_def.name,
                missing_operation.unwrap_or("record construction"),
            ));
        }

        for prop in &record_shape.fields {
            let value = if let Some(value) = overrides.remove(prop.name.as_str()) {
                value
            } else if let Some(default_expr) = prop.default {
                self.eval_expr(module, ctx, default_expr)?
            } else if matches!(&prop.ty, ast::TypeRef::Nullable(_)) {
                Value::Null
            } else if let Some(operation) = missing_operation {
                return Err(self.missing_required_record_field_error(
                    &record_def.name,
                    &prop.name,
                    operation,
                ));
            } else {
                Value::Null
            };

            let value = self.coerce_value_to_type(
                module,
                value,
                &prop.ty,
                &format!("record field '{}'", prop.name.as_str()),
            )?;
            overrides.insert(SmolStr::new(prop.name.as_str()), value);
        }

        Ok(Value::Record {
            type_name: record_shape.record.name,
            fields: overrides,
        })
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nx_diagnostics::{TextSize, TextSpan};
    use nx_hir::{lower as lower_hir, SourceId};
    use nx_syntax::parse_str;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::sync::Arc;

    #[test]
    fn test_interpreter_creation() {
        let _interpreter = Interpreter::new();
        // Successfully created
    }

    #[test]
    fn test_function_not_found() {
        let interpreter = Interpreter::new();
        let module = LoweredModule::new(SourceId::new(0));
        let result = interpreter.execute_function(&module, "nonexistent", vec![]);
        assert!(result.is_err());
    }

    fn lower_module(source: &str) -> LoweredModule {
        let parse_result = parse_str(source, "handler-test.nx");
        assert!(
            parse_result.errors.is_empty(),
            "Expected handler test source to parse without errors, got {:?}",
            parse_result.errors
        );
        let tree = parse_result
            .tree
            .expect("Expected handler test source to parse");
        lower_hir(tree.root(), SourceId::new(0))
    }

    fn single_module_runtime(
        module: LoweredModule,
        identity: &str,
        fingerprint_seed: &str,
    ) -> (Arc<LoweredModule>, Interpreter) {
        let module = Arc::new(module);
        let mut hasher = DefaultHasher::new();
        identity.hash(&mut hasher);
        fingerprint_seed.hash(&mut hasher);
        let interpreter = Interpreter::from_resolved_program(ResolvedProgram::single_root_module(
            hasher.finish(),
            identity.to_string(),
            module.clone(),
        ));
        (module, interpreter)
    }

    fn lower_module_runtime(source: &str) -> (Arc<LoweredModule>, Interpreter) {
        single_module_runtime(lower_module(source), "handler-test.nx", source)
    }

    fn extract_handler<'a>(value: &'a Value, field: &str) -> &'a Value {
        let Value::Record { fields, .. } = value else {
            panic!(
                "Expected component invocation result record, got {:?}",
                value
            );
        };
        fields
            .get(field)
            .unwrap_or_else(|| panic!("Expected field '{}'", field))
    }

    fn extract_record_field<'a>(value: &'a Value, field: &str) -> &'a Value {
        let Value::Record { fields, .. } = value else {
            panic!("Expected record value, got {:?}", value);
        };

        fields
            .get(field)
            .unwrap_or_else(|| panic!("Expected field '{}'", field))
    }

    fn span(start: u32, end: u32) -> TextSpan {
        TextSpan::new(TextSize::from(start), TextSize::from(end))
    }

    #[test]
    fn test_bare_interpreter_rejects_action_handler_creation_without_resolved_program() {
        let source = r#"
            action SearchSubmitted = { searchString:string }
            action DoSearch = { search:string }

            component <SearchBox emits { SearchSubmitted } /> = {
              <TextInput />
            }

            let render() = <SearchBox onSearchSubmitted=<DoSearch search={action.searchString} /> />
        "#;

        let module = lower_module(source);
        let interpreter = Interpreter::new();
        let error = interpreter
            .execute_function(&module, "render", vec![])
            .expect_err("Expected bare interpreter to reject handler creation");

        assert!(matches!(
            error.kind(),
            RuntimeErrorKind::ResolvedProgramRequired { operation }
                if operation == "action handler creation"
        ));
    }

    #[test]
    fn test_bare_interpreter_rejects_component_snapshot_creation_without_resolved_program() {
        let source = r#"
            component <SearchBox /> = {
              <TextInput />
            }
        "#;

        let module = lower_module(source);
        let interpreter = Interpreter::new();
        let error = interpreter
            .initialize_component(
                &module,
                "SearchBox",
                Value::Record {
                    type_name: Name::new("object"),
                    fields: FxHashMap::default(),
                },
            )
            .expect_err("Expected bare interpreter to reject snapshot creation");

        assert!(matches!(
            error.kind(),
            RuntimeErrorKind::ResolvedProgramRequired { operation }
                if operation == "component state snapshot creation"
        ));
    }

    #[test]
    fn test_invoke_action_handler_captures_values_and_returns_single_action() {
        let source = r#"
            action SearchSubmitted = { searchString:string }
            action DoSearch = { userId:string search:string }

            component <SearchBox emits { SearchSubmitted } /> = {
              <TextInput />
            }

            let render(userId:string) = <SearchBox onSearchSubmitted=<DoSearch userId={userId} search={action.searchString} /> />
        "#;

        let (module, interpreter) = lower_module_runtime(source);
        let render_result = interpreter
            .execute_function(
                module.as_ref(),
                "render",
                vec![Value::String(SmolStr::new("u1"))],
            )
            .expect("Expected render to succeed");

        let handler = extract_handler(&render_result, "onSearchSubmitted");
        match handler {
            Value::ActionHandler {
                component,
                emit,
                action_name,
                captured,
                ..
            } => {
                assert_eq!(component.as_str(), "SearchBox");
                assert_eq!(emit.as_str(), "SearchSubmitted");
                assert_eq!(action_name.as_str(), "SearchSubmitted");
                assert_eq!(
                    captured.get("userId"),
                    Some(&Value::String(SmolStr::new("u1")))
                );
            }
            other => panic!("Expected action handler value, got {:?}", other),
        }

        let mut input_fields = FxHashMap::default();
        input_fields.insert(
            SmolStr::new("searchString"),
            Value::String(SmolStr::new("docs")),
        );
        let actions = interpreter
            .invoke_action_handler(
                module.as_ref(),
                handler,
                Value::Record {
                    type_name: Name::new("SearchSubmitted"),
                    fields: input_fields,
                },
            )
            .expect("Expected shared action handler invocation to succeed");

        assert_eq!(actions.len(), 1);
        match &actions[0] {
            Value::Record { type_name, fields } => {
                assert_eq!(type_name.as_str(), "DoSearch");
                assert_eq!(
                    fields.get("userId"),
                    Some(&Value::String(SmolStr::new("u1")))
                );
                assert_eq!(
                    fields.get("search"),
                    Some(&Value::String(SmolStr::new("docs")))
                );
            }
            other => panic!("Expected action record result, got {:?}", other),
        }
    }

    #[test]
    fn test_invoke_action_handler_shadows_outer_action_and_captures_other_values() {
        let source = r#"
            action SearchSubmitted = { searchString:string }
            action DoSearch = { source:string search:string }

            component <SearchBox emits { SearchSubmitted } /> = {
              <TextInput />
            }

            let render(action:string, source:string) = <SearchBox onSearchSubmitted=<DoSearch source={source} search={action.searchString} /> />
        "#;

        let (module, interpreter) = lower_module_runtime(source);
        let render_result = interpreter
            .execute_function(
                module.as_ref(),
                "render",
                vec![
                    Value::String(SmolStr::new("outer-action")),
                    Value::String(SmolStr::new("captured-source")),
                ],
            )
            .expect("Expected render to succeed");

        let handler = extract_handler(&render_result, "onSearchSubmitted");
        match handler {
            Value::ActionHandler { captured, .. } => {
                assert_eq!(
                    captured.get("source"),
                    Some(&Value::String(SmolStr::new("captured-source")))
                );
                assert!(
                    !captured.contains_key("action"),
                    "Implicit handler bindings should shadow any outer 'action' variable"
                );
            }
            other => panic!("Expected action handler value, got {:?}", other),
        }

        let mut input_fields = FxHashMap::default();
        input_fields.insert(
            SmolStr::new("searchString"),
            Value::String(SmolStr::new("docs")),
        );
        let actions = interpreter
            .invoke_action_handler(
                module.as_ref(),
                handler,
                Value::Record {
                    type_name: Name::new("SearchSubmitted"),
                    fields: input_fields,
                },
            )
            .expect("Expected shared action handler invocation to succeed");

        match &actions[0] {
            Value::Record { type_name, fields } => {
                assert_eq!(type_name.as_str(), "DoSearch");
                assert_eq!(
                    fields.get("source"),
                    Some(&Value::String(SmolStr::new("captured-source")))
                );
                assert_eq!(
                    fields.get("search"),
                    Some(&Value::String(SmolStr::new("docs")))
                );
            }
            other => panic!("Expected action record result, got {:?}", other),
        }
    }

    #[test]
    fn test_action_handler_captures_snapshot_not_live_reference() {
        let source = r#"
            action SearchSubmitted = { searchString:string }
            action DoSearch = { userId:string search:string }
        "#;

        let mut lowered_module = lower_module(source);
        let user_id_expr = lowered_module.alloc_expr(ast::Expr::Ident(Name::new("userId")));
        let action_ident = lowered_module.alloc_expr(ast::Expr::Ident(Name::new("action")));
        let search_expr = lowered_module.alloc_expr(ast::Expr::Member {
            base: action_ident,
            member: Name::new("searchString"),
            span: span(0, 0),
        });
        let body = lowered_module.alloc_expr(ast::Expr::RecordLiteral {
            record: Name::new("DoSearch"),
            properties: vec![
                (Name::new("userId"), user_id_expr),
                (Name::new("search"), search_expr),
            ],
            span: span(0, 0),
        });
        let handler_expr = lowered_module.alloc_expr(ast::Expr::ActionHandler {
            component: Name::new("SearchBox"),
            emit: Name::new("SearchSubmitted"),
            action_name: Name::new("SearchSubmitted"),
            body,
            span: span(0, 0),
        });

        let (module, interpreter) = single_module_runtime(
            lowered_module,
            "manual-handler-test.nx",
            "test_action_handler_captures_snapshot_not_live_reference",
        );
        let mut ctx = ExecutionContext::new();
        ctx.define_variable(SmolStr::new("userId"), Value::String(SmolStr::new("u1")));
        ctx.define_variable(
            SmolStr::new("action"),
            Value::String(SmolStr::new("outer-action")),
        );

        let handler = interpreter
            .eval_expr(module.as_ref(), &mut ctx, handler_expr)
            .expect("Expected handler expression to evaluate");
        ctx.update_variable("userId", Value::String(SmolStr::new("u2")))
            .expect("Expected outer variable update to succeed");

        let mut input_fields = FxHashMap::default();
        input_fields.insert(
            SmolStr::new("searchString"),
            Value::String(SmolStr::new("docs")),
        );
        let actions = interpreter
            .invoke_action_handler(
                module.as_ref(),
                &handler,
                Value::Record {
                    type_name: Name::new("SearchSubmitted"),
                    fields: input_fields,
                },
            )
            .expect("Expected handler invocation to use captured snapshot");

        match &actions[0] {
            Value::Record { type_name, fields } => {
                assert_eq!(type_name.as_str(), "DoSearch");
                assert_eq!(
                    fields.get("userId"),
                    Some(&Value::String(SmolStr::new("u1")))
                );
                assert_eq!(
                    fields.get("search"),
                    Some(&Value::String(SmolStr::new("docs")))
                );
            }
            other => panic!("Expected action record result, got {:?}", other),
        }
    }

    #[test]
    fn test_record_literal_reports_missing_record_type() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let body = module.alloc_expr(ast::Expr::RecordLiteral {
            record: Name::new("MissingRecord"),
            properties: vec![],
            span: span(0, 0),
        });
        module.add_item(Item::Function(Function {
            name: Name::new("make"),
            visibility: nx_hir::Visibility::Export,
            params: vec![],
            return_type: None,
            body,
            span: span(0, 0),
        }));

        let interpreter = Interpreter::new();
        let error = interpreter
            .execute_function(&module, "make", vec![])
            .expect_err("Expected missing record type to fail");

        assert_eq!(
            error.kind(),
            &RuntimeErrorKind::RecordTypeNotFound {
                name: SmolStr::new("MissingRecord"),
            }
        );
    }

    #[test]
    fn test_runtime_prepared_module_is_cached_per_runtime_module() {
        let source = r#"
            type User = { name:string }
            let root() = { <User name={"Ada"} /> }
        "#;

        let (module, interpreter) = lower_module_runtime(source);
        let first = interpreter.runtime_prepared_module(module.as_ref());
        let second = interpreter.runtime_prepared_module(module.as_ref());

        assert!(
            Arc::ptr_eq(&first, &second),
            "Expected runtime prepared modules to be cached per runtime module"
        );
    }

    #[test]
    fn test_invoke_action_handler_normalizes_host_supplied_action_defaults_and_nullable_fields() {
        let source = r#"
            action SearchSubmitted = { searchString:string = "docs" source:string? }
            action DoSearch = { search:string source:string? }

            component <SearchBox emits { SearchSubmitted } /> = {
              <TextInput />
            }

            let render() = <SearchBox onSearchSubmitted=<DoSearch search={action.searchString} source={action.source} /> />
        "#;

        let (module, interpreter) = lower_module_runtime(source);
        let render_result = interpreter
            .execute_function(module.as_ref(), "render", vec![])
            .expect("Expected render to succeed");
        let handler = extract_handler(&render_result, "onSearchSubmitted");

        let actions = interpreter
            .invoke_action_handler(
                module.as_ref(),
                handler,
                Value::Record {
                    type_name: Name::new("SearchSubmitted"),
                    fields: FxHashMap::default(),
                },
            )
            .expect("Expected handler input defaults to be normalized");

        match &actions[0] {
            Value::Record { type_name, fields } => {
                assert_eq!(type_name.as_str(), "DoSearch");
                assert_eq!(
                    fields.get("search"),
                    Some(&Value::String(SmolStr::new("docs")))
                );
                assert_eq!(fields.get("source"), Some(&Value::Null));
            }
            other => panic!("Expected action record result, got {:?}", other),
        }
    }

    #[test]
    fn test_invoke_action_handler_applies_inherited_inline_emit_defaults() {
        let source = r#"
            abstract action InputAction = { source:string = "ui" }
            action LogSearch = { source:string }

            component <SearchBox emits { ValueChanged extends InputAction { value:string } } /> = {
              <TextInput />
            }

            let render() = <SearchBox onValueChanged=<LogSearch source={action.source} /> />
        "#;

        let (module, interpreter) = lower_module_runtime(source);
        let render_result = interpreter
            .execute_function(module.as_ref(), "render", vec![])
            .expect("Expected render to succeed");
        let handler = extract_handler(&render_result, "onValueChanged");

        let mut input_fields = FxHashMap::default();
        input_fields.insert(SmolStr::new("value"), Value::String(SmolStr::new("docs")));

        let actions = interpreter
            .invoke_action_handler(
                module.as_ref(),
                handler,
                Value::Record {
                    type_name: Name::new("SearchBox.ValueChanged"),
                    fields: input_fields,
                },
            )
            .expect("Expected inherited action defaults to be normalized");

        match &actions[0] {
            Value::Record { type_name, fields } => {
                assert_eq!(type_name.as_str(), "LogSearch");
                assert_eq!(
                    fields.get("source"),
                    Some(&Value::String(SmolStr::new("ui")))
                );
            }
            other => panic!("Expected action record result, got {:?}", other),
        }
    }

    #[test]
    fn test_invoke_action_handler_rejects_missing_required_action_input_fields() {
        let source = r#"
            action SearchSubmitted = { searchString:string source:string? }
            action DoSearch = { search:string }

            component <SearchBox emits { SearchSubmitted } /> = {
              <TextInput />
            }

            let render() = <SearchBox onSearchSubmitted=<DoSearch search="ok" /> />
        "#;

        let (module, interpreter) = lower_module_runtime(source);
        let render_result = interpreter
            .execute_function(module.as_ref(), "render", vec![])
            .expect("Expected render to succeed");
        let handler = extract_handler(&render_result, "onSearchSubmitted");

        let error = interpreter
            .invoke_action_handler(
                module.as_ref(),
                handler,
                Value::Record {
                    type_name: Name::new("SearchSubmitted"),
                    fields: FxHashMap::default(),
                },
            )
            .expect_err("Expected missing required action field to fail before body evaluation");

        assert!(matches!(
            error.kind(),
            RuntimeErrorKind::MissingRequiredRecordField {
                record,
                field,
                operation,
            } if record == "SearchSubmitted"
                && field == "searchString"
                && operation == "action handler input for SearchBox.SearchSubmitted"
        ));
    }

    #[test]
    fn test_invoke_action_handler_rejects_missing_required_inherited_fields() {
        let source = r#"
            abstract action InputAction = { source:string }
            action LogSearch = { source:string }

            component <SearchBox emits { ValueChanged extends InputAction { value:string } } /> = {
              <TextInput />
            }

            let render() = <SearchBox onValueChanged=<LogSearch source={action.source} /> />
        "#;

        let (module, interpreter) = lower_module_runtime(source);
        let render_result = interpreter
            .execute_function(module.as_ref(), "render", vec![])
            .expect("Expected render to succeed");
        let handler = extract_handler(&render_result, "onValueChanged");

        let mut input_fields = FxHashMap::default();
        input_fields.insert(SmolStr::new("value"), Value::String(SmolStr::new("docs")));

        let error = interpreter
            .invoke_action_handler(
                module.as_ref(),
                handler,
                Value::Record {
                    type_name: Name::new("SearchBox.ValueChanged"),
                    fields: input_fields,
                },
            )
            .expect_err("Expected missing inherited action field to fail");

        assert!(matches!(
            error.kind(),
            RuntimeErrorKind::MissingRequiredRecordField {
                record,
                field,
                operation,
            } if record == "SearchBox.ValueChanged"
                && field == "source"
                && operation == "action handler input for SearchBox.ValueChanged"
        ));
    }

    #[test]
    fn test_invoke_action_handler_supports_inline_emit_public_names_and_multiple_actions() {
        let source = r#"
            action LogSearch = { value:string }
            action TrackSearch = { value:string }

            component <SearchBox emits { ValueChanged { value:string } } /> = {
              <TextInput />
            }

            let render() = <SearchBox onValueChanged={<LogSearch value={action.value} /> <TrackSearch value={action.value} />} />
        "#;

        let (module, interpreter) = lower_module_runtime(source);
        let render_result = interpreter
            .execute_function(module.as_ref(), "render", vec![])
            .expect("Expected render to succeed");
        let handler = extract_handler(&render_result, "onValueChanged");
        match handler {
            Value::ActionHandler {
                component,
                emit,
                action_name,
                ..
            } => {
                assert_eq!(component.as_str(), "SearchBox");
                assert_eq!(emit.as_str(), "ValueChanged");
                assert_eq!(action_name.as_str(), "SearchBox.ValueChanged");
            }
            other => panic!("Expected action handler value, got {:?}", other),
        }

        let mut input_fields = FxHashMap::default();
        input_fields.insert(SmolStr::new("value"), Value::String(SmolStr::new("docs")));

        let actions = interpreter
            .invoke_action_handler(
                module.as_ref(),
                &handler,
                Value::Record {
                    type_name: Name::new("SearchBox.ValueChanged"),
                    fields: input_fields,
                },
            )
            .expect("Expected inline action handler invocation to succeed");

        assert_eq!(actions.len(), 2);
        match &actions[0] {
            Value::Record { type_name, fields } => {
                assert_eq!(type_name.as_str(), "LogSearch");
                assert_eq!(
                    fields.get("value"),
                    Some(&Value::String(SmolStr::new("docs")))
                );
            }
            other => panic!("Expected first action record, got {:?}", other),
        }
        match &actions[1] {
            Value::Record { type_name, fields } => {
                assert_eq!(type_name.as_str(), "TrackSearch");
                assert_eq!(
                    fields.get("value"),
                    Some(&Value::String(SmolStr::new("docs")))
                );
            }
            other => panic!("Expected second action record, got {:?}", other),
        }
    }

    #[test]
    fn test_invoke_action_handler_rejects_empty_and_non_action_results() {
        let source = r#"
            action SearchSubmitted = { queries:string[] }
            action DoSearch = { search:string }

            component <SearchBox emits { SearchSubmitted } /> = {
              <TextInput />
            }

            let renderEmpty() = <SearchBox onSearchSubmitted={for query in action.queries { <DoSearch search={query} /> }} />
            let renderWrong() = <SearchBox onSearchSubmitted="docs" />
        "#;

        let (module, interpreter) = lower_module_runtime(source);
        let empty_render_result = interpreter
            .execute_function(module.as_ref(), "renderEmpty", vec![])
            .expect("Expected empty-result render to succeed");
        let empty_handler = extract_handler(&empty_render_result, "onSearchSubmitted");

        let wrong_render_result = interpreter
            .execute_function(module.as_ref(), "renderWrong", vec![])
            .expect("Expected wrong-result render to succeed");
        let wrong_handler = extract_handler(&wrong_render_result, "onSearchSubmitted");

        let mut empty_input_fields = FxHashMap::default();
        empty_input_fields.insert(SmolStr::new("queries"), Value::Array(vec![]));
        let empty_action_input = Value::Record {
            type_name: Name::new("SearchSubmitted"),
            fields: empty_input_fields,
        };

        let empty_error = interpreter
            .invoke_action_handler(module.as_ref(), empty_handler, empty_action_input)
            .expect_err("Expected empty handler result to fail");
        assert!(matches!(
            empty_error.kind(),
            RuntimeErrorKind::TypeMismatch { expected, actual, .. }
                if expected == "one or more actions" && actual == "empty array"
        ));

        let mut wrong_input_fields = FxHashMap::default();
        wrong_input_fields.insert(
            SmolStr::new("queries"),
            Value::Array(vec![Value::String(SmolStr::new("docs"))]),
        );
        let wrong_action_input = Value::Record {
            type_name: Name::new("SearchSubmitted"),
            fields: wrong_input_fields,
        };

        let wrong_error = interpreter
            .invoke_action_handler(module.as_ref(), wrong_handler, wrong_action_input)
            .expect_err("Expected non-action handler result to fail");
        assert!(matches!(
            wrong_error.kind(),
            RuntimeErrorKind::TypeMismatch { expected, actual, .. }
                if expected == "action or action array" && actual == "string"
        ));
    }

    #[test]
    fn test_invoke_action_handler_rejects_wrong_action_type_name() {
        let source = r#"
            action SearchSubmitted = { searchString:string }
            action ValueChanged = { searchString:string }
            action DoSearch = { search:string }

            component <SearchBox emits { SearchSubmitted } /> = {
              <TextInput />
            }

            let render() = <SearchBox onSearchSubmitted=<DoSearch search={action.searchString} /> />
        "#;

        let (module, interpreter) = lower_module_runtime(source);
        let render_result = interpreter
            .execute_function(module.as_ref(), "render", vec![])
            .expect("Expected render to succeed");
        let handler = extract_handler(&render_result, "onSearchSubmitted");

        let mut input_fields = FxHashMap::default();
        input_fields.insert(
            SmolStr::new("searchString"),
            Value::String(SmolStr::new("docs")),
        );
        let error = interpreter
            .invoke_action_handler(
                module.as_ref(),
                handler,
                Value::Record {
                    type_name: Name::new("ValueChanged"),
                    fields: input_fields,
                },
            )
            .expect_err("Expected wrong action type name to fail");

        assert!(matches!(
            error.kind(),
            RuntimeErrorKind::TypeMismatch { expected, actual, .. }
                if expected == "SearchSubmitted" && actual == "ValueChanged"
        ));
    }

    #[test]
    fn test_invoke_action_handler_rejects_non_action_record_results() {
        let source = r#"
            action SearchSubmitted = { searchString:string }
            type SearchResult = { search:string }

            component <SearchBox emits { SearchSubmitted } /> = {
              <TextInput />
            }

            let renderRecord() = <SearchBox onSearchSubmitted=<SearchResult search={action.searchString} /> />
        "#;

        let (module, interpreter) = lower_module_runtime(source);
        let render_result = interpreter
            .execute_function(module.as_ref(), "renderRecord", vec![])
            .expect("Expected non-action render to succeed");
        let handler = extract_handler(&render_result, "onSearchSubmitted");

        let mut input_fields = FxHashMap::default();
        input_fields.insert(
            SmolStr::new("searchString"),
            Value::String(SmolStr::new("docs")),
        );
        let error = interpreter
            .invoke_action_handler(
                module.as_ref(),
                handler,
                Value::Record {
                    type_name: Name::new("SearchSubmitted"),
                    fields: input_fields,
                },
            )
            .expect_err("Expected non-action record handler result to fail");

        assert!(matches!(
            error.kind(),
            RuntimeErrorKind::TypeMismatch { expected, actual, .. }
                if expected == "action" && actual == "SearchResult"
        ));
    }

    #[test]
    fn test_initialize_component_applies_prop_and_state_defaults_once() {
        let source = r#"
            component <SearchBox placeholder:string = "Find docs" /> = {
              state {
                query:string = {placeholder}
                preview:string = {query}
              }
              <TextInput value={preview} placeholder={placeholder} />
            }
        "#;

        let (module, interpreter) = lower_module_runtime(source);
        let init = interpreter
            .initialize_component(
                module.as_ref(),
                "SearchBox",
                Value::Record {
                    type_name: Name::new("object"),
                    fields: FxHashMap::default(),
                },
            )
            .expect("Expected component initialization to succeed");

        let rendered_value = extract_record_field(&init.rendered, "value");
        assert_eq!(rendered_value, &Value::String(SmolStr::new("Find docs")));
        let rendered_placeholder = extract_record_field(&init.rendered, "placeholder");
        assert_eq!(
            rendered_placeholder,
            &Value::String(SmolStr::new("Find docs"))
        );

        let snapshot = interpreter
            .decode_component_snapshot(module.as_ref(), &init.state_snapshot)
            .expect("Expected snapshot to decode");
        assert_eq!(
            snapshot.props.get("placeholder"),
            Some(&Value::String(SmolStr::new("Find docs")))
        );
        assert_eq!(
            snapshot.state.get("query"),
            Some(&Value::String(SmolStr::new("Find docs")))
        );
        assert_eq!(
            snapshot.state.get("preview"),
            Some(&Value::String(SmolStr::new("Find docs")))
        );
    }

    #[test]
    fn test_initialize_component_succeeds_without_state_group() {
        let source = r#"
            component <Button text:string /> = {
              <button>{text}</button>
            }
        "#;

        let (module, interpreter) = lower_module_runtime(source);
        let mut props = FxHashMap::default();
        props.insert(SmolStr::new("text"), Value::String(SmolStr::new("Save")));

        let init = interpreter
            .initialize_component(
                module.as_ref(),
                "Button",
                Value::Record {
                    type_name: Name::new("object"),
                    fields: props,
                },
            )
            .expect("Expected stateless component initialization to succeed");

        let snapshot = interpreter
            .decode_component_snapshot(module.as_ref(), &init.state_snapshot)
            .expect("Expected snapshot to decode");
        assert!(snapshot.state.is_empty(), "Expected empty component state");
        assert_eq!(
            extract_record_field(&init.rendered, "content"),
            &Value::String(SmolStr::new("Save"))
        );
    }

    #[test]
    fn test_initialize_component_rejects_missing_required_state_initializer() {
        let source = r#"
            component <SearchBox /> = {
              state { query:string }
              <TextInput value={query} />
            }
        "#;

        let (module, interpreter) = lower_module_runtime(source);
        let error = interpreter
            .initialize_component(
                module.as_ref(),
                "SearchBox",
                Value::Record {
                    type_name: Name::new("object"),
                    fields: FxHashMap::default(),
                },
            )
            .expect_err("Expected missing required state field to fail");

        assert!(matches!(
            error.kind(),
            RuntimeErrorKind::MissingRequiredComponentField {
                component,
                field,
                phase,
            } if component == "SearchBox"
                && field == "query"
                && phase == "state initialization"
        ));
    }

    #[test]
    fn test_dispatch_component_actions_preserves_state_snapshot_without_reevaluating_defaults() {
        let source = r#"
            component <SearchBox placeholder:string = "Find docs" emits { SearchSubmitted } /> = {
              state { query:string = {placeholder} }
              <TextInput value={query} placeholder={placeholder} />
            }
        "#;

        let (module, interpreter) = lower_module_runtime(source);
        let init = interpreter
            .initialize_component(
                module.as_ref(),
                "SearchBox",
                Value::Record {
                    type_name: Name::new("object"),
                    fields: FxHashMap::default(),
                },
            )
            .expect("Expected initialization to succeed");
        let mut snapshot = interpreter
            .decode_component_snapshot(module.as_ref(), &init.state_snapshot)
            .expect("Expected snapshot to decode");
        snapshot.state.insert(
            SmolStr::new("query"),
            Value::String(SmolStr::new("persisted")),
        );
        let mutated_snapshot = interpreter
            .encode_component_snapshot(
                snapshot.component_module_id,
                &snapshot.component,
                &snapshot.props,
                &snapshot.state,
            )
            .expect("Expected snapshot to encode");

        let dispatch = interpreter
            .dispatch_component_actions(module.as_ref(), &mutated_snapshot, vec![])
            .expect("Expected empty dispatch to succeed");
        let next_snapshot = interpreter
            .decode_component_snapshot(module.as_ref(), &dispatch.state_snapshot)
            .expect("Expected next snapshot to decode");

        assert_eq!(
            next_snapshot.state.get("query"),
            Some(&Value::String(SmolStr::new("persisted")))
        );
    }

    #[test]
    fn test_dispatch_component_actions_orders_host_actions_and_handler_effects() {
        let source = r#"
            action SearchSubmitted = { searchString:string }
            action LogSearch = { search:string }
            action DoSearch = { search:string }
            action TrackSearch = { value:string }

            component <SearchBox placeholder:string emits {
                SearchSubmitted
                ValueChanged { value:string }
            } /> = {
              state { query:string = {placeholder} }
              <TextInput value={query} placeholder={placeholder} />
            }

            let withHandlers() = <SearchBox
                placeholder="Find docs"
                onSearchSubmitted={<LogSearch search={action.searchString} /> <DoSearch search={action.searchString} />}
                onValueChanged=<TrackSearch value={action.value} />
            />
            let withoutHandlers() = <SearchBox placeholder="Find docs" />
        "#;

        let (module, interpreter) = lower_module_runtime(source);

        let props_with_handlers = interpreter
            .execute_function(module.as_ref(), "withHandlers", vec![])
            .expect("Expected handler-bound props to evaluate");
        let init = interpreter
            .initialize_component(module.as_ref(), "SearchBox", props_with_handlers)
            .expect("Expected component initialization with handlers to succeed");
        let dispatch = interpreter
            .dispatch_component_actions(
                module.as_ref(),
                &init.state_snapshot,
                vec![
                    Value::Record {
                        type_name: Name::new("SearchSubmitted"),
                        fields: FxHashMap::from_iter([(
                            SmolStr::new("searchString"),
                            Value::String(SmolStr::new("docs")),
                        )]),
                    },
                    Value::Record {
                        type_name: Name::new("SearchBox.ValueChanged"),
                        fields: FxHashMap::from_iter([(
                            SmolStr::new("value"),
                            Value::String(SmolStr::new("docs")),
                        )]),
                    },
                ],
            )
            .expect("Expected ordered dispatch to succeed");

        let effect_names: Vec<_> = dispatch
            .effects
            .iter()
            .map(|effect| match effect {
                Value::Record { type_name, .. } => type_name.as_str().to_string(),
                other => panic!("Expected effect action record, got {:?}", other),
            })
            .collect();
        assert_eq!(effect_names, vec!["LogSearch", "DoSearch", "TrackSearch"]);

        let props_without_handlers = interpreter
            .execute_function(module.as_ref(), "withoutHandlers", vec![])
            .expect("Expected handler-free props to evaluate");
        let init_without_handlers = interpreter
            .initialize_component(module.as_ref(), "SearchBox", props_without_handlers)
            .expect("Expected component initialization without handlers to succeed");
        let empty_dispatch = interpreter
            .dispatch_component_actions(
                module.as_ref(),
                &init_without_handlers.state_snapshot,
                vec![Value::Record {
                    type_name: Name::new("SearchSubmitted"),
                    fields: FxHashMap::from_iter([(
                        SmolStr::new("searchString"),
                        Value::String(SmolStr::new("docs")),
                    )]),
                }],
            )
            .expect("Expected dispatch without handlers to succeed");
        assert!(empty_dispatch.effects.is_empty());
    }

    #[test]
    fn test_dispatch_component_actions_rejects_undeclared_action_types() {
        let source = r#"
            action SearchSubmitted = { searchString:string }
            action ValueChanged = { value:string }

            component <SearchBox emits { SearchSubmitted } /> = {
              <TextInput />
            }
        "#;

        let (module, interpreter) = lower_module_runtime(source);
        let init = interpreter
            .initialize_component(
                module.as_ref(),
                "SearchBox",
                Value::Record {
                    type_name: Name::new("object"),
                    fields: FxHashMap::default(),
                },
            )
            .expect("Expected component initialization to succeed");

        let error = interpreter
            .dispatch_component_actions(
                module.as_ref(),
                &init.state_snapshot,
                vec![Value::Record {
                    type_name: Name::new("ValueChanged"),
                    fields: FxHashMap::from_iter([(
                        SmolStr::new("value"),
                        Value::String(SmolStr::new("docs")),
                    )]),
                }],
            )
            .expect_err("Expected undeclared action type to fail");

        assert!(matches!(
            error.kind(),
            RuntimeErrorKind::UnsupportedComponentAction { component, action }
                if component == "SearchBox" && action == "ValueChanged"
        ));
    }

    #[test]
    fn test_dispatch_component_actions_rejects_non_record_action_values() {
        let source = r#"
            action SearchSubmitted = { searchString:string }

            component <SearchBox emits { SearchSubmitted } /> = {
              <TextInput />
            }
        "#;

        let (module, interpreter) = lower_module_runtime(source);
        let init = interpreter
            .initialize_component(
                module.as_ref(),
                "SearchBox",
                Value::Record {
                    type_name: Name::new("object"),
                    fields: FxHashMap::default(),
                },
            )
            .expect("Expected component initialization to succeed");

        let error = interpreter
            .dispatch_component_actions(
                module.as_ref(),
                &init.state_snapshot,
                vec![Value::String(SmolStr::new("docs"))],
            )
            .expect_err("Expected non-record action input to fail");

        assert!(matches!(
            error.kind(),
            RuntimeErrorKind::TypeMismatch { expected, actual, .. }
                if expected == "action" && actual == "string"
        ));
    }

    #[test]
    fn test_dispatch_component_actions_rejects_invalid_or_incompatible_snapshots() {
        let source = r#"
            action SearchSubmitted = { searchString:string }
            action DoSearch = { search:string }

            component <SearchBox emits { SearchSubmitted } /> = {
              <TextInput />
            }

            let withHandler() = <SearchBox onSearchSubmitted=<DoSearch search={action.searchString} /> />
        "#;
        let other_module_source = r#"
            component <OtherButton text:string /> = {
              <button>{text}</button>
            }
        "#;

        let (module, interpreter) = lower_module_runtime(source);
        let (other_module, other_interpreter) = single_module_runtime(
            lower_module(other_module_source),
            "other-module-test.nx",
            other_module_source,
        );

        let malformed_error = interpreter
            .dispatch_component_actions(module.as_ref(), b"not-a-valid-snapshot", vec![])
            .expect_err("Expected malformed snapshot to fail");
        assert!(matches!(
            malformed_error.kind(),
            RuntimeErrorKind::InvalidComponentStateSnapshot { .. }
        ));

        let init = interpreter
            .initialize_component(
                module.as_ref(),
                "SearchBox",
                Value::Record {
                    type_name: Name::new("object"),
                    fields: FxHashMap::default(),
                },
            )
            .expect("Expected component initialization to succeed");
        let incompatible_error = other_interpreter
            .dispatch_component_actions(other_module.as_ref(), &init.state_snapshot, vec![])
            .expect_err("Expected incompatible snapshot to fail");
        assert!(matches!(
            incompatible_error.kind(),
            RuntimeErrorKind::InvalidComponentStateSnapshot { reason }
                if reason.contains("snapshot fingerprint")
        ));
    }
}
