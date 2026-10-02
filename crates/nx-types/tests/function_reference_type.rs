//! Type-checking behavior for the function reference type, `<function ... />: R`.
//!
//! One case per scenario in the `function-reference-type` capability.

use nx_types::check_str;

fn diagnostics(source: &str) -> Vec<(String, String)> {
    check_str(source, "test.nx")
        .errors()
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.code().unwrap_or_default().to_string(),
                diagnostic.message().to_string(),
            )
        })
        .collect()
}

fn errors(source: &str) -> Vec<String> {
    diagnostics(source)
        .into_iter()
        .map(|(_, message)| message)
        .collect()
}

fn assert_clean(source: &str) {
    let errors = errors(source);
    assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
}

/// The one error `source` reports, which must contain every needle.
fn assert_one_error(source: &str, needles: &[&str]) -> String {
    let errors = errors(source);
    assert_eq!(errors.len(), 1, "expected one error, got: {errors:?}");
    for needle in needles {
        assert!(
            errors[0].contains(needle),
            "expected {needle:?} in {:?}",
            errors[0]
        );
    }
    errors[0].clone()
}

const TOOL: &str = "type Tool = { fn: <function ... />: object* }\n";
const ANY_FN: &str = "type AnyFn = <function ... />: object*\n";
const DOUBLE: &str = "let double(n:int): int = {n * 2}\n";
const GREET: &str = "let greet(name:string): string = {name}\n";
const ARGS: &str = "type Args = { q:string }\ntype BuildTool = { build: <function ... />: Args }\n";

// ---------------------------------------------------------------------------------------------
// A function type may leave its parameters unspecified
// ---------------------------------------------------------------------------------------------

#[test]
fn a_field_is_declared_at_a_function_reference_type() {
    assert_clean(&format!(
        "{ARGS}let make(q:string, limit?:int): Args = <Args q={{q}} />\n\
         let t = <BuildTool build={{make}} />"
    ));
}

#[test]
fn a_function_reference_type_is_aliased() {
    assert_clean(
        "type Args = { q:string }\ntype Builder = <function ... />: Args\n\
         type A = { build:Builder }\ntype B = { build: <function ... />: Args }\n\
         let make(q:string): Args = <Args q={q} />\n\
         let a = <A build={make} />\nlet b = <B build={a.build} />\nlet c = <A build={b.build} />",
    );
}

#[test]
fn a_suffix_after_the_result_binds_to_the_result() {
    assert_clean(
        "type Maybe = <function ... />: string?\ntype Many = (<function ... />: string)+\n\
         let none(): string? = {}\nlet one(): string = \"a\"\n\
         let a:Maybe = {none}\nlet b:Many = {one one}",
    );
    // `Maybe` is exactly one function, so it takes no empty value.
    assert_one_error(
        "type Maybe = <function ... />: string?\nlet a:Maybe = {}",
        &["<function ... />: string?"],
    );
    // Each function of `Many` returns exactly one string.
    assert_one_error(
        "type Many = (<function ... />: string)+\nlet none(): string? = {}\nlet b:Many = {none}",
        &["the result string? is not string"],
    );
}

#[test]
fn function_is_not_a_type_name() {
    let diagnostics = diagnostics("type Tool = { fn:Function }");
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].0, "unresolved-type");
    assert!(diagnostics[0].1.contains("Function"), "{diagnostics:?}");
}

// ---------------------------------------------------------------------------------------------
// A function of any parameters satisfies a function reference type by its result
// ---------------------------------------------------------------------------------------------

#[test]
fn functions_of_unlike_signatures_satisfy_the_widest_type() {
    assert_clean(&format!(
        "{TOOL}{DOUBLE}let <Row Item:object Index:int />: string = \"r\"\n\
         let none(): string = \"n\"\nlet many(): string* = {{\"a\" \"b\"}}\nlet maybe(): string? = {{}}\n\
         let a = <Tool fn={{double}} />\nlet b = <Tool fn={{Row}} />\nlet c = <Tool fn={{none}} />\n\
         let d = <Tool fn={{many}} />\nlet e = <Tool fn={{maybe}} />"
    ));
}

#[test]
fn an_exactly_one_result_excludes_a_result_under_an_occurrence() {
    assert_one_error(
        "type Tool = { fn: <function ... />: object }\nlet one(): string = \"n\"\n\
         let many(): string* = {\"a\" \"b\"}\nlet a = <Tool fn={one} />\nlet b = <Tool fn={many} />",
        &[
            "expects <function ... />: object",
            "the result string* is not object",
        ],
    );
}

#[test]
fn the_result_is_checked() {
    assert_one_error(
        &format!(
            "{ARGS}let make(q:string): Args = <Args q={{q}} />\nlet wrong(q:string): string = {{q}}\n\
             let a = <BuildTool build={{make}} />\nlet b = <BuildTool build={{wrong}} />"
        ),
        &[
            "expects <function ... />: Args",
            "the result string is not Args",
        ],
    );
}

#[test]
fn the_result_is_covariant() {
    let shapes = "abstract type Shape = { id:int }\ntype Circle extends Shape = { r:int }\n";
    assert_clean(&format!(
        "{shapes}let make(r:int): Circle = <Circle id=1 r={{r}} />\n\
         let ok: <function ... />: Shape = {{make}}"
    ));
    assert_one_error(
        &format!(
            "{shapes}let wide(): Shape = <Circle id=1 r=1 />\n\
             let bad: <function ... />: Circle = {{wide}}"
        ),
        &["the result Shape is not Circle"],
    );
}

#[test]
fn a_function_typed_binding_satisfies_it() {
    assert_clean(&format!(
        "{TOOL}let wrap(f: <function n:int />: int): Tool = <Tool fn={{f}} />"
    ));
}

#[test]
fn a_non_function_is_rejected() {
    let errors = errors(&format!(
        "{TOOL}type Plan = {{ name:string }}\nlet a = <Tool fn=\"double\" />\n\
         let b = <Tool fn=1 />\nlet c = <Tool fn=<Plan name=\"p\" /> />"
    ));
    assert_eq!(errors.len(), 3, "{errors:?}");
    for error in &errors {
        assert!(
            error.contains("expects <function ... />: object*"),
            "{error}"
        );
    }
    assert!(errors[0].contains("found string"), "{errors:?}");
}

#[test]
fn an_element_named_function_is_not_a_function_value() {
    assert_one_error(
        &format!("{TOOL}let t = <Tool fn=<Function module=\"main.nx\" name=\"double\" /> />"),
        &["expects <function ... />: object*", "found Function"],
    );
}

#[test]
fn an_undefined_name_is_still_undefined() {
    let errors = errors(&format!("{TOOL}let t = <Tool fn={{noSuchFunction}} />"));
    assert!(
        errors.iter().any(|error| error.contains("noSuchFunction")),
        "{errors:?}"
    );
}

#[test]
fn the_function_keeps_its_own_type() {
    assert_clean(&format!(
        "{TOOL}{DOUBLE}let t = <Tool fn={{double}} />\n\
         let typed: <function n:int />: int = {{double}}"
    ));
}

// ---------------------------------------------------------------------------------------------
// A function reference type satisfies only a wider one and `object`
// ---------------------------------------------------------------------------------------------

#[test]
fn a_function_reference_value_is_not_a_callable_function_value() {
    assert_one_error(
        "let narrow(f: <function ... />: object*): <function n:int />: int = {f}",
        &[
            "expects <function n:int />: int",
            "found <function ... />: object*",
            "the value's parameters are not stated",
        ],
    );
    assert_one_error(
        "let none(f: <function ... />: int): <function />: int = {f}",
        &["the value's parameters are not stated"],
    );
}

#[test]
fn a_narrower_result_satisfies_a_wider_one() {
    assert_clean("let widen(f: <function ... />: int): <function ... />: object* = {f}");
    assert_one_error(
        "let narrow(f: <function ... />: object*): <function ... />: int = {f}",
        &["the result object* is not int"],
    );
}

#[test]
fn a_function_reference_value_is_an_object() {
    assert_clean("let widen(f: <function ... />: object*): object = {f}");
}

#[test]
fn a_function_reference_parameter_is_checked_contravariantly() {
    assert_clean(&format!(
        "{ANY_FN}let <Keep Fn:AnyFn />: string = \"k\"\n\
         let ok: <function Fn:<function n:int />: int />: string = {{Keep}}"
    ));
    assert_one_error(
        &format!(
            "{ANY_FN}let <Use Fn:<function n:int />: int />: string = \"u\"\n\
             let bad: <function Fn:AnyFn />: string = {{Use}}"
        ),
        &["parameter 'Fn' is supplied as <function ... />: object*"],
    );
}

// ---------------------------------------------------------------------------------------------
// A function reference value is passed, stored and compared
// ---------------------------------------------------------------------------------------------

#[test]
fn a_function_reference_value_is_forwarded_and_returned() {
    assert_clean(&format!(
        "{ANY_FN}type Tool = {{ fn:AnyFn }}\nlet make(f:AnyFn): Tool = <Tool fn={{f}} />\n\
         {DOUBLE}let root() = {{make(double)}}"
    ));
}

#[test]
fn function_reference_values_compare_by_declaration() {
    assert_clean(&format!(
        "{ANY_FN}{DOUBLE}{GREET}let same(f:AnyFn, g:AnyFn): boolean = {{f == g}}\n\
         let a() = {{same(double, double)}}\nlet b() = {{same(double, greet)}}"
    ));
}

#[test]
fn a_function_reference_value_compares_with_a_function_typed_value() {
    assert_clean(&format!(
        "{DOUBLE}let same(f: <function ... />: object*, g: <function n:int />: int): boolean = {{f == g}}\n\
         let differ(f: <function ... />: object*, g: <function n:int />: int): boolean = {{f != g}}\n\
         let a() = {{same(double, double)}}"
    ));
}

#[test]
fn function_values_compare_whatever_types_they_were_declared_at() {
    // `double` satisfies both parameter types, and neither type satisfies the other.
    assert_clean(&format!(
        "{DOUBLE}let same(f: <function ... />: int, g: <function n:int />: object): boolean = {{f == g}}\n\
         let differ(f: <function ... />: int, g: <function ... />: string): boolean = {{f != g}}\n\
         let stated(f: <function n:int />: int, g: <function m:int />: int): boolean = {{f == g}}\n\
         let a() = {{same(double, double)}}"
    ));
    // A function still does not compare with a value that is not one.
    assert_one_error(
        "let bad(f: <function ... />: int, s:string): boolean = {f == s}",
        &["Cannot compare types"],
    );
}

// ---------------------------------------------------------------------------------------------
// A function reference value is not invocable
// ---------------------------------------------------------------------------------------------

fn assert_not_callable(source: &str, name: &str) {
    let diagnostics = diagnostics(source);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    let (code, message) = &diagnostics[0];
    assert_eq!(code, "function-reference-not-callable", "{message}");
    assert!(
        message.contains(&format!(
            "'{name}' cannot be called because its parameters are not stated"
        )),
        "{message}"
    );
    assert!(
        message.contains("function type with its parameters"),
        "{message}"
    );
}

#[test]
fn an_element_call_on_a_function_reference_binding_is_rejected() {
    assert_not_callable(
        "let invoke(f: <function ... />: int, n:int): int = <f n={n} />",
        "f",
    );
}

#[test]
fn a_paren_call_on_a_function_reference_binding_is_rejected() {
    assert_not_callable(
        "let invoke(f: <function ... />: int, n:int): int = {f(n)}",
        "f",
    );
}

#[test]
fn a_function_reference_prop_is_not_a_tag() {
    assert_not_callable(
        "component <Section Row: <function ... />: object* /> = { <Row Item=\"a\" /> }",
        "Row",
    );
}

#[test]
fn a_function_reference_let_is_not_invocable() {
    assert_not_callable(
        &format!("{DOUBLE}let f: <function ... />: int = {{double}}\nlet root(): int = <f n=1 />"),
        "f",
    );
}

#[test]
fn an_optional_or_sequence_function_reference_binding_is_not_invocable() {
    assert_not_callable("let opt(f?: <function ... />: int): int = <f n=1 />", "f");
    assert_not_callable("let opt(f?: <function ... />: int): int = {f(1)}", "f");
    assert_not_callable(
        "let many(f: (<function ... />: int)+): int = <f n=1 />",
        "f",
    );
}

// ---------------------------------------------------------------------------------------------
// A function reference type takes occurrences and defaults like any exactly-one type
// ---------------------------------------------------------------------------------------------

#[test]
fn occurrences_over_a_function_reference_type() {
    let kit = format!(
        "{ANY_FN}type Kit = {{ all:AnyFn+ extra?:AnyFn+ one?:AnyFn listed: (<function ... />: string)+ }}\n\
         {DOUBLE}{GREET}let k = <Kit all={{double greet}} listed={{greet}} />\n"
    );
    assert_clean(&kit);
    assert_one_error(
        &format!("{kit}let a:int = {{k.all}}"),
        &["found (<function ... />: object*)+"],
    );
    assert_one_error(
        &format!("{kit}let a:int = {{k.extra}}"),
        &["found (<function ... />: object*)*"],
    );
    assert_one_error(
        &format!("{kit}let a:int = {{k.one}}"),
        &["found (<function ... />: object*)?"],
    );
    assert_one_error(
        &format!("{kit}let a:int = {{k.listed}}"),
        &["found (<function ... />: string)+"],
    );
}

#[test]
fn an_empty_value_is_rejected_where_one_is_required() {
    assert_one_error(
        &format!("{TOOL}let t = <Tool fn={{}} />"),
        &["expects <function ... />: object*"],
    );
}

#[test]
fn a_default_names_a_function() {
    assert_clean(
        "let identity(value:string): string = {value}\n\
         type Tool = { fn: <function ... />: object* = {identity} }\nlet t = <Tool />",
    );
}

#[test]
fn a_function_reference_type_is_a_type_argument() {
    assert_clean(&format!(
        "{ANY_FN}type Holder = {{ T:type item:T }}\n{DOUBLE}\
         let h:<Holder T=AnyFn/> = <Holder T=AnyFn item={{double}} />"
    ));
}

// ---------------------------------------------------------------------------------------------
// Two unlike function types join to the widest function reference type
// ---------------------------------------------------------------------------------------------

#[test]
fn a_sequence_of_unlike_functions_is_a_sequence_of_the_widest_type() {
    let both = format!("{DOUBLE}{GREET}let both = {{double greet}}\n");
    assert_clean(&format!(
        "{both}let tools: (<function ... />: object*)+ = {{both}}\nlet anything:object+ = {{both}}"
    ));
    assert_one_error(
        &format!("{both}let a:int = {{both}}"),
        &["found (<function ... />: object*)+"],
    );
}

#[test]
fn a_function_that_satisfies_the_other_joins_to_the_other() {
    assert_one_error(
        "let none(): string = \"n\"\nlet one(n:int): string = \"o\"\nlet both = {none one}\n\
         let a:int = {both}",
        &["found (<function n:int />: string)+"],
    );
}

#[test]
fn a_function_and_a_non_function_still_join_to_object() {
    assert_one_error(
        &format!("{DOUBLE}let mixed = {{double \"x\"}}\nlet a:int = {{mixed}}"),
        &["found object+"],
    );
}

#[test]
fn a_function_reference_type_joins_with_a_function_type() {
    assert_one_error(
        &format!("{DOUBLE}let pair(f: <function ... />: string): int = {{f double}}"),
        &["found (<function ... />: object*)+"],
    );
}

// ---------------------------------------------------------------------------------------------
// A function reference type is displayed in NX spelling
// ---------------------------------------------------------------------------------------------

#[test]
fn a_suggested_property_form_keeps_the_parentheses() {
    assert_one_error(
        "type B = { c:(<function ... />: int)* }",
        &["write `c?:(<function ... />: int)+`"],
    );
    assert_one_error(
        &format!("{ANY_FN}type B = {{ c:AnyFn* }}"),
        &["write `c?:(<function ... />: object*)+`"],
    );
    assert_one_error(
        "type B = { c:(<function n:int />: int)* }",
        &["write `c?:(<function n:int />: int)+`"],
    );
    assert_one_error(
        "type B = { c:(<function ... />: int)? }",
        &["write `c?:<function ... />: int`"],
    );
}

#[test]
fn a_mismatch_diagnostic_shows_the_type() {
    assert_one_error(
        &format!("{TOOL}let t = <Tool fn=\"double\" />"),
        &["expects <function ... />: object*, found string"],
    );
}
