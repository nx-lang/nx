//! What one call used, reported to a host that asks.

use std::sync::atomic::{AtomicU64, Ordering};

/// The value of a number the last call did not report.
const ABSENT: u64 = u64::MAX;

/// What one call of an evaluation method used, in the units `docs/nx-ir-format.md` defines.
///
/// <para>A host shares one with the runtime through
/// [`RuntimeOptions::usage`](crate::RuntimeOptions::usage) and reads it when the call returns,
/// whether it succeeded or failed. Every call given one clears it first, so it never holds an
/// earlier call's numbers. Calls that run at the same time and share one overwrite each other;
/// give each its own.</para>
#[derive(Debug)]
pub struct Usage {
    operations: AtomicU64,
    input_size: AtomicU64,
}

impl Default for Usage {
    fn default() -> Self {
        Self {
            operations: AtomicU64::new(ABSENT),
            input_size: AtomicU64::new(ABSENT),
        }
    }
}

impl Usage {
    /// A report that holds nothing yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// The operations the last call used, when it ran under
    /// [`RuntimeOptions::max_operations`](crate::RuntimeOptions::max_operations), and `None`
    /// otherwise: with no budget the runtime counts nothing.
    ///
    /// <para>For a call that succeeded this is its operation count, the least budget it succeeds
    /// under. For one that failed it is what was charged before the failure; a charge the budget
    /// refused is not among them, so the number is never more than the budget.</para>
    pub fn operations(&self) -> Option<u64> {
        read(&self.operations)
    }

    /// The input size of the last call, when it ran under
    /// [`RuntimeOptions::max_input_size`](crate::RuntimeOptions::max_input_size) and its input
    /// was within the limit, and `None` otherwise: input that is refused is not measured to its
    /// end.
    pub fn input_size(&self) -> Option<u64> {
        read(&self.input_size)
    }

    pub(crate) fn clear(&self) {
        self.record(None, None);
    }

    pub(crate) fn record(&self, operations: Option<u64>, input_size: Option<u64>) {
        self.operations
            .store(operations.unwrap_or(ABSENT), Ordering::Relaxed);
        self.input_size
            .store(input_size.unwrap_or(ABSENT), Ordering::Relaxed);
    }
}

fn read(number: &AtomicU64) -> Option<u64> {
    match number.load(Ordering::Relaxed) {
        ABSENT => None,
        value => Some(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_holds_nothing_until_a_call_fills_it_and_nothing_after_it_is_cleared() {
        let usage = Usage::new();
        assert_eq!((usage.operations(), usage.input_size()), (None, None));
        usage.record(Some(0), Some(12));
        assert_eq!(
            (usage.operations(), usage.input_size()),
            (Some(0), Some(12))
        );
        usage.record(Some(29), None);
        assert_eq!((usage.operations(), usage.input_size()), (Some(29), None));
        usage.clear();
        assert_eq!((usage.operations(), usage.input_size()), (None, None));
    }
}
