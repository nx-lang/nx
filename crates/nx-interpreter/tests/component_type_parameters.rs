//! A type argument is consumed by the type checker and never reaches evaluation.
//!
//! One case for the `component-type-parameters` scenario about the runtime record: the external
//! component record an element with a type argument evaluates to carries the props and nothing
//! for the parameter. The JSON form is covered in `nx-api`, which owns the value conversion.

use nx_interpreter::{Interpreter, ResolvedProgram, Value};

#[test]
fn the_runtime_record_has_no_type_parameter_field() {
    let source = "type Contact = { name:string }\n\
                  external component <SkiaLayout TItem:type itemsSource?:TItem+ />\n\
                  let root() = { <SkiaLayout TItem=Contact itemsSource={<Contact name=\"Ada\" />} /> }";
    let result = nx_types::check_str(source, "type-parameters.nx");
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
        "type-parameters.nx",
        module.clone(),
    ));

    let value = interpreter
        .execute_function(module.as_ref(), "root", Vec::new())
        .expect("Expected root to evaluate");
    let Value::Record { type_name, fields } = &value else {
        panic!("Expected an external component record, got {value:?}");
    };
    assert_eq!(type_name.as_str(), "SkiaLayout");
    assert!(
        fields.contains_key("itemsSource"),
        "the prop should be a field of the record: {fields:?}"
    );
    assert!(
        !fields.contains_key("TItem"),
        "the type argument must not be a field of the record: {fields:?}"
    );
}
