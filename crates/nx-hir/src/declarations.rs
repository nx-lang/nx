//! Checks that a module's top-level declaration names are unique.
//!
//! <para>Every consumer of a module resolves a top-level declaration by name: imports bring names
//! into scope, a runtime linking an NX IR artifact looks a reference up by its name, and a host
//! asks for an entrypoint by name. Two declarations sharing one name would make every one of those
//! lookups a guess, so a module that declares a name twice is rejected here rather than resolved
//! to whichever declaration came last.</para>

use crate::{Item, LoweredModule, Name};
use nx_diagnostics::TextSpan;
use rustc_hash::FxHashMap;

/// A top-level declaration whose name an earlier declaration of the same module already took.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateDeclarationError {
    /// The name both declarations use.
    pub name: Name,
    /// Where the earlier declaration is.
    pub first_span: TextSpan,
    /// Where the later declaration is.
    pub span: TextSpan,
}

impl DuplicateDeclarationError {
    pub fn code(&self) -> &'static str {
        "duplicate-declaration"
    }

    pub fn message(&self) -> String {
        format!(
            "'{}' is declared more than once in this module; top-level names must be unique",
            self.name
        )
    }
}

/// Reports every top-level declaration whose name an earlier declaration already took.
///
/// Derived declarations are skipped: lowering synthesizes `T.Update` and `T.Property` beside each
/// record, action and component named `T`, so a duplicated `T` would otherwise be reported twice
/// more under names the author never wrote.
pub fn validate_declaration_names(module: &LoweredModule) -> Vec<DuplicateDeclarationError> {
    let mut first_spans: FxHashMap<&str, TextSpan> = FxHashMap::default();
    let mut errors = Vec::new();
    for item in module.items() {
        if is_derived_item(item) {
            continue;
        }
        let name = item.name();
        let span = item_span(item);
        match first_spans.get(name.as_str()) {
            Some(first_span) => errors.push(DuplicateDeclarationError {
                name: name.clone(),
                first_span: *first_span,
                span,
            }),
            None => {
                first_spans.insert(name.as_str(), span);
            }
        }
    }
    errors
}

fn is_derived_item(item: &Item) -> bool {
    match item {
        Item::Record(record) => record.update_target().is_some(),
        Item::Union(union_def) => union_def.property_target().is_some(),
        _ => false,
    }
}

fn item_span(item: &Item) -> TextSpan {
    match item {
        Item::Function(item) => item.span,
        Item::Value(item) => item.span,
        Item::Component(item) => item.span,
        Item::TypeAlias(item) => item.span,
        Item::Union(item) => item.span,
        Item::Record(item) => item.span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lower_source_module;

    fn module(source: &str) -> LoweredModule {
        lower_source_module(source, "test.nx").expect("source lowers")
    }

    #[test]
    fn a_value_and_a_type_of_one_name_are_reported_with_both_spans() {
        let source = "let Card = 1\ntype Card = { }\n";
        let errors = validate_declaration_names(&module(source));
        assert_eq!(errors.len(), 1, "{errors:?}");
        let error = &errors[0];
        assert_eq!(error.name.as_str(), "Card");
        assert!(error.message().contains("'Card'"), "{}", error.message());
        assert_eq!(
            &source[error.first_span.start().into()..error.first_span.end().into()],
            "let Card = 1"
        );
        assert!(
            source[error.span.start().into()..error.span.end().into()].starts_with("type Card"),
            "{:?}",
            error.span
        );
    }

    #[test]
    fn a_record_and_its_derived_declarations_are_not_duplicates() {
        let errors = validate_declaration_names(&module("type Card = { title:string }\n"));
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn two_records_of_one_name_are_reported_once() {
        let errors = validate_declaration_names(&module(
            "type Card = { title:string }\ntype Card = { body:string }\n",
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
    }
}
