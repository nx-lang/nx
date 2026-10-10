//! The update helpers a host calls on values it holds.

use crate::error::Result;
use crate::eval::{Meter, Stack, NX_DEFAULT_MAX_STACK_BYTES};
use crate::program::Program;
use crate::update;
use crate::value::{from_host, to_host, Value};
use nx_value::NxValue;

/// The stack a helper may use. A helper takes no options and evaluates nothing: it walks values
/// no deeper than a value may nest, under the default budget.
fn host_stack() -> Stack {
    Stack::begin(NX_DEFAULT_MAX_STACK_BYTES)
}

fn record<'a>(value: &'a Value, helper: &str) -> Result<&'a crate::value::Record> {
    update::record_argument(Some(value), helper)
}

/// `apply(record, update)`: the record with each field present in the update replaced, and each
/// field the update clears — present as `null` or the empty value — left out of the result,
/// which is how the canonical encoding writes an empty optional field. The update must be the
/// record's own `<Type>.Update`.
pub fn apply(target: &NxValue, update: &NxValue) -> Result<NxValue> {
    let stack = host_stack();
    let (target, update) = (from_host(target, &stack)?, from_host(update, &stack)?);
    to_host(
        &update::apply(record(&target, "apply")?, record(&update, "apply")?, None)?,
        None,
        &Meter::free(),
        &stack,
    )
}

/// `merge(first, second)`: every field present in either update, the second winning, a cleared
/// field included. A cleared field is `null` in the result, as the canonical encoding spells it.
pub fn merge(first: &NxValue, second: &NxValue) -> Result<NxValue> {
    let stack = host_stack();
    let (first, second) = (from_host(first, &stack)?, from_host(second, &stack)?);
    to_host(
        &update::merge(record(&first, "merge")?, record(&second, "merge")?, None)?,
        None,
        &Meter::free(),
        &stack,
    )
}

/// `diff(before, after)`: the `<Type>.Update` carrying exactly the fields whose values differ,
/// each with its value from `after`, comparing records and lists structurally. A field either
/// record leaves out is empty there, so a field `after` clears is present and `null` in the
/// result.
pub fn diff(before: &NxValue, after: &NxValue) -> Result<NxValue> {
    let stack = host_stack();
    let (before, after) = (from_host(before, &stack)?, from_host(after, &stack)?);
    to_host(
        &update::diff(
            record(&before, "diff")?,
            record(&after, "diff")?,
            None,
            &Meter::free(),
            &stack,
        )?,
        None,
        &Meter::free(),
        &stack,
    )
}

impl Program {
    /// `changed(update)`: the names of the fields present in the update, cleared ones included,
    /// in the order the update record's declaration in this program lists them. Fails when the
    /// program does not declare the update record, since the order is then unknowable from the
    /// value.
    pub fn changed(&self, update: &NxValue) -> Result<Vec<String>> {
        let stack = host_stack();
        let update = from_host(update, &stack)?;
        let update = record(&update, "changed")?;
        let order = update::declared_order(&self.data, update)?;
        Ok(match update::changed(update, &order)? {
            Value::Seq(names) => names
                .iter()
                .filter_map(Value::as_text)
                .map(str::to_string)
                .collect(),
            _ => Vec::new(),
        })
    }
}
