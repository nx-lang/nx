//! Type checking of the update intrinsics `apply`, `merge`, `diff`, and `changed`: reserved-name
//! resolution, arity, argument kinds, and the same-target rule. Covers the intrinsic requirements
//! of the update-records capability.

use nx_types::{check_str, TypeCheckResult};

fn diagnostics(result: &TypeCheckResult) -> Vec<(Option<&str>, String)> {
    result
        .diagnostics
        .iter()
        .map(|diagnostic| (diagnostic.code(), diagnostic.message().to_string()))
        .collect()
}

fn assert_ok(source: &str, file_name: &str) -> TypeCheckResult {
    let result = check_str(source, file_name);
    assert!(
        result.errors().is_empty(),
        "Expected '{}' to type check, got {:?}",
        file_name,
        diagnostics(&result)
    );
    result
}

fn assert_error(source: &str, file_name: &str, code: &str, message_part: &str) {
    let result = check_str(source, file_name);
    assert!(
        result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code() == Some(code) && diagnostic.message().contains(message_part)
        }),
        "Expected '{}' to report {} containing {:?}, got {:?}",
        file_name,
        code,
        message_part,
        diagnostics(&result)
    );
}

fn value_type(result: &TypeCheckResult, name: &str) -> String {
    result
        .type_env
        .lookup(&nx_hir::Name::new(name))
        .map(|ty| ty.to_string())
        .unwrap_or_else(|| panic!("Expected '{}' to be typed", name))
}

// ============================================================================
// Intrinsic functions operate on update records generically
// ============================================================================

#[test]
fn intrinsic_names_cannot_be_redeclared() {
    assert_error(
        "let apply(a:int, b:int) = {a + b}",
        "intrinsic-redeclared.nx",
        "intrinsic-name-reserved",
        "'apply' is an intrinsic function",
    );
    assert_error(
        "let changed = 1",
        "intrinsic-value.nx",
        "intrinsic-name-reserved",
        "'changed' is an intrinsic function",
    );
}

#[test]
fn intrinsic_names_are_not_shadowed_by_a_parameter() {
    let result = assert_ok(
        r#"
        type User = { name:string }
        let f(merge:User) = {merge(<User.Update />, <User.Update />)}
        "#,
        "intrinsic-shadow.nx",
    );
    assert_eq!(value_type(&result, "f"), "(User) => User.Update");
}

#[test]
fn wrong_argument_count_is_rejected() {
    assert_error(
        r#"
        type User = { name:string }
        let a = {apply(<User name="Ada" />)}
        "#,
        "intrinsic-arity.nx",
        "intrinsic-arg-count",
        "'apply' expects 2 arguments, got 1",
    );
}

#[test]
fn wrong_argument_types_are_rejected() {
    assert_error(
        r#"
        type User = { name:string }
        let b = {apply("Ada", <User.Update />)}
        "#,
        "intrinsic-arg-type.nx",
        "intrinsic-argument-type",
        "must be a record or action; found 'string'",
    );
}

// ============================================================================
// apply
// ============================================================================

#[test]
fn apply_yields_the_record_type() {
    let result = assert_ok(
        r#"
        type User = { name:string email:string? }
        let u = <User name="Ada" email="ada@example.com" />
        let v = {apply(u, <User.Update email={null} />)}
        let w = {apply(<User name="Ada" />, <User.Update />)}
        "#,
        "apply-ok.nx",
    );
    assert_eq!(value_type(&result, "v"), "User");
    assert_eq!(value_type(&result, "w"), "User");
}

#[test]
fn apply_rejects_an_update_for_a_different_target() {
    assert_error(
        r#"
        type User = { name:string }
        type Team = { name:string }
        let v = {apply(<User name="Ada" />, <Team.Update name="Core" />)}
        "#,
        "apply-mismatch.nx",
        "intrinsic-target-mismatch",
        "'Team.Update' is not 'User.Update'",
    );
}

#[test]
fn apply_rejects_a_base_records_update_on_a_derived_record() {
    assert_error(
        r#"
        abstract type Named = { name:string }
        type User extends Named = { email:string }
        let v = {apply(<User name="Ada" email="a@b" />, <Named.Update name="Bo" />)}
        "#,
        "apply-base.nx",
        "intrinsic-target-mismatch",
        "'Named.Update' is not 'User.Update'",
    );
}

// ============================================================================
// merge
// ============================================================================

#[test]
fn merge_yields_the_update_type() {
    let result = assert_ok(
        r#"
        type User = { name:string email:string? age:int? }
        let m = {merge(<User.Update name="Ada" email="x@y" />, <User.Update email={null} />)}
        "#,
        "merge-ok.nx",
    );
    assert_eq!(value_type(&result, "m"), "User.Update");
}

#[test]
fn updates_for_different_targets_cannot_be_merged() {
    assert_error(
        r#"
        type User = { name:string }
        type Team = { name:string }
        let m = {merge(<User.Update />, <Team.Update />)}
        "#,
        "merge-mismatch.nx",
        "intrinsic-target-mismatch",
        "target different records",
    );
}

// ============================================================================
// diff
// ============================================================================

#[test]
fn diff_yields_the_update_type() {
    let result = assert_ok(
        r#"
        type User = { name:string email:string? }
        let d = {diff(<User name="Ada" email="x@y" />, <User name="Ada" email={null} />)}
        "#,
        "diff-ok.nx",
    );
    assert_eq!(value_type(&result, "d"), "User.Update");
}

#[test]
fn diff_rejects_records_of_different_types_and_update_records() {
    assert_error(
        r#"
        type User = { name:string }
        type Team = { name:string }
        let d = {diff(<User name="Ada" />, <Team name="Core" />)}
        "#,
        "diff-mismatch.nx",
        "intrinsic-target-mismatch",
        "'Team' is not 'User'",
    );
    assert_error(
        r#"
        type User = { name:string }
        let d = {diff(<User.Update name="Ada" />, <User.Update name="Bo" />)}
        "#,
        "diff-update.nx",
        "intrinsic-argument-type",
        "must be a record or action; found 'User.Update'",
    );
}

// ============================================================================
// changed
// ============================================================================

#[test]
fn changed_yields_a_list_of_property_cases() {
    let result = assert_ok(
        r#"
        type User = { name:string email:string? age:int? }
        let keys = {changed(<User.Update age={null} name="Ada" />)}
        let none = {changed(<User.Update />)}
        "#,
        "changed-ok.nx",
    );
    assert_eq!(value_type(&result, "keys"), "User.Property[]");
    assert_eq!(value_type(&result, "none"), "User.Property[]");
}

#[test]
fn changed_rejects_a_non_update_argument() {
    assert_error(
        r#"
        type User = { name:string }
        let keys = {changed(<User name="Ada" />)}
        "#,
        "changed-record.nx",
        "intrinsic-argument-type",
        "must be an update record; found 'User'",
    );
}
