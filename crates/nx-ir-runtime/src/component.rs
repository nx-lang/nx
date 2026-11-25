//! Components: descriptors, instances, handlers and dispatch.

use crate::error::{fail, Result};
use crate::eval::{
    bind, bind_content, Caller, Cx, Frame, Machine, Meter, RuntimeOptions, Stack,
    NX_DEFAULT_MAX_STACK_BYTES,
};
use crate::input::Measure;
use crate::kept::{Input, Session};
use crate::module::{ComponentDecl, DeclarationKind, Node};
use crate::normalize::{require_record, Labeled, Path};
use crate::program::{Program, ProgramData};
use crate::stored::{refuse, Stored};
use crate::value::{
    fields_from_host, fields_to_host, from_host, get_field, handlers_equal, to_host, token_text,
    too_deep, Depths, Fields, Handler, Record, Tokens, Value, ACTION_HANDLER_TYPE,
    HANDLER_INVOCATION_TYPE, MAX_VALUE_DEPTH,
};
use nx_value::NxValue;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, HashMap};
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
    /// The render behind the instance's rendered output, shared by every instance that keeps it.
    pub render: Arc<Render>,
}

/// One render of a component's body: what its output's tokens name, and what it read.
///
/// <para>An instance whose dispatch or new props left its output as it was shares the render of
/// the instance before it, so the instance tree finds it the same render: its handlers, its
/// generation and its tokens. The props, the handlers bound for its emits and the state are the
/// instance's own, outside the render.</para>
#[derive(Debug)]
pub(crate) struct Render {
    /// The handlers in the rendered output, in the order their tokens number them.
    pub handlers: Vec<Arc<Handler>>,
    /// The render generation the tokens belong to.
    pub generation: u64,
    /// The props and state fields the render read, in memory only: `None` for an instance
    /// restored from storage, which counts as having read every one.
    pub reads: Option<RenderReads>,
}

/// The props and state fields a render read, by name: a `slot` node of the body evaluated, a
/// slot an override formula the body wrote captured, and, for props only, a slot a handler the
/// body bound reads. A content parameter is one prop. A handler's state slots are not reads: in
/// the instance tree every handler runs against its owner with the state live.
#[derive(Debug, Clone, Default)]
pub(crate) struct RenderReads {
    pub props: Vec<Arc<str>>,
    pub state: Vec<Arc<str>>,
}

impl RenderReads {
    /// Whether the render read the state field `name`.
    pub(crate) fn read_state(&self, name: &str) -> bool {
        self.state.iter().any(|read| &**read == name)
    }
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
        self.data.render.generation
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

// What the instance tree reads of an instance. None of it is the host's to read: a host names a
// handler by its token and never holds one.
impl ComponentInstance {
    /// Whether this is the very instance `other` is, not one that holds the same things: a token
    /// names the same handler in both only then.
    ///
    /// <para>It is the render that is compared: an instance that took new state or new props and
    /// kept its rendered output shares the render of the one before, so its tokens name the same
    /// handlers.</para>
    pub(crate) fn is(&self, other: &ComponentInstance) -> bool {
        Arc::ptr_eq(&self.data.render, &other.data.render)
    }

    /// What the render behind the instance read of its props and state; `None` when unknown.
    pub(crate) fn render_reads(&self) -> Option<&RenderReads> {
        self.data.render.reads.as_ref()
    }

    /// The instance's normalized props.
    pub(crate) fn props(&self) -> &Fields {
        &self.data.props
    }

    /// The handlers the parent bound on the instance for its emits.
    pub(crate) fn handler_props(&self) -> &HandlerProps {
        &self.data.handler_props
    }

    /// This instance with other props and handlers bound for its emits, keeping its render.
    pub(crate) fn with_props(&self, props: Fields, handler_props: HandlerProps) -> Self {
        let data = &*self.data;
        ComponentInstance {
            data: Arc::new(InstanceData {
                program: Arc::clone(&data.program),
                component: Arc::clone(&data.component),
                props,
                handler_props,
                state: data.state.clone(),
                render: Arc::clone(&data.render),
            }),
        }
    }

    /// The handler the instance's rendered output holds under `token`.
    pub(crate) fn handler(&self, token: &str) -> Option<&Arc<Handler>> {
        self.data.handler(token)
    }

    /// The token under which the instance's rendered output holds `handler`, when it does.
    ///
    /// <para>The very handler, not one equal to it. A handler reaches another instance only by
    /// being handed on, so within a tree of instances the one allocation is the one handler, and
    /// two instances of a component with equal props and state hold handlers that are equal and
    /// are not one: a tap on the inner of two such instances is not a tap on the outer.</para>
    pub(crate) fn token_of(&self, handler: &Arc<Handler>) -> Option<String> {
        let number = self
            .data
            .render
            .handlers
            .iter()
            .position(|held| Arc::ptr_eq(held, handler))?;
        Some(token_text(
            self.data.render.generation,
            number.saturating_add(1),
        ))
    }

    /// The handler a parent bound on the instance under `property` (`onTapped`).
    pub(crate) fn bound_handler(&self, property: &str) -> Option<&Arc<Handler>> {
        self.data
            .handler_props
            .iter()
            .find(|(name, _)| &**name == property)
            .map(|(_, handler)| handler)
    }
}

/// The handlers of a parent's earlier instance and the handlers of its current instance that
/// replace them, by allocation, for rebinding a node that did not render again.
///
/// <para>Pairs are recorded first, from the places the two descriptors hold their handlers at.
/// Substituting then extends the record: a handler that captures one being replaced is itself
/// replaced by a copy that captures the replacement, and a handler found to capture none is
/// remembered as kept. A list or record is remembered the same way, so a value many handlers
/// capture is walked once, and each value walked costs one operation.</para>
#[derive(Debug, Default)]
pub(crate) struct Rebinding {
    /// For each handler met, by address, the handler itself, which keeps the address in use, and
    /// what replaces it; `None` when it is kept.
    replaced: HashMap<usize, (Arc<Handler>, Option<Arc<Handler>>)>,
    /// The same for each list and record met.
    values: HashMap<usize, (Value, Option<Value>)>,
}

impl Rebinding {
    /// Records that `new` replaces `old`. Fails, returning false, when `old` is already replaced
    /// by another handler: the two descriptors then do not pair their handlers one to one. Two
    /// earlier handlers replaced by one are allowed: a render that handed one handler to two
    /// places writes it at both, as the new render does.
    pub(crate) fn pair(&mut self, old: &Arc<Handler>, new: &Arc<Handler>) -> bool {
        let replacement = (!Arc::ptr_eq(old, new)).then(|| Arc::clone(new));
        match self.replaced.get(&address(old)) {
            Some((_, existing)) => match (existing, &replacement) {
                (None, None) => true,
                (Some(existing), Some(new)) => Arc::ptr_eq(existing, new),
                _ => false,
            },
            None => {
                self.replaced
                    .insert(address(old), (Arc::clone(old), replacement));
                true
            }
        }
    }

    /// The handler that replaces `handler`, or `None` when it is kept.
    fn handler(
        &mut self,
        handler: &Arc<Handler>,
        depth: u32,
        meter: &Meter<'_, '_>,
        stack: &Stack,
    ) -> Result<Option<Arc<Handler>>> {
        if let Some((_, replacement)) = self.replaced.get(&address(handler)) {
            return Ok(replacement.clone());
        }
        meter.charge(1)?;
        let mut captured = None;
        for (slot, value) in handler.captured.iter().enumerate() {
            let Some(value) = value else { continue };
            if let Some(value) = self.value(value, depth, meter, stack)? {
                if let Some(entry) = captured
                    .get_or_insert_with(|| handler.captured.clone())
                    .get_mut(slot)
                {
                    *entry = Some(value);
                }
            }
        }
        let replacement = captured.map(|captured| {
            Arc::new(Handler {
                captured,
                ..Handler::clone(handler)
            })
        });
        self.replaced
            .insert(address(handler), (Arc::clone(handler), replacement.clone()));
        Ok(replacement)
    }

    /// `value` with every handler replaced, or `None` when nothing in it is.
    fn value(
        &mut self,
        value: &Value,
        depth: u32,
        meter: &Meter<'_, '_>,
        stack: &Stack,
    ) -> Result<Option<Value>> {
        if depth > MAX_VALUE_DEPTH {
            return too_deep();
        }
        if matches!(value, Value::Seq(_) | Value::Record(_)) && !stack.within() {
            return stack.value_too_deep();
        }
        let deeper = depth.saturating_add(1);
        let held = match value {
            Value::Handler(handler) => {
                return Ok(self
                    .handler(handler, deeper, meter, stack)?
                    .map(Value::Handler))
            }
            Value::Seq(items) => items.as_ptr() as usize,
            Value::Record(record) => Arc::as_ptr(record) as usize,
            _ => return Ok(None),
        };
        // An empty list holds nothing and may share its address with any other.
        if matches!(value, Value::Seq(items) if items.is_empty()) {
            return Ok(None);
        }
        if let Some((_, replacement)) = self.values.get(&held) {
            return Ok(replacement.clone());
        }
        meter.charge(1)?;
        let replacement = match value {
            Value::Seq(items) => {
                let mut changed: Option<Vec<Value>> = None;
                for (index, item) in items.iter().enumerate() {
                    if let Some(item) = self.value(item, deeper, meter, stack)? {
                        if let Some(entry) =
                            changed.get_or_insert_with(|| items.to_vec()).get_mut(index)
                        {
                            *entry = item;
                        }
                    }
                }
                changed.map(Value::seq)
            }
            Value::Record(record) => {
                self.fields(&record.fields, deeper, meter, stack)?
                    .map(|fields| {
                        Value::record_from(record.type_name.clone(), fields, record.origin.clone())
                    })
            }
            _ => None,
        };
        self.values
            .insert(held, (value.clone(), replacement.clone()));
        Ok(replacement)
    }

    /// `fields` with every handler replaced, or `None` when nothing in them is.
    fn fields(
        &mut self,
        fields: &Fields,
        depth: u32,
        meter: &Meter<'_, '_>,
        stack: &Stack,
    ) -> Result<Option<Fields>> {
        let mut changed: Option<Fields> = None;
        for (index, (_, value)) in fields.iter().enumerate() {
            if let Some(value) = self.value(value, depth, meter, stack)? {
                if let Some(entry) = changed.get_or_insert_with(|| fields.clone()).get_mut(index) {
                    entry.1 = value;
                }
            }
        }
        Ok(changed)
    }
}

fn address(handler: &Arc<Handler>) -> usize {
    Arc::as_ptr(handler) as usize
}

impl ComponentInstance {
    /// This instance with every handler `rebinding` replaces swapped for its replacement, in its
    /// props, the handlers bound for its emits, its state, the handlers of its rendered output
    /// and what its own handlers capture. Its generation and the order of its handlers are kept,
    /// so every token of the output it rendered names the handler at the same place. `None`
    /// when nothing in it is replaced.
    pub(crate) fn rebound(
        &self,
        rebinding: &mut Rebinding,
        meter: &Meter<'_, '_>,
        stack: &Stack,
    ) -> Result<Option<ComponentInstance>> {
        let data = &*self.data;
        let props = rebinding.fields(&data.props, 0, meter, stack)?;
        let state = rebinding.fields(&data.state, 0, meter, stack)?;
        let mut handler_props = None;
        for (index, (_, handler)) in data.handler_props.iter().enumerate() {
            if let Some(handler) = rebinding.handler(handler, 1, meter, stack)? {
                if let Some(entry) = handler_props
                    .get_or_insert_with(|| data.handler_props.clone())
                    .get_mut(index)
                {
                    entry.1 = handler;
                }
            }
        }
        let mut handlers = None;
        for (index, handler) in data.render.handlers.iter().enumerate() {
            if let Some(handler) = rebinding.handler(handler, 1, meter, stack)? {
                if let Some(entry) = handlers
                    .get_or_insert_with(|| data.render.handlers.clone())
                    .get_mut(index)
                {
                    *entry = handler;
                }
            }
        }
        if props.is_none() && state.is_none() && handler_props.is_none() && handlers.is_none() {
            return Ok(None);
        }
        Ok(Some(ComponentInstance {
            data: Arc::new(InstanceData {
                program: Arc::clone(&data.program),
                component: Arc::clone(&data.component),
                props: props.unwrap_or_else(|| data.props.clone()),
                handler_props: handler_props.unwrap_or_else(|| data.handler_props.clone()),
                state: state.unwrap_or_else(|| data.state.clone()),
                // A render none of whose handlers is replaced is the same render: its tokens
                // name what they named.
                render: match handlers {
                    None => Arc::clone(&data.render),
                    Some(handlers) => Arc::new(Render {
                        handlers,
                        generation: data.render.generation,
                        reads: data.render.reads.clone(),
                    }),
                },
            }),
        }))
    }
}

impl Program {
    /// Runs `work`, the instance tree's comparison and rebinding of a kept node of `component`,
    /// under `options`' operation budget, named after the component when it runs out, and its
    /// stack budget.
    ///
    /// <para>It is not a lifecycle call: it leaves the usage and origins reports as they were,
    /// and a visit that compares and then initializes the node spends a budget on each.</para>
    pub(crate) fn metered<T>(
        &self,
        component: &str,
        options: &RuntimeOptions,
        work: impl FnOnce(&Meter<'_, '_>, &Stack) -> Result<T>,
    ) -> Result<T> {
        let machine = Machine::new(&self.data, options);
        machine.begin_environment()?;
        let (index, _, _) = machine.component(component)?;
        let meter = machine.meter(entry_cx(index), None);
        work(&meter, &machine.stack)
    }
}

/// Whether two handlers a host holds are one handler. No evaluation pays for the comparison.
///
/// <para>A handler a host was handed and hands back is the same allocation, which is the case
/// this exists for, so that is asked first: it is not what the language's `==` asks, where a
/// capture that holds a NaN makes a handler unequal to itself.</para>
///
/// <para>The comparison takes no options, so it walks a capture under the default stack budget; a
/// capture it cannot walk within it is not found equal.</para>
fn same_handler(left: &Arc<Handler>, right: &Arc<Handler>) -> bool {
    Arc::ptr_eq(left, right)
        || handlers_equal(
            left,
            right,
            &Meter::free(),
            &Stack::begin(NX_DEFAULT_MAX_STACK_BYTES),
        )
        .unwrap_or(false)
}

impl Serialize for ComponentInstance {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        Stored::of(&self.data).serialize(serializer)
    }
}

impl InstanceData {
    /// The handler a token of this instance's rendered output names.
    fn handler(&self, token: &str) -> Option<&Arc<Handler>> {
        self.render.handler(token)
    }
}

impl Render {
    /// The handler a token of this render's output names.
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

/// What the instance tree's dispatch returns: as [`ComponentDispatchResult`], with no rendered
/// output when the dispatch kept the instance's render.
#[derive(Debug, Clone)]
pub(crate) struct TrackedDispatch {
    pub rendered: Option<NxValue>,
    pub effects: Vec<NxValue>,
    pub state: BTreeMap<String, NxValue>,
    pub instance: ComponentInstance,
}

/// A component's props normalized, and the handlers bound for its emits, ready to initialize it
/// from.
#[derive(Debug, Clone)]
pub(crate) struct NormalizedProps {
    pub props: Fields,
    pub handler_props: HandlerProps,
    /// The direct inputs of normalizing them, in a call the instance tree made: what the
    /// defaults read.
    pub record: Vec<Input>,
}

/// What a call the instance tree makes brings beside its options.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TreeCall<'a, 's> {
    /// The tree's kept results.
    pub session: &'a Session<'s>,
    /// The operations a record check spent at the node ahead of the call, under the same
    /// budget: the call starts with what the check left and reports them as its own.
    pub spent: u64,
    /// The direct inputs of normalizing the props the call starts from, when it does not
    /// normalize them itself.
    pub seed: &'a [Input],
}

/// A render's record in a call the instance tree makes: its direct inputs in the order it read
/// them, normalizing its props first, and those of normalizing its props alone.
#[derive(Debug, Default)]
pub(crate) struct RenderRecord {
    pub inputs: Vec<Input>,
    pub props: Vec<Input>,
}

/// The props an initialization starts from.
#[derive(Debug, Clone, Copy)]
pub(crate) enum InitProps<'a> {
    /// As the host gives them: normalized by the initialization.
    Host(&'a BTreeMap<String, NxValue>),
    /// Normalized already, by [`Program::normalize_component_props`].
    Normalized(&'a NormalizedProps),
}

/// What a render read of its props and state, from the slots it read: the props first, then the
/// state, in the order the component declares them.
fn render_reads(component: &ComponentDecl, read: &[bool]) -> RenderReads {
    let was_read = |slot: usize| read.get(slot).copied().unwrap_or(false);
    RenderReads {
        props: component
            .props
            .iter()
            .enumerate()
            .filter(|(slot, _)| was_read(*slot))
            .map(|(_, field)| Arc::clone(&field.name))
            .collect(),
        state: component
            .state
            .iter()
            .enumerate()
            .filter(|(offset, _)| was_read(component.props.len().saturating_add(*offset)))
            .map(|(_, field)| Arc::clone(&field.name))
            .collect(),
    }
}

/// The property a parent binds a handler for `emit` under: `onTapped` for `Tapped`.
pub(crate) fn handler_property(emit: &str) -> String {
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
    stack: &Stack,
) -> Result<Value> {
    if depth > MAX_VALUE_DEPTH {
        return too_deep();
    }
    if matches!(value, Value::Seq(_) | Value::Record(_)) && !stack.within() {
        return stack.value_too_deep();
    }
    let deeper = depth.saturating_add(1);
    match value {
        Value::Seq(items) => Ok(Value::seq(
            items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    resolve_parent_handlers(item, parent, &Path::Index(path, index), deeper, stack)
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
                        resolve_parent_handlers(
                            item,
                            parent,
                            &Path::Field(path, name),
                            deeper,
                            stack,
                        )?,
                    ))
                })
                .collect::<Result<_>>()?,
        )),
        other => Ok(other.clone()),
    }
}

/// Props normalized, with the frame normalizing bound them in.
struct Normalized {
    props: Fields,
    handler_props: HandlerProps,
    frame: Frame,
}

impl<'p> Machine<'p> {
    /// Normalizes host-supplied props of the component `index` of the entry module: handler
    /// records resolved through `parent`, handlers bound for emits split off, and every prop
    /// checked with its default evaluated, into a frame that holds them in their slots.
    fn normalize_props(
        &self,
        index: u32,
        name: &str,
        component: &ComponentDecl,
        props: &BTreeMap<String, NxValue>,
        parent: Option<&ComponentInstance>,
    ) -> Result<Normalized> {
        let path = Labeled(name, " props");
        let props = crate::environment::without_env(props);
        let supplied = Value::record(None, fields_from_host(&props, &self.stack)?);
        let resolved = resolve_parent_handlers(
            &supplied,
            parent.map(|parent| &*parent.data),
            &Path::Root(&path),
            0,
            &self.stack,
        )?;
        let resolved = match resolved {
            Value::Record(record) => record.fields.clone(),
            _ => Vec::new(),
        };
        let (fields, handler_props) = self.split_handler_properties(0, index, resolved, &path)?;
        let mut frame = Vec::new();
        let props = self.normalize_fields(
            entry_cx(index),
            &component.props,
            &fields,
            &mut frame,
            &Path::Root(&path),
            false,
        )?;
        Ok(Normalized {
            props,
            handler_props,
            frame,
        })
    }

    /// Whether a state field a render read was patched to another value: each field in
    /// `patched` the render read compared once, its value in `before` against its value in
    /// `after`, as a host would see it written. A render whose reads are unknown read every one.
    fn read_field_changed(
        &self,
        reads: Option<&RenderReads>,
        patched: &[Arc<str>],
        before: &Fields,
        after: &Fields,
        meter: &Meter<'_, 'p>,
    ) -> Result<bool> {
        let empty = Value::empty();
        for field in patched {
            if reads.is_some_and(|reads| !reads.read_state(field)) {
                continue;
            }
            let old = get_field(before, field).unwrap_or(&empty);
            let new = get_field(after, field).unwrap_or(&empty);
            if !crate::kept::same_written(old, new, self.tracks_origins(), meter, &self.stack)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

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
        Ok(Value::record_from(
            Some(Arc::clone(expected)),
            self.normalize_fields(
                cx,
                &record.fields,
                &input.fields,
                &mut Vec::new(),
                &Path::Root(path),
                false,
            )?,
            self.carried(input),
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

    /// Whether `value` is the update record of the state record with `state_key`.
    fn is_update_record_for(&self, value: &Value, state_key: &str) -> bool {
        let Some(discriminator) = value.as_record().and_then(Record::type_name) else {
            return false;
        };
        self.program.shapes(discriminator).any(|(module, shape)| {
            shape
                .update_target
                .as_ref()
                .is_some_and(|target| self.program.key(module, target) == state_key)
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
        let expected = format!("{name}.State.Update");
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
            &Path::Root(&Labeled(name, " state")),
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
            &self.stack,
        )?;
        Ok(next)
    }
}

pub(crate) fn entry_cx(declaration: u32) -> Cx {
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
    /// refused before the program is looked at and before anything is converted, and the usage
    /// report is written however `body` returns. `input` counts the call's host values on the
    /// measure it is given; it is not called when there is no limit. The origins report is
    /// cleared first and filled only when `body` succeeds, with what it collected through
    /// [`Machine::report`].</para>
    fn call<T>(
        &self,
        options: &RuntimeOptions,
        input: impl FnOnce(&mut Measure),
        body: impl FnOnce(&Machine<'_>) -> Result<T>,
    ) -> Result<T> {
        self.call_in(options, None, input, body)
    }

    /// [`call`](Self::call) in a call the instance tree makes, with its kept results and what a
    /// check spent ahead of it, when it gives them.
    fn call_in<T>(
        &self,
        options: &RuntimeOptions,
        tree: Option<TreeCall<'_, '_>>,
        input: impl FnOnce(&mut Measure),
        body: impl FnOnce(&Machine<'_>) -> Result<T>,
    ) -> Result<T> {
        let usage = options.usage.as_deref();
        if let Some(usage) = usage {
            usage.clear();
        }
        let origins = options.origins.as_deref();
        if let Some(origins) = origins {
            origins.clear();
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
        let machine = match tree {
            Some(tree) => {
                let machine = Machine::with_session(&self.data, options, tree.session);
                machine.take_on(tree.spent);
                machine
            }
            None => Machine::new(&self.data, options),
        };
        let result = machine.begin_environment().and_then(|()| body(&machine));
        if let Some(usage) = usage {
            usage.record(machine.used(), input_size);
        }
        if let (Some(origins), Ok(_)) = (origins, &result) {
            origins.record(machine.take_origins());
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
        self.evaluate_function_value(name, args, options)
            .map(|value| value.canonical)
    }

    /// Evaluates a function retaining case and function identities for read-only tooling.
    pub fn evaluate_function_value(
        &self,
        name: &str,
        args: &[NxValue],
        options: &RuntimeOptions,
    ) -> Result<crate::FunctionValue> {
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
                    .map(|arg| from_host(arg, &machine.stack).map(Some))
                    .collect::<Result<_>>()?;
                let value = machine.invoke(0, index, args, 0, Caller::Host)?;
                let canonical = entry_result(machine, 0, index, value.clone())?;
                Ok(crate::FunctionValue { value, canonical })
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
                let Some(function) = crate::normalize::as_function_record(&from_host(function, &machine.stack)?)
                else {
                    return fail(
                        "nx-ir-function-value",
                        "call_function expects a Function record: { $type: \"Function\", module, name }.",
                    );
                };
                let (module, index) = machine.resolve_function(&function, &"call_function")?;
                let args = fields_from_host(args, &machine.stack)?;
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
                let (mut input, handlers) = machine.split_handler_properties(
                    0,
                    index,
                    fields_from_host(props, &machine.stack)?,
                    &path,
                )?;
                // A host supplies content as an argument, with no body to have been written or
                // not, so no content passed means no content and the declared default stands.
                let content = content
                    .iter()
                    .map(|value| from_host(value, &machine.stack))
                    .collect::<Result<Vec<_>>>()?;
                bind_content(&mut input, &component.props, content, &name, false)?;
                let mut fields = machine.normalize_fields(
                    entry_cx(index),
                    &component.props,
                    &input,
                    &mut Vec::new(),
                    &Path::Root(&path),
                    false,
                )?;
                for (name, handler) in handlers {
                    crate::value::set_field(&mut fields, name, Value::Handler(handler));
                }
                to_host(
                    &Value::record(Some(Arc::clone(declared)), fields),
                    None,
                    &machine.meter(entry_cx(index), None),
                    &machine.stack,
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
        self.initialize(name, InitProps::Host(props), init, options, None)
            .map(|(result, _)| result)
    }

    /// The instance tree's initialization: from props as the host gives them or as
    /// [`normalize_component_props`](Self::normalize_component_props) normalized them, with the
    /// tree's kept results. Returns the render's record too.
    pub(crate) fn initialize_tracked(
        &self,
        name: &str,
        props: InitProps<'_>,
        init: &ComponentInit<'_>,
        options: &RuntimeOptions,
        tree: TreeCall<'_, '_>,
    ) -> Result<(ComponentInitResult, RenderRecord)> {
        let tree = match props {
            InitProps::Normalized(normalized) => TreeCall {
                seed: &normalized.record,
                ..tree
            },
            InitProps::Host(_) => tree,
        };
        self.initialize(name, props, init, options, Some(tree))
            .map(|(result, record)| (result, record.unwrap_or_default()))
    }

    fn initialize(
        &self,
        name: &str,
        props: InitProps<'_>,
        init: &ComponentInit<'_>,
        options: &RuntimeOptions,
        tree: Option<TreeCall<'_, '_>>,
    ) -> Result<(ComponentInitResult, Option<RenderRecord>)> {
        self.call_in(
            options,
            tree,
            |input| {
                if let InitProps::Host(props) = props {
                    input.record(props);
                }
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
                let cx = entry_cx(index);
                // What the props' defaults read is the render's.
                machine.begin_render_track(tree.map(|tree| tree.seed).unwrap_or_default());
                let (props, handler_props, mut frame) = match props {
                    InitProps::Host(props) => {
                        let normalized =
                            machine.normalize_props(index, name, component, props, init.parent)?;
                        (normalized.props, normalized.handler_props, normalized.frame)
                    }
                    InitProps::Normalized(normalized) => {
                        let mut frame: Frame = Vec::with_capacity(
                            component.props.len().saturating_add(component.state.len()),
                        );
                        for field in component.props.iter() {
                            frame.push(Some(
                                get_field(&normalized.props, &field.name)
                                    .cloned()
                                    .unwrap_or_else(Value::empty),
                            ));
                        }
                        (
                            normalized.props.clone(),
                            normalized.handler_props.clone(),
                            frame,
                        )
                    }
                };
                let track = machine.suspend_render_track();
                let props_record = track
                    .as_ref()
                    .map(|track| track.inputs().to_vec())
                    .unwrap_or_default();
                let state_path = Labeled(name, " state");
                let state = match init.state {
                    None => machine.normalize_fields(
                        cx,
                        &component.state,
                        &[],
                        &mut frame,
                        &Path::Root(&state_path),
                        false,
                    )?,
                    Some(state) => machine.normalize_fields(
                        cx,
                        &component.state,
                        &fields_from_host(state, &machine.stack)?,
                        &mut frame,
                        &Path::Root(&state_path),
                        true,
                    )?,
                };
                machine.resume_render_track(track);
                let mut tokens = Tokens::new(1);
                let output = machine.meter(cx, None);
                machine.begin_render(&frame, component.props.len(), component.state.len());
                let value = machine.eval(cx, &mut frame, body)?;
                let (read, record) = machine.end_render();
                let rendered = machine.report(&value, Some(&mut tokens), &output)?;
                Ok((
                    ComponentInitResult {
                        rendered,
                        state: fields_to_host(&state, &output, &machine.stack)?,
                        instance: ComponentInstance {
                            data: Arc::new(InstanceData {
                                program: Arc::clone(&self.data.images),
                                component: Arc::clone(declared),
                                props,
                                handler_props,
                                state,
                                render: Arc::new(Render {
                                    handlers: tokens.handlers,
                                    generation: 1,
                                    reads: Some(render_reads(component, &read)),
                                }),
                            }),
                        },
                    },
                    record.map(|inputs| RenderRecord {
                        inputs,
                        props: props_record,
                    }),
                ))
            },
        )
    }

    /// Normalizes the props of a descriptor of `name` without rendering, charged as
    /// initialization charges normalizing them: handler records resolved through `parent`,
    /// handlers bound for emits split off, each prop checked and each default evaluated, with
    /// what the defaults read recorded. The instance tree compares what a node's render read of
    /// them with what it holds, and initializes the node from them when they differ.
    pub(crate) fn normalize_component_props(
        &self,
        name: &str,
        props: &BTreeMap<String, NxValue>,
        parent: Option<&ComponentInstance>,
        options: &RuntimeOptions,
        tree: TreeCall<'_, '_>,
    ) -> Result<NormalizedProps> {
        self.call_in(
            options,
            Some(tree),
            |input| {
                input.record(props);
            },
            |machine| {
                let (index, _, component) = machine.component(name)?;
                if let Some(parent) = parent {
                    belongs(&self.data, &parent.data.program, &parent.data.component)?;
                }
                machine.begin_render_track(&[]);
                let normalized = machine.normalize_props(index, name, component, props, parent)?;
                let record = machine
                    .suspend_render_track()
                    .map(crate::reuse::Track::into_inputs)
                    .unwrap_or_default();
                Ok(NormalizedProps {
                    props: normalized.props,
                    handler_props: normalized.handler_props,
                    record,
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
                let props = crate::environment::without_env(props);
                let (fields, _) = machine.split_handler_properties(
                    0,
                    index,
                    fields_from_host(&props, &machine.stack)?,
                    &path,
                )?;
                let cx = entry_cx(index);
                let mut frame = Vec::new();
                machine.normalize_fields(
                    cx,
                    &component.props,
                    &fields,
                    &mut frame,
                    &Path::Root(&path),
                    false,
                )?;
                machine.normalize_fields(
                    cx,
                    &component.state,
                    &fields_from_host(state, &machine.stack)?,
                    &mut frame,
                    &Path::Root(&Labeled(name, " state")),
                    true,
                )?;
                machine.report(
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
    /// it with the component's `State.Update` records; any other handler sees only what it
    /// captured, and everything it returns is an effect. The body is rendered once against the
    /// state the batch produced. A failure returns before anything else does, so the instance
    /// given stays the state of record.</para>
    pub fn dispatch_component_actions(
        &self,
        instance: &ComponentInstance,
        batch: &[NxValue],
        options: &RuntimeOptions,
    ) -> Result<ComponentDispatchResult> {
        let (result, _) = self.dispatch(instance, batch, None, options)?;
        Ok(ComponentDispatchResult {
            rendered: result.rendered.unwrap_or(NxValue::Null),
            effects: result.effects,
            state: result.state,
            instance: result.instance,
        })
    }

    /// The instance tree's dispatch: the batch run as
    /// [`dispatch_component_actions`](Self::dispatch_component_actions) runs it, with the tree's
    /// kept results in `session`, and the body rendered only when the tree needs it. That is
    /// when `must_render`, because the node's other records do not hold, or when a state field
    /// the last render read was patched to another value, each field compared once, its value
    /// at the end against its value at the start, charged as `diff` charges a field. Otherwise
    /// nothing is rendered or written for the output, and the instance keeps its render. A
    /// render's record starts with the tree's `seed`, what normalizing the instance's props read.
    pub(crate) fn dispatch_tracked(
        &self,
        instance: &ComponentInstance,
        batch: &[NxValue],
        must_render: bool,
        options: &RuntimeOptions,
        tree: TreeCall<'_, '_>,
    ) -> Result<(TrackedDispatch, Vec<Input>)> {
        self.dispatch(instance, batch, Some((must_render, tree)), options)
            .map(|(result, record)| (result, record.unwrap_or_default()))
    }

    fn dispatch(
        &self,
        instance: &ComponentInstance,
        batch: &[NxValue],
        tree: Option<(bool, TreeCall<'_, '_>)>,
        options: &RuntimeOptions,
    ) -> Result<(TrackedDispatch, Option<Vec<Input>>)> {
        self.call_in(
            options,
            tree.map(|(_, tree)| tree),
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
                // A component's state record is declared beside it, in the entry module.
                let state_key = component
                    .state_record
                    .as_ref()
                    .map(|state| self.data.key(0, state));
                let mut working = instance.state.clone();
                let mut effects = Vec::new();
                let mut depths = Depths::default();
                // The state fields a patch of the batch supplied.
                let mut patched: Vec<Arc<str>> = Vec::new();

                for (position, entry) in batch.iter().enumerate() {
                    let path = Position("dispatch entry ", position, "");
                    let entry = from_host(entry, &machine.stack)?;
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
                                        && state_key.as_deref().is_some_and(|state_key| {
                                            machine.is_update_record_for(&result, state_key)
                                        }) =>
                                {
                                    for (field, _) in &patch.fields {
                                        if !patched.contains(field) {
                                            patched.push(Arc::clone(field));
                                        }
                                    }
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

                let output = machine.meter(entry_cx(index), None);
                let render = match tree {
                    None => true,
                    Some((must_render, _)) => {
                        must_render
                            || machine.read_field_changed(
                                instance.render.reads.as_ref(),
                                &patched,
                                &instance.state,
                                &working,
                                &output,
                            )?
                    }
                };
                let effects_to_host = |effects: &[Value]| {
                    effects
                        .iter()
                        .map(|effect| to_host(effect, None, &output, &machine.stack))
                        .collect::<Result<Vec<_>>>()
                };
                if !render {
                    return Ok((
                        TrackedDispatch {
                            rendered: None,
                            effects: effects_to_host(&effects)?,
                            state: fields_to_host(&working, &output, &machine.stack)?,
                            instance: ComponentInstance {
                                data: Arc::new(InstanceData {
                                    program: Arc::clone(&instance.program),
                                    component: Arc::clone(&instance.component),
                                    props: instance.props.clone(),
                                    handler_props: instance.handler_props.clone(),
                                    state: working,
                                    render: Arc::clone(&instance.render),
                                }),
                            },
                        },
                        None,
                    ));
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
                let generation = instance.render.generation.saturating_add(1);
                let mut tokens = Tokens::new(generation);
                machine.begin_render_track(tree.map(|(_, tree)| tree.seed).unwrap_or_default());
                machine.begin_render(&frame, component.props.len(), component.state.len());
                let value = machine.eval(entry_cx(index), &mut frame, body)?;
                let (read, record) = machine.end_render();
                let rendered = machine.report(&value, Some(&mut tokens), &output)?;
                Ok((
                    TrackedDispatch {
                        rendered: Some(rendered),
                        effects: effects_to_host(&effects)?,
                        state: fields_to_host(&working, &output, &machine.stack)?,
                        instance: ComponentInstance {
                            data: Arc::new(InstanceData {
                                program: Arc::clone(&instance.program),
                                component: Arc::clone(&instance.component),
                                props: instance.props.clone(),
                                handler_props: instance.handler_props.clone(),
                                state: working,
                                render: Arc::new(Render {
                                    handlers: tokens.handlers,
                                    generation,
                                    reads: Some(render_reads(component, &read)),
                                }),
                            }),
                        },
                    },
                    record,
                ))
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
                    &fields_from_host(state, &machine.stack)?,
                    &mut props_frame(component),
                    &Path::Root(&Labeled(name, " state")),
                    true,
                )?;
                fields_to_host(
                    &state,
                    &machine.meter(entry_cx(index), None),
                    &machine.stack,
                )
            },
        )
    }

    /// Applies a patch to host-owned component state and returns the validated next state.
    ///
    /// <para>The patch is a record: either plain, or the update record of the component's state,
    /// `<Component>.State.Update`. Either way a present field replaces the current value, an absent
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
                let patch = from_host(patch, &machine.stack)?;
                let patch = require_record(&patch, &"the state patch")?;
                let state = machine.patch_state(
                    index,
                    name,
                    component,
                    &fields_from_host(current, &machine.stack)?,
                    patch,
                    &mut Depths::default(),
                )?;
                fields_to_host(
                    &state,
                    &machine.meter(entry_cx(index), None),
                    &machine.stack,
                )
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
                &Path::Root(&Labeled(name, " props")),
                true,
            )
            .or_else(refuse)?;
        data.state = machine
            .normalize_fields(
                cx,
                &component.state,
                &data.state,
                &mut frame,
                &Path::Root(&Labeled(name, " state")),
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
    machine.report(&value, None, &machine.meter(cx, None))
}

struct Position(&'static str, usize, &'static str);

impl fmt::Display for Position {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}{}{}", self.0, self.1, self.2)
    }
}

#[cfg(test)]
mod rebinding_tests {
    use super::*;

    fn handler(node: u32) -> Arc<Handler> {
        Arc::new(Handler {
            module: Arc::from("main"),
            declaration: Arc::from("Page"),
            node,
            component: Arc::from("main::Card"),
            component_name: Arc::from("Card"),
            emit: Arc::from("Logged"),
            action_name: Arc::from("Card.Logged"),
            owner: Some(Arc::from("main::Page")),
            captured: Vec::new(),
        })
    }

    #[test]
    fn an_old_handler_replaced_by_two_new_ones_is_refused() {
        let (old, first, second) = (handler(1), handler(1), handler(1));
        let mut rebinding = Rebinding::default();
        assert!(rebinding.pair(&old, &first));
        assert!(rebinding.pair(&old, &first));
        assert!(!rebinding.pair(&old, &second));
    }

    #[test]
    fn two_old_handlers_replaced_by_one_new_one_are_allowed() {
        let (one, two, new) = (handler(1), handler(1), handler(1));
        let mut rebinding = Rebinding::default();
        assert!(rebinding.pair(&one, &new));
        assert!(rebinding.pair(&two, &new));
    }

    #[test]
    fn a_kept_handler_and_a_replaced_one_do_not_mix() {
        let (old, new) = (handler(1), handler(1));
        let mut rebinding = Rebinding::default();
        assert!(rebinding.pair(&old, &old));
        assert!(!rebinding.pair(&old, &new));
    }
}
