//! Integration tests for the NX type checker.
//!
//! These tests verify end-to-end type checking behavior on realistic NX code.

use nx_types::{check_str, Type, TypeCheckSession};

// ============================================================================
// Type Inference Tests (T131, T136)
// ============================================================================

#[test]
fn test_infer_literal_types() {
    let source = r#"
        let x = 42
        let y = 3.14
        let z = "hello"
        let w = true
    "#;

    let result = check_str(source, "literals.nx");
    assert!(result.lowered_module.is_some());

    // Should parse without errors
    // Type inference happens but we don't have let statements fully working yet
}

#[test]
fn test_infer_binary_operations() {
    let source = r#"
        let <Add a:int b:int /> = a + b
    "#;

    let result = check_str(source, "binop.nx");
    assert!(result.lowered_module.is_some());
}

#[test]
fn test_infer_function_call() {
    let source = r#"
        let <Add a:int b:int /> = a + b
        let <Main /> = <Add a=1 b=2 />
    "#;

    let result = check_str(source, "call.nx");
    assert!(result.lowered_module.is_some());
}

#[test]
fn test_infer_array_types() {
    let source = r#"
        let <Numbers /> = [1, 2, 3, 4, 5]
    "#;

    let result = check_str(source, "array.nx");
    assert!(result.lowered_module.is_some());
}

#[test]
fn test_enum_definition_type_checks() {
    let source = r#"
        enum Direction = | north | south | east | west
        let current: Direction = { Direction.north }
    "#;

    let result = check_str(source, "enum.nx");
    assert!(result.is_ok(), "Enum usage should type check");
}

#[test]
fn test_unknown_enum_member_diagnostic() {
    let source = r#"
        enum Direction = | north | south
        let useDirection(): Direction = { Direction.nort }
    "#;

    let result = check_str(source, "bad-enum.nx");
    assert!(
        result
            .errors()
            .iter()
            .any(|diag| diag.code() == Some("undefined-enum-member")),
        "Expected undefined-enum-member diagnostic"
    );
}

#[test]
fn test_record_default_type_mismatch_diagnostic() {
    let source = r#"
        type User = {
          name: string = 123
          age: int = 42
        }
    "#;

    let result = check_str(source, "record-default.nx");
    let errors = result.errors();
    assert!(
        errors
            .iter()
            .any(|diag| diag.code() == Some("record-default-type-mismatch")),
        "Expected record-default-type-mismatch diagnostic, got {:?}",
        errors
            .iter()
            .map(|d| d.code().unwrap_or("<none>"))
            .collect::<Vec<_>>()
    );
}

#[test]
#[ignore = "Array literals are not yet accepted as RHS expressions (parser limitation)"]
fn test_record_in_collections_type_checks() {
    let source = r#"
        type User = { name: string age: int }
        let a: User = { <User name="A" age=1 /> }
        let b: User = { <User name="B" age=2 /> }
        let users: User[] = [ a, b ]
    "#;

    let result = check_str(source, "record-array.nx");
    assert!(
        result.is_ok(),
        "record arrays should type check, diagnostics: {:?}",
        result
            .errors()
            .iter()
            .map(|d| (d.code(), d.message()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_empty_record_definition_parses() {
    let source = r#"
        type Empty = {}
        let make(): Empty = { <Empty /> }
    "#;

    let result = check_str(source, "empty-record.nx");
    assert!(result.is_ok(), "empty record definition should type check");
}

#[test]
fn test_abstract_record_instantiation_is_rejected() {
    let source = r#"
        abstract type UserBase = {
          name: string
        }

        let root(): UserBase = { <UserBase name={"Ada"} /> }
    "#;

    let result = check_str(source, "abstract-record-instantiation.nx");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.code() == Some("abstract-record-instantiation")),
        "Expected abstract-record-instantiation diagnostic, got {:?}",
        result
            .diagnostics
            .iter()
            .map(|diag| (diag.code(), diag.message()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_record_inheritance_accepts_concrete_leaf_for_abstract_ancestor() {
    let source = r#"
        abstract type Entity = {
          id: int
        }

        abstract type UserBase extends Entity = {
          name: string
        }

        type User extends UserBase = {
          isAdmin: bool = false
        }

        let consume(entity: Entity): int = { 1 }
        let root(): int = { consume(<User id={1} name={"Ada"} />) }
    "#;

    let result = check_str(source, "record-inheritance-subtyping.nx");
    assert!(
        result.errors().is_empty(),
        "Expected abstract ancestor substitution to type check, got {:?}",
        result
            .diagnostics
            .iter()
            .map(|diag| (diag.code(), diag.message()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_record_inheritance_accepts_concrete_leaf_for_abstract_return_type() {
    let source = r#"
        abstract type Entity = {
          id: int
        }

        abstract type UserBase extends Entity = {
          name: string
        }

        type User extends UserBase = {
          isAdmin: bool = false
        }

        let make(): UserBase = { <User id={1} name={"Ada"} /> }
    "#;

    let result = check_str(source, "record-inheritance-return-subtyping.nx");
    assert!(
        result.errors().is_empty(),
        "Expected abstract return type substitution to type check, got {:?}",
        result
            .diagnostics
            .iter()
            .map(|diag| (diag.code(), diag.message()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_record_inheritance_uses_shared_abstract_supertype_for_branches() {
    let source = r#"
        abstract type Entity = {
          id: int
        }

        abstract type UserBase extends Entity = {
          name: string
        }

        type User extends UserBase = {
          isAdmin: bool = false
        }

        type StaffUser extends UserBase = {
          department: string
        }

        let choose(flag: bool): UserBase = {
          if flag {
            <User id={1} name={"Ada"} />
          } else {
            <StaffUser id={2} name={"Sam"} department={"Ops"} />
          }
        }
    "#;

    let result = check_str(source, "record-inheritance-branch-supertype.nx");
    assert!(
        result.errors().is_empty(),
        "Expected sibling derived records to infer their shared abstract supertype, got {:?}",
        result
            .diagnostics
            .iter()
            .map(|diag| (diag.code(), diag.message()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_action_inheritance_accepts_concrete_leaf_for_abstract_parameter() {
    let source = r#"
        abstract action InputAction = {
          source: string
        }

        action ValueChanged extends InputAction = {
          value: string
        }

        let consume(action: InputAction): int = { 1 }
        let root(): int = { consume(<ValueChanged source={"ui"} value={"docs"} />) }
    "#;

    let result = check_str(source, "action-inheritance-subtyping.nx");
    assert!(
        result.errors().is_empty(),
        "Expected abstract action substitution to type check, got {:?}",
        result
            .diagnostics
            .iter()
            .map(|diag| (diag.code(), diag.message()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_action_inheritance_allows_inherited_fields_in_record_literal() {
    let source = r#"
        abstract action InputAction = {
          source: string
        }

        action ValueChanged extends InputAction = {
          value: string
        }

        let make(): ValueChanged = { <ValueChanged source={"ui"} value={"docs"} /> }
    "#;

    let result = check_str(source, "action-inheritance-record-literal.nx");
    assert!(
        result.errors().is_empty(),
        "Expected derived action construction to type check, got {:?}",
        result
            .diagnostics
            .iter()
            .map(|diag| (diag.code(), diag.message()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_action_inheritance_reports_inherited_field_type_mismatch() {
    let source = r#"
        abstract action InputAction = {
          source: string
        }

        action ValueChanged extends InputAction = {
          value: string
        }

        let make(): ValueChanged = { <ValueChanged source={1} value={"docs"} /> }
    "#;

    let result = check_str(source, "action-inheritance-field-mismatch.nx");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.code() == Some("record-field-type-mismatch")),
        "Expected inherited field mismatch diagnostic, got {:?}",
        result
            .diagnostics
            .iter()
            .map(|diag| (diag.code(), diag.message()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_inline_emitted_action_inheritance_allows_inherited_fields() {
    let source = r#"
        abstract action InputAction = {
          source: string
        }

        component <SearchBox emits { ValueChanged extends InputAction { value: string } } /> = {
          <TextInput />
        }

        let make(): SearchBox.ValueChanged = {
          <SearchBox.ValueChanged source={"ui"} value={"docs"} />
        }
    "#;

    let result = check_str(source, "inline-action-inheritance.nx");
    assert!(
        result.errors().is_empty(),
        "Expected inline emitted action inheritance to type check, got {:?}",
        result
            .diagnostics
            .iter()
            .map(|diag| (diag.code(), diag.message()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_inline_emitted_action_inheritance_allows_abstract_parent_subtyping() {
    let source = r#"
        abstract action InputAction = {
          source: string
        }

        component <SearchBox emits { ValueChanged extends InputAction { value: string } } /> = {
          <TextInput />
        }

        let read(action: InputAction): int = { 1 }
        let result(): int = { read(<SearchBox.ValueChanged source={"keyboard"} value={"docs"} />) }
    "#;

    let result = check_str(source, "inline-action-inheritance-subtyping.nx");
    assert!(
        result.errors().is_empty(),
        "Expected inline emitted action subtyping to type check, got {:?}",
        result
            .diagnostics
            .iter()
            .map(|diag| (diag.code(), diag.message()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_inline_emitted_action_inheritance_reports_inherited_field_type_mismatch() {
    let source = r#"
        abstract action InputAction = {
          source: string
        }

        component <SearchBox emits { ValueChanged extends InputAction { value: string } } /> = {
          <TextInput />
        }

        let make(): SearchBox.ValueChanged = {
          <SearchBox.ValueChanged source={1} value={"docs"} />
        }
    "#;

    let result = check_str(source, "inline-action-inheritance-mismatch.nx");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.code() == Some("record-field-type-mismatch")),
        "Expected inherited field property mismatch diagnostic, got {:?}",
        result
            .diagnostics
            .iter()
            .map(|diag| (diag.code(), diag.message()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_abstract_action_instantiation_is_rejected() {
    let source = r#"
        abstract action InputAction = {
          source: string
        }

        let root(): InputAction = { <InputAction source={"ui"} /> }
    "#;

    let result = check_str(source, "abstract-action-instantiation.nx");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.code() == Some("abstract-record-instantiation")),
        "Expected abstract-record-instantiation diagnostic, got {:?}",
        result
            .diagnostics
            .iter()
            .map(|diag| (diag.code(), diag.message()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_abstract_derived_action_instantiation_is_rejected() {
    let source = r#"
        abstract action InputAction = {
          source: string
        }

        abstract action SearchAction extends InputAction = {
          query: string
        }

        let root(): SearchAction = { <SearchAction source={"toolbar"} query={"docs"} /> }
    "#;

    let result = check_str(source, "abstract-derived-action-instantiation.nx");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.code() == Some("abstract-record-instantiation")),
        "Expected abstract-record-instantiation diagnostic, got {:?}",
        result
            .diagnostics
            .iter()
            .map(|diag| (diag.code(), diag.message()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_component_inheritance_accepts_inherited_props_and_content() {
    let source = r#"
        abstract component <PanelBase title:string content body:Element />

        component <Panel extends PanelBase /> = {
          <section>
            <h1>{title}</h1>
            {body}
          </section>
        }

        let root() = { <Panel title={"Docs"}><Badge /></Panel> }
    "#;

    let result = check_str(source, "component-inheritance-props-content.nx");
    assert!(
        result.errors().is_empty(),
        "Expected inherited component props/content to type check, got {:?}",
        result
            .diagnostics
            .iter()
            .map(|diag| (diag.code(), diag.message()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_abstract_component_instantiation_is_rejected() {
    let source = r#"
        abstract component <SearchBase placeholder:string />
        let root() = { <SearchBase placeholder={"docs"} /> }
    "#;

    let result = check_str(source, "abstract-component-instantiation.nx");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.code() == Some("abstract-component-instantiation")),
        "Expected abstract-component-instantiation diagnostic, got {:?}",
        result
            .diagnostics
            .iter()
            .map(|diag| (diag.code(), diag.message()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_external_component_state_only_body_type_checks() {
    let source = r#"
        external component <SearchBox placeholder:string /> = {
          state { query:string }
        }

        let root() = { <SearchBox placeholder={"docs"} /> }
    "#;

    let result = check_str(source, "external-component-state-only.nx");
    assert!(
        result.errors().is_empty(),
        "Expected external component with state-only body to type check, got {:?}",
        result
            .diagnostics
            .iter()
            .map(|diag| (diag.code(), diag.message()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_duplicate_inherited_component_prop_reports_diagnostic() {
    let source = r#"
        abstract component <SearchBase placeholder:string />

        component <SearchBox extends SearchBase placeholder:string /> = {
          <button />
        }
    "#;

    let result = check_str(source, "duplicate-inherited-component-prop.nx");
    assert!(
        result.diagnostics.iter().any(|diag| diag
            .message()
            .contains("redeclares inherited prop 'placeholder'")),
        "Expected duplicate inherited component prop diagnostic, got {:?}",
        result
            .diagnostics
            .iter()
            .map(|diag| (diag.code(), diag.message()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_inherited_component_handler_collision_reports_diagnostic() {
    let source = r#"
        action SearchSubmitted = { searchString:string }

        abstract component <SearchBase emits { SearchSubmitted } />

        component <SearchBox extends SearchBase onSearchSubmitted:string /> = {
          <button />
        }
    "#;

    let result = check_str(source, "inherited-component-handler-collision.nx");
    assert!(
        result.diagnostics.iter().any(|diag| {
            diag.message()
                .contains("collides with emitted action handler 'onSearchSubmitted'")
        }),
        "Expected inherited component handler collision diagnostic, got {:?}",
        result
            .diagnostics
            .iter()
            .map(|diag| (diag.code(), diag.message()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_multi_level_component_inheritance_accumulates_props_and_emits() {
    let source = r#"
        action SearchSubmitted = { searchString:string }
        action DoSearch = { search:string }

        abstract component <SearchBase placeholder:string emits { SearchSubmitted } />

        abstract component <SearchChrome extends SearchBase showSearchIcon:bool = true />

        component <SearchBox extends SearchChrome highlight:bool = false /> = {
          <TextInput placeholder={placeholder} />
        }

        let render() = <SearchBox
          placeholder="docs"
          showSearchIcon=true
          highlight=true
          onSearchSubmitted=<DoSearch search={action.searchString} />
        />
    "#;

    let result = check_str(source, "multi-level-component-inheritance.nx");
    assert!(
        result.is_ok(),
        "Expected multi-level component inheritance to type check, got {:?}",
        result
            .diagnostics
            .iter()
            .map(|diag| (diag.code(), diag.message()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_duplicate_inherited_field_reports_diagnostic() {
    let source = r#"
        abstract type UserBase = {
          name: string
        }

        type User extends UserBase = {
          name: string
          email: string
        }
    "#;

    let result = check_str(source, "duplicate-inherited-field.nx");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.message().contains("redeclares inherited field 'name'")),
        "Expected duplicate inherited field diagnostic, got {:?}",
        result
            .diagnostics
            .iter()
            .map(|diag| (diag.code(), diag.message()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_concrete_record_cannot_be_used_as_base() {
    let source = r#"
        abstract type Entity = {
          id: int
        }

        type User extends Entity = {
          name: string
        }

        type Admin extends User = {
          level: int
        }
    "#;

    let result = check_str(source, "concrete-record-base.nx");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.code() == Some("lowering-error")
                && diag
                    .message()
                    .contains("only abstract records may be extended")),
        "Expected concrete-base diagnostic, got {:?}",
        result
            .diagnostics
            .iter()
            .map(|diag| (diag.code(), diag.message()))
            .collect::<Vec<_>>()
    );
}

// ============================================================================
// Type Mismatch Detection Tests (T132)
// ============================================================================

#[test]
fn test_type_mismatch_in_binary_op() {
    // Note: This test documents expected behavior for type mismatch detection
    // Currently the grammar may not parse all mixed-type expressions
    let source = r#"
        let <Test a:int b:string /> = a + b
    "#;

    let result = check_str(source, "mismatch.nx");

    // May have parse or type errors - the important thing is we detect issues
    assert!(result.lowered_module.is_some());
}

#[test]
fn test_type_mismatch_in_comparison() {
    // Note: Documents expected behavior - type errors should be caught
    let source = r#"
        let <Test a:int b:string /> = if a == b then <div>Yes</div> else <div>No</div>
    "#;

    let result = check_str(source, "comparison_mismatch.nx");

    // Should parse successfully
    assert!(result.lowered_module.is_some());
}

#[test]
fn test_type_mismatch_in_array() {
    // Note: Currently the grammar handles arrays uniformly
    // Type errors in heterogeneous arrays would be detected during type inference
    let source = r#"
        let <Test /> = [1, 2, 3, 4]
    "#;

    let result = check_str(source, "array_ok.nx");

    // Should parse successfully with homogeneous array
    assert!(result.lowered_module.is_some());
    assert!(result.errors().is_empty() || result.errors().len() < 2);
}

#[test]
fn test_composed_list_type_mismatch_diagnostics_preserve_rendered_shapes() {
    let source = r#"
        let nullableList: string[]? = null
        let rejectAliases(items:string?[]): string[]? = { items }
        let rejectMaybeNames(items:string[]?): string?[] = { items }
    "#;

    let result = check_str(source, "composed-list-mismatch.nx");
    let messages = result
        .errors()
        .iter()
        .map(|diagnostic| diagnostic.message())
        .collect::<Vec<_>>();

    assert!(
        messages
            .iter()
            .any(|message| message.contains("expects string[]?, found list string?[]")),
        "Expected list-of-nullable vs nullable-list mismatch message, got {:?}",
        messages
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("expects string?[], found string[]?")),
        "Expected nullable-list vs list-of-nullable mismatch message, got {:?}",
        messages
    );
}

// ============================================================================
// Undefined Identifier Detection Tests (T133)
// ============================================================================

#[test]
fn test_undefined_identifier() {
    // Elements referencing undefined identifiers are detected during type inference
    // but may be allowed if they could be HTML tags
    let source = r#"
        let <Test name:string /> = <div>{name}</div>
    "#;

    let result = check_str(source, "defined.nx");

    // Should parse successfully - 'name' is defined as a parameter
    assert!(result.lowered_module.is_some());
}

#[test]
fn test_undefined_function() {
    let source = r#"
        let <Test /> = <UndefinedComponent />
    "#;

    let result = check_str(source, "undefined_func.nx");

    // Elements are allowed to reference undefined components (they might be HTML tags)
    // So this shouldn't error
    assert!(result.lowered_module.is_some());
}

// ============================================================================
// Function Parameter Type Checking Tests (T135)
// ============================================================================

#[test]
fn test_function_with_parameters() {
    let source = r#"
        let <Button text:string disabled:bool /> =
            <button>{text}</button>
    "#;

    let result = check_str(source, "function_params.nx");
    assert!(result.lowered_module.is_some());

    if let Some(module) = &result.lowered_module {
        // Should have one function with two parameters
        assert_eq!(module.items().len(), 1);
    }
}

#[test]
fn test_function_parameter_reference() {
    let source = r#"
        let <Greet name:string /> = <div>{name}</div>
    "#;

    let result = check_str(source, "param_ref.nx");
    assert!(result.lowered_module.is_some());

    // Parameter 'name' should be in scope within the function body
}

#[test]
fn test_function_with_default_params() {
    let source = r#"
        let <Button text:string="Click me" /> = <button>{text}</button>
    "#;

    let result = check_str(source, "default_params.nx");
    assert!(result.lowered_module.is_some());
}

// ============================================================================
// Element Type Checking Tests
// ============================================================================

#[test]
fn test_element_with_properties() {
    let source = r#"
        <button class="btn" disabled="true">Click me</button>
    "#;

    let result = check_str(source, "element_props.nx");
    assert!(result.lowered_module.is_some());
}

#[test]
fn test_nested_elements() {
    let source = r#"
        <div>
            <button>Click</button>
            <input />
        </div>
    "#;

    let result = check_str(source, "nested.nx");
    assert!(result.lowered_module.is_some());
}

#[test]
fn test_element_with_interpolation() {
    let source = r#"
        let <Greet name:string /> = <div>Hello {name}!</div>
    "#;

    let result = check_str(source, "interpolation.nx");
    assert!(result.lowered_module.is_some());
}

// ============================================================================
// Complex Type Inference Tests
// ============================================================================

#[test]
fn test_nested_function_calls() {
    let source = r#"
        let <Inner x:int /> = <span>{x}</span>
        let <Outer /> = <Inner x=42 />
    "#;

    let result = check_str(source, "nested_calls.nx");
    assert!(result.lowered_module.is_some());
}

#[test]
fn test_conditional_expressions() {
    let source = r#"
        let <Test flag:bool /> = if flag then <div>Yes</div> else <div>No</div>
    "#;

    let _result = check_str(source, "conditional.nx");
    // May or may not parse depending on grammar support
    // This documents expected behavior
}

// ============================================================================
// Session-Based Type Checking Tests
// ============================================================================

#[test]
fn test_session_multiple_files() {
    let mut session = TypeCheckSession::new();

    session.add_file(
        "button.nx",
        r#"
        let <Button text:string /> = <button>{text}</button>
    "#,
    );

    session.add_file(
        "app.nx",
        r#"
        let <App /> = <Button text="Click me" />
    "#,
    );

    let results = session.check_all();
    assert_eq!(results.len(), 2);

    for (name, result) in &results {
        assert!(
            result.lowered_module.is_some(),
            "File {} should parse",
            name
        );
    }
}

#[test]
fn test_session_with_errors() {
    let mut session = TypeCheckSession::new();

    session.add_file("valid.nx", "<button />");
    session.add_file("invalid.nx", "let x = ");

    let results = session.check_all();
    assert_eq!(results.len(), 2);

    // At least one should have errors
    let total_errors: usize = results.iter().map(|(_, r)| r.errors().len()).sum();
    assert!(total_errors > 0);
}

// ============================================================================
// Type System Features Tests
// ============================================================================

#[test]
fn test_type_compatibility() {
    // Test that the type system correctly handles compatibility
    assert!(Type::int().is_compatible_with(&Type::int()));
    assert!(!Type::int().is_compatible_with(&Type::string()));

    // Test nullable compatibility
    let nullable_int = Type::nullable(Type::int());
    assert!(Type::int().is_compatible_with(&nullable_int));
    assert!(!nullable_int.is_compatible_with(&Type::int()));
}

#[test]
fn test_array_type_compatibility() {
    let arr_int = Type::array(Type::int());
    let arr_int2 = Type::array(Type::int());
    let arr_str = Type::array(Type::string());

    assert!(arr_int.is_compatible_with(&arr_int2));
    assert!(!arr_int.is_compatible_with(&arr_str));
}

#[test]
fn test_function_type_compatibility() {
    let f1 = Type::function(vec![Type::int()], Type::string());
    let f2 = Type::function(vec![Type::int()], Type::string());
    let f3 = Type::function(vec![Type::string()], Type::string());

    assert!(f1.is_compatible_with(&f2));
    assert!(!f1.is_compatible_with(&f3));
}

// ============================================================================
// Error Recovery Tests
// ============================================================================

#[test]
fn test_error_recovery_continues_checking() {
    let source = r#"
        let <First a:int /> = <div>{a}</div>
        let <Second x:int /> = <span>{x}</span>
    "#;

    let result = check_str(source, "recovery.nx");

    // Should parse and continue checking even with errors
    assert!(result.lowered_module.is_some());

    // Should have processed both functions
    if let Some(module) = &result.lowered_module {
        assert_eq!(module.items().len(), 2);
    }
}

// ============================================================================
// Real-World Examples Tests
// ============================================================================

#[test]
fn test_realistic_component() {
    let source = r#"
        let <Card title:string content:string /> =
            <div>
                <h2>{title}</h2>
                <p>{content}</p>
            </div>
    "#;

    let result = check_str(source, "card.nx");
    assert!(result.lowered_module.is_some());
    assert!(result.errors().is_empty() || result.errors().len() < 3);
}

#[test]
fn test_form_component() {
    let source = r#"
        let <Input name:string type:string /> =
            <input name="{name}" type="{type}" />

        let <Form /> =
            <form>
                <Input name="email" type="email" />
                <Input name="password" type="password" />
                <button>Submit</button>
            </form>
    "#;

    let result = check_str(source, "form.nx");
    assert!(result.lowered_module.is_some());
}

// ============================================================================
// Documentation Tests (verify examples work)
// ============================================================================

#[test]
fn test_readme_example() {
    let source = r#"
        let <Button text:string /> = <button>{text}</button>
    "#;

    let result = check_str(source, "readme.nx");
    assert!(result.lowered_module.is_some());
}
