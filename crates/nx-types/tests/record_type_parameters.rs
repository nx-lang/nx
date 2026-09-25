//! Type-checking behavior for record type parameters.
//!
//! One case per scenario in the `record-type-parameters` capability that the checker answers for:
//! resolving an applied type, rejecting a malformed one, the identity and invariance of an
//! instantiation, constructing a generic record, and the derived companions.

use nx_types::{analyze_str, check_str};

fn errors(source: &str) -> Vec<String> {
    check_str(source, "test.nx")
        .errors()
        .iter()
        .map(|diagnostic| diagnostic.message().to_string())
        .collect()
}

fn assert_clean(source: &str) {
    let errors = errors(source);
    assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
}

fn assert_reports(source: &str, needle: &str) -> Vec<String> {
    let errors = errors(source);
    assert!(
        errors.iter().any(|message| message.contains(needle)),
        "expected an error containing {needle:?}, got: {errors:?}"
    );
    errors
}

const RANGE: &str = "type Range = { T:type start:T end:T }\n";
const BOX: &str = "type Box = { T:type value:T }\n";
const PAIR: &str = "type Pair = { TKey:type TValue:type key:TKey value:TValue }\n";
const PAGE: &str = "type Page = { T:type items:T+ }\n";

// ---------------------------------------------------------------------------------------------
// An applied type names one instantiation
// ---------------------------------------------------------------------------------------------

#[test]
fn an_applied_type_annotates_a_binding() {
    assert_clean(&format!(
        "{RANGE}let r:<Range T=int/> = <Range T=int start={{1}} end={{5}} />"
    ));
}

#[test]
fn suffixes_compose_after_an_applied_type() {
    assert_clean(&format!(
        "{RANGE}type Schedule = {{ slots:<Range T=int/>+ override?:<Range T=int/> }}\n\
         let s = <Schedule slots={{ <Range T=int start={{1}} end={{5}} /> }} />\n\
         let first:<Range T=int/>+ = {{s.slots}}\n\
         let over:<Range T=int/>? = {{s.override}}"
    ));
}

#[test]
fn an_alias_names_an_instantiation() {
    assert_clean(&format!(
        "{RANGE}type IntRange = <Range T=int/>\n\
         let r:IntRange = <Range T=int start={{1}} end={{5}} />"
    ));
}

#[test]
fn applied_types_nest_and_reject_an_optional_argument() {
    assert_clean(&format!(
        "{BOX}type IntBox = <Box T=int/>\n\
         let a:<Box T=<Box T=int/>/> = <Box T=IntBox value={{<Box T=int value={{1}} />}} />"
    ));
    // A type argument is exactly one value: optionality belongs on the field that holds it.
    let errors = errors(&format!(
        "{BOX}type MaybeInt = int?\nlet b:<Box T=int?/> = <Box T=MaybeInt value={{}} />"
    ));
    let rejected = errors
        .iter()
        .filter(|message| message.contains("A type argument must be exactly one value"))
        .count();
    assert_eq!(
        rejected, 2,
        "the annotation and the argument, once each: {errors:?}"
    );
    assert!(
        errors
            .iter()
            .any(|message| message.contains("'MaybeInt' carries an occurrence")),
        "the alias is named where it was written: {errors:?}"
    );
    let errors = assert_reports(
        "type OptBox = { T:type value?:T }\nlet c = <OptBox T=int />\nlet bad:string = {c.value}",
        "expects string, found int?",
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
}

#[test]
fn a_sequence_type_is_not_a_type_argument() {
    // Substituting a sequence for `T` would make a `T+` field a sequence of sequences.
    for source in [
        format!("{BOX}let a:<Box T=int+/> = <Box T=int value={{ 1 }} />"),
        format!("{BOX}type Ints = int+\nlet a:<Box T=Ints/> = <Box T=int value={{ 1 }} />"),
        format!("{BOX}type Ints = int+\nlet a = <Box T=Ints value={{ 1 }} />"),
        format!("{PAGE}type Ints = int+\nlet p:<Page T=Ints/> = <Page T=int items={{ 1 }} />"),
    ] {
        let errors = assert_reports(&source, "A type argument must be exactly one value");
        assert!(
            errors
                .iter()
                .all(|message| !message.contains("found object")),
            "{errors:?}"
        );
    }

    // The alias is named where one was written.
    assert_reports(
        &format!("{BOX}type Ints = int+\nlet a = <Box T=Ints value={{ 1 }} />"),
        "'Ints' carries an occurrence",
    );
}

#[test]
fn a_suffixed_type_argument_at_a_construction_site_is_rejected_once() {
    // A construction site writes a type argument as a bare name, and a suffix glued to it parses
    // so the checker can say what is wrong rather than leaving a syntax error.
    for suffixed in ["int?", "int+", "int*"] {
        let errors = errors(&format!(
            "type OptBox = {{ T:type value?:T }}\nlet b = <OptBox T={suffixed} value={{ 1 }} />"
        ));
        assert_eq!(
            errors,
            vec![format!(
                "A type argument must be exactly one value; '{suffixed}' carries an occurrence"
            )],
            "{suffixed}"
        );
    }

    // A rejected argument still binds its parameter, so it is not also reported as unspecified.
    let errors = errors(&format!(
        "{BOX}type MaybeInt = int?\nlet b = <Box T=MaybeInt value={{ 1 }} />"
    ));
    assert_eq!(errors.len(), 1, "{errors:?}");
}

#[test]
fn a_suffix_on_a_value_is_rejected() {
    let errors = assert_reports(
        "type Fit = fill | cover\ntype Img = { fit:Fit }\nlet i = <Img fit=cover? />",
        "'cover?' puts an occurrence suffix on a value",
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
}

#[test]
fn a_type_parameter_in_scope_is_a_type_argument() {
    assert_clean(&format!(
        "{RANGE}component <Slider TValue:type range:<Range T=TValue/> /> = {{ <Label /> }}\n\
         type Span = {{ T:type bounds:<Range T=T/> }}"
    ));
}

#[test]
fn a_generic_record_refers_to_itself() {
    assert_clean("type Node = { T:type value:T next?:<Node T=T/> }");
}

#[test]
fn arguments_are_matched_by_name() {
    assert_clean(&format!(
        "{PAIR}let p:<Pair TValue=int TKey=string/> = <Pair TKey=string TValue=int key=\"a\" value={{1}} />"
    ));
}

// ---------------------------------------------------------------------------------------------
// A malformed applied type is rejected by name
// ---------------------------------------------------------------------------------------------

#[test]
fn a_bare_generic_record_name_is_not_a_type() {
    assert_reports(
        &format!("{RANGE}type Slider = {{ range:Range }}"),
        "Type parameter 'T' of record 'Range' was not specified; write <Range T=.../>",
    );
}

#[test]
fn a_bare_generic_record_name_is_a_diagnostic_in_every_type_position() {
    for position in [
        "type Slider = { range:Range }",
        "type Slider = { range:Range+ }",
        "type Slider = { range?:Range }",
        "let r:Range = <Range T=int start={1} end={5} />",
        "let f(r:Range): int = 1",
        "let g(x:int): Range = <Range T=int start={1} end={5} />",
        // An alias is resolved where it is used, so the diagnostic lands at the use.
        "type Alias = Range\nlet r:Alias = <Range T=int start={1} end={5} />",
        "type Status = | only { r:Range }",
        "external component <Slider range:Range />",
        "type Holder = { f:(<function r:Range />: int) }",
    ] {
        let errors = errors(&format!("{RANGE}{position}"));
        assert!(
            errors
                .iter()
                .any(|message| message.contains("of record 'Range' was not specified")),
            "expected a missing-argument diagnostic for {position:?}, got: {errors:?}"
        );
    }
}

#[test]
fn a_diagnostic_about_a_type_reference_lands_on_the_declaration_that_wrote_it() {
    // A `TypeRef` carries no span, so each of these positions has to hand the converter the span
    // of the construct that wrote the reference. Without that the label falls back to offset 0 and
    // underlines the first character of the file, which is what this locks down: the diagnostic
    // has to land inside the declaration under test, and there has to be exactly one.
    for position in [
        "type Slider = { range:<Range U=int/> }",
        "let r:<Range U=int/> = {}",
        "let f(r:<Range U=int/>): int = {1}",
        "let g(x:int): <Range U=int/> = {}",
        "type Status = | only { r:<Range U=int/> }",
        "external component <Slider range:<Range U=int/> />",
        "component <Slider /> = { state { s:<Range U=int/> = {} } <text value=\"x\" /> }",
    ] {
        let source = format!("{RANGE}{position}");
        let declaration = RANGE.len();
        let diagnostics = check_str(&source, "test.nx");
        let all = diagnostics.errors();
        let reported: Vec<_> = all
            .iter()
            .filter(|diagnostic| diagnostic.message().contains("record 'Range'"))
            .collect();
        assert_eq!(
            reported.len(),
            1,
            "expected the unknown argument, once, for {position:?}, got: {:?}",
            reported
                .iter()
                .map(|diagnostic| diagnostic.message())
                .collect::<Vec<_>>()
        );
        for diagnostic in reported {
            let label = diagnostic
                .labels()
                .iter()
                .find(|label| label.primary)
                .unwrap_or_else(|| panic!("no primary label for {position:?}"));
            let start: usize = label.range.start().into();
            let end: usize = label.range.end().into();
            assert!(
                start >= declaration && end > start && end <= source.len(),
                "expected {position:?} to be underlined inside its own declaration \
                 ({declaration}..{}), got {start}..{end} for {:?}",
                source.len(),
                diagnostic.message()
            );
        }
    }
}

#[test]
fn a_declarations_type_reference_is_reported_once_however_often_a_use_site_resolves_it() {
    // A use site resolves the declaration's own field and prop types again — to build a
    // construction's binding spec, to read a field, to check an inherited prop. The pass that owns
    // the declaration is the one that reports, and it reports with a span; a use site resolving
    // the same reference again would print a second copy of each message at whatever span happened
    // to be current, which at the top of a use-site check is offset 0.
    for (position, occurrences) in [
        (
            "type S = { r:<Range U=int/> }\nlet s = <S r={1} />\nlet x = {s.r}",
            1,
        ),
        (
            "external component <W range:<Range U=int/> />\nlet root() = <W range={1} />",
            1,
        ),
        (
            "abstract external component <Base range:<Range U=int/> />\n\
             external component <Derived extends Base extra:int />\n\
             let root() = <Derived range={1} extra={2} />",
            1,
        ),
        (
            "type Case = | only { r:<Range U=int/> }\nlet c = <Case.only r={1} />",
            1,
        ),
    ] {
        let source = format!("{RANGE}{position}");
        let messages = errors(&source);
        for (needle, occurrences) in [
            ("'U' is not a type parameter of record 'Range'", occurrences),
            // The misspelled argument is the missing one, so `T` is not reported on top of it.
            ("Type parameter 'T' of record 'Range' was not specified", 0),
        ] {
            let count = messages
                .iter()
                .filter(|message| message.contains(needle))
                .count();
            assert_eq!(
                count, occurrences,
                "expected {needle:?} exactly {occurrences} time(s) for {position:?}, \
                 got: {messages:?}"
            );
        }
    }
}

#[test]
fn a_missing_argument_is_rejected() {
    assert_reports(
        &format!("{PAIR}type Bad = {{ p:<Pair TKey=string/> }}"),
        "Type parameter 'TValue' of record 'Pair' was not specified",
    );
}

#[test]
fn an_unknown_or_repeated_argument_is_rejected() {
    assert_reports(
        &format!("{RANGE}type A = {{ r:<Range U=int/> }}"),
        "'U' is not a type parameter of record 'Range'",
    );
    assert_reports(
        &format!("{RANGE}type B = {{ r:<Range T=int T=string/> }}"),
        "Type parameter 'T' of record 'Range' is already bound",
    );
}

#[test]
fn applying_a_record_with_no_type_parameters_is_rejected() {
    assert_reports(
        "type Contact = { name:string }\ntype Bad = { c:<Contact T=int/> }",
        "Record 'Contact' has no type parameters",
    );
}

#[test]
fn an_unresolved_type_argument_is_rejected() {
    assert_reports(
        &format!("{RANGE}type Bad = {{ r:<Range T=Contatc/> }}"),
        "Contatc",
    );
}

#[test]
fn a_builtin_type_name_is_a_type_argument() {
    // `Element` is a type a declaration can write but no declaration declares, so it reaches the
    // argument resolver through the builtin list rather than through the record, union or
    // component tables. A component use site resolves its arguments the same way, so both are
    // covered here.
    assert_clean(&format!(
        "{BOX}component <Holder TItem:type item:TItem /> = {{ <Box T=int value={{1}} /> }}\n\
         let b:<Box T=Element/> = <Box T=Element value={{<Holder TItem=int item={{1}} />}} />\n\
         let h = <Holder TItem=Element item={{<Box T=int value={{1}} />}} />"
    ));
}

// ---------------------------------------------------------------------------------------------
// The declaration's own fields
// ---------------------------------------------------------------------------------------------

#[test]
fn a_parameter_typed_field_rejects_a_concrete_default() {
    assert_reports(
        "type Bad = { T:type value:T = \"text\" }",
        "expects T, found string",
    );
}

#[test]
fn a_type_parameter_composes_with_suffixes_and_function_types() {
    assert_clean("type Page = { T:type items:T+ next?:T render?:<function item:T />: string }");
}

#[test]
fn a_type_parameter_is_not_a_value_or_a_field() {
    assert_reports(
        &format!("{BOX}let b = <Box T=int value={{1}} />\nlet x = {{b.T}}"),
        "has no field 'T'",
    );
}

// ---------------------------------------------------------------------------------------------
// Applied types are distinct per argument and invariant
// ---------------------------------------------------------------------------------------------

#[test]
fn different_arguments_are_different_types() {
    assert_reports(
        &format!(
            "{RANGE}let ints:<Range T=int/> = <Range T=int start={{1}} end={{5}} />\n\
             let bad:<Range T=string/> = {{ints}}"
        ),
        "expects <Range T=string/>, found <Range T=int/>",
    );
}

#[test]
fn arguments_do_not_convert() {
    assert_reports(
        &format!(
            "{RANGE}let ints:<Range T=int/> = <Range T=int start={{1}} end={{5}} />\n\
             let bad:<Range T=float64/> = {{ints}}"
        ),
        "expects <Range T=float64/>, found <Range T=int/>",
    );
}

#[test]
fn an_alias_argument_is_the_same_type_as_its_target() {
    assert_clean(&format!(
        "{RANGE}type Count = int\nlet a:<Range T=Count/> = <Range T=int start={{1}} end={{5}} />"
    ));
}

#[test]
fn field_access_substitutes_the_argument() {
    let source = "type Page = { T:type items:T+ next?:T }\n\
         let p:<Page T=string/> = <Page T=string items={ \"a\" } />\n\
         let first:string+ = {p.items}\n\
         let bad:int? = {p.next}";
    let errors = assert_reports(source, "expects int?, found string?");
    assert_eq!(errors.len(), 1, "only `bad` should fail: {errors:?}");
}

#[test]
fn an_applied_type_satisfies_object() {
    assert_clean(&format!(
        "{RANGE}let o:object = <Range T=int start={{1}} end={{5}} />"
    ));
}

/// A list of two of one instantiation is a list of that instantiation, arguments and all.
#[test]
fn equal_instantiations_join_to_themselves() {
    assert_reports(
        &format!(
            "{BOX}let a = <Box T=int value={{1}} />\n\
             let b = <Box T=int value={{2}} />\n\
             let bad:int = {{ a b }}"
        ),
        "found <Box T=int/>+",
    );
}

/// Invariance decides the join as it decides assignment: two instantiations that differ are
/// unrelated, so the only type above both is `object`. Naming the declaration alone would invent a
/// bare `Box`, which is not a type NX accepts and whose fields still mention the record's own `T`.
#[test]
fn differing_instantiations_join_to_object() {
    assert_reports(
        &format!(
            "{BOX}let a = <Box T=int value={{1}} />\n\
             let b = <Box T=string value=\"x\" />\n\
             let bad:int = {{ a b }}"
        ),
        "found object+",
    );
    assert_clean(&format!(
        "{BOX}let a = <Box T=int value={{1}} />\n\
         let b = <Box T=string value=\"x\" />\n\
         let mixed:object+ = {{ a b }}"
    ));
}

/// An instantiation and an unrelated record have nothing below `object` either, and the generic
/// record's own parameter never escapes into the join.
#[test]
fn an_instantiation_joins_with_an_unrelated_record_at_object() {
    assert_reports(
        &format!(
            "{BOX}type Contact = {{ name:string }}\n\
             let a = <Box T=int value={{1}} />\n\
             let c = <Contact name=\"a\" />\n\
             let bad:int = {{ a c }}"
        ),
        "found object+",
    );
}

// ---------------------------------------------------------------------------------------------
// Constructing a generic record binds every type argument
// ---------------------------------------------------------------------------------------------

#[test]
fn construction_produces_the_applied_type() {
    assert_clean(
        "type Range = { T:type start:T end:T endInclusive:boolean = false }\n\
         let r = <Range T=int start={1} end={5} />\n\
         let typed:<Range T=int/> = {r}",
    );
}

#[test]
fn a_field_binding_is_checked_against_the_substituted_type() {
    assert_reports(
        &format!("{RANGE}let bad = <Range T=int start=\"a\" end={{5}} />"),
        "expects int, found string",
    );
}

#[test]
fn a_literal_converts_to_the_argument_type() {
    assert_clean(&format!(
        "{RANGE}let r:<Range T=float64/> = <Range T=float64 start={{0}} end={{1}} />"
    ));
}

#[test]
fn an_unbound_type_parameter_is_rejected() {
    let errors = assert_reports(
        &format!("{RANGE}let bad = <Range start={{1}} end={{5}} />"),
        "Type parameter 'T' of record 'Range' was not specified; write T=<type>",
    );
    assert!(
        !errors.iter().any(|message| message.contains("never")),
        "the field bindings the parameter types are skipped: {errors:?}"
    );
}

#[test]
fn a_braced_quoted_or_conditional_argument_is_rejected() {
    for source in [
        format!("{BOX}let a = <Box T={{int}} value={{1}} />"),
        format!("{BOX}let b = <Box T=\"int\" value={{1}} />"),
        format!("{BOX}let flag = true\nlet c = <Box if flag {{ T=int }} value={{1}} />"),
    ] {
        let errors = errors(&source);
        assert!(
            errors.iter().any(|message| message.contains("T=")),
            "expected the bare form to be suggested for {source:?}, got: {errors:?}"
        );
    }
}

#[test]
fn a_type_argument_is_removed_from_the_construction() {
    let artifact = analyze_str(
        &format!("{RANGE}let r = <Range T=int start={{1}} end={{5}} />"),
        "test.nx",
    );
    assert!(artifact.is_ok(), "got: {:?}", artifact.errors());
    let module = artifact
        .lowered_module
        .as_ref()
        .expect("the module should have been lowered");

    let names = module
        .exprs()
        .find_map(|(_, expr)| match expr {
            nx_hir::ast::Expr::RecordLiteral { properties, .. } => Some(
                properties
                    .iter()
                    .map(|property| property.name.as_str().to_string())
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .expect("the Range construction");
    assert_eq!(names, vec!["start", "end"]);
}

// ---------------------------------------------------------------------------------------------
// A generic record's derived companions follow its type parameters
// ---------------------------------------------------------------------------------------------

#[test]
fn the_update_companion_is_applied_like_its_record() {
    assert_clean(&format!(
        "{RANGE}let u:<Range.Update T=int/> = <Range.Update T=int end={{9}} />"
    ));
}

#[test]
fn intrinsics_carry_the_argument() {
    assert_clean(&format!(
        "{RANGE}let r = <Range T=int start={{1}} end={{5}} />\n\
         let moved:<Range T=int/> = {{apply(r, <Range.Update T=int end={{9}} />)}}\n\
         let d:<Range.Update T=int/> = {{diff(r, moved)}}"
    ));
    assert_reports(
        &format!(
            "{RANGE}let r = <Range T=int start={{1}} end={{5}} />\n\
             let bad = {{apply(r, <Range.Update T=string end=\"z\" />)}}"
        ),
        "is not '<Range.Update T=int/>'",
    );
    // Two instantiations of one record are two types, so they cannot be diffed against each
    // other: the result would carry one operand's arguments over the other's field values.
    assert_reports(
        &format!(
            "{RANGE}let a = <Range T=int start={{1}} end={{5}} />\n\
             let b = <Range T=string start=\"x\" end=\"y\" />\n\
             let bad = {{diff(a, b)}}"
        ),
        "'<Range T=string/>' is not '<Range T=int/>'",
    );
}

#[test]
fn the_property_union_ignores_type_parameters() {
    assert_clean(&format!(
        "{RANGE}let k:Range.Property = {{Range.Property.start}}"
    ));
    assert_reports(
        &format!("{RANGE}let k:Range.Property = {{Range.Property.T}}"),
        "T",
    );
}

// ---------------------------------------------------------------------------------------------
// Type parameters remain rejected in inheritance
// ---------------------------------------------------------------------------------------------

#[test]
fn a_generic_record_cannot_take_part_in_inheritance() {
    assert_reports(
        "abstract type Base = { T:type value:T }",
        "Record 'Base' declares type parameters and cannot be abstract",
    );
    assert_reports(
        "abstract type Shape = { name:string }\ntype Tagged extends Shape = { T:type tag:T }",
        "Record 'Tagged' declares type parameters and cannot extend a base",
    );
}
