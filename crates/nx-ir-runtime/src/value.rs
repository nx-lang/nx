//! The value the evaluator computes with, and its conversion to and from the host's `NxValue`.
//!
//! <para>Inside the runtime a constant union case, a function and an action handler are values of
//! their own. A host sees the canonical forms: the case's bare name, a `Function` record, and an
//! `ActionHandler` record carrying a token. The empty value is the empty sequence, here as
//! everywhere in NX; the host's `null` is read as it and written in the two places the canonical
//! encoding spells `null`.</para>

use crate::error::{fail_limit, Limit, Result};
use crate::eval::{utf16_len, Meter, Stack, TEXT_UNITS};
use nx_value::NxValue;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// The `$type` of a rendered handler, and of the batch entry that invokes one by token.
pub(crate) const ACTION_HANDLER_TYPE: &str = "ActionHandler";
pub(crate) const HANDLER_INVOCATION_TYPE: &str = "ActionHandlerInvocation";
/// The `$type` of a rendered function value: a reference to a declaration by module and name.
pub(crate) const FUNCTION_TYPE: &str = "Function";

/// How deeply a value crossing the host boundary may nest. The walks over a value recurse, so
/// this is what keeps a hostile host value, or state a program grew one level per dispatch, from
/// exhausting the native stack of a host that has the default stack budget free. A host with
/// less says so, and the walks that convert a value ask the budget as well.
pub(crate) const MAX_VALUE_DEPTH: u32 = 256;

#[derive(Debug, Clone)]
pub(crate) enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Arc<str>),
    /// A sequence. The empty one is the empty value.
    Seq(Arc<[Value]>),
    Record(Arc<Record>),
    /// A constant union case: its bare name to a host, and to every string operation.
    Case(Arc<CaseValue>),
    Function(Arc<FunctionRef>),
    Handler(Arc<Handler>),
}

pub(crate) type Fields = Vec<(Arc<str>, Value)>;

#[derive(Debug, Clone, Default)]
pub(crate) struct Record {
    pub type_name: Option<Arc<str>>,
    /// In the order the fields were written; a name appears once.
    pub fields: Fields,
}

#[derive(Debug, Clone)]
pub(crate) struct CaseValue {
    pub union: Arc<str>,
    pub case: Arc<str>,
}

/// A function as a value: a declaration of a linked module, by name. It captures nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FunctionRef {
    pub module: Arc<str>,
    pub name: Arc<str>,
}

/// A handler between the render that created it and the dispatch that runs it.
///
/// <para>Everything here is a name, an index or a value, so an instance that holds handlers is
/// plain data. What the handler answers and accepts is read from the handler node when it runs;
/// the copies here are what a host-facing check or a canonical record needs without the
/// program.</para>
#[derive(Debug, Clone)]
pub(crate) struct Handler {
    /// The module and declaration the handler node was written in, where its body is evaluated.
    pub module: Arc<str>,
    pub declaration: Arc<str>,
    /// The handler node's index in that module's node table.
    pub node: u32,
    /// The declaration key of the component whose emit the handler answers, and its name.
    pub component: Arc<str>,
    pub component_name: Arc<str>,
    pub emit: Arc<str>,
    /// The action record the handler accepts.
    pub action_name: Arc<str>,
    /// The declaration key of the component whose body bound the handler, if one did.
    pub owner: Option<Arc<str>>,
    /// The frame as it stood when the handler was created: the by-value capture.
    pub captured: Vec<Option<Value>>,
}

impl Value {
    pub(crate) fn empty() -> Value {
        Value::Seq(Arc::from(Vec::new()))
    }

    pub(crate) fn seq(items: Vec<Value>) -> Value {
        Value::Seq(Arc::from(items))
    }

    pub(crate) fn str(text: &str) -> Value {
        Value::Str(Arc::from(text))
    }

    pub(crate) fn record(type_name: Option<Arc<str>>, fields: Fields) -> Value {
        Value::Record(Arc::new(Record { type_name, fields }))
    }

    pub(crate) fn is_empty(&self) -> bool {
        matches!(self, Value::Seq(items) if items.is_empty())
    }

    /// The text of a string or of a constant union case.
    pub(crate) fn as_text(&self) -> Option<&str> {
        match self {
            Value::Str(text) => Some(text),
            Value::Case(case) => Some(&case.case),
            _ => None,
        }
    }

    pub(crate) fn as_record(&self) -> Option<&Record> {
        match self {
            Value::Record(record) => Some(record),
            _ => None,
        }
    }

    /// What a diagnostic calls a value of this kind.
    pub(crate) fn describe(&self) -> &'static str {
        match self {
            Value::Bool(_) => "a boolean",
            Value::Int(_) | Value::Float(_) => "a number",
            Value::Str(_) | Value::Case(_) => "a string",
            Value::Seq(_) => "a list",
            Value::Record(_) => "a record",
            Value::Function(_) => "a function",
            Value::Handler(_) => "an action handler",
        }
    }
}

impl Record {
    pub(crate) fn get(&self, name: &str) -> Option<&Value> {
        get_field(&self.fields, name)
    }

    pub(crate) fn type_name(&self) -> Option<&str> {
        self.type_name.as_deref()
    }
}

pub(crate) fn get_field<'a>(fields: &'a [(Arc<str>, Value)], name: &str) -> Option<&'a Value> {
    fields
        .iter()
        .find(|(key, _)| &**key == name)
        .map(|(_, value)| value)
}

/// Sets a field, replacing a value already stored under the name where it stands.
pub(crate) fn set_field(fields: &mut Fields, name: Arc<str>, value: Value) {
    match fields.iter_mut().find(|(key, _)| *key == name) {
        Some(entry) => entry.1 = value,
        None => fields.push((name, value)),
    }
}

pub(crate) fn remove_field(fields: &mut Fields, name: &str) {
    fields.retain(|(key, _)| &**key != name);
}

/// The one equality `==`, match patterns and `diff` share: numbers, strings and booleans by
/// value, lists by their items in order, records by their fields, and a function value by the
/// declaration it names. Every value compares as a sequence, so an item equals a one-element
/// list holding an equal item, and the empty value equals only the empty value.
///
/// <para>Comparing two values costs `meter` one operation for the pair. Two sequences of one
/// length then compare their items in order and stop at the first pair that differs: the order
/// of a sequence is the same in every runtime, so what was compared, and its cost, is too. Two
/// records of one type are lined up by name and compare every pair of field values, whether or
/// not an earlier pair already differed: the order fields are held in is not the same in every
/// runtime, so stopping at the first difference would make the count depend on it. A name only
/// one of them holds costs one. Text costs its length wherever it is read: two strings one more for
/// every 64 UTF-16 code units of the shorter, two records likewise for their type names, and
/// every field name for each record that holds it. So the cost of a comparison is a property of
/// the two values.</para>
///
/// <para>The result never depends on the meter. A walk nobody pays for stops at the first
/// difference between two records as well, which cannot change it; it does not take two values
/// that are one allocation to be equal, which can, since a value that holds a NaN is not equal
/// to itself.</para>
///
/// <para>The walk recurses once for each level the two values nest, so at each pair that can
/// hold others it asks `stack` whether the call still has stack for them.</para>
pub(crate) fn values_equal(
    left: &Value,
    right: &Value,
    meter: &Meter<'_, '_>,
    stack: &Stack,
) -> Result<bool> {
    meter.charge(1)?;
    if (holds_values(left) || holds_values(right)) && !stack.within() {
        return stack.value_too_deep();
    }
    Ok(match (left, right) {
        (Value::Seq(left), Value::Seq(right)) => {
            let mut equal = left.len() == right.len();
            if equal {
                for (left, right) in left.iter().zip(right.iter()) {
                    if !values_equal(left, right, meter, stack)? {
                        equal = false;
                        break;
                    }
                }
            }
            equal
        }
        (Value::Seq(items), other) => match &**items {
            [only] => values_equal(only, other, meter, stack)?,
            _ => false,
        },
        (other, Value::Seq(items)) => match &**items {
            [only] => values_equal(other, only, meter, stack)?,
            _ => false,
        },
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::Int(left), Value::Int(right)) => left == right,
        (Value::Float(left), Value::Float(right)) => left == right,
        (Value::Int(integer), Value::Float(float)) | (Value::Float(float), Value::Int(integer)) => {
            *integer as f64 == *float
        }
        (Value::Function(left), Value::Function(right)) => left == right,
        (Value::Handler(left), Value::Handler(right)) => handlers_equal(left, right, meter, stack)?,
        (Value::Record(left), Value::Record(right)) => records_equal(left, right, meter, stack)?,
        _ => match (left.as_text(), right.as_text()) {
            (Some(left), Some(right)) => {
                meter.charge(shared_length_cost(left, right, meter))?;
                left == right
            }
            _ => false,
        },
    })
}

/// What reading `text` costs beyond the value it belongs to: one operation for every 64 UTF-16
/// code units. Text shorter than that in bytes is shorter in code units, and is not measured.
fn length_cost(text: &str) -> u64 {
    if text.len() < TEXT_UNITS {
        return 0;
    }
    (utf16_len(text) / TEXT_UNITS) as u64
}

/// What comparing two pieces of text costs beyond the pair: the length of the shorter. Nothing
/// is measured for a walk nobody pays for.
fn shared_length_cost(left: &str, right: &str, meter: &Meter<'_, '_>) -> u64 {
    if !meter.is_charged() || left.len().min(right.len()) < TEXT_UNITS {
        return 0;
    }
    (utf16_len(left).min(utf16_len(right)) / TEXT_UNITS) as u64
}

/// Whether two records are equal: one type, the same field names, and equal values under each.
///
/// <para>Two records of different types cost the pair and the shorter type name, and nothing
/// about their fields is read. Two of one type are lined up by field name, and every name
/// either holds is paid for: its length, for each record that holds it, before any name is read;
/// then the pair of values where both hold it, and one operation where only one does. So a
/// comparison that fails on the names costs as much as the names it had to read, however wide
/// the records are and whichever is the wider.</para>
///
/// <para>Fields that line up in order are compared in step, which is how two values of one
/// declaration are held. Otherwise the fields of `right` are indexed once, so the comparison
/// takes time proportional to the two widths. A walk nobody pays for answers from the field
/// counts alone when they differ, and stops at the first difference.</para>
fn records_equal(
    left: &Record,
    right: &Record,
    meter: &Meter<'_, '_>,
    stack: &Stack,
) -> Result<bool> {
    if let (Some(left), Some(right)) = (left.type_name(), right.type_name()) {
        meter.charge(shared_length_cost(left, right, meter))?;
    }
    if left.type_name != right.type_name {
        return Ok(false);
    }
    let charged = meter.is_charged();
    let same_width = left.fields.len() == right.fields.len();
    if charged {
        for (name, _) in left.fields.iter().chain(right.fields.iter()) {
            meter.charge(length_cost(name))?;
        }
    } else if !same_width {
        return Ok(false);
    }
    let in_step = same_width
        && left
            .fields
            .iter()
            .zip(right.fields.iter())
            .all(|((left, _), (right, _))| left == right);
    let mut equal = same_width;
    if in_step {
        for ((_, value), (_, other)) in left.fields.iter().zip(right.fields.iter()) {
            if !values_equal(value, other, meter, stack)? {
                equal = false;
                if !charged {
                    break;
                }
            }
        }
        return Ok(equal);
    }
    // Each field of `right`, and whether `left` holds one of its name.
    let mut index: HashMap<&str, (&Value, bool)> = right
        .fields
        .iter()
        .map(|(name, value)| (&**name, (value, false)))
        .collect();
    for (name, value) in &left.fields {
        let same = match index.get_mut(&**name) {
            Some((other, held)) => {
                *held = true;
                values_equal(value, other, meter, stack)?
            }
            None => {
                meter.charge(1)?;
                false
            }
        };
        if !same {
            equal = false;
            if !charged {
                return Ok(false);
            }
        }
    }
    let only_right = index.values().filter(|(_, held)| !held).count();
    meter.charge(only_right as u64)?;
    Ok(equal && only_right == 0)
}

/// Whether two handlers are one handler: the same node with the same capture. The captured
/// values are compared slot by slot, each pair paid for as `values_equal` compares any pair, up
/// to the first slot that differs: the slots of a frame are in one order in every runtime.
pub(crate) fn handlers_equal(
    left: &Arc<Handler>,
    right: &Arc<Handler>,
    meter: &Meter<'_, '_>,
    stack: &Stack,
) -> Result<bool> {
    let mut equal = left.module == right.module
        && left.declaration == right.declaration
        && left.node == right.node
        && left.captured.len() == right.captured.len();
    if equal {
        for pair in left.captured.iter().zip(right.captured.iter()) {
            let same = match pair {
                (Some(left), Some(right)) => values_equal(left, right, meter, stack)?,
                (None, None) => true,
                _ => false,
            };
            if !same {
                equal = false;
                break;
            }
        }
    }
    Ok(equal)
}

/// Whether a value can hold other values, so that a walk over it goes a level deeper.
fn holds_values(value: &Value) -> bool {
    matches!(value, Value::Seq(_) | Value::Record(_) | Value::Handler(_))
}

pub(crate) fn too_deep<T>() -> Result<T> {
    fail_limit(
        Limit {
            name: "maxValueNesting",
            value: Some(u64::from(MAX_VALUE_DEPTH)),
        },
        format!("A value nests more than {MAX_VALUE_DEPTH} levels deep."),
    )
}

/// What one call has learned about how deeply values nest, for the check that state nests no
/// deeper than a value may after a patch, a handler's capture counting as a level.
///
/// <para>This walk is the runtime's own and no budget pays for it, so it must not grow with the
/// size of the state. It is given only the fields a patch supplies, and it remembers, for the
/// call, how many levels each allocation that more than one thing holds was found to nest: the
/// value a field already held, supplied again or wrapped in a new one, is not walked a second
/// time, and a value that holds another twice is walked once and not as the tree it unfolds to.
/// An allocation it remembers is kept alive, so its address cannot come to name another.</para>
#[derive(Default)]
pub(crate) struct Depths {
    /// The levels each remembered allocation nests, itself included, by its address.
    levels: HashMap<usize, (Value, u32)>,
}

impl Depths {
    /// Fails when a value of `fields` nests deeper than a value may, or when walking it would
    /// take the call past `stack`.
    pub(crate) fn check<'a>(
        &mut self,
        fields: impl IntoIterator<Item = &'a (Arc<str>, Value)>,
        stack: &Stack,
    ) -> Result<()> {
        fields
            .into_iter()
            .try_for_each(|(_, value)| self.levels_of(value, 1, stack).map(|_| ()))
    }

    /// How many levels `value` nests, itself included, having checked that none of them is past
    /// the limit when `value` sits at `depth`.
    fn levels_of(&mut self, value: &Value, depth: u32, stack: &Stack) -> Result<u32> {
        if depth > MAX_VALUE_DEPTH {
            return too_deep();
        }
        if holds_values(value) && !stack.within() {
            return stack.value_too_deep();
        }
        let (address, holders) = match value {
            Value::Seq(items) => (
                Arc::as_ptr(items).cast::<()>() as usize,
                Arc::strong_count(items),
            ),
            Value::Record(record) => (Arc::as_ptr(record) as usize, Arc::strong_count(record)),
            Value::Handler(handler) => (Arc::as_ptr(handler) as usize, Arc::strong_count(handler)),
            _ => return Ok(1),
        };
        // An allocation one thing holds is reached once for each time its holder is, and needs
        // no record.
        let shared = holders > 1;
        if shared {
            if let Some((_, levels)) = self.levels.get(&address) {
                let levels = *levels;
                if depth.saturating_add(levels).saturating_sub(1) > MAX_VALUE_DEPTH {
                    return too_deep();
                }
                return Ok(levels);
            }
        }
        let deeper = depth.saturating_add(1);
        let mut below = 0;
        match value {
            Value::Seq(items) => {
                for item in items.iter() {
                    below = below.max(self.levels_of(item, deeper, stack)?);
                }
            }
            Value::Record(record) => {
                for (_, field) in &record.fields {
                    below = below.max(self.levels_of(field, deeper, stack)?);
                }
            }
            Value::Handler(handler) => {
                for item in handler.captured.iter().flatten() {
                    below = below.max(self.levels_of(item, deeper, stack)?);
                }
            }
            _ => {}
        }
        let levels = below.saturating_add(1);
        if shared {
            self.levels.insert(address, (value.clone(), levels));
        }
        Ok(levels)
    }
}

/// Reads a host value. Every `null` is the empty value; nothing else is interpreted until the
/// value reaches a typed site.
///
/// <para>The walk recurses once for each level the value nests, so besides the levels it counts
/// it asks `stack`, at each value that holds others, whether the call still has stack for
/// them.</para>
pub(crate) fn from_host(value: &NxValue, stack: &Stack) -> Result<Value> {
    from_host_at(value, 0, stack)
}

fn from_host_at(value: &NxValue, depth: u32, stack: &Stack) -> Result<Value> {
    if depth > MAX_VALUE_DEPTH {
        return too_deep();
    }
    if matches!(value, NxValue::Array(_) | NxValue::Record { .. }) && !stack.within() {
        return stack.value_too_deep();
    }
    let deeper = depth.saturating_add(1);
    Ok(match value {
        NxValue::Bool(value) => Value::Bool(*value),
        NxValue::Int32(value) => Value::Int(i64::from(*value)),
        NxValue::Int(value) => Value::Int(*value),
        NxValue::Float32(value) => Value::Float(f64::from(*value)),
        NxValue::Float(value) => Value::Float(*value),
        NxValue::String(value) => Value::str(value),
        NxValue::Null => Value::empty(),
        NxValue::Array(items) => Value::seq(
            items
                .iter()
                .map(|item| from_host_at(item, deeper, stack))
                .collect::<Result<_>>()?,
        ),
        NxValue::Record {
            type_name,
            properties,
        } => Value::record(
            type_name.as_deref().map(Arc::from),
            fields_from_host_at(properties, deeper, stack)?,
        ),
    })
}

pub(crate) fn fields_from_host(
    properties: &BTreeMap<String, NxValue>,
    stack: &Stack,
) -> Result<Fields> {
    fields_from_host_at(properties, 1, stack)
}

fn fields_from_host_at(
    properties: &BTreeMap<String, NxValue>,
    depth: u32,
    stack: &Stack,
) -> Result<Fields> {
    properties
        .iter()
        .map(|(name, value)| Ok((Arc::from(name.as_str()), from_host_at(value, depth, stack)?)))
        .collect()
}

/// The handlers a rendered output carries, in the order their tokens number them.
pub(crate) struct Tokens {
    pub generation: u64,
    pub handlers: Vec<Arc<Handler>>,
}

impl Tokens {
    pub(crate) fn new(generation: u64) -> Self {
        Self {
            generation,
            handlers: Vec::new(),
        }
    }
}

/// The token of the `number`th handler of a generation's rendered output, counting from one.
pub(crate) fn token_text(generation: u64, number: usize) -> String {
    format!("h{generation}-{number}")
}

/// Writes a value in the canonical encoding.
///
/// <para>A handler becomes its `ActionHandler` record: the public name of the action it accepts
/// and, when `tokens` is given, a token numbered by a walk that visits lists in order and record
/// fields by name, which is the walk every NX runtime uses. A function becomes its `Function`
/// record. A present empty field of an update record is written as `null`, the one place a
/// record's field spells it.</para>
///
/// <para>Writing a value for the host costs `meter` one operation for each value written, a
/// sequence and the empty value included, and one more for every 64 UTF-16 code units of a
/// string, of a record's type name and of each of its field names, charged before the value is
/// written. Every value costs one because every value takes a place in what is written: a list
/// of empty values is as long to copy as a list of numbers. This is where a value held once and reached by
/// many paths becomes as many copies, so it is where its size as a tree is paid for.</para>
///
/// <para>Like [`from_host`], the walk asks `stack` at each value that holds others.</para>
pub(crate) fn to_host(
    value: &Value,
    tokens: Option<&mut Tokens>,
    meter: &Meter<'_, '_>,
    stack: &Stack,
) -> Result<NxValue> {
    let mut tokens = tokens;
    to_host_at(value, &mut tokens, 0, meter, stack)
}

pub(crate) fn fields_to_host(
    fields: &[(Arc<str>, Value)],
    meter: &Meter<'_, '_>,
    stack: &Stack,
) -> Result<BTreeMap<String, NxValue>> {
    let mut output = BTreeMap::new();
    for (name, value) in fields {
        output.insert(
            name.to_string(),
            to_host_at(value, &mut None, 1, meter, stack)?,
        );
    }
    Ok(output)
}

/// What writing a string of `text` for the host costs: one for the value and one for every 64
/// UTF-16 code units.
fn text_cost(text: &str) -> u64 {
    length_cost(text).saturating_add(1)
}

/// What writing a record for the host costs, apart from its field values: one for the record,
/// and the length of its type name and of each field name, which are text written with it. A
/// declared name is short and costs nothing; one a host supplied may not be.
fn record_cost(record: &Record) -> u64 {
    record
        .fields
        .iter()
        .map(|(name, _)| length_cost(name))
        .chain(record.type_name().map(length_cost))
        .fold(1, u64::saturating_add)
}

fn to_host_at(
    value: &Value,
    tokens: &mut Option<&mut Tokens>,
    depth: u32,
    meter: &Meter<'_, '_>,
    stack: &Stack,
) -> Result<NxValue> {
    if depth > MAX_VALUE_DEPTH {
        return too_deep();
    }
    if matches!(value, Value::Seq(_) | Value::Record(_)) && !stack.within() {
        return stack.value_too_deep();
    }
    let deeper = depth.saturating_add(1);
    // Every value is one value, a sequence included, and then its items pay for themselves. A
    // constant case is written as its name, which is a string to the host.
    if meter.is_charged() {
        match value {
            Value::Str(text) => meter.charge(text_cost(text))?,
            Value::Case(case) => meter.charge(text_cost(&case.case))?,
            Value::Record(record) => meter.charge(record_cost(record))?,
            _ => meter.charge(1)?,
        }
    }
    Ok(match value {
        Value::Bool(value) => NxValue::Bool(*value),
        Value::Int(value) => NxValue::Int(*value),
        Value::Float(value) => NxValue::Float(*value),
        Value::Str(value) => NxValue::String(value.to_string()),
        Value::Case(case) => NxValue::String(case.case.to_string()),
        Value::Seq(items) => NxValue::Array(
            items
                .iter()
                .map(|item| to_host_at(item, tokens, deeper, meter, stack))
                .collect::<Result<_>>()?,
        ),
        Value::Function(function) => NxValue::Record {
            type_name: Some(FUNCTION_TYPE.to_string()),
            properties: BTreeMap::from([
                (
                    "module".to_string(),
                    NxValue::String(function.module.to_string()),
                ),
                (
                    "name".to_string(),
                    NxValue::String(function.name.to_string()),
                ),
            ]),
        },
        Value::Handler(handler) => {
            let mut properties = BTreeMap::from([(
                "action".to_string(),
                NxValue::String(handler.action_name.to_string()),
            )]);
            if let Some(tokens) = tokens {
                tokens.handlers.push(Arc::clone(handler));
                properties.insert(
                    "token".to_string(),
                    NxValue::String(token_text(tokens.generation, tokens.handlers.len())),
                );
            }
            NxValue::Record {
                type_name: Some(ACTION_HANDLER_TYPE.to_string()),
                properties,
            }
        }
        Value::Record(record) => {
            let is_update = record
                .type_name()
                .is_some_and(|name| name.ends_with(".Update"));
            // Fields are visited by name so the token numbering is the same in every runtime.
            let mut ordered: Vec<&(Arc<str>, Value)> = record.fields.iter().collect();
            ordered.sort_by(|left, right| left.0.cmp(&right.0));
            let mut properties = BTreeMap::new();
            for (name, field) in ordered {
                let written = to_host_at(field, tokens, deeper, meter, stack)?;
                let written = if is_update && written.is_empty_value() {
                    NxValue::Null
                } else {
                    written
                };
                properties.insert(name.to_string(), written);
            }
            NxValue::Record {
                type_name: record.type_name().map(str::to_string),
                properties,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrap(inner: Value) -> Value {
        Value::record(Some(Arc::from("w")), vec![(Arc::from("v"), inner)])
    }

    /// A value that nests `levels` levels, itself and its leaf included.
    fn chain(levels: u32) -> Value {
        (1..levels).fold(Value::Int(0), |value, _| wrap(value))
    }

    fn field(value: Value) -> Fields {
        vec![(Arc::from("f"), value)]
    }

    /// A stack budget no walk of these tests meets.
    fn roomy() -> Stack {
        Stack::begin(crate::NX_DEFAULT_MAX_STACK_BYTES)
    }

    /// Whether `result` is the failure of a walk that met the stack budget `bytes`.
    fn met_stack_budget<T: std::fmt::Debug>(result: Result<T>, bytes: u64) -> bool {
        match result {
            Err(error) => {
                let limit = error.diagnostics[0].limit.expect("a limit");
                assert_eq!((limit.name, limit.value), ("maxStackBytes", Some(bytes)));
                true
            }
            Ok(_) => false,
        }
    }

    /// The walks the evaluator makes over values it holds, each over a value 200 levels deep,
    /// which a value may be: under a budget a few levels use up, each stops with a diagnostic
    /// where it would otherwise recurse to the end of the value on whatever stack there is.
    #[test]
    fn a_comparison_and_the_nesting_walk_stop_at_the_stack_budget() {
        let (left, right) = (chain(200), chain(200));
        let free = Meter::free();
        assert!(matches!(
            values_equal(&left, &right, &free, &roomy()),
            Ok(true)
        ));
        assert!(met_stack_budget(
            values_equal(&left, &right, &free, &Stack::begin(1 << 10)),
            1024
        ));
        // A list holding one value is compared with the value itself, another way down.
        let listed = (0..200).fold(Value::Int(0), |value, _| Value::seq(vec![value]));
        assert!(met_stack_budget(
            values_equal(&listed, &Value::Int(1), &free, &Stack::begin(1 << 10)),
            1024
        ));

        let capturing = |value: Value| {
            Arc::new(Handler {
                module: Arc::from("m"),
                declaration: Arc::from("d"),
                node: 0,
                component: Arc::from("m::C"),
                component_name: Arc::from("C"),
                emit: Arc::from("E"),
                action_name: Arc::from("C.E"),
                owner: None,
                captured: vec![Some(value)],
            })
        };
        let (first, second) = (capturing(chain(200)), capturing(chain(200)));
        assert!(matches!(
            handlers_equal(&first, &second, &free, &roomy()),
            Ok(true)
        ));
        assert!(met_stack_budget(
            handlers_equal(&first, &second, &free, &Stack::begin(1 << 10)),
            1024
        ));

        assert!(Depths::default()
            .check(&field(chain(200)), &roomy())
            .is_ok());
        assert!(met_stack_budget(
            Depths::default().check(&field(chain(200)), &Stack::begin(1 << 10)),
            1024
        ));
    }

    fn too_deep_for(depths: &mut Depths, value: Value) -> bool {
        match depths.check(&field(value), &roomy()) {
            Ok(()) => false,
            Err(error) => {
                let limit = error.diagnostics[0].limit.expect("a limit");
                assert_eq!((limit.name, limit.value), ("maxValueNesting", Some(256)));
                true
            }
        }
    }

    #[test]
    fn the_nesting_limit_is_exact_for_a_value_walked_and_for_one_remembered() {
        assert!(!too_deep_for(&mut Depths::default(), chain(256)));
        assert!(too_deep_for(&mut Depths::default(), chain(257)));

        // Two hundred levels that two things hold, so they are remembered. Under 56 wraps their
        // deepest level is the 256th; under 57 it is one too deep. Each order of meeting them is
        // tried: remembered shallow and then met deep, where only the sum can refuse, and walked
        // deep first.
        let shared = chain(200);
        let under = |wraps: u32| (0..wraps).fold(shared.clone(), |value, _| wrap(value));
        let mut depths = Depths::default();
        assert!(!too_deep_for(&mut depths, shared.clone()));
        assert!(!too_deep_for(&mut depths, under(56)));
        assert!(too_deep_for(&mut depths, under(57)));
        assert!(!too_deep_for(&mut depths, under(56)));
        let mut depths = Depths::default();
        assert!(too_deep_for(&mut depths, under(57)));
        assert!(!too_deep_for(&mut depths, under(56)));
    }

    #[test]
    fn a_value_held_twice_at_every_level_is_walked_once() {
        // Sixty levels, each holding the one below twice: 2^60 values to a walk that follows
        // every path, and sixty to one that remembers. The walk runs on a thread of its own so
        // that one which does not remember fails here, in seconds, and does not hang the run.
        let (done, finished) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let pair = |value: Value| {
                Value::record(
                    Some(Arc::from("p")),
                    vec![(Arc::from("a"), value.clone()), (Arc::from("b"), value)],
                )
            };
            let shared = (0..60).fold(Value::Int(0), |value, _| pair(value));
            let mut depths = Depths::default();
            let fits = !too_deep_for(&mut depths, shared);
            let deep = (0..300).fold(Value::Int(0), |value, _| pair(value));
            let _ = done.send(fits && too_deep_for(&mut depths, deep));
        });
        match finished.recv_timeout(std::time::Duration::from_secs(10)) {
            Ok(as_expected) => assert!(as_expected),
            Err(_) => panic!("the depth check walked a shared value once for every path to it"),
        }
    }
}
