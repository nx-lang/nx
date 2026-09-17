//! The hint a type error carries when an indexed `for` reads like its names were swapped.
//!
//! `for item, index in items` binds the item first. Writing the index first is a mistake that
//! surfaces as an operator or member access on the wrong type, so the error says which name is
//! which.

use nx_types::check_str;

/// Every error in `source`, as `(message, help)`.
fn errors(source: &str) -> Vec<(String, Option<String>)> {
    check_str(source, "test.nx")
        .errors()
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.message().to_string(),
                diagnostic.help().map(str::to_string),
            )
        })
        .collect()
}

fn single_help(source: &str) -> Option<String> {
    let errors = errors(source);
    assert_eq!(errors.len(), 1, "expected one error, got: {errors:?}");
    errors.into_iter().next().unwrap().1
}

const ROW: &str = "type Row = { label:string }\n";

#[test]
fn the_item_used_as_the_index_names_both() {
    let help = single_help(&format!(
        "{ROW}let f(rows:Row[]) = {{ for index, row in rows {{ index % 2 == 0 }} }}"
    ))
    .expect("a hint");
    assert_eq!(
        help,
        "In `for index, row in ...`, `index` is each item (Row) and `row` is its zero-based \
         index (int): the item comes first"
    );
}

#[test]
fn a_string_item_compared_with_an_int_names_both() {
    let help = single_help("let f(names:string[]) = { for index, name in names { index == 0 } }")
        .expect("a hint");
    assert!(help.contains("`index` is each item (string)"), "{help}");
}

#[test]
fn the_index_used_as_the_item_names_both() {
    let help = single_help(&format!(
        "{ROW}let f(rows:Row[]) = {{ for index, row in rows {{ row.label }} }}"
    ))
    .expect("a hint");
    assert!(
        help.contains("`row` is its zero-based index (int)"),
        "{help}"
    );
}

#[test]
fn an_unrelated_error_on_the_item_has_no_hint() {
    assert_eq!(
        single_help(&format!(
            "{ROW}let f(rows:Row[]) = {{ for row, index in rows {{ row.label == 5 }} }}"
        )),
        None
    );
}

#[test]
fn a_loop_over_ints_has_no_hint() {
    assert_eq!(
        single_help("let f(counts:int[]) = { for index, count in counts { index == \"a\" } }"),
        None
    );
}

#[test]
fn a_shadowed_name_has_no_hint() {
    assert_eq!(
        single_help(&format!(
            "{ROW}let f(rows:Row[], names:string[]) = \
             {{ for row, index in rows {{ for index in names {{ index == 0 }} }} }}"
        )),
        None
    );
}
