//! The update intrinsics: `apply`, `merge`, `diff` and `changed`, over records.

use crate::error::{fail, Result};
use crate::eval::{Meter, Stack};
use crate::program::ProgramData;
use crate::value::{remove_field, set_field, values_equal, Record, Value};
use std::sync::Arc;

/// The record an intrinsic's argument is: a value with a `$type`.
pub(crate) fn record_argument<'a>(value: Option<&'a Value>, intrinsic: &str) -> Result<&'a Record> {
    match value {
        Some(Value::Record(record)) if record.type_name.is_some() => Ok(record),
        other => fail(
            "nx-ir-intrinsic",
            format!(
                "Intrinsic '{intrinsic}' expects a record value, got {}.",
                other.map(Value::describe).unwrap_or("nothing")
            ),
        ),
    }
}

fn type_name(record: &Record) -> &str {
    record.type_name().unwrap_or("")
}

/// `apply(record, update)`: every present field of the update replaces the record's. A present
/// empty value clears the field, and a record stores no entry for an empty optional field, so
/// the entry is removed rather than set to the empty value.
pub(crate) fn apply(record: &Record, update: &Record) -> Result<Value> {
    let expected = format!("{}.Update", type_name(record));
    if type_name(update) != expected {
        return fail(
            "nx-ir-intrinsic",
            format!(
                "Cannot apply '{}' to a '{}': only '{expected}' patches it.",
                type_name(update),
                type_name(record)
            ),
        );
    }
    let mut fields = record.fields.clone();
    for (name, value) in &update.fields {
        if value.is_empty() {
            remove_field(&mut fields, name);
        } else {
            set_field(&mut fields, Arc::clone(name), value.clone());
        }
    }
    Ok(Value::record(record.type_name.clone(), fields))
}

/// `merge(first, second)`: every field present in either update, the second winning, a cleared
/// field included.
pub(crate) fn merge(first: &Record, second: &Record) -> Result<Value> {
    if first.type_name != second.type_name {
        return fail(
            "nx-ir-intrinsic",
            format!(
                "Cannot merge '{}' with '{}': the updates target different records.",
                type_name(first),
                type_name(second)
            ),
        );
    }
    let mut fields = first.fields.clone();
    for (name, value) in &second.fields {
        set_field(&mut fields, Arc::clone(name), value.clone());
    }
    Ok(Value::record(first.type_name.clone(), fields))
}

/// `diff(before, after)`: the update carrying exactly the fields whose values differ, each with
/// its value from `after`. A field either record leaves out is an empty optional there, so a
/// field only one of them carries still compares, and one `after` leaves out is present and
/// empty in the result. Each comparison is paid for by `meter`, as any equality is.
pub(crate) fn diff(
    before: &Record,
    after: &Record,
    meter: &Meter<'_, '_>,
    stack: &Stack,
) -> Result<Value> {
    if before.type_name != after.type_name {
        return fail(
            "nx-ir-intrinsic",
            format!(
                "Cannot diff '{}' against '{}': the records have different types.",
                type_name(before),
                type_name(after)
            ),
        );
    }
    let empty = Value::empty();
    let mut fields = Vec::new();
    let names = before.fields.iter().map(|(name, _)| name).chain(
        after
            .fields
            .iter()
            .map(|(name, _)| name)
            .filter(|name| before.get(name).is_none()),
    );
    for name in names {
        let next = after.get(name).unwrap_or(&empty);
        if !values_equal(before.get(name).unwrap_or(&empty), next, meter, stack)? {
            fields.push((Arc::clone(name), next.clone()));
        }
    }
    Ok(Value::record(
        Some(Arc::from(format!("{}.Update", type_name(before)))),
        fields,
    ))
}

/// The field names of the update record's declaration, in declaration order. Fails when the
/// program does not declare exactly one record of the name, since the order is then unknowable
/// from the value.
pub(crate) fn declared_order(program: &ProgramData, update: &Record) -> Result<Vec<Arc<str>>> {
    let mut shapes = program.shapes(type_name(update));
    match (shapes.next(), shapes.next()) {
        (Some((_, shape)), None) => Ok(shape
            .fields
            .iter()
            .map(|field| Arc::clone(&field.name))
            .collect()),
        _ => fail(
            "nx-ir-intrinsic",
            format!(
                "Cannot order the fields of '{}': the program does not declare it.",
                type_name(update)
            ),
        ),
    }
}

/// `changed(update)`: the names of the fields present in the update, cleared ones included, in
/// `order`, with any name `order` lacks after them.
pub(crate) fn changed(update: &Record, order: &[Arc<str>]) -> Result<Value> {
    let position = |name: &Arc<str>| {
        order
            .iter()
            .position(|known| known == name)
            .unwrap_or(order.len())
    };
    let mut names: Vec<&Arc<str>> = update.fields.iter().map(|(name, _)| name).collect();
    names.sort_by_key(|name| position(name));
    Ok(Value::seq(
        names
            .into_iter()
            .map(|name| Value::Str(Arc::clone(name)))
            .collect(),
    ))
}
