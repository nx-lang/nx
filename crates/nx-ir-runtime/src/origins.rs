//! Where the records of one call's value were constructed, reported to a host that asks.

use crate::error::SourceSpan;
use crate::module::ModuleData;
use std::fmt::{self, Write};
use std::sync::{Arc, Mutex, PoisonError};

/// One record of a call's value and where it was constructed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginEntry {
    /// The record's JSON pointer (RFC 6901) within the value the host received: `""` for the
    /// value itself, `/content/0` for the first item of its `content` field.
    pub path: String,
    /// The module identity and the byte span, in that module's source, of the element expression
    /// that constructed the record.
    pub source: SourceSpan,
}

/// Where the records of one call's value were constructed.
///
/// <para>A host shares one with the runtime through
/// [`RuntimeOptions::origins`](crate::RuntimeOptions::origins) and reads it when the call
/// returns. Every call given one clears it first, so it never holds an earlier call's entries.
/// A call of [`evaluate_function`](crate::Program::evaluate_function),
/// [`call_function`](crate::Program::call_function),
/// [`initialize_component`](crate::Program::initialize_component),
/// [`evaluate_component`](crate::Program::evaluate_component) or
/// [`dispatch_component_actions`](crate::Program::dispatch_component_actions) that succeeds fills
/// it with one entry for each record of its value, or of its rendered output, that has an
/// origin; a call that fails, and every other call, leaves it empty.</para>
///
/// <para>Entries are in the order of a depth-first walk of the value: a record before its fields,
/// a list's items in order and a record's fields by name, the walk that numbers handler tokens.
/// A record has an origin only when the image of the module that constructed it carries its debug
/// section, so a report over images without one stays empty. Calls that run at the same time and
/// share one overwrite each other; give each its own.</para>
#[derive(Debug, Default)]
pub struct Origins {
    entries: Mutex<Vec<OriginEntry>>,
}

impl Origins {
    /// A report that holds nothing yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// The entries of the last call.
    pub fn entries(&self) -> Vec<OriginEntry> {
        self.lock().clone()
    }

    pub(crate) fn clear(&self) {
        self.lock().clear();
    }

    pub(crate) fn record(&self, entries: Vec<OriginEntry>) {
        *self.lock() = entries;
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<OriginEntry>> {
        // A panic while the lock was held can only have been in a host's own code between two
        // calls; what the report holds is still a list of entries.
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The node that constructed a record: the module it belongs to and its index in that module's
/// node table. Set on a record only in calls given a report.
#[derive(Clone)]
pub(crate) struct Origin {
    pub module: Arc<ModuleData>,
    pub node: u32,
}

impl Origin {
    /// The node's span from its image's debug section, or `None` for an image without one.
    pub(crate) fn source(&self) -> Option<SourceSpan> {
        self.module
            .image()
            .node_span(self.node)
            .map(|(start, end)| SourceSpan {
                identity: self.module.identity.to_string(),
                start,
                end,
            })
    }
}

impl fmt::Debug for Origin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}#{}", self.module.identity, self.node)
    }
}

/// The entries a walk over a value collects, and the JSON pointer of the value it is at.
#[derive(Default)]
pub(crate) struct Collected {
    pub entries: Vec<OriginEntry>,
    path: String,
}

impl Collected {
    /// Adds an entry for the record the walk is at, when it has an origin with a span.
    pub(crate) fn record(&mut self, origin: Option<&Origin>) {
        if let Some(source) = origin.and_then(Origin::source) {
            self.entries.push(OriginEntry {
                path: self.path.clone(),
                source,
            });
        }
    }

    /// The length of the pointer before the walk entered a field or an item, for
    /// [`leave`](Self::leave).
    pub(crate) fn enter_field(&mut self, name: &str) -> usize {
        let before = self.path.len();
        self.path.push('/');
        for character in name.chars() {
            match character {
                '~' => self.path.push_str("~0"),
                '/' => self.path.push_str("~1"),
                other => self.path.push(other),
            }
        }
        before
    }

    pub(crate) fn enter_item(&mut self, index: usize) -> usize {
        let before = self.path.len();
        self.path.push('/');
        // Writing to a `String` cannot fail.
        let _ = write!(self.path, "{index}");
        before
    }

    pub(crate) fn leave(&mut self, before: usize) {
        self.path.truncate(before);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pointer_escapes_a_tilde_and_a_slash_in_a_field_name() {
        let mut collected = Collected::default();
        let field = collected.enter_field("a/b~c");
        let item = collected.enter_item(3);
        assert_eq!(collected.path, "/a~1b~0c/3");
        collected.leave(item);
        assert_eq!(collected.path, "/a~1b~0c");
        collected.leave(field);
        assert_eq!(collected.path, "");
    }

    #[test]
    fn a_report_holds_what_was_recorded_until_it_is_cleared() {
        let origins = Origins::new();
        assert!(origins.entries().is_empty());
        let entry = OriginEntry {
            path: String::new(),
            source: SourceSpan {
                identity: "main.nx".to_string(),
                start: 1,
                end: 2,
            },
        };
        origins.record(vec![entry.clone()]);
        assert_eq!(origins.entries(), vec![entry]);
        origins.clear();
        assert!(origins.entries().is_empty());
    }
}
