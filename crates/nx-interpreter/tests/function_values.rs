//! Evaluation of function values: a function name as a value, and a function-typed binding
//! invoked as an element with arguments bound by name under the subset rule.
//!
//! One case per `function-values` scenario the interpreter owns.

use nx_interpreter::{Interpreter, ResolvedProgram, Value};
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::sync::Arc;

const FILE: &str = "function-values.nx";

struct Runtime {
    module: Arc<nx_hir::LoweredModule>,
    interpreter: Interpreter,
}

impl Runtime {
    fn new(source: &str) -> Self {
        let result = nx_types::check_str(source, FILE);
        assert!(
            result.errors().is_empty(),
            "Expected source to pass analysis, got {:?}",
            result
                .errors()
                .iter()
                .map(|diagnostic| diagnostic.message().to_string())
                .collect::<Vec<_>>()
        );
        let module = result.lowered_module.expect("lowered module");
        let interpreter = Interpreter::from_resolved_program(ResolvedProgram::single_root_module(
            source.len() as u64,
            FILE,
            module.clone(),
        ));
        Self {
            module,
            interpreter,
        }
    }

    fn call(&self, name: &str) -> Value {
        self.interpreter
            .execute_function(self.module.as_ref(), name, Vec::new())
            .unwrap_or_else(|error| panic!("Expected `{name}` to evaluate, got {error}"))
    }
}

fn function(name: &str) -> Value {
    Value::Function {
        module: SmolStr::new(FILE),
        name: SmolStr::new(name),
    }
}

fn record(type_name: &str, fields: &[(&str, Value)]) -> Value {
    Value::Record {
        type_name: nx_hir::Name::new(type_name),
        fields: fields
            .iter()
            .map(|(name, value)| (SmolStr::new(*name), value.clone()))
            .collect::<FxHashMap<_, _>>(),
    }
}

#[test]
fn a_function_bound_to_a_function_typed_property_is_the_function_value() {
    let runtime = Runtime::new(
        "type Contact = { name:string }\n\
         external component <List TItem:type ItemsSource:TItem[]? ItemTemplate:(<function Item:TItem Index:int />: string)? />\n\
         let <Row Item:Contact Index:int />: string = {Item.name}\n\
         let contacts:Contact[] = {}\n\
         let root() = <List TItem=Contact ItemsSource={contacts} ItemTemplate={Row} />",
    );
    let Value::Record { type_name, fields } = runtime.call("root") else {
        panic!("Expected a List record");
    };
    assert_eq!(type_name.as_str(), "List");
    assert_eq!(fields.get("ItemTemplate"), Some(&function("Row")));
}

#[test]
fn function_values_are_equal_when_they_name_the_same_declaration() {
    assert_eq!(function("Row"), function("Row"));
    assert_ne!(function("Row"), function("Compact"));
}

#[test]
fn a_function_typed_parameter_is_invoked_as_an_element() {
    let runtime = Runtime::new(
        "type Contact = { name:string }\n\
         let <Section Item:Contact Row:<function Item:Contact Index:int />: string />: string = <Row Item={Item} Index=0 />\n\
         let <ContactRow Item:Contact Index:int />: string = {Item.name}\n\
         let root() = <Section Item=<Contact name=\"a\" /> Row={ContactRow} />",
    );
    assert_eq!(runtime.call("root"), Value::String(SmolStr::new("a")));
}

#[test]
fn a_function_typed_parameter_of_a_paren_function_is_invoked_as_an_element() {
    let runtime = Runtime::new(
        "let invoke(f: <function n:int />: int, n:int): int = <f n={n} />\n\
         let double(n:int): int = {n * 2}\n\
         let root() = {invoke(double, 4)}",
    );
    assert_eq!(runtime.call("root"), Value::Int(8));
}

#[test]
fn the_function_ignores_a_parameter_the_type_supplied() {
    let runtime = Runtime::new(
        "type Contact = { name:string }\n\
         let <Section Item:Contact Row:<function Item:Contact Index:int />: string />: string = <Row Item={Item} Index=3 />\n\
         let <Compact Item:Contact />: string = {Item.name}\n\
         let root() = <Section Item=<Contact name=\"a\" /> Row={Compact} />",
    );
    assert_eq!(runtime.call("root"), Value::String(SmolStr::new("a")));
}

#[test]
fn a_function_typed_component_prop_is_invoked_as_an_element() {
    let runtime = Runtime::new(
        "abstract external component <DrawnNode />\n\
         external component <SkiaLabel extends DrawnNode Text:string? />\n\
         type Contact = { name:string }\n\
         component <Section extends DrawnNode Item:Contact Row:<function Item:Contact Index:int />: DrawnNode /> = { <Row Item={Item} Index=0 /> }\n\
         let <ContactRow Item:Contact Index:int />: DrawnNode = <SkiaLabel Text={Item.name} />",
    );
    let props = record(
        "Section",
        &[
            (
                "Item",
                record("Contact", &[("name", Value::String(SmolStr::new("a")))]),
            ),
            ("Row", function("ContactRow")),
        ],
    );
    let result = runtime
        .interpreter
        .initialize_component(runtime.module.as_ref(), "Section", props)
        .expect("Expected initialization to succeed");
    assert_eq!(
        result.rendered,
        record("SkiaLabel", &[("Text", Value::String(SmolStr::new("a")))])
    );
}

#[test]
fn a_declared_parameter_without_an_argument_is_an_error() {
    // Analysis rejects binding a function that needs `Index` to a type without it, so the value
    // arrives from the host: a component initialized with a function record of the wider shape.
    let runtime = Runtime::new(
        "type Contact = { name:string }\n\
         component <Section Item:Contact Row:<function Item:Contact />: string /> = { <Row Item={Item} /> }\n\
         let <Wide Item:Contact Index:int />: string = {Item.name}",
    );
    let props = record(
        "Section",
        &[
            (
                "Item",
                record("Contact", &[("name", Value::String(SmolStr::new("a")))]),
            ),
            ("Row", function("Wide")),
        ],
    );
    let error = runtime
        .interpreter
        .initialize_component(runtime.module.as_ref(), "Section", props)
        .expect_err("Expected the call to fail on the missing parameter")
        .to_string();
    assert!(error.contains("'Index'"), "{error}");
    assert!(error.contains("missing"), "{error}");
}

#[test]
fn a_lexical_binding_shadows_a_function_of_the_same_name() {
    let runtime = Runtime::new(
        "let <Row Item:object />: string = \"x\"\n\
         let pick(Row:string): string = {Row}\n\
         let root() = {pick(\"shadowed\")}",
    );
    assert_eq!(
        runtime.call("root"),
        Value::String(SmolStr::new("shadowed"))
    );
}

#[test]
fn an_undefined_name_is_still_undefined() {
    let result = nx_types::check_str("let root() = {NoSuchFunction}", FILE);
    let module = result.lowered_module.expect("lowered module");
    let interpreter = Interpreter::from_resolved_program(ResolvedProgram::single_root_module(
        1,
        FILE,
        module.clone(),
    ));
    let error = interpreter
        .execute_function(module.as_ref(), "root", Vec::new())
        .expect_err("Expected an undefined name to fail");
    assert!(error.to_string().contains("NoSuchFunction"), "{error}");
}

#[test]
fn two_function_values_are_equal_exactly_when_they_name_the_same_declaration() {
    let runtime = Runtime::new(
        "let <A Item:object />: string = \"a\"\n\
         let <B Item:object />: string = \"b\"\n\
         let f: <function Item:object />: string = {A}\n\
         let g: <function Item:object />: string = {A}\n\
         let h: <function Item:object />: string = {B}\n\
         let same() = {f == g}\n\
         let other() = {f == h}\n\
         let differs() = {f != h}",
    );
    assert_eq!(runtime.call("same"), Value::Boolean(true));
    assert_eq!(runtime.call("other"), Value::Boolean(false));
    assert_eq!(runtime.call("differs"), Value::Boolean(true));
}
