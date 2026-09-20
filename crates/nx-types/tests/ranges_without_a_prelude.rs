//! A range expression in a build that has no prelude.
//!
//! `nx-types` checks a module on its own, without the library machinery that carries the prelude,
//! so `Range` resolves to nothing here. The operator has no record to construct and says so; the
//! range tests that need a real `Range` live in `nx-api`, where builds carry the prelude.

use nx_types::check_str;

#[test]
fn a_range_expression_reports_that_the_prelude_is_not_available() {
    let result = check_str("let r = {1..5}\n", "test.nx");
    let messages = result
        .errors()
        .iter()
        .map(|diagnostic| diagnostic.message().to_string())
        .collect::<Vec<_>>();

    assert!(
        messages
            .iter()
            .any(|message| message.contains("the prelude is not available")),
        "expected a missing-prelude diagnostic, got: {messages:?}"
    );
}
