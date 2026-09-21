//! The flat sequence model: a sequence never contains a sequence, and an item in a collecting
//! position contributes its items.
//!
//! One case per scenario in the `sequence-model` capability.

use nx_hir::Name;
use nx_types::{check_str, Type, TypeCheckResult};

fn check(source: &str) -> TypeCheckResult {
    check_str(source, "test.nx")
}

fn errors(source: &str) -> Vec<String> {
    check(source)
        .errors()
        .iter()
        .map(|diagnostic| diagnostic.message().to_string())
        .collect()
}

fn assert_clean(source: &str) {
    let errors = errors(source);
    assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
}

fn binding_type(source: &str, name: &str) -> Type {
    check(source)
        .type_env
        .lookup(&Name::new(name))
        .unwrap_or_else(|| panic!("no binding named {name:?}"))
        .clone()
}

// ---------------------------------------------------------------------------
// A sequence never contains a sequence
// ---------------------------------------------------------------------------

#[test]
fn a_nested_sequence_reached_through_an_alias_is_rejected() {
    let errors = errors("type Names = string[]\ntype Rows = Names[]");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("A sequence cannot contain sequences") && errors[0].contains("Names"),
        "{errors:?}"
    );
}

#[test]
fn an_alias_to_a_sequence_still_takes_a_nullable_suffix() {
    assert_clean("type Names = string[]\ntype MaybeNames = Names?\nlet n:MaybeNames = null");
}

#[test]
fn a_nested_sequence_through_an_alias_is_rejected_at_a_use_site() {
    let errors = errors("type Names = string[]\ntype Holder = { rows:Names[] }");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("A sequence cannot contain sequences"),
        "{errors:?}"
    );
}

#[test]
fn a_nullable_sequence_and_a_sequence_of_nullables_remain_item_typed() {
    assert_clean("type A = { names:string[]? aliases:string?[] }");
}

#[test]
fn a_record_is_how_data_nests() {
    assert_clean(
        "type Row = { cells:int[] }\nlet grid:Row[] = { <Row cells={1 2}/> <Row cells={3 4}/> }",
    );
    let grid = binding_type(
        "type Row = { cells:int[] }\nlet grid:Row[] = { <Row cells={1 2}/> }",
        "grid",
    );
    match &grid {
        Type::Array(element) => assert!(
            matches!(element.as_ref(), Type::Named(named) if named.name.as_str() == "Row"),
            "grid should be a sequence of records: {grid:?}"
        ),
        other => panic!("grid should be a sequence: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// A collecting position splices a sequence-valued item
// ---------------------------------------------------------------------------

const XS: &str = "let xs:string[] = {\"a\" \"b\"}\nlet ys:string[] = {\"c\"}\n";

#[test]
fn two_sequences_in_a_braced_value_concatenate() {
    let source = format!("{XS}let all = {{xs ys}}");
    assert_clean(&source);
    assert_eq!(binding_type(&source, "all"), Type::array(Type::string()));
}

#[test]
fn a_sequence_and_an_item_in_a_braced_value_share_an_item_type() {
    let source = format!("{XS}let all = {{xs \"c\"}}");
    assert_clean(&source);
    assert_eq!(binding_type(&source, "all"), Type::array(Type::string()));
}

#[test]
fn body_content_and_braced_values_follow_the_same_rule() {
    const LIST: &str = "type Badge = { n:int = 1 }\n\
         external component <List content items:Badge[] />\n\
         let some:Badge[] = { <Badge/> <Badge/> }\n";
    assert_clean(&format!("{LIST}let a = <List>{{some}}<Badge/></List>"));
    assert_clean(&format!("{LIST}let b = <List items={{some <Badge/>}} />"));
}

#[test]
fn a_for_concatenates_what_its_body_yields() {
    assert_clean(
        "type Row = { cells:int[] }\nlet flat(rows:Row[]): int[] = {for r in rows { r.cells }}",
    );
}

#[test]
fn a_for_body_that_yields_one_item_still_yields_a_sequence() {
    assert_clean("let squares(ns:int[]): int[] = {for n in ns { n * n }}");
}

#[test]
fn a_conditional_for_body_is_the_filter_idiom() {
    assert_clean("let evens(ns:int[]): int[] = {for n in ns { if (n % 2 == 0) { n } }}");
}

// ---------------------------------------------------------------------------
// A conditional with no `else` contributes no items
// ---------------------------------------------------------------------------

const BOX: &str = "type A = { n:int = 1 }\n\
     type Box = { content items:A[] }\n\
     let c = false\n";

#[test]
fn a_conditional_child_does_not_widen_the_declared_element_type() {
    assert_clean(&format!(
        "{BOX}let root() = {{ <Box><A/>{{if c {{ <A/> }}}}</Box> }}"
    ));
}

#[test]
fn a_conditional_that_is_the_whole_body_is_accepted() {
    assert_clean(&format!(
        "{BOX}let root() = {{ <Box>{{if c {{ <A/> }}}}</Box> }}"
    ));
}

// ---------------------------------------------------------------------------
// A conditional with no `else` is nullable in a value position
// ---------------------------------------------------------------------------

#[test]
fn a_nullable_annotation_needs_the_else_written() {
    // A conditional with no `else` carries an implicit `else { }`, so it is a sequence and does
    // not satisfy a nullable site by being able to take no branch. Null is written, not implied.
    let errors = errors(
        "let c = false\ntype Holder = { v:string? }\nlet h = <Holder v={if { c => \"x\" }} />",
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("string[]"), "{errors:?}");

    assert_clean(
        "let c = false\ntype Holder = { v:string? }\n\
         let h = <Holder v={if { c => \"x\" else => null }} />",
    );
}

#[test]
fn a_scalar_annotation_is_rejected_naming_the_sequence_type() {
    let errors = errors(
        "let c = false\ntype Holder = { v:string }\nlet h = <Holder v={if { c => \"x\" }} />",
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("string[]"), "{errors:?}");
    assert!(!errors[0].contains("void"), "{errors:?}");
}

#[test]
fn a_conditional_with_an_else_is_not_made_a_sequence() {
    assert_clean("let c = false\ntype Holder = { v:string }\nlet h = <Holder v={if { c => \"x\" else => \"y\" }} />");
    assert_clean(
        "let c = false\ntype Holder = { v:int }\nlet h = <Holder v={if { c => 1 else => 2 }} />",
    );
}

#[test]
fn an_exhaustive_union_match_is_not_made_a_sequence() {
    assert_clean(
        "type Fit = fill | cover\nlet f:Fit = fill\ntype Holder = { v:string }\n\
         let h = <Holder v={if f is { fill => \"a\" cover => \"b\" }} />",
    );
}

#[test]
fn a_non_exhaustive_union_match_does_not_report_the_top_type() {
    let errors = errors(
        "type Fit = fill | cover\nlet f:Fit = fill\nlet xs:string[] = {if f is { fill => \"a\" }}",
    );
    assert!(
        errors
            .iter()
            .any(|message| message.contains("missing cases")),
        "{errors:?}"
    );
    assert!(
        errors
            .iter()
            .all(|message| !message.contains("found object") && !message.contains("void")),
        "{errors:?}"
    );
}

// ---------------------------------------------------------------------------
// An item is a sequence of one, and only one level deep
// ---------------------------------------------------------------------------

#[test]
fn a_scalar_lifts_once_at_a_sequence_site() {
    assert_clean("type Box = { items:int[] }\nlet b = <Box items={1} />");
}

#[test]
fn a_nullable_item_does_not_lift_to_a_nullable_sequence() {
    let errors = errors(
        "type Box = { names:string[]? }\nlet maybe:string? = null\nlet b = <Box names={maybe} />",
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("string?") && errors[0].contains("string[]?"),
        "{errors:?}"
    );
}

#[test]
fn two_sequence_valued_branches_still_join_as_a_sequence() {
    // The one caller of the sequence-to-sequence join that the contribution rule does not remove:
    // a value position whose branches are both sequences. This is not a collecting position, so
    // the branches are joined whole rather than by what each contributes.
    assert_clean("let pick(c:boolean, xs:string[], ys:string[]): string[] = {if c {xs} else {ys}}");

    // Element types that differ join by the checker's own rules, which is what makes the join a
    // method on the inference context rather than the structural one in `semantics`: only the
    // checker knows that `Admin` and `User` share the base record `Person`. Without that recursion
    // this reports `expects Person[], found object[]`.
    assert_clean(
        "abstract type Person = { name:string }\n\
         type Admin extends Person = { level:int }\n\
         type User extends Person = { email:string }\n\
         let pick(c:boolean, admins:Admin[], users:User[]): Person[] = {if c {admins} else {users}}",
    );
}

#[test]
fn a_conditional_nested_in_a_conditional_contributes_an_item_not_a_nullable_one() {
    // The contribution of a conditional with no `else` is what its branch *contributes*, not what
    // its branch *is*. The two differ exactly here: the inner conditional's own type is `A?`,
    // because a value position gets null when it is not taken, but what it contributes is an `A`
    // or nothing at all. Recording the type instead widens the element type one `?` per level, so
    // this binding would be rejected as `A?[]` at an `A[]` site.
    assert_clean(
        "type A = { n:int = 1 }\n\
         type Box = { content items:A[] }\n\
         let c = true\n\
         let d = false\n\
         let root() = { <Box><A/>{if c { if d { <A/> } }}</Box> }",
    );

    // Three levels deep is still an `A`: the recursion bottoms out at the item, not at a `?` per
    // level.
    assert_clean(
        "type A = { n:int = 1 }\n\
         type Box = { content items:A[] }\n\
         let c = true\n\
         let root() = { <Box>{if c { if c { if c { <A/> } } }}</Box> }",
    );

    // A match with an uncovered path is the same rule over arms rather than over one branch.
    assert_clean(
        "type A = { n:int = 1 }\n\
         type Box = { content items:A[] }\n\
         let which = \"a\"\n\
         let c = false\n\
         let root() = { <Box><A/>{if which is { \"a\" => { if c { <A/> } } }}</Box> }",
    );
}

#[test]
fn a_nested_conditional_is_a_sequence_of_items_not_a_sequence_of_sequences() {
    // Two missing `else` arms are two implicit `{}`s, and a sequence never contains a sequence, so
    // the type is `int[]` at any depth rather than gaining a layer per level.
    for source in [
        "let c = true\nlet d = false\nlet v = { if c { if d { 1 } } }",
        "let c = true\nlet v = { if c { if c { if c { 1 } } } }",
    ] {
        let ty = check(source)
            .type_env
            .lookup(&Name::new("v"))
            .cloned()
            .expect("binding v");
        assert_eq!(ty, Type::array(Type::int()), "{source:?} got: {ty}");
    }
}

#[test]
fn nesting_a_sequence_through_a_name_is_reported_once_at_the_name_that_nests_it() {
    // An alias's target is walked again at every use, so the count used to grow with the uses and
    // every report but one pointed at a line naming neither the alias's target nor a `[]`.
    let through_an_alias = errors(
        "type Names = string[]\n\
         type Rows = Names[]\n\
         let a:Rows = { \"x\" }\n\
         let b:Rows = { \"y\" }",
    );
    assert_eq!(through_an_alias.len(), 1, "{through_an_alias:?}");
    assert!(
        through_an_alias[0].contains("'Names' is already a sequence"),
        "{through_an_alias:?}"
    );

    // An alias nobody uses is still reported, at its own declaration.
    let unused = errors("type Names = string[]\ntype Unused = Names[]\nlet v = 1");
    assert_eq!(unused.len(), 1, "{unused:?}");

    // Where the nesting is written out, post-parse validation has already rejected the `[]`
    // itself, at a better span than a whole declaration -- so that report is the only one. The
    // resolver used to add a second saying the same thing about the alias as a whole.
    let spelled = errors("type Matrix = string[][]\nlet m:Matrix = { }");
    assert_eq!(spelled.len(), 1, "{spelled:?}");

    // `?` changes a layer rather than adding one, so an alias to a nullable sequence is fine.
    assert_clean("type Names = string[]\ntype Ok = Names?\nlet v:Ok = { \"a\" }");

    // The count is a property of resolving each alias once, not of the message it produces, so a
    // spelling whose diagnostic cannot name the alias is reported once as well.
    let through_a_nullable = errors(
        "type Names = string[]\n\
         type X = Names?[]\n\
         let a:X = { \"x\" }\n\
         let b:X = { \"y\" }",
    );
    assert_eq!(through_a_nullable.len(), 1, "{through_a_nullable:?}");

    // Same for an alias whose target is wrong for a reason that has nothing to do with sequences:
    // resolving it once per use used to multiply every one of those reports too.
    let type_argument = errors(
        "type Box = { T:type v:T }\n\
         type Bad = <Box T=int[]/>\n\
         let a:Bad = null\n\
         let b:Bad = null",
    );
    assert_eq!(type_argument.len(), 1, "{type_argument:?}");

    let missing_argument = errors("type Box = { T:type v:T }\ntype Bare = Box\nlet a:Bare = null");
    assert_eq!(missing_argument.len(), 1, "{missing_argument:?}");

    // Two declarations that each write the mistake are two mistakes, at their own spans.
    let two_fields = errors(
        "type Names = string[]\n\
         type A = { x:Names[] }\n\
         type B = { y:Names[] }\n\
         let v = 1",
    );
    assert_eq!(two_fields.len(), 2, "{two_fields:?}");
}

#[test]
fn where_a_nested_sequence_is_reported_does_not_depend_on_declaration_order() {
    // An alias may name one declared after it, so the eager pass reaches the later alias while
    // resolving the earlier one. Both the count and the span have to come out the same either
    // way: one report, on the alias whose target actually writes the `[]`.
    let in_order = check(
        "type Names = string[]\n\
         type Rows = Names[]\n\
         type Alias2 = Rows\n\
         let v = 1",
    );
    let reordered = check(
        "type Names = string[]\n\
         type Alias2 = Rows\n\
         type Rows = Names[]\n\
         let v = 1",
    );

    for (label, result) in [("in order", &in_order), ("reordered", &reordered)] {
        let reported = result.errors();
        assert_eq!(reported.len(), 1, "{label}: {reported:?}");
        assert!(
            reported[0]
                .message()
                .contains("'Names' is already a sequence"),
            "{label}: {reported:?}"
        );
    }

    // `Rows` is the alias that wrote the `[]`, and it is on a different line in each source, so
    // comparing the two spans directly would prove nothing. What has to hold is that each report
    // lands on its own source's `Rows`, not on the `Alias2` that merely reached it first.
    let line_of = |result: &TypeCheckResult, source: &str| {
        let span = result.errors()[0].labels()[0].range.start();
        source[..usize::from(span)].matches('\n').count() + 1
    };
    assert_eq!(
        line_of(
            &in_order,
            "type Names = string[]\ntype Rows = Names[]\ntype Alias2 = Rows\nlet v = 1"
        ),
        2
    );
    assert_eq!(
        line_of(
            &reordered,
            "type Names = string[]\ntype Alias2 = Rows\ntype Rows = Names[]\nlet v = 1"
        ),
        3
    );
}

#[test]
fn a_reported_alias_span_is_always_one_this_module_wrote() {
    // A target's problem is moved onto the alias that wrote it, which is only sound for an alias
    // this module declares: `type_aliases` holds imported ones too, under the name this module
    // reaches them by but with the span the *declaring* module wrote them at, and a span from
    // another file cannot underline anything in this one. Every span reported here has to fall
    // inside this source.
    let source = "type Names = string[]\n\
                  type Alias2 = Rows\n\
                  type Rows = Names[]\n\
                  let v = 1";
    let result = check(source);
    let reported = result.errors();
    assert_eq!(reported.len(), 1, "{reported:?}");
    for diagnostic in reported {
        for label in diagnostic.labels() {
            assert!(
                usize::from(label.range.end()) <= source.len(),
                "span outside the source: {:?} in {} bytes",
                label.range,
                source.len()
            );
        }
    }
}

#[test]
fn a_self_referential_alias_reports_only_its_cycle() {
    // Cycle recovery answers `Type::Error` for the inner name. Wrapping that in a sequence would
    // make the outer `[]` believe the name denoted one, and the author would be told that `A` is
    // "already a sequence" on top of being told it is not a type at all.
    for source in ["type A = A[]\nlet v = 1", "type A = A?[]\nlet v = 1"] {
        let reported = errors(source);
        assert_eq!(reported.len(), 1, "{source:?}: {reported:?}");
        assert!(reported[0].contains("forms a cycle"), "{reported:?}");
    }
}

fn type_of(source: &str, name: &str) -> Type {
    check(source)
        .type_env
        .lookup(&Name::new(name))
        .cloned()
        .unwrap_or_else(|| panic!("binding {name}"))
}

#[test]
fn a_missing_else_joins_exactly_as_an_empty_arm_would() {
    // The missing `else` is read as `else { }`, so writing the empty arm out changes nothing. That
    // is what the join lifting an item to a sequence buys: before it, the written form joined
    // `A` with `never[]` to `object` while the missing form was nullable, so the two disagreed.
    let decls = "type A = { n:int = 1 }\nlet c = true\n";
    let missing = type_of(&format!("{decls}let v = {{ if c {{ <A/> }} }}"), "v");
    let written = type_of(
        &format!("{decls}let w = {{ if c {{ <A/> }} else {{ }} }}"),
        "w",
    );
    assert_eq!(missing, written);
    assert!(
        matches!(&missing, Type::Array(inner) if matches!(inner.as_ref(), Type::Named(n) if n.name.as_str() == "A")),
        "got: {missing}"
    );

    // And at a declared site, an empty arm no longer reports `found object[]`.
    assert_clean(
        "type A = { n:int = 1 }\n\
         type Box = { content items:A[] }\n\
         let c = false\n\
         let root() = { <Box><A/>{if c { <A n=2 /> } else { }}</Box> }",
    );
}

#[test]
fn how_many_items_sit_beside_a_conditional_makes_no_difference() {
    // At arity one the braces used to be grouping, so the conditional was a nullable value and
    // failed at a list site, while at arity two it collected. Now the conditional is a sequence
    // on its own account, so the two agree without either being a special case.
    assert_clean("let c = true\nlet xs:int[] = { if c { 1 } }");
    assert_clean("let c = true\nlet ys:int[] = { if c { 1 } 2 }");
    assert_clean("type Box = { items:int[] }\nlet c = true\nlet b = <Box items={if c { 1 }} />");
}

#[test]
fn a_branch_that_is_already_a_sequence_is_not_wrapped_again() {
    let ty = type_of(
        "let c = true\nlet xs:string[] = {\"a\"}\nlet v = { if c { xs } }",
        "v",
    );
    assert_eq!(ty, Type::array(Type::string()), "got: {ty}");
}

#[test]
fn a_nullable_branch_gives_a_sequence_of_nullables_not_a_nullable_sequence() {
    // `if c { maybe }` yields zero or one items, each of which may be null. That is `int?[]`: the
    // conditional itself is never null, it is the item that may be.
    let ty = type_of(
        "let c = true\nlet maybe:int? = null\nlet v = { if c { maybe } }",
        "v",
    );
    assert_eq!(ty, Type::array(Type::nullable(Type::int())), "got: {ty}");
}

#[test]
fn lifting_never_builds_a_sequence_of_sequences() {
    // The lifting arm applies only when exactly one side is sequence-shaped, looking through `?`.
    // A nullable sequence beside a sequence is two sequence-shaped sides, so it joins as a
    // nullable sequence rather than being lifted into `string[]?[]`.
    let ty = type_of(
        "let c = true\nlet a:string[]? = null\nlet b:string[] = {\"x\"}\nlet v = { if c { a } else { b } }",
        "v",
    );
    assert_eq!(ty, Type::nullable(Type::array(Type::string())), "got: {ty}");
}

#[test]
fn lifting_beside_a_nullable_sequence_keeps_its_question_mark() {
    // A nullable sequence may still be absent after the other side is lifted beside it, so the
    // join is `string[]?`. Dropping the `?` let a null value through to a `string[]` site, where
    // every engine then failed differently.
    let ty = type_of(
        "let c = true\nlet maybe:string[]? = null\nlet v = { if c { maybe } else { \"x\" } }",
        "v",
    );
    assert_eq!(ty, Type::nullable(Type::array(Type::string())), "got: {ty}");

    let errors = errors(
        "let c = true\nlet maybe:string[]? = null\nlet v:string[] = { if c { maybe } else { \"x\" } }",
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("string[]?"), "{errors:?}");
}

#[test]
fn a_written_null_beside_a_sequence_is_a_nullable_sequence_not_a_null_item() {
    // The lifting arm leaves the untyped null literal to the nullable arm, so `else { null }` --
    // the spelling of a nullable value -- works beside a sequence the way it works beside a scalar.
    for (source, name) in [
        (
            "let c = true\nlet xs:string[] = {\"a\"}\nlet v = { if c { xs } else { null } }",
            "v",
        ),
        (
            "let c = true\nlet xs:string[] = {\"a\"}\nlet v = { if c { null } else { xs } }",
            "v",
        ),
    ] {
        let ty = type_of(source, name);
        assert_eq!(
            ty,
            Type::nullable(Type::array(Type::string())),
            "{source:?} got: {ty}"
        );
    }
    assert_clean(
        "let c = true\nlet xs:string[] = {\"a\"}\nlet v:string[]? = { if c { xs } else { null } }",
    );
    assert_clean(
        "let s = \"b\"\nlet xs:string[] = {\"a\"}\nlet v:string[]? = { if s is { \"a\" => xs else => null } }",
    );

    // A branch *typed* nullable is a real item, and still lifts.
    let ty = type_of(
        "let c = true\nlet maybe:int? = null\nlet v = { if c { maybe } }",
        "v",
    );
    assert_eq!(ty, Type::array(Type::nullable(Type::int())), "got: {ty}");
}

#[test]
fn a_nullable_sequence_item_contributes_a_nullable_item_not_itself() {
    // Present, a nullable sequence splices its items; absent, it is one null item -- which is what
    // every engine does. So it contributes `T?`. Contributing itself built `string[]?[]`, a
    // sequence of sequences that no type can spell, so no annotation could accept the value.
    let decls = "let c = false\nlet maybeXs:string[]? = null\nlet xs:string[] = {\"a\" \"b\"}\n";
    for (expr, name) in [
        ("{ maybeXs \"c\" }", "v"),
        ("{ \"z\" if c { xs } else { null } }", "v"),
    ] {
        let ty = type_of(&format!("{decls}let v = {expr}"), name);
        assert_eq!(
            ty,
            Type::array(Type::nullable(Type::string())),
            "{expr} got: {ty}"
        );
        assert_clean(&format!("{decls}let v:string?[] = {expr}"));
    }

    // A `for` body yielding a nullable sequence is the same rule.
    let ty = type_of(
        &format!("{decls}let ns:int[] = {{1 2}}\nlet v = {{ for n in ns {{ maybeXs }} }}"),
        "v",
    );
    assert_eq!(ty, Type::array(Type::nullable(Type::string())), "got: {ty}");

    // A sequence of nullables already has nullable items, which stay singly nullable.
    let ty = type_of("let maybe:string?[]? = null\nlet v = { maybe \"c\" }", "v");
    assert_eq!(ty, Type::array(Type::nullable(Type::string())), "got: {ty}");
}
