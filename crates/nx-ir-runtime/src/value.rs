//! The value the evaluator computes with, and its conversion to and from the host's `NxValue`.
//!
//! <para>Inside the runtime a constant union case, a function and an action handler are values of
//! their own. A host sees the canonical forms: the case's bare name, a `Function` record, and an
//! `ActionHandler` record carrying a token. The empty value is the empty sequence, here as
//! everywhere in NX; the host's `null` is read as it and written in the two places the canonical
//! encoding spells `null`.</para>

use crate::error::{fail, Result};
use nx_value::NxValue;
use std::collections::BTreeMap;
use std::sync::Arc;

/// The `$type` of a rendered handler, and of the batch entry that invokes one by token.
pub(crate) const ACTION_HANDLER_TYPE: &str = "ActionHandler";
pub(crate) const HANDLER_INVOCATION_TYPE: &str = "ActionHandlerInvocation";
/// The `$type` of a rendered function value: a reference to a declaration by module and name.
pub(crate) const FUNCTION_TYPE: &str = "Function";

/// How deeply a value crossing the host boundary may nest. The walks over a value recurse, so
/// this is what keeps a hostile host value, or state a program grew one level per dispatch, from
/// exhausting the native stack.
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
pub(crate) fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Seq(left), Value::Seq(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right.iter())
                    .all(|(left, right)| values_equal(left, right))
        }
        (Value::Seq(items), other) | (other, Value::Seq(items)) => match &**items {
            [only] => values_equal(only, other),
            _ => false,
        },
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::Int(left), Value::Int(right)) => left == right,
        (Value::Float(left), Value::Float(right)) => left == right,
        (Value::Int(integer), Value::Float(float)) | (Value::Float(float), Value::Int(integer)) => {
            *integer as f64 == *float
        }
        (Value::Function(left), Value::Function(right)) => left == right,
        (Value::Handler(left), Value::Handler(right)) => handlers_equal(left, right),
        (Value::Record(left), Value::Record(right)) => {
            Arc::ptr_eq(left, right)
                || (left.type_name == right.type_name
                    && left.fields.len() == right.fields.len()
                    && left.fields.iter().all(|(name, value)| {
                        right
                            .get(name)
                            .is_some_and(|other| values_equal(value, other))
                    }))
        }
        _ => match (left.as_text(), right.as_text()) {
            (Some(left), Some(right)) => left == right,
            _ => false,
        },
    }
}

/// Whether two handlers are one handler: the same node with the same capture.
pub(crate) fn handlers_equal(left: &Arc<Handler>, right: &Arc<Handler>) -> bool {
    Arc::ptr_eq(left, right)
        || (left.module == right.module
            && left.declaration == right.declaration
            && left.node == right.node
            && left.captured.len() == right.captured.len()
            && left
                .captured
                .iter()
                .zip(right.captured.iter())
                .all(|pair| match pair {
                    (Some(left), Some(right)) => values_equal(left, right),
                    (None, None) => true,
                    _ => false,
                }))
}

pub(crate) fn too_deep<T>() -> Result<T> {
    fail(
        "nx-ir-resource-limit",
        format!("A value nests more than {MAX_VALUE_DEPTH} levels deep."),
    )
}

/// Fails when fields nest deeper than a value may, a handler's capture counting as a level.
pub(crate) fn check_depth(fields: &[(Arc<str>, Value)]) -> Result<()> {
    check_depth_fields(fields, 1)
}

fn check_depth_at(value: &Value, depth: u32) -> Result<()> {
    if depth > MAX_VALUE_DEPTH {
        return too_deep();
    }
    let deeper = depth.saturating_add(1);
    match value {
        Value::Seq(items) => items
            .iter()
            .try_for_each(|item| check_depth_at(item, deeper)),
        Value::Record(record) => check_depth_fields(&record.fields, deeper),
        Value::Handler(handler) => handler
            .captured
            .iter()
            .flatten()
            .try_for_each(|item| check_depth_at(item, deeper)),
        _ => Ok(()),
    }
}

fn check_depth_fields(fields: &[(Arc<str>, Value)], depth: u32) -> Result<()> {
    fields
        .iter()
        .try_for_each(|(_, value)| check_depth_at(value, depth))
}

/// Reads a host value. Every `null` is the empty value; nothing else is interpreted until the
/// value reaches a typed site.
pub(crate) fn from_host(value: &NxValue) -> Result<Value> {
    from_host_at(value, 0)
}

fn from_host_at(value: &NxValue, depth: u32) -> Result<Value> {
    if depth > MAX_VALUE_DEPTH {
        return too_deep();
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
                .map(|item| from_host_at(item, deeper))
                .collect::<Result<_>>()?,
        ),
        NxValue::Record {
            type_name,
            properties,
        } => Value::record(
            type_name.as_deref().map(Arc::from),
            fields_from_host_at(properties, deeper)?,
        ),
    })
}

pub(crate) fn fields_from_host(properties: &BTreeMap<String, NxValue>) -> Result<Fields> {
    fields_from_host_at(properties, 1)
}

fn fields_from_host_at(properties: &BTreeMap<String, NxValue>, depth: u32) -> Result<Fields> {
    properties
        .iter()
        .map(|(name, value)| Ok((Arc::from(name.as_str()), from_host_at(value, depth)?)))
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
fn token_text(generation: u64, number: usize) -> String {
    format!("h{generation}-{number}")
}

/// Writes a value in the canonical encoding.
///
/// <para>A handler becomes its `ActionHandler` record: the public name of the action it accepts
/// and, when `tokens` is given, a token numbered by a walk that visits lists in order and record
/// fields by name, which is the walk every NX runtime uses. A function becomes its `Function`
/// record. A present empty field of an update record is written as `null`, the one place a
/// record's field spells it.</para>
pub(crate) fn to_host(value: &Value, tokens: Option<&mut Tokens>) -> Result<NxValue> {
    let mut tokens = tokens;
    to_host_at(value, &mut tokens, 0)
}

pub(crate) fn fields_to_host(fields: &[(Arc<str>, Value)]) -> Result<BTreeMap<String, NxValue>> {
    let mut output = BTreeMap::new();
    for (name, value) in fields {
        output.insert(name.to_string(), to_host_at(value, &mut None, 1)?);
    }
    Ok(output)
}

fn to_host_at(value: &Value, tokens: &mut Option<&mut Tokens>, depth: u32) -> Result<NxValue> {
    if depth > MAX_VALUE_DEPTH {
        return too_deep();
    }
    let deeper = depth.saturating_add(1);
    Ok(match value {
        Value::Bool(value) => NxValue::Bool(*value),
        Value::Int(value) => NxValue::Int(*value),
        Value::Float(value) => NxValue::Float(*value),
        Value::Str(value) => NxValue::String(value.to_string()),
        Value::Case(case) => NxValue::String(case.case.to_string()),
        Value::Seq(items) => NxValue::Array(
            items
                .iter()
                .map(|item| to_host_at(item, tokens, deeper))
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
                let written = to_host_at(field, tokens, deeper)?;
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
