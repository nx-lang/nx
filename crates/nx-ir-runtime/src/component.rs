//! Components: descriptors, instances, handlers and dispatch.

use crate::error::{fail, Result};
use crate::eval::{bind, bind_content, Caller, Cx, Frame, Machine, Meter, RuntimeOptions};
use crate::input::Measure;
use crate::module::{ComponentDecl, DeclarationKind, Node};
use crate::normalize::{require_record, Labeled, Path};
use crate::program::{Program, ProgramData};
use crate::stored::{refuse, Stored};
use crate::value::{
    fields_from_host, fields_to_host, from_host, get_field, handlers_equal, to_host, too_deep,
    Depths, Fields, Handler, Record, Tokens, Value, ACTION_HANDLER_TYPE, HANDLER_INVOCATION_TYPE,
    MAX_VALUE_DEPTH,
};
use nx_value::NxValue;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

pub(crate) type HandlerProps = Vec<(Arc<str>, Arc<Handler>)>;

#[derive(Debug)]
pub(crate) struct InstanceData {
    /// The identity and image hash of every module of the program that rendered the instance.
    pub program: Arc<[(Arc<str>, u64)]>,
    pub component: Arc<str>,
    /// The declared props, normalized.
    pub props: Fields,
    /// The handlers the parent bound, by property name (`onTapped`). Never in the body's scope.
    pub handler_props: HandlerProps,
    pub state: Fields,
    /// The handlers in the most recent rendered output, in the order their tokens number them.
    pub handlers: Vec<Arc<Handler>>,
    /// The render generation the tokens belong to.
    pub generation: u64,
}

/// A component instance: what dispatch needs to run a batch against a component that
/// initialization or a previous dispatch rendered.
///
/// <para>An instance is never modified; every dispatch returns a new one, and the one it was
/// given stays valid for a retry. It is plain data and serializes with any `serde` format. The
/// only way back from the serialized form is [`Program::restore_component_instance`], which
/// checks the instance against the program, so an instance a caller holds either came from this
/// runtime or passed that check, and dispatch does not check it again.</para>
#[derive(Debug, Clone)]
pub struct ComponentInstance {
    data: Arc<InstanceData>,
}

impl ComponentInstance {
    /// The name of the component the instance is of.
    pub fn component(&self) -> &str {
        &self.data.component
    }

    /// The render generation the instance's handler tokens belong to.
    pub fn generation(&self) -> u64 {
        self.data.generation
    }

    /// Whether the handler this instance holds under `token` is the handler `other` holds under
    /// `other_token`: the same handler node with the same capture.
    pub fn same_handler(&self, token: &str, other: &ComponentInstance, other_token: &str) -> bool {
        match (self.data.handler(token), other.data.handler(other_token)) {
            (Some(left), Some(right)) => same_handler(left, right),
            _ => false,
        }
    }
    /// Whether the handler a parent bound on this instance under `property` (`onTapped`) is the
    /// handler `parent` holds under `token`: what a child initialized from its parent's rendered
    /// output received.
    pub fn bound_handler_is(
        &self,
        property: &str,
        parent: &ComponentInstance,
        token: &str,
    ) -> bool {
        let bound = self
            .data
            .handler_props
            .iter()
            .find(|(name, _)| &**name == property);
        match (bound, parent.data.handler(token)) {
            (Some((_, left)), Some(right)) => same_handler(left, right),
            _ => false,
        }
    }
}

/// Whether two handlers a host holds are one handler. No evaluation pays for the comparison.
///
/// <para>A handler a host was handed and hands back is the same allocation, which is the case
/// this exists for, so that is asked first: it is not what the language's `==` asks, where a
/// capture that holds a NaN makes a handler unequal to itself.</para>
fn same_handler(left: &Arc<Handler>, right: &Arc<Handler>) -> bool {
    Arc::ptr_eq(left, right) || handlers_equal(left, right, &Meter::free()).unwrap_or(false)
}

impl Serialize for ComponentInstance {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        Stored::of(&self.data).serialize(serializer)
    }
}

impl InstanceData {
    /// The handler a token of this instance's rendered output names.
    fn handler(&self, token: &str) -> Option<&Arc<Handler>> {
        let number = token
            .strip_prefix('h')?
            .strip_prefix(self.generation.to_string().as_str())?
            .strip_prefix('-')?;
        // A token is written without padding, so only the canonical spelling names a handler.
        let index: usize = number.parse().ok()?;
        if index.to_string() != number {
            return None;
        }
        self.handlers.get(index.checked_sub(1)?)
    }
}

/// What to initialize a component with, beyond its props.
#[derive(Debug, Clone, Copy, Default)]
pub struct ComponentInit<'a> {
    /// The instance whose rendered output the props were read from. Every `ActionHandler` record
    /// in the props, at any depth, is replaced by the handler that instance holds under the
    /// record's token, which is how a parent's binding reaches a child the host initializes.
    pub parent: Option<&'a ComponentInstance>,
    /// The state to use in place of the initial one, validated as a complete state for the
    /// component. Initializing again with the state an instance holds and new props is how a
    /// host re-renders an instance whose props changed without losing its state.
    pub state: Option<&'a BTreeMap<String, NxValue>>,
}

#[derive(Debug, Clone)]
pub struct ComponentInitResult {
    pub rendered: NxValue,
    pub state: BTreeMap<String, NxValue>,
    pub instance: ComponentInstance,
}

#[derive(Debug, Clone)]
pub struct ComponentDispatchResult {
    /// The body rendered against the next state, its handlers carrying fresh tokens.
    pub rendered: NxValue,
    /// Everything the handlers returned for the host, in dispatch order.
    pub effects: Vec<NxValue>,
    pub state: BTreeMap<String, NxValue>,
    pub instance: ComponentInstance,
}

/// The property a parent binds a handler for `emit` under: `onTapped` for `Tapped`.
fn handler_property(emit: &str) -> String {
    format!("on{emit}")
}

fn is_handler_record(value: &Value) -> bool {
    value
        .as_record()
        .is_some_and(|record| record.type_name() == Some(ACTION_HANDLER_TYPE))
}

/// Replaces every `ActionHandler` record in host-supplied props, at any depth, by the handler
/// the parent instance holds under its token. Without a parent such a record names nothing.
fn resolve_parent_handlers(
    value: &Value,
    parent: Option<&InstanceData>,
    path: &Path<'_>,
    depth: u32,
) -> Result<Value> {
    if depth > MAX_VALUE_DEPTH {
        return too_deep();
    }
    let deeper = depth.saturating_add(1);
    match value {
        Value::Seq(items) => Ok(Value::seq(
            items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    resolve_parent_handlers(item, parent, &Path::Index(path, index), deeper)
                })
                .collect::<Result<_>>()?,
        )),
        Value::Record(record) if record.type_name() == Some(ACTION_HANDLER_TYPE) => {
            let Some(parent) = parent else {
                return fail(
                    "nx-ir-boundary-field",
                    format!("Unknown field {path}: an ActionHandler record names a handler only through a parent instance."),
                );
            };
            let token = record.get("token").and_then(Value::as_text).unwrap_or("");
            match parent.handler(token) {
                Some(handler) => Ok(Value::Handler(Arc::clone(handler))),
                None => fail(
                    "nx-ir-handler-token",
                    format!(
                        "Unknown handler token '{token}' at {path} for the '{}' instance.",
                        parent.component
                    ),
                ),
            }
        }
        Value::Record(record) => Ok(Value::record(
            record.type_name.clone(),
            record
                .fields
                .iter()
                .map(|(name, item)| {
                    Ok((
                        Arc::clone(name),
                        resolve_parent_handlers(item, parent, &Path::Field(path, name), deeper)?,
                    ))
                })
                .collect::<Result<_>>()?,
        )),
        other => Ok(other.clone()),
    }
}

impl<'p> Machine<'p> {
    fn component(&self, name: &str) -> Result<(u32, &'p Arc<str>, &'p ComponentDecl)> {
        let entry = &self.program.entry;
        match entry.entrypoint(&entry.component_entrypoints, name) {
            Some((index, declaration)) => match &declaration.kind {
                DeclarationKind::Component(component) => Ok((index, &declaration.name, component)),
                _ => fail(
                    "nx-ir-component",
                    format!("Component '{name}' was not found."),
                ),
            },
            None => fail(
                "nx-ir-component",
                format!("Component '{name}' was not found."),
            ),
        }
    }

    /// Splits a component's handler properties from its declared props.
    ///
    /// <para>A property named `on<Emit>` for an emit the component declares is a handler
    /// property: it is not a prop, a body cannot read it, and it goes around normalization to
    /// ride on the descriptor or the instance. Its value must be a handler for that very emit of
    /// that very component. A handler under any other name matches no emit, and an
    /// `ActionHandler` record names a handler only through a parent instance.</para>
    pub(crate) fn split_handler_properties(
        &self,
        module: u32,
        index: u32,
        input: Fields,
        path: &dyn fmt::Display,
    ) -> Result<(Fields, HandlerProps)> {
        let declaration = self
            .program
            .linked(module)
            .and_then(|linked| linked.module.declaration(index));
        let Some((name, DeclarationKind::Component(component))) =
            declaration.map(|declaration| (&declaration.name, &declaration.kind))
        else {
            return fail("nx-ir-component", "The declaration is not a component.");
        };
        let mut fields = Vec::with_capacity(input.len());
        let mut handlers = Vec::new();
        for (key, value) in input {
            let emit = key.strip_prefix("on").and_then(|emit| {
                component
                    .emits
                    .iter()
                    .find(|candidate| &*candidate.name == emit)
            });
            let Some(emit) = emit else {
                if matches!(value, Value::Handler(_)) {
                    return fail(
                        "nx-ir-boundary-field",
                        format!("Unknown {path} field '{key}': '{name}' emits nothing a handler named '{key}' would answer."),
                    );
                }
                fields.push((key, value));
                continue;
            };
            if is_handler_record(&value) {
                return fail(
                    "nx-ir-boundary-field",
                    format!("Unknown {path} field '{key}': an ActionHandler record names a handler only through a parent instance."),
                );
            }
            let Value::Handler(handler) = value else {
                return fail(
                    "nx-ir-type",
                    format!(
                        "Expected {path}.{key} to be an action handler for {name}.{}.",
                        emit.name
                    ),
                );
            };
            let component_key = format!("{}::{name}", self.program.identity(module));
            if *handler.component != *component_key || handler.emit != emit.name {
                // Two components of one name in different modules are told apart by their keys.
                let (expected, got) = if handler.component_name == *name {
                    (component_key.as_str(), &*handler.component)
                } else {
                    (&**name, &*handler.component_name)
                };
                return fail(
                    "nx-ir-type",
                    format!(
                        "Expected {path}.{key} to be an action handler for {expected}.{}, got one for {got}.{}.",
                        emit.name, handler.emit
                    ),
                );
            }
            handlers.push((key, handler));
        }
        Ok((fields, handlers))
    }

    /// Constructs a host-supplied action against the record the emit declares, defaults and all.
    fn normalize_action(
        &self,
        module: u32,
        index: u32,
        input: &Record,
        path: &dyn fmt::Display,
    ) -> Result<Value> {
        let declaration = self
            .program
            .linked(module)
            .and_then(|linked| linked.module.declaration(index));
        let Some((expected, DeclarationKind::Record(record))) =
            declaration.map(|declaration| (&declaration.name, &declaration.kind))
        else {
            return fail("nx-ir-type", "The action is not an action record.");
        };
        if let Some(discriminator) = input.type_name().filter(|name| *name != &**expected) {
            return fail(
                "nx-ir-type",
                format!("Expected {path} to be a '{expected}' action, got '{discriminator}'."),
            );
        }
        let cx = Cx {
            module,
            declaration: index,
            depth: 0,
        };
        Ok(Value::record(
            Some(Arc::clone(expected)),
            self.normalize_fields(
                cx,
                &record.fields,
                &input.fields,
                &mut Vec::new(),
                path,
                false,
            )?,
        ))
    }

    /// Runs a handler: the action is constructed against the record the handler accepts, the
    /// body runs over the captured frame with `action` in its slot, and, for a handler its own
    /// component dispatches, `live` is that component and its working state, whose slots the
    /// body reads instead of what was captured. The result is one record or a non-empty list of
    /// them.
    fn invoke_handler(
        &self,
        handler: &Handler,
        action: &Record,
        live: Option<(&ComponentDecl, &Fields)>,
    ) -> Result<Vec<Value>> {
        let label = Labeled2(&handler.component_name, &handler.emit);
        let stale = || {
            fail(
                "nx-ir-component",
                format!("Handler {label} is not a handler of this program."),
            )
        };
        let Some(module) = self.program.by_identity.get(&handler.module).copied() else {
            return stale();
        };
        let Some(linked) = self.program.linked(module) else {
            return stale();
        };
        let Some((declaration, _)) = linked.module.find(&handler.declaration) else {
            return stale();
        };
        let Ok(Node::ActionHandler(node)) = linked.module.node(handler.node) else {
            return stale();
        };
        let Ok((action_module, action_index, action_declaration)) =
            self.program.resolve(module, &node.action)
        else {
            return stale();
        };
        let expected = &action_declaration.name;
        if action.type_name() != Some(&**expected) {
            return fail(
                "nx-ir-type",
                format!(
                    "Expected an action of type '{expected}' for handler {label}, got '{}'.",
                    action.type_name().unwrap_or("undefined")
                ),
            );
        }
        let action = self.normalize_action(
            action_module,
            action_index,
            action,
            &Labeled3(&label, " action"),
        )?;
        let mut frame: Frame = handler.captured.clone();
        if let Some((component, state)) = live {
            for (offset, field) in component.state.iter().enumerate() {
                let value = get_field(state, &field.name)
                    .cloned()
                    .unwrap_or_else(Value::empty);
                bind(
                    &mut frame,
                    component.props.len().saturating_add(offset),
                    value,
                )?;
            }
        }
        bind(&mut frame, node.action_slot as usize, action)?;
        let cx = Cx {
            module,
            declaration,
            depth: 0,
        };
        let results = match self.eval(cx, &mut frame, node.body)? {
            Value::Seq(items) => items.to_vec(),
            other => vec![other],
        };
        if results.is_empty() {
            return fail(
                "nx-ir-handler-result",
                format!("Handler {label} returned an empty list; a handler returns an action, an update record, or a list of them."),
            );
        }
        for item in &results {
            if !item
                .as_record()
                .is_some_and(|record| record.type_name.is_some())
            {
                return fail(
                    "nx-ir-handler-result",
                    format!(
                        "Handler {label} returned {}; a handler returns an action, an update record, or a list of them.",
                        item.describe()
                    ),
                );
            }
        }
        Ok(results)
    }

    /// Whether `value` is the update record of the component with `owner_key`.
    fn is_update_record_for(&self, value: &Value, owner_key: &str) -> bool {
        let Some(discriminator) = value.as_record().and_then(Record::type_name) else {
            return false;
        };
        self.program.shapes(discriminator).any(|(module, shape)| {
            shape
                .update_target
                .as_ref()
                .is_some_and(|target| self.program.key(module, target) == owner_key)
        })
    }

    /// Applies a patch to a component's state with full validation.
    fn patch_state(
        &self,
        index: u32,
        name: &str,
        component: &ComponentDecl,
        current: &[(Arc<str>, Value)],
        patch: &Record,
        depths: &mut Depths,
    ) -> Result<Fields> {
        let expected = format!("{name}.Update");
        if let Some(discriminator) = patch
            .type_name()
            .filter(|discriminator| *discriminator != expected)
        {
            return fail(
                "nx-ir-state-patch",
                format!(
                    "Cannot apply '{discriminator}' to {name} state; only '{expected}' patches it."
                ),
            );
        }
        let mut merged: Fields = current.to_vec();
        for (key, value) in &patch.fields {
            if !component.state.iter().any(|field| field.name == *key) {
                return fail(
                    "nx-ir-state-field",
                    format!("Unknown {name} state field '{key}'."),
                );
            }
            crate::value::set_field(&mut merged, Arc::clone(key), value.clone());
        }
        let next = self.normalize_fields(
            entry_cx(index),
            &component.state,
            &merged,
            &mut props_frame(component),
            &Labeled(name, " state"),
            true,
        )?;
        // A batch can nest the state one level per entry, and nothing else looks at the state
        // until the batch is over. Only a field the patch supplies can have changed, so only
        // those are walked, and `depths` remembers what earlier patches of the call found: a
        // patch that touches a counter does not pay for the rest of the state, nor one that
        // supplies a large value again for walking it again.
        depths.check(
            next.iter()
                .filter(|(name, _)| get_field(&patch.fields, name).is_some()),
        )?;
        Ok(next)
    }
}

fn entry_cx(declaration: u32) -> Cx {
    Cx {
        module: 0,
        declaration,
        depth: 0,
    }
}

/// A frame with the prop slots left unbound, for validating state on its own: no default is
/// evaluated there, so nothing reads a prop, and a slot that is read anyway fails as unbound
/// rather than yielding a value the props never held.
fn props_frame(component: &ComponentDecl) -> Frame {
    vec![None; component.props.len()]
}

struct Labeled2<'a>(&'a str, &'a str);

impl fmt::Display for Labeled2<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}", self.0, self.1)
    }
}

struct Labeled3<'a>(&'a dyn fmt::Display, &'static str);

impl fmt::Display for Labeled3<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}{}", self.0, self.1)
    }
}

/// Whether an instance was rendered by `program`: by the very images it is linked from, since
/// a handler names its node and its captured slots by index.
fn belongs(program: &ProgramData, images: &[(Arc<str>, u64)], component: &str) -> Result<()> {
    if std::ptr::eq(&*program.images, images) || *program.images == *images {
        return Ok(());
    }
    fail(
        "nx-ir-component",
        format!("The instance of '{component}' was initialized by another program."),
    )
}

impl Program {
    /// Runs one call of an evaluation method: measures what the host passed, when it set an
    /// input limit, runs `body`, and reports what the call used, when it asked.
    ///
    /// <para>Every method that takes options and a host value runs through here, so the input is
    /// refused before the program is looked at and before anything is converted, and the report
    /// is written however `body` returns. `input` counts the call's host values on the measure
    /// it is given; it is not called when there is no limit.</para>
    fn call<T>(
        &self,
        options: &RuntimeOptions,
        input: impl FnOnce(&mut Measure),
        body: impl FnOnce(&Machine<'_>) -> Result<T>,
    ) -> Result<T> {
        let usage = options.usage.as_deref();
        if let Some(usage) = usage {
            usage.clear();
        }
        let input_size = match options.max_input_size {
            None => None,
            Some(limit) => {
                let mut measure = Measure::new(Some(limit));
                input(&mut measure);
                match measure.finish() {
                    Ok(size) => Some(size),
                    Err(error) => {
                        // Nothing ran, so under a budget the call used no operations.
                        if let Some(usage) = usage {
                            usage.record(options.max_operations.map(|_| 0), None);
                        }
                        return Err(error);
                    }
                }
            }
        };
        let machine = Machine::new(&self.data, options);
        let result = body(&machine);
        if let Some(usage) = usage {
            usage.record(machine.used(), input_size);
        }
        result
    }

    /// Evaluates a function entrypoint of the entry module by name, with positional arguments.
    ///
    /// <para>An empty result of a function whose result type is a standalone `T?` is returned as
    /// the host's `null`; every other result keeps its encoding, so an empty `T*` is the empty
    /// list.</para>
    pub fn evaluate_function(
        &self,
        name: &str,
        args: &[NxValue],
        options: &RuntimeOptions,
    ) -> Result<NxValue> {
        self.call(
            options,
            |input| {
                input.values(args);
            },
            |machine| {
                let entry = &self.data.entry;
                let Some((index, _)) = entry.entrypoint(&entry.function_entrypoints, name) else {
                    return fail(
                        "nx-ir-missing-entrypoint",
                        format!("Function entrypoint '{name}' was not found."),
                    );
                };
                let args = args
                    .iter()
                    .map(|arg| from_host(arg).map(Some))
                    .collect::<Result<_>>()?;
                entry_result(
                    machine,
                    0,
                    index,
                    machine.invoke(0, index, args, 0, Caller::Host)?,
                )
            },
        )
    }

    /// Calls the function a canonical `Function` record names with arguments keyed by parameter
    /// name. An argument the function does not declare is dropped; a parameter it declares and
    /// the arguments lack is a diagnostic naming it.
    pub fn call_function(
        &self,
        function: &NxValue,
        args: &BTreeMap<String, NxValue>,
        options: &RuntimeOptions,
    ) -> Result<NxValue> {
        self.call(
            options,
            |input| {
                input.value(function).record(args);
            },
            |machine| {
                let Some(function) = crate::normalize::as_function_record(&from_host(function)?)
                else {
                    return fail(
                        "nx-ir-function-value",
                        "call_function expects a Function record: { $type: \"Function\", module, name }.",
                    );
                };
                let (module, index) = machine.resolve_function(&function, &"call_function")?;
                let args = fields_from_host(args)?;
                entry_result(
                    machine,
                    module,
                    index,
                    machine.invoke_by_name(module, index, &args, 0, Caller::Host)?,
                )
            },
        )
    }

    /// Constructs a component's descriptor from props and content, as an element in a body
    /// would.
    pub fn construct_component_descriptor(
        &self,
        name: &str,
        props: &BTreeMap<String, NxValue>,
        content: &[NxValue],
        options: &RuntimeOptions,
    ) -> Result<NxValue> {
        self.call(
            options,
            |input| {
                input.record(props).values(content);
            },
            |machine| {
                let (index, declared, component) = machine.component(name)?;
                let path = Labeled(name, " props");
                let (mut input, handlers) =
                    machine.split_handler_properties(0, index, fields_from_host(props)?, &path)?;
                // A host supplies content as an argument, with no body to have been written or
                // not, so no content passed means no content and the declared default stands.
                let content = content.iter().map(from_host).collect::<Result<Vec<_>>>()?;
                bind_content(&mut input, &component.props, content, &name, false)?;
                let mut fields = machine.normalize_fields(
                    entry_cx(index),
                    &component.props,
                    &input,
                    &mut Vec::new(),
                    &path,
                    false,
                )?;
                for (name, handler) in handlers {
                    crate::value::set_field(&mut fields, name, Value::Handler(handler));
                }
                to_host(
                    &Value::record(Some(Arc::clone(declared)), fields),
                    None,
                    &machine.meter(entry_cx(index), None),
                )
            },
        )
    }

    /// Initializes a component from props: renders its body against its initial state and
    /// returns the rendered output, the state, and the instance dispatch runs against.
    pub fn initialize_component(
        &self,
        name: &str,
        props: &BTreeMap<String, NxValue>,
        init: &ComponentInit<'_>,
        options: &RuntimeOptions,
    ) -> Result<ComponentInitResult> {
        self.call(
            options,
            |input| {
                input.record(props);
                if let Some(state) = init.state {
                    input.record(state);
                }
            },
            |machine| {
                let (index, declared, component) = machine.component(name)?;
                let Some(body) = component.body.filter(|_| !component.is_abstract) else {
                    return fail(
                        "nx-ir-component",
                        format!("Component '{name}' cannot be initialized because it has no body."),
                    );
                };
                if let Some(parent) = init.parent {
                    belongs(&self.data, &parent.data.program, &parent.data.component)?;
                }
                let path = Labeled(name, " props");
                let supplied = Value::record(None, fields_from_host(props)?);
                let resolved = resolve_parent_handlers(
                    &supplied,
                    init.parent.map(|parent| &*parent.data),
                    &Path::Root(&path),
                    0,
                )?;
                let resolved = match resolved {
                    Value::Record(record) => record.fields.clone(),
                    _ => Vec::new(),
                };
                let (fields, handler_props) =
                    machine.split_handler_properties(0, index, resolved, &path)?;
                let cx = entry_cx(index);
                let mut frame = Vec::new();
                let props = machine.normalize_fields(
                    cx,
                    &component.props,
                    &fields,
                    &mut frame,
                    &path,
                    false,
                )?;
                let state_path = Labeled(name, " state");
                let state = match init.state {
                    None => machine.normalize_fields(
                        cx,
                        &component.state,
                        &[],
                        &mut frame,
                        &state_path,
                        false,
                    )?,
                    Some(state) => machine.normalize_fields(
                        cx,
                        &component.state,
                        &fields_from_host(state)?,
                        &mut frame,
                        &state_path,
                        true,
                    )?,
                };
                let mut tokens = Tokens::new(1);
                let output = machine.meter(cx, None);
                let rendered = to_host(
                    &machine.eval(cx, &mut frame, body)?,
                    Some(&mut tokens),
                    &output,
                )?;
                Ok(ComponentInitResult {
                    rendered,
                    state: fields_to_host(&state, &output)?,
                    instance: ComponentInstance {
                        data: Arc::new(InstanceData {
                            program: Arc::clone(&self.data.images),
                            component: Arc::clone(declared),
                            props,
                            handler_props,
                            state,
                            handlers: tokens.handlers,
                            generation: 1,
                        }),
                    },
                })
            },
        )
    }

    /// Evaluates a component's body from props and explicit state. The output carries no tokens.
    pub fn evaluate_component(
        &self,
        name: &str,
        props: &BTreeMap<String, NxValue>,
        state: &BTreeMap<String, NxValue>,
        options: &RuntimeOptions,
    ) -> Result<NxValue> {
        self.call(
            options,
            |input| {
                input.record(props).record(state);
            },
            |machine| {
                let (index, _, component) = machine.component(name)?;
                let Some(body) = component.body.filter(|_| !component.is_abstract) else {
                    return fail(
                        "nx-ir-component",
                        format!("Component '{name}' cannot be evaluated because it has no body."),
                    );
                };
                let path = Labeled(name, " props");
                let (fields, _) =
                    machine.split_handler_properties(0, index, fields_from_host(props)?, &path)?;
                let cx = entry_cx(index);
                let mut frame = Vec::new();
                machine.normalize_fields(
                    cx,
                    &component.props,
                    &fields,
                    &mut frame,
                    &path,
                    false,
                )?;
                machine.normalize_fields(
                    cx,
                    &component.state,
                    &fields_from_host(state)?,
                    &mut frame,
                    &Labeled(name, " state"),
                    true,
                )?;
                to_host(
                    &machine.eval(cx, &mut frame, body)?,
                    None,
                    &machine.meter(cx, None),
                )
            },
        )
    }

    /// Dispatches a batch against an instance and returns the next one, without touching the
    /// instance given.
    ///
    /// <para>Each entry is either an action the component emits, which runs the handler the
    /// parent bound on the instance's props, or an `ActionHandlerInvocation` naming a handler of
    /// the instance's most recent rendered output by token, with the action to feed it. Entries
    /// run in order. A handler the component's own body bound reads the state live and patches
    /// it with the component's update records; any other handler sees only what it captured, and
    /// everything it returns is an effect. The body is rendered once against the state the batch
    /// produced. A failure returns before anything else does, so the instance given stays the
    /// state of record.</para>
    pub fn dispatch_component_actions(
        &self,
        instance: &ComponentInstance,
        batch: &[NxValue],
        options: &RuntimeOptions,
    ) -> Result<ComponentDispatchResult> {
        self.call(
            options,
            |input| {
                input.values(batch);
            },
            |machine| {
                let instance = &instance.data;
                belongs(&self.data, &instance.program, &instance.component)?;
                let (index, name, component) = machine.component(&instance.component)?;
                let Some(body) = component.body else {
                    return fail(
                        "nx-ir-component",
                        format!("Component '{name}' has no body."),
                    );
                };
                let owner_key = format!("{}::{name}", self.data.entry.identity);
                let mut working = instance.state.clone();
                let mut effects = Vec::new();
                let mut depths = Depths::default();

                for (position, entry) in batch.iter().enumerate() {
                    let path = Position("dispatch entry ", position, "");
                    let entry = from_host(entry)?;
                    let object = require_record(&entry, &path)?;
                    if object.type_name() == Some(HANDLER_INVOCATION_TYPE) {
                        let Some(token) = object.get("token").and_then(Value::as_text) else {
                            return fail(
                                "nx-ir-boundary-type",
                                format!(
                            "Expected {path} to carry a string 'token' read from rendered output."
                        ),
                            );
                        };
                        let Some(handler) = instance.handler(token) else {
                            return fail(
                                "nx-ir-handler-token",
                                format!(
                                    "Unknown handler token '{token}' for the '{name}' instance."
                                ),
                            );
                        };
                        let empty = Value::empty();
                        let action = require_record(
                            object.get("action").unwrap_or(&empty),
                            &Position("dispatch entry ", position, ".action"),
                        )?;
                        let owned = handler.owner.as_deref() == Some(owner_key.as_str());
                        let live = owned.then_some((component, &working));
                        for result in machine.invoke_handler(handler, action, live)? {
                            match result.as_record() {
                                Some(patch)
                                    if owned
                                        && machine.is_update_record_for(&result, &owner_key) =>
                                {
                                    working = machine.patch_state(
                                        index,
                                        name,
                                        component,
                                        &working,
                                        patch,
                                        &mut depths,
                                    )?;
                                }
                                _ => effects.push(result),
                            }
                        }
                        continue;
                    }
                    let Some(type_name) = object.type_name() else {
                        return fail(
                    "nx-ir-boundary-type",
                    format!("Expected {path} to be an action record with a '$type' discriminator."),
                );
                    };
                    let emitted = component.emits.iter().find_map(|emit| {
                        let (module, action, declaration) =
                            self.data.resolve(0, &emit.action).ok()?;
                        (&*declaration.name == type_name).then_some((emit, module, action))
                    });
                    let Some((emit, action_module, action_index)) = emitted else {
                        return fail(
                            "nx-ir-component-action",
                            format!("Component '{name}' does not emit '{type_name}'."),
                        );
                    };
                    // The entry is host input, so it is constructed against the emitted action
                    // before the handler is looked up: a malformed payload fails whether or not
                    // the parent bound one.
                    let action = machine.normalize_action(
                        action_module,
                        action_index,
                        object,
                        &Labeled(type_name, " action"),
                    )?;
                    let property = handler_property(&emit.name);
                    let bound = instance
                        .handler_props
                        .iter()
                        .find(|(key, _)| **key == *property);
                    if let (Some((_, handler)), Some(action)) = (bound, action.as_record()) {
                        // The parent bound this handler, so everything it returns belongs to the
                        // parent.
                        effects.extend(machine.invoke_handler(handler, action, None)?);
                    }
                }

                // The body sees the declared props and the state, as it did at initialization. A
                // field the instance carries no entry for is an empty optional.
                let mut frame: Frame =
                    Vec::with_capacity(component.props.len().saturating_add(component.state.len()));
                for field in component.props.iter() {
                    frame.push(Some(
                        get_field(&instance.props, &field.name)
                            .cloned()
                            .unwrap_or_else(Value::empty),
                    ));
                }
                for field in component.state.iter() {
                    frame.push(Some(
                        get_field(&working, &field.name)
                            .cloned()
                            .unwrap_or_else(Value::empty),
                    ));
                }
                let generation = instance.generation.saturating_add(1);
                let mut tokens = Tokens::new(generation);
                let output = machine.meter(entry_cx(index), None);
                let rendered = to_host(
                    &machine.eval(entry_cx(index), &mut frame, body)?,
                    Some(&mut tokens),
                    &output,
                )?;
                Ok(ComponentDispatchResult {
                    rendered,
                    effects: effects
                        .iter()
                        .map(|effect| to_host(effect, None, &output))
                        .collect::<Result<_>>()?,
                    state: fields_to_host(&working, &output)?,
                    instance: ComponentInstance {
                        data: Arc::new(InstanceData {
                            program: Arc::clone(&instance.program),
                            component: Arc::clone(&instance.component),
                            props: instance.props.clone(),
                            handler_props: instance.handler_props.clone(),
                            state: working,
                            handlers: tokens.handlers,
                            generation,
                        }),
                    },
                })
            },
        )
    }

    /// Validates a complete state for a component and returns it normalized.
    pub fn normalize_component_state(
        &self,
        name: &str,
        state: &BTreeMap<String, NxValue>,
        options: &RuntimeOptions,
    ) -> Result<BTreeMap<String, NxValue>> {
        self.call(
            options,
            |input| {
                input.record(state);
            },
            |machine| {
                let (index, _, component) = machine.component(name)?;
                let state = machine.normalize_fields(
                    entry_cx(index),
                    &component.state,
                    &fields_from_host(state)?,
                    &mut props_frame(component),
                    &Labeled(name, " state"),
                    true,
                )?;
                fields_to_host(&state, &machine.meter(entry_cx(index), None))
            },
        )
    }

    /// Applies a patch to host-owned component state and returns the validated next state.
    ///
    /// <para>The patch is a record: either plain, or the component's own update record,
    /// `<Component>.Update`. Either way a present field replaces the current value, an absent
    /// one keeps it, and a present empty value clears an optional state field, so the next state
    /// carries no entry for it; for a field that is not optional it is rejected.</para>
    pub fn apply_component_state_patch(
        &self,
        name: &str,
        current: &BTreeMap<String, NxValue>,
        patch: &NxValue,
        options: &RuntimeOptions,
    ) -> Result<BTreeMap<String, NxValue>> {
        self.call(
            options,
            |input| {
                input.record(current).value(patch);
            },
            |machine| {
                let (index, name, component) = machine.component(name)?;
                let patch = from_host(patch)?;
                let patch = require_record(&patch, &"the state patch")?;
                let state = machine.patch_state(
                    index,
                    name,
                    component,
                    &fields_from_host(current)?,
                    patch,
                    &mut Depths::default(),
                )?;
                fields_to_host(&state, &machine.meter(entry_cx(index), None))
            },
        )
    }

    /// Restores an instance from its serialized form, for this program.
    ///
    /// <para>This is where an instance that did not come from this process is checked, once: it
    /// must belong to a program linked from the same images, everything it holds must name
    /// handlers and functions of that program, and its props, state and handler properties must
    /// be ones the component accepts. An instance from another revision of the program, or
    /// bytes someone altered, is refused with a diagnostic rather than run.</para>
    pub fn restore_component_instance<'de, D: Deserializer<'de>>(
        &self,
        deserializer: D,
    ) -> Result<ComponentInstance> {
        let stored = Stored::deserialize(deserializer).map_err(|error| {
            crate::error::NxIrRuntimeError::new(
                "nx-ir-component",
                format!("The input is not a serialized component instance: {error}."),
            )
        })?;
        belongs(&self.data, &stored.program, &stored.component)?;
        let options = RuntimeOptions::default();
        let machine = Machine::new(&self.data, &options);
        let (index, name, component) = machine.component(&stored.component)?;
        let mut data = stored.restore(&machine)?;

        // The props and the state are validated as host input is, with nothing defaulted: an
        // instance holds every field that is not an empty optional.
        let cx = entry_cx(index);
        let mut frame = Vec::new();
        data.props = machine
            .normalize_fields(
                cx,
                &component.props,
                &data.props,
                &mut frame,
                &Labeled(name, " props"),
                true,
            )
            .or_else(refuse)?;
        data.state = machine
            .normalize_fields(
                cx,
                &component.state,
                &data.state,
                &mut frame,
                &Labeled(name, " state"),
                true,
            )
            .or_else(refuse)?;
        let bound = std::mem::take(&mut data.handler_props)
            .into_iter()
            .map(|(key, handler)| (key, Value::Handler(handler)))
            .collect();
        let (others, handler_props) = machine
            .split_handler_properties(0, index, bound, &Labeled(name, " props"))
            .or_else(refuse)?;
        if let Some((key, _)) = others.first() {
            return refuse(format_args!(
                "'{key}' is not a handler property of '{name}'"
            ));
        }
        data.handler_props = handler_props;
        Ok(ComponentInstance {
            data: Arc::new(data),
        })
    }
}

/// An entry call's result as the host reads it: canonical, and `null` where the function's
/// result type is a standalone `T?` and holds nothing.
fn entry_result(machine: &Machine<'_>, module: u32, index: u32, value: Value) -> Result<NxValue> {
    let is_optional = machine
        .program
        .linked(module)
        .and_then(|linked| linked.module.declaration(index))
        .is_some_and(|declaration| {
            matches!(&declaration.kind, DeclarationKind::Function(function) if function.is_optional_result)
        });
    if is_optional && value.is_empty() {
        return Ok(NxValue::Null);
    }
    let cx = Cx {
        module,
        declaration: index,
        depth: 0,
    };
    to_host(&value, None, &machine.meter(cx, None))
}

struct Position(&'static str, usize, &'static str);

impl fmt::Display for Position {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}{}{}", self.0, self.1, self.2)
    }
}
