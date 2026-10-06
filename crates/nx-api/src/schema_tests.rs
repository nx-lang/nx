//! Tests for the JSON Schema export of functions and declared types.

use crate::{
    build_program_artifact_from_source, build_workspace_program_artifact,
    program_artifact_function_schema, program_artifact_type_schema, DeclarationRef, FunctionSchema,
    FunctionSchemaOptions, LibraryRegistry, NxDiagnostic, NxLibraryModule, NxLibrarySource,
    NxWorkspace, NxWorkspaceModule, ProgramArtifact, ProgramBuildContext, SchemaDirection,
    SchemaValue, TypeSchema, TypeSchemaOptions,
};
use serde_json::{json, Value};

const SCHEMA: &str = "https://json-schema.org/draft/2020-12/schema";

fn workspace(files: &[(&str, &str)]) -> NxWorkspace {
    NxWorkspace::new(
        files
            .iter()
            .map(|(identity, source)| {
                NxWorkspaceModule::from_source(*identity, *source).expect("workspace module")
            })
            .collect(),
    )
    .expect("workspace")
}

/// Builds `files` with the first as the entry, against `context`.
fn build_in(files: &[(&str, &str)], context: &ProgramBuildContext) -> ProgramArtifact {
    let artifact = build_workspace_program_artifact(&workspace(files), files[0].0, context)
        .unwrap_or_else(|diagnostics| panic!("build failed: {diagnostics:#?}"));
    artifact
}

fn build(files: &[(&str, &str)]) -> ProgramArtifact {
    build_in(files, &ProgramBuildContext::empty())
}

fn source(text: &str) -> ProgramArtifact {
    build(&[("main.nx", text)])
}

fn function_with(
    artifact: &ProgramArtifact,
    reference: DeclarationRef,
    options: FunctionSchemaOptions,
) -> FunctionSchema {
    program_artifact_function_schema(artifact, &reference, &options)
        .unwrap_or_else(|diagnostics| panic!("function schema failed: {diagnostics:#?}"))
}

fn function(artifact: &ProgramArtifact, name: &str) -> FunctionSchema {
    function_with(
        artifact,
        DeclarationRef::entry(name),
        FunctionSchemaOptions::default(),
    )
}

fn type_in(artifact: &ProgramArtifact, name: &str, direction: SchemaDirection) -> TypeSchema {
    program_artifact_type_schema(
        artifact,
        &DeclarationRef::entry(name),
        &TypeSchemaOptions { direction },
    )
    .unwrap_or_else(|diagnostics| panic!("type schema failed: {diagnostics:#?}"))
}

fn output_type(artifact: &ProgramArtifact, name: &str) -> TypeSchema {
    type_in(artifact, name, SchemaDirection::Output)
}

fn value(schema: &SchemaValue) -> Value {
    serde_json::to_value(schema).expect("schema serializes")
}

fn input(schema: &FunctionSchema) -> Value {
    value(schema.input_schema.as_ref().unwrap_or_else(|| {
        panic!("no input schema: {:#?}", schema.diagnostics);
    }))
}

fn output(schema: &FunctionSchema) -> Value {
    value(schema.output_schema.as_ref().unwrap_or_else(|| {
        panic!("no output schema: {:#?}", schema.diagnostics);
    }))
}

fn document(schema: &TypeSchema) -> Value {
    value(schema.schema.as_ref().unwrap_or_else(|| {
        panic!("no schema: {:#?}", schema.diagnostics);
    }))
}

fn text(schema: &SchemaValue) -> String {
    serde_json::to_string(schema).expect("schema serializes")
}

fn codes(diagnostics: &[NxDiagnostic]) -> Vec<&str> {
    diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic.code.as_deref())
        .collect()
}

fn messages(diagnostics: &[NxDiagnostic]) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

// ---- The answer and the request -------------------------------------------------------------

#[test]
fn an_empty_answer_serializes_with_the_documented_names() {
    let answer = FunctionSchema {
        module: "main.nx".to_string(),
        name: "ping".to_string(),
        description: None,
        summary: None,
        parameters: Vec::new(),
        input_schema: None,
        output_schema: None,
        result_type: "string".to_string(),
        declaration: None,
        diagnostics: Vec::new(),
    };
    assert_eq!(
        serde_json::to_string(&answer).unwrap(),
        r#"{"module":"main.nx","name":"ping","parameters":[],"resultType":"string","diagnostics":[]}"#
    );
    let type_answer = TypeSchema {
        module: "main.nx".to_string(),
        name: "Plan".to_string(),
        description: None,
        summary: None,
        schema: None,
        diagnostics: Vec::new(),
    };
    assert_eq!(
        serde_json::to_string(&type_answer).unwrap(),
        r#"{"module":"main.nx","name":"Plan","diagnostics":[]}"#
    );
}

#[test]
fn requests_read_the_documented_names() {
    let options: FunctionSchemaOptions = serde_json::from_str(
        r#"{"hostSuppliedTypes":[{"module":"@nx/agent/agent.nx","name":"ToolContext"}]}"#,
    )
    .unwrap();
    assert_eq!(
        options.host_supplied_types,
        vec![DeclarationRef::in_module(
            "@nx/agent/agent.nx",
            "ToolContext"
        )]
    );
    let options: TypeSchemaOptions = serde_json::from_str(r#"{"direction":"input"}"#).unwrap();
    assert_eq!(options.direction, SchemaDirection::Input);
    let options: TypeSchemaOptions = serde_json::from_str("{}").unwrap();
    assert_eq!(options.direction, SchemaDirection::Output);
    let reference: DeclarationRef = serde_json::from_str(r#"{"name":"findPlans"}"#).unwrap();
    assert_eq!(reference, DeclarationRef::entry("findPlans"));
}

// ---- Finding the declaration -------------------------------------------------------------------

#[test]
fn a_function_is_found_in_the_entry_module_exported_or_not() {
    let artifact =
        source("let hidden(): int = 1\nexport let shown(): int = 2\nlet root() = <div />");
    assert_eq!(function(&artifact, "hidden").module, "main.nx");
    assert_eq!(function(&artifact, "shown").name, "shown");
}

#[test]
fn a_function_in_another_root_module_is_named_by_its_identity() {
    let artifact = build(&[
        ("main.nx", "import \"./tools.nx\"\nlet root() = 1"),
        ("tools.nx", "let helper(n:int): int = { n }"),
    ]);
    let schema = function_with(
        &artifact,
        DeclarationRef::in_module("tools.nx", "helper"),
        FunctionSchemaOptions::default(),
    );
    assert_eq!(schema.module, "tools.nx");
    assert_eq!(
        input(&schema)["properties"]["n"],
        json!({ "type": "integer" })
    );
}

#[test]
fn a_library_function_is_described_through_the_program_that_links_it() {
    let registry = LibraryRegistry::new();
    registry
        .load_library_from_sources(&NxLibrarySource::new(
            "libraries/catalog",
            vec![NxLibraryModule::new(
                "Plans.nx",
                "export type Plan = { name:string seats:int }\nexport let findPlans(teamSize:int): Plan* = { {} }",
            )],
        ))
        .expect("library loads");
    let artifact = build_in(
        &[(
            "main.nx",
            "import \"libraries/catalog\"\nlet root() = { findPlans(2) }",
        )],
        &registry.build_context(),
    );
    let schema = function_with(
        &artifact,
        DeclarationRef::in_module("libraries/catalog/Plans.nx", "findPlans"),
        FunctionSchemaOptions::default(),
    );
    assert_eq!(schema.module, "libraries/catalog/Plans.nx");
    assert_eq!(
        output(&schema),
        json!({
            "$schema": SCHEMA,
            "type": "array",
            "items": { "$ref": "#/$defs/Plan" },
            "$defs": {
                "Plan": {
                    "type": "object",
                    "properties": {
                        "$type": { "const": "Plan" },
                        "name": { "type": "string" },
                        "seats": { "type": "integer" }
                    },
                    "required": ["$type", "name", "seats"],
                    "additionalProperties": false
                }
            }
        })
    );
}

#[test]
fn a_standard_library_type_is_described_by_its_module_identity() {
    let artifact = source("import \"@nx/agent\"\nlet root() = 1");
    let schema = program_artifact_type_schema(
        &artifact,
        &DeclarationRef::in_module("@nx/agent/agent.nx", "HttpMethod"),
        &TypeSchemaOptions::default(),
    )
    .unwrap();
    assert_eq!(schema.module, "@nx/agent/agent.nx");
    assert_eq!(
        document(&schema)["$defs"]["HttpMethod"]["enum"],
        json!(["get", "post", "put", "patch", "delete"])
    );
}

#[test]
fn a_missing_name_or_module_is_an_unknown_declaration() {
    let artifact = source("let root() = 1");
    let missing = program_artifact_function_schema(
        &artifact,
        &DeclarationRef::entry("missing"),
        &FunctionSchemaOptions::default(),
    )
    .unwrap_err();
    assert_eq!(codes(&missing), vec!["schema-unknown-declaration"]);
    assert!(missing[0].message.contains("missing"), "{missing:#?}");

    let no_module = program_artifact_function_schema(
        &artifact,
        &DeclarationRef::in_module("other.nx", "root"),
        &FunctionSchemaOptions::default(),
    )
    .unwrap_err();
    assert_eq!(codes(&no_module), vec!["schema-unknown-declaration"]);
    assert!(no_module[0].message.contains("other.nx"), "{no_module:#?}");

    let no_type = program_artifact_type_schema(
        &artifact,
        &DeclarationRef::entry("root"),
        &TypeSchemaOptions::default(),
    )
    .unwrap_err();
    assert_eq!(codes(&no_type), vec!["schema-unknown-declaration"]);

    // The artifact is untouched and still emits.
    nx_codegen_free_check(&artifact);
}

/// The artifact still answers queries after a failed one.
fn nx_codegen_free_check(artifact: &ProgramArtifact) {
    let schema = program_artifact_function_schema(
        artifact,
        &DeclarationRef::entry("root"),
        &FunctionSchemaOptions::default(),
    )
    .expect("root is still described");
    assert_eq!(schema.name, "root");
}

// ---- Functions ---------------------------------------------------------------------------------

#[test]
fn a_functions_arguments_and_result_are_described() {
    let artifact = source(
        "type Plan = { name:string seats:int }\nlet findPlans(teamSize:int, maxMonthlyPrice?:int): Plan* = { {} }",
    );
    let schema = function(&artifact, "findPlans");
    assert_eq!(
        text(schema.input_schema.as_ref().unwrap()),
        r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object","properties":{"teamSize":{"type":"integer"},"maxMonthlyPrice":{"type":"integer"}},"required":["teamSize"],"additionalProperties":false}"#
    );
    assert_eq!(output(&schema)["type"], "array");
    assert_eq!(output(&schema)["items"], json!({ "$ref": "#/$defs/Plan" }));
    assert!(output(&schema)["minItems"].is_null());
    let parameters = serde_json::to_value(&schema.parameters).unwrap();
    assert_eq!(
        parameters,
        json!([
            { "name": "teamSize", "type": "int", "required": true },
            { "name": "maxMonthlyPrice", "type": "int", "required": false }
        ])
    );
    assert_eq!(schema.result_type, "Plan*");
    assert!(schema.diagnostics.is_empty());
}

#[test]
fn an_element_style_function_is_described_by_its_parameter_names() {
    let artifact = source(
        "let <Greeting name:string content body:string />: string = { name + body }\nlet root() = 1",
    );
    let schema = function(&artifact, "Greeting");
    assert_eq!(
        input(&schema),
        json!({
            "$schema": SCHEMA,
            "type": "object",
            "properties": { "name": { "type": "string" }, "body": { "type": "string" } },
            "required": ["name", "body"],
            "additionalProperties": false
        })
    );
}

#[test]
fn a_function_with_no_parameters_has_an_input_schema_with_no_properties() {
    let artifact = source("let ping(): string = \"pong\"");
    let schema = function(&artifact, "ping");
    assert_eq!(
        input(&schema),
        json!({
            "$schema": SCHEMA,
            "type": "object",
            "properties": {},
            "additionalProperties": false
        })
    );
    assert_eq!(
        output(&schema),
        json!({ "$schema": SCHEMA, "type": "string" })
    );
}

#[test]
fn an_unannotated_result_uses_the_inferred_type() {
    let artifact = source("let double(n:int) = { n * 2 }");
    let schema = function(&artifact, "double");
    assert_eq!(
        output(&schema),
        json!({ "$schema": SCHEMA, "type": "integer" })
    );
    assert_eq!(schema.result_type, "int");
}

#[test]
fn a_function_over_primitives_has_no_definitions() {
    let artifact = source("let add(a:int, b:int): int = { a + b }");
    let schema = function(&artifact, "add");
    assert!(input(&schema).get("$defs").is_none());
    assert!(output(&schema).get("$defs").is_none());
}

#[test]
fn the_declaration_span_covers_the_function() {
    let tools =
        "type Plan = { name:string }\n\n\nlet findPlans(\n  teamSize:int\n): Plan* = {\n  {}\n}\n";
    let artifact = build(&[
        ("main.nx", "import \"./tools.nx\"\nlet root() = 1"),
        ("tools.nx", tools),
    ]);
    let schema = function_with(
        &artifact,
        DeclarationRef::in_module("tools.nx", "findPlans"),
        FunctionSchemaOptions::default(),
    );
    let span = schema.declaration.expect("a declaration span");
    assert_eq!(span.start_line, 4);
    assert_eq!(span.end_line, 8);
    assert!(tools[span.start_byte as usize..].starts_with("let findPlans"));
}

#[test]
fn the_declaration_span_is_absent_without_the_modules_source() {
    let mut artifact = source("let ping(): string = \"pong\"");
    artifact.source_map.clear();
    assert!(function(&artifact, "ping").declaration.is_none());
}

#[test]
fn a_parameter_entry_names_its_declared_record_or_union() {
    let artifact = build(&[
        (
            "main.nx",
            "import \"./types.nx\"\nlet book(note:string, status:Status, request?:Booking): string = { note }",
        ),
        (
            "types.nx",
            "export type Booking = { host:string }\nexport type Status = active | paused",
        ),
    ]);
    let schema = function(&artifact, "book");
    let parameters = serde_json::to_value(&schema.parameters).unwrap();
    assert!(parameters[0].get("typeRef").is_none());
    assert_eq!(
        parameters[1]["typeRef"],
        json!({ "module": "types.nx", "name": "Status" })
    );
    assert_eq!(
        parameters[2]["typeRef"],
        json!({ "module": "types.nx", "name": "Booking" })
    );
}

#[test]
fn a_single_source_artifact_is_described() {
    let artifact = build_program_artifact_from_source(
        "let add(a:int, b:int): int = { a + b }",
        "input.nx",
        &ProgramBuildContext::empty(),
    )
    .expect("source builds");
    let schema = program_artifact_function_schema(
        &artifact,
        &DeclarationRef::entry("add"),
        &FunctionSchemaOptions::default(),
    )
    .unwrap();
    assert_eq!(schema.module, "input.nx");
}

// ---- Documents ---------------------------------------------------------------------------------

#[test]
fn a_recursive_type_is_described_once() {
    let artifact = source("type Folder = { name:string children?:Folder+ }");
    let schema = output_type(&artifact, "Folder");
    let document = document(&schema);
    assert_eq!(document["$ref"], "#/$defs/Folder");
    assert_eq!(
        document["$defs"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        vec!["Folder"]
    );
    assert_eq!(
        document["$defs"]["Folder"]["properties"]["children"],
        json!({ "type": "array", "items": { "$ref": "#/$defs/Folder" }, "minItems": 1 })
    );
}

#[test]
fn two_declarations_of_one_name_get_distinct_keys() {
    let artifact = build(&[
        (
            "main.nx",
            "import { Card as Left.Card } from \"./left.nx\"\nimport { Card as Right.Card } from \"./right.nx\"\nlet pair(a:Left.Card, b:Right.Card): string = { \"\" }",
        ),
        ("left.nx", "export type Card = { title:string }"),
        ("right.nx", "export type Card = { count:int }"),
    ]);
    let schema = function(&artifact, "pair");
    let input = input(&schema);
    assert_eq!(input["properties"]["a"], json!({ "$ref": "#/$defs/Card" }));
    assert_eq!(
        input["properties"]["b"],
        json!({ "$ref": "#/$defs/Card_2" })
    );
    assert_eq!(
        input["$defs"]["Card"]["properties"],
        json!({ "title": { "type": "string" } })
    );
    assert_eq!(
        input["$defs"]["Card_2"]["properties"],
        json!({ "count": { "type": "integer" } })
    );
}

#[test]
fn definitions_are_listed_in_the_order_the_document_first_mentions_them() {
    let artifact = source(
        "type A = { b:B c:C }\ntype B = { c:C }\ntype C = { n:int }\nlet make(): A = { <A b=<B c=<C n=1 /> /> c=<C n=2 /> /> }",
    );
    let output = function(&artifact, "make").output_schema.unwrap();
    let text = text(&output);
    let a = text.find("\"A\":").unwrap();
    let b = text.find("\"B\":").unwrap();
    let c = text.find("\"C\":").unwrap();
    assert!(a < b && b < c, "{text}");
    assert!(text.starts_with(r#"{"$schema":"#), "{text}");
    assert!(
        text.contains(r#""properties":{"$type":{"const":"A"},"b":"#),
        "{text}"
    );
}

// ---- Primitives and occurrences ---------------------------------------------------------------

#[test]
fn primitives_map_to_json_schema_types() {
    let artifact = source(
        "type Sizes = { s:string f:boolean n:int a:int32 b:int64 c:float32 d:float64 o:object }",
    );
    let document = document(&type_in(&artifact, "Sizes", SchemaDirection::Input));
    assert_eq!(
        document["$defs"]["Sizes"]["properties"],
        json!({
            "s": { "type": "string" },
            "f": { "type": "boolean" },
            "n": { "type": "integer" },
            "a": { "type": "integer", "minimum": -2147483648i64, "maximum": 2147483647 },
            "b": { "type": "integer" },
            "c": { "type": "number" },
            "d": { "type": "number" },
            "o": { "not": { "type": "null" } }
        })
    );
}

#[test]
fn occurrences_map_to_arrays_and_to_absence() {
    let artifact = source(
        "type Box = { items:string+ tags?:string+ notes?:string }\nlet find(id:string, region?:string): Box? = { {} }\nlet all(): Box* = { {} }",
    );
    let document = document(&type_in(&artifact, "Box", SchemaDirection::Input));
    let box_def = &document["$defs"]["Box"];
    let non_empty = json!({ "type": "array", "items": { "type": "string" }, "minItems": 1 });
    assert_eq!(box_def["properties"]["items"], non_empty);
    assert_eq!(box_def["properties"]["tags"], non_empty);
    assert_eq!(box_def["properties"]["notes"], json!({ "type": "string" }));
    assert_eq!(box_def["required"], json!(["items"]));

    let find = function(&artifact, "find");
    assert_eq!(
        input(&find)["properties"]["region"],
        json!({ "type": "string" })
    );
    assert_eq!(input(&find)["required"], json!(["id"]));
    assert_eq!(
        output(&find)["anyOf"],
        json!([{ "$ref": "#/$defs/Box" }, { "type": "null" }])
    );

    let all = function(&artifact, "all");
    assert_eq!(output(&all)["type"], "array");
    assert!(output(&all).get("minItems").is_none());
}

// ---- Records, defaults and directions ---------------------------------------------------------

const BOOKING: &str = "type Booking = { host:string notes?:string durationMinutes:int = 30 }";

#[test]
fn required_optional_and_defaulted_fields_in_input() {
    let artifact = source(BOOKING);
    let document = document(&type_in(&artifact, "Booking", SchemaDirection::Input));
    assert_eq!(
        document["$defs"]["Booking"],
        json!({
            "type": "object",
            "properties": {
                "host": { "type": "string" },
                "notes": { "type": "string" },
                "durationMinutes": { "type": "integer", "default": 30 }
            },
            "required": ["host"],
            "additionalProperties": false
        })
    );
}

#[test]
fn a_defaulted_field_is_always_present_in_output() {
    let artifact = source(BOOKING);
    let document = document(&output_type(&artifact, "Booking"));
    assert_eq!(
        document["$defs"]["Booking"]["required"],
        json!(["$type", "host", "durationMinutes"])
    );
    assert_eq!(
        document["$defs"]["Booking"]["properties"]["$type"],
        json!({ "const": "Booking" })
    );
}

#[test]
fn inherited_fields_come_first() {
    let artifact =
        source("abstract type Entity = { id:int }\ntype User extends Entity = { name:string }");
    let schema = output_type(&artifact, "User");
    let text = text(schema.schema.as_ref().unwrap());
    assert!(
        text.contains(r#""properties":{"$type":{"const":"User"},"id":{"type":"integer"},"name":{"type":"string"}}"#),
        "{text}"
    );
}

#[test]
fn an_action_is_described_as_a_record() {
    let artifact = source("action Saved = { id:int }");
    let document = document(&output_type(&artifact, "Saved"));
    assert_eq!(
        document["$defs"]["Saved"]["required"],
        json!(["$type", "id"])
    );
}

#[test]
fn field_names_are_kept_as_written() {
    let artifact = source("type Box = { aria-label?:string }");
    let document = document(&output_type(&artifact, "Box"));
    assert_eq!(
        document["$defs"]["Box"]["properties"]["aria-label"],
        json!({ "type": "string" })
    );
}

#[test]
fn literal_defaults_are_written_and_computed_ones_are_not() {
    let artifact = source(
        "type Status = active | paused\nlet prefix = \"x\"\ntype Defaults = {\n  s:string = \"hi\"\n  n:int = -3\n  f:float64 = 1.5\n  b:boolean = true\n  status:Status = active\n  other:Status = {Status.paused}\n  label:string = { prefix + \"y\" }\n}",
    );
    let document = document(&type_in(&artifact, "Defaults", SchemaDirection::Input));
    let properties = &document["$defs"]["Defaults"]["properties"];
    assert_eq!(properties["s"]["default"], "hi");
    assert_eq!(properties["n"]["default"], -3);
    assert_eq!(properties["f"]["default"], 1.5);
    assert_eq!(properties["b"]["default"], true);
    assert_eq!(
        properties["status"],
        json!({ "$ref": "#/$defs/Status", "default": "active" })
    );
    assert_eq!(
        properties["other"],
        json!({ "$ref": "#/$defs/Status", "default": "paused" })
    );
    assert_eq!(properties["label"], json!({ "type": "string" }));
    assert!(document["$defs"]["Defaults"].get("required").is_none());
}

#[test]
fn a_parameter_default_is_written_when_it_is_a_literal() {
    let artifact = source("let page(size:int = 20, from:int = { size * 2 }): int = { size }");
    let input = input(&function(&artifact, "page"));
    assert_eq!(
        input["properties"]["size"],
        json!({ "type": "integer", "default": 20 })
    );
    assert_eq!(input["properties"]["from"], json!({ "type": "integer" }));
    assert!(input.get("required").is_none());
}

#[test]
fn an_argument_record_needs_no_discriminator_and_a_result_record_carries_one() {
    let artifact = source(&format!(
        "{BOOKING}\nlet book(request:Booking): string = {{ request.host }}\nlet next(): Booking = {{ <Booking host=\"a\" /> }}"
    ));
    let book = input(&function(&artifact, "book"));
    assert!(book["$defs"]["Booking"]["properties"]
        .get("$type")
        .is_none());
    let next = output(&function(&artifact, "next"));
    assert_eq!(
        next["$defs"]["Booking"]["properties"]["$type"],
        json!({ "const": "Booking" })
    );
    assert_eq!(next["$defs"]["Booking"]["required"][0], "$type");
}

#[test]
fn a_polymorphic_argument_carries_its_discriminator() {
    let artifact = source(
        "abstract type Shape = { label?:string }\ntype Circle extends Shape = { r:float64 }\ntype Square extends Shape = { side:float64 }\nlet describe(shape:Shape): string = { \"\" }",
    );
    let input = input(&function(&artifact, "describe"));
    assert_eq!(
        input["properties"]["shape"],
        json!({ "anyOf": [{ "$ref": "#/$defs/Circle" }, { "$ref": "#/$defs/Square" }] })
    );
    assert_eq!(input["$defs"]["Circle"]["required"], json!(["$type", "r"]));
    assert_eq!(
        input["$defs"]["Square"]["required"],
        json!(["$type", "side"])
    );
}

#[test]
fn a_record_mentioned_both_ways_has_one_definition_with_its_discriminator() {
    let artifact = source(
        "abstract type Shape = { label?:string }\ntype Circle extends Shape = { r:float64 }\nlet both(exact:Circle, any:Shape): string = { \"\" }",
    );
    let input = input(&function(&artifact, "both"));
    assert_eq!(
        input["properties"]["exact"],
        json!({ "$ref": "#/$defs/Circle" })
    );
    assert_eq!(input["$defs"].as_object().unwrap().len(), 1);
    assert_eq!(input["$defs"]["Circle"]["required"], json!(["$type", "r"]));
}

// ---- Unions ------------------------------------------------------------------------------------

#[test]
fn a_constant_union_is_a_string_enum() {
    let artifact = source("type HttpMethod = get | post | put | patch | delete");
    let document = document(&output_type(&artifact, "HttpMethod"));
    assert_eq!(document["$ref"], "#/$defs/HttpMethod");
    assert_eq!(
        document["$defs"]["HttpMethod"],
        json!({ "type": "string", "enum": ["get", "post", "put", "patch", "delete"] })
    );
}

#[test]
fn a_payload_case_carries_a_scoped_discriminator_in_both_directions() {
    let artifact = source("type LoadState = | failed { message:string }");
    for direction in [SchemaDirection::Input, SchemaDirection::Output] {
        let document = document(&type_in(&artifact, "LoadState", direction));
        assert_eq!(
            document["$defs"]["LoadState"],
            json!({ "anyOf": [{ "$ref": "#/$defs/LoadState.failed" }] })
        );
        assert_eq!(
            document["$defs"]["LoadState.failed"],
            json!({
                "type": "object",
                "properties": {
                    "$type": { "const": "LoadState.failed" },
                    "message": { "type": "string" }
                },
                "required": ["$type", "message"],
                "additionalProperties": false
            })
        );
    }
}

#[test]
fn a_mixed_union_lists_constant_cases_then_payload_cases() {
    let artifact = source("type Shape = circle | square { n:int }");
    let document = document(&output_type(&artifact, "Shape"));
    assert_eq!(
        document["$defs"]["Shape"],
        json!({
            "anyOf": [
                { "type": "string", "enum": ["circle"] },
                { "$ref": "#/$defs/Shape.square" }
            ]
        })
    );
}

#[test]
fn a_fieldless_case_of_a_union_with_a_base_is_an_object() {
    let artifact = source(
        "abstract type EventBase = { source:string = \"ui\" }\ntype UiEvent extends EventBase = | closed",
    );
    let document = document(&output_type(&artifact, "UiEvent"));
    assert_eq!(
        document["$defs"]["UiEvent"],
        json!({ "anyOf": [{ "$ref": "#/$defs/UiEvent.closed" }] })
    );
    assert_eq!(
        document["$defs"]["UiEvent.closed"]["properties"],
        json!({
            "$type": { "const": "UiEvent.closed" },
            "source": { "type": "string", "default": "ui" }
        })
    );
}

#[test]
fn a_property_union_is_a_constant_union() {
    let artifact = source("type User = { name:string email?:string }");
    let document = document(&output_type(&artifact, "User.Property"));
    assert_eq!(
        document["$defs"]["User.Property"],
        json!({ "type": "string", "enum": ["name", "email"] })
    );
}

// ---- Abstract records --------------------------------------------------------------------------

#[test]
fn descendants_across_modules_are_listed() {
    let registry = LibraryRegistry::new();
    registry
        .load_library_from_sources(&NxLibrarySource::new(
            "libraries/sources",
            vec![NxLibraryModule::new(
                "Source.nx",
                "export abstract type Source = { name?:string }\nexport type WebSource extends Source = { allowedDomains?:string+ }",
            )],
        ))
        .expect("library loads");
    let artifact = build_in(
        &[(
            "main.nx",
            "import \"libraries/sources\"\ntype TableSource extends Source = { table:string }\nlet pick(): Source = { <TableSource table=\"t\" /> }",
        )],
        &registry.build_context(),
    );
    let output = output(&function(&artifact, "pick"));
    // A library's modules come before the modules that build on it.
    assert_eq!(
        output["anyOf"],
        json!([{ "$ref": "#/$defs/WebSource" }, { "$ref": "#/$defs/TableSource" }])
    );
    assert_eq!(output["$defs"]["WebSource"]["required"], json!(["$type"]));
}

#[test]
fn a_union_extending_the_abstract_record_contributes_its_cases() {
    let artifact = source(
        "abstract type EventBase = { source:string }\ntype UiEvent extends EventBase = | clicked { x:int } | closed",
    );
    let document = document(&output_type(&artifact, "EventBase"));
    assert_eq!(
        document["anyOf"],
        json!([{ "$ref": "#/$defs/UiEvent.clicked" }, { "$ref": "#/$defs/UiEvent.closed" }])
    );
}

#[test]
fn an_abstract_record_with_no_descendant_has_no_json_form() {
    let artifact = source("abstract type Lonely = { id:int }");
    let schema = output_type(&artifact, "Lonely");
    assert!(schema.schema.is_none());
    assert_eq!(
        codes(&schema.diagnostics),
        vec!["schema-inexpressible-type"]
    );
    assert!(messages(&schema.diagnostics).contains("Lonely"));
}

#[test]
fn two_descendants_with_one_discriminator_are_refused() {
    let artifact = build(&[
        (
            "main.nx",
            "import \"./item.nx\"\nimport \"./a.nx\"\nimport \"./b.nx\"\nlet show(item:Item): string = { \"\" }",
        ),
        ("item.nx", "export abstract type Item = { id:int }"),
        ("a.nx", "import \"./item.nx\"\nexport type Card extends Item = { title:string }"),
        ("b.nx", "import \"./item.nx\"\nexport type Card extends Item = { count:int }"),
    ]);
    let schema = function(&artifact, "show");
    assert!(schema.input_schema.is_none());
    assert_eq!(
        codes(&schema.diagnostics),
        vec!["schema-ambiguous-discriminator"]
    );
    let message = messages(&schema.diagnostics);
    assert!(
        message.contains("Card") && message.contains("Item"),
        "{message}"
    );
}

// ---- Generic records, aliases and update records ----------------------------------------------

const PAGE: &str = "type Plan = { name:string }\ntype Page = { T:type items:T+ total:int }";

#[test]
fn an_applied_type_substitutes_its_argument() {
    let artifact = source(&format!(
        "{PAGE}\nlet first(): <Page T=Plan /> = {{ <Page T=Plan items=<Plan name=\"a\" /> total=1 /> }}"
    ));
    let output = output(&function(&artifact, "first"));
    assert_eq!(output["$ref"], "#/$defs/Page_Plan");
    let page = &output["$defs"]["Page_Plan"];
    assert_eq!(
        page["properties"]["items"],
        json!({ "type": "array", "items": { "$ref": "#/$defs/Plan" }, "minItems": 1 })
    );
    assert!(page["properties"].get("T").is_none());
    assert_eq!(page["properties"]["$type"], json!({ "const": "Page" }));
}

#[test]
fn two_instantiations_are_two_entries() {
    let artifact = source(&format!(
        "{PAGE}\nlet both(a:<Page T=Plan />, b:<Page T=string />): int = {{ 1 }}"
    ));
    let schema = function(&artifact, "both");
    let text = text(schema.input_schema.as_ref().unwrap());
    let keys = ["\"Page_Plan\":", "\"Plan\":", "\"Page_string\":"].map(|key| text.find(key));
    assert!(keys.iter().all(Option::is_some), "{text}");
    assert!(keys[0] < keys[1] && keys[1] < keys[2], "{text}");
    let input = input(&schema);
    assert_eq!(
        input["$defs"]["Page_string"]["properties"]["items"]["items"],
        json!({ "type": "string" })
    );
}

#[test]
fn an_unapplied_generic_record_leaves_its_parameter_open() {
    let artifact = source(PAGE);
    let document = document(&output_type(&artifact, "Page"));
    assert_eq!(
        document["$defs"]["Page"]["properties"]["items"],
        json!({ "type": "array", "items": { "not": { "type": "null" } }, "minItems": 1 })
    );
}

#[test]
fn an_alias_is_transparent() {
    let artifact = source("type Names = string+\nlet greet(names:Names): string = { \"\" }");
    let input = input(&function(&artifact, "greet"));
    assert_eq!(
        input["properties"]["names"],
        json!({ "type": "array", "items": { "type": "string" }, "minItems": 1 })
    );
    assert!(input.get("$defs").is_none());
    let document = document(&output_type(&artifact, "Names"));
    assert_eq!(document["type"], "array");
}

#[test]
fn an_update_record_makes_every_field_optional_and_a_clearable_field_nullable() {
    let artifact = source("type User = { name:string email?:string age:int = 1 }");
    let document = document(&output_type(&artifact, "User.Update"));
    assert_eq!(document["$ref"], "#/$defs/User.Update");
    assert_eq!(
        document["$defs"]["User.Update"],
        json!({
            "type": "object",
            "properties": {
                "$type": { "const": "User.Update" },
                "name": { "type": "string" },
                "email": { "anyOf": [{ "type": "string" }, { "type": "null" }] },
                "age": { "type": "integer" }
            },
            "required": ["$type"],
            "additionalProperties": false
        })
    );
}

// ---- Descriptions ------------------------------------------------------------------------------

const DOCUMENTED: &str = r#"/// A plan a team can buy.
///
/// Prices are monthly.
type Plan = {
  name:string   /// The plan's display name.
  seats:int
}

/// How a plan is billed.
type Billing = monthly | yearly

/// Finds the plans that fit a team.
///
/// Plans are sorted by price.
let findPlans(
  teamSize:int,         /// Number of people who need a seat.
  billing?:Billing,     /// Ignored when [maxMonthlyPrice] is set.
  maxMonthlyPrice?:int
): Plan* = { {} }
"#;

#[test]
fn a_documented_function_and_its_parameters() {
    let artifact = source(DOCUMENTED);
    let schema = function(&artifact, "findPlans");
    assert_eq!(
        schema.description.as_deref(),
        Some("Finds the plans that fit a team.\n\nPlans are sorted by price.")
    );
    assert_eq!(
        schema.summary.as_deref(),
        Some("Finds the plans that fit a team.")
    );
    let input = input(&schema);
    assert_eq!(
        input["properties"]["teamSize"],
        json!({ "type": "integer", "description": "Number of people who need a seat." })
    );
    assert_eq!(
        schema.parameters[0].description.as_deref(),
        Some("Number of people who need a seat.")
    );
    // A doc link is written as code, beside a reference.
    assert_eq!(
        input["properties"]["billing"],
        json!({
            "$ref": "#/$defs/Billing",
            "description": "Ignored when `maxMonthlyPrice` is set."
        })
    );
    assert_eq!(
        input["$defs"]["Billing"]["description"],
        "How a plan is billed."
    );
    assert!(input["properties"]["maxMonthlyPrice"]
        .get("description")
        .is_none());

    let output = output(&schema);
    let plan = &output["$defs"]["Plan"];
    assert_eq!(
        plan["description"],
        "A plan a team can buy.\n\nPrices are monthly."
    );
    assert_eq!(
        plan["properties"]["name"]["description"],
        "The plan's display name."
    );
    assert!(plan["properties"]["seats"].get("description").is_none());
}

#[test]
fn a_documented_field_beside_a_reference() {
    let artifact = source(
        "type Plan = { name:string }\ntype Team = {\n  /// The plan the team chose.\n  plan:Plan\n}",
    );
    let document = document(&output_type(&artifact, "Team"));
    assert_eq!(
        document["$defs"]["Team"]["properties"]["plan"],
        json!({ "$ref": "#/$defs/Plan", "description": "The plan the team chose." })
    );
}

#[test]
fn documented_constant_cases_are_listed_on_the_union() {
    let artifact = source(
        "/// How a call is sent.\ntype HttpMethod =\n  | get\n  /// Creates a resource.\n  ///\n  /// Not idempotent.\n  | post",
    );
    let method = document(&output_type(&artifact, "HttpMethod"));
    assert_eq!(
        method["$defs"]["HttpMethod"]["description"],
        "How a call is sent.\n\n- `post`: Creates a resource."
    );
    let undocumented_union = source("type Mode =\n  /// Fills the box.\n  | fill\n  | fit");
    let mode = document(&output_type(&undocumented_union, "Mode"));
    assert_eq!(
        mode["$defs"]["Mode"]["description"],
        "- `fill`: Fills the box."
    );
}

#[test]
fn payload_cases_and_their_fields_and_inherited_fields_carry_their_documentation() {
    let artifact = source(
        "abstract type Base = {\n  /// Where it came from.\n  source:string\n}\ntype Event extends Base =\n  /// The user clicked.\n  | clicked {\n    /// Horizontal position.\n    x:int\n  }",
    );
    let event = document(&output_type(&artifact, "Event"));
    let clicked = &event["$defs"]["Event.clicked"];
    assert_eq!(clicked["description"], "The user clicked.");
    assert_eq!(
        clicked["properties"]["x"]["description"],
        "Horizontal position."
    );
    assert_eq!(
        clicked["properties"]["source"]["description"],
        "Where it came from."
    );

    let records = source(
        "abstract type Entity = {\n  /// The stable id.\n  id:int\n}\ntype User extends Entity = { name:string }",
    );
    let user = document(&output_type(&records, "User"));
    assert_eq!(
        user["$defs"]["User"]["properties"]["id"]["description"],
        "The stable id."
    );
}

#[test]
fn a_types_documentation_is_on_its_answer() {
    let artifact = source("/// Names of people.\n///\n/// At least one.\ntype Names = string+");
    let schema = output_type(&artifact, "Names");
    assert_eq!(
        schema.description.as_deref(),
        Some("Names of people.\n\nAt least one.")
    );
    assert_eq!(schema.summary.as_deref(), Some("Names of people."));
}

#[test]
fn undocumented_source_has_no_descriptions() {
    let artifact = source(
        "type Plan = { name:string }\ntype Mode = fill | fit\nlet find(mode:Mode): Plan* = { {} }",
    );
    let schema = function(&artifact, "find");
    assert!(schema.description.is_none() && schema.summary.is_none());
    let both = format!(
        "{}{}",
        text(schema.input_schema.as_ref().unwrap()),
        text(schema.output_schema.as_ref().unwrap())
    );
    assert!(!both.contains("description"), "{both}");
}

#[test]
fn editing_documentation_leaves_the_structure_alone() {
    let before =
        source("type Plan = { name:string }\nlet find(teamSize:int, plan:Plan): int = { 1 }");
    let after = source(
        "type Plan = { name:string }\nlet find(\n  teamSize:int,   /// Seats needed.\n  plan:Plan\n): int = { 1 }",
    );
    let mut before = input(&function(&before, "find"));
    let mut after = input(&function(&after, "find"));
    assert_eq!(
        after["properties"]["teamSize"]["description"],
        "Seats needed."
    );
    after["properties"]["teamSize"]
        .as_object_mut()
        .unwrap()
        .remove("description");
    before["properties"]["teamSize"]
        .as_object_mut()
        .unwrap()
        .remove("description");
    assert_eq!(before, after);
}

// ---- Types with no JSON form -------------------------------------------------------------------

#[test]
fn a_function_typed_parameter_leaves_the_input_schema_out_and_the_output_in() {
    let artifact = source(
        "let render(template:<function Item:string />: string, title:string): string = { title }",
    );
    let schema = function(&artifact, "render");
    assert!(schema.input_schema.is_none());
    assert_eq!(
        output(&schema),
        json!({ "$schema": SCHEMA, "type": "string" })
    );
    assert_eq!(
        codes(&schema.diagnostics),
        vec!["schema-inexpressible-type"]
    );
    let message = messages(&schema.diagnostics);
    assert!(
        message.contains("`template`") && message.contains("<function Item:string />: string"),
        "{message}"
    );
    assert_eq!(schema.diagnostics[0].labels[0].file, "main.nx");
    assert_eq!(schema.diagnostics[0].labels[0].span.start_line, 1);
}

#[test]
fn a_function_typed_field_deep_in_a_result_is_found() {
    let artifact = source(
        "type Row = { template:<function Item:string />: string }\ntype Catalog = { rows:Row+ }\nlet render(Item:string): string = { Item }\nlet catalog(): Catalog = { <Catalog rows=<Row template={render} /> /> }",
    );
    let schema = function(&artifact, "catalog");
    assert!(schema.output_schema.is_none());
    assert!(schema.input_schema.is_some());
    let message = messages(&schema.diagnostics);
    assert!(message.contains("`Row.template`"), "{message}");
    assert!(message.contains("Catalog.rows.template"), "{message}");
}

#[test]
fn every_problem_is_reported_in_one_call() {
    let artifact = source(
        "abstract type Lonely = { id:int }\nlet both(a:<function x:int />: int, b:Lonely): string = { \"\" }",
    );
    let schema = function(&artifact, "both");
    assert_eq!(
        codes(&schema.diagnostics),
        vec!["schema-inexpressible-type", "schema-inexpressible-type"]
    );
    assert!(schema.input_schema.is_none());
    assert!(schema.output_schema.is_some());
}

#[test]
fn a_component_used_as_a_type_has_no_json_form() {
    let artifact = source(
        "external component <Badge label:string />\nlet show(badge:Badge): string = { \"\" }",
    );
    let schema = function(&artifact, "show");
    assert!(schema.input_schema.is_none());
    let message = messages(&schema.diagnostics);
    assert!(message.contains("Parameter `badge`"), "{message}");
    let type_answer = output_type(&artifact, "Badge");
    assert!(type_answer.schema.is_none());
    assert_eq!(
        codes(&type_answer.diagnostics),
        vec!["schema-inexpressible-type"]
    );
}

#[test]
fn a_function_reference_field_is_not_described_as_a_record() {
    let artifact = source(
        "type FunctionTool = { function: <function ... />: object* }\ntype Args = { q:string }\ntype HttpTool = { arguments: <function ... />: Args }",
    );
    let tool = output_type(&artifact, "FunctionTool");
    assert!(tool.schema.is_none());
    let message = messages(&tool.diagnostics);
    assert!(
        message.contains("`FunctionTool.function`")
            && message.contains("<function ... />: object*"),
        "{message}"
    );
    let http = output_type(&artifact, "HttpTool");
    assert!(http.schema.is_none());
    let message = messages(&http.diagnostics);
    assert!(
        message.contains("`HttpTool.arguments`") && message.contains("<function ... />: Args"),
        "{message}"
    );
}

#[test]
fn a_function_reference_field_is_told_apart_from_an_object_field() {
    let artifact = source(
        "type Pair = { run: <function ... />: object* data:object }\ntype Bag = { data:object }",
    );
    let pair = output_type(&artifact, "Pair");
    assert_eq!(pair.diagnostics.len(), 1);
    assert!(messages(&pair.diagnostics).contains("`Pair.run`"));
    let bag = document(&output_type(&artifact, "Bag"));
    assert_eq!(
        bag["$defs"]["Bag"]["properties"]["data"],
        json!({ "not": { "type": "null" } })
    );
}

// ---- Host-supplied parameters ------------------------------------------------------------------

const AGENT_MODULE: &str = "@nx/agent/agent.nx";

fn tool_context() -> FunctionSchemaOptions {
    FunctionSchemaOptions {
        host_supplied_types: vec![DeclarationRef::in_module(AGENT_MODULE, "ToolContext")],
    }
}

const LOOKUP: &str = "import \"@nx/agent\"\nlet lookupOrder(orderId:string, context:ToolContext): HttpArguments = { <HttpArguments /> }";

#[test]
fn a_context_parameter_is_left_out_of_the_input_schema() {
    let artifact = source(LOOKUP);
    let schema = function_with(
        &artifact,
        DeclarationRef::entry("lookupOrder"),
        tool_context(),
    );
    let input = input(&schema);
    assert_eq!(
        input["properties"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        vec!["orderId"]
    );
    assert_eq!(input["required"], json!(["orderId"]));
    let parameters = serde_json::to_value(&schema.parameters).unwrap();
    assert_eq!(
        parameters[1]["hostSupplied"],
        json!({ "module": AGENT_MODULE, "name": "ToolContext" })
    );
    // `ToolContext` is abstract and nothing here extends it, which is no problem for a parameter
    // the host fills in.
    assert!(schema.diagnostics.is_empty(), "{:#?}", schema.diagnostics);
}

#[test]
fn a_result_holding_an_object_field_is_open_at_that_field() {
    let artifact = source(LOOKUP);
    let schema = function_with(
        &artifact,
        DeclarationRef::entry("lookupOrder"),
        tool_context(),
    );
    let arguments = &output(&schema)["$defs"]["HttpArguments"];
    let mut body = arguments["properties"]["body"].clone();
    // The agent library documents the field; apart from that it admits anything.
    body.as_object_mut().unwrap().remove("description");
    assert_eq!(body, json!({ "not": { "type": "null" } }));
    assert!(!arguments["required"]
        .as_array()
        .unwrap()
        .contains(&json!("body")));
}

#[test]
fn without_the_list_every_parameter_is_an_argument() {
    let artifact = source(LOOKUP);
    let schema = function(&artifact, "lookupOrder");
    assert!(schema.input_schema.is_none());
    assert!(messages(&schema.diagnostics).contains("ToolContext"));
    let parameters = serde_json::to_value(&schema.parameters).unwrap();
    assert!(parameters[1].get("hostSupplied").is_none());

    let with_subtype = source(
        "import \"@nx/agent\"\ntype MyContext extends ToolContext = { user:string }\nlet lookupOrder(orderId:string, context:ToolContext): string = { orderId }",
    );
    let schema = function(&with_subtype, "lookupOrder");
    assert_eq!(
        input(&schema)["properties"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        vec!["context", "orderId"]
    );
}

#[test]
fn a_subtype_of_a_listed_type_across_a_library_boundary_is_host_supplied() {
    let registry = LibraryRegistry::new();
    registry
        .load_library_from_sources(&NxLibrarySource::new(
            "libraries/chat-link",
            vec![NxLibraryModule::new(
                "ChatLink.nx",
                "import \"@nx/agent\"\nexport type ChatToolContext extends ToolContext = { conversationId:string contactEmail?:string }",
            )],
        ))
        .expect("library loads");
    let artifact = build_in(
        &[(
            "main.nx",
            "import \"libraries/chat-link\"\nlet lookupOrder(orderId:string, context:ChatToolContext): string = { orderId }",
        )],
        &registry.build_context(),
    );
    let schema = function_with(
        &artifact,
        DeclarationRef::entry("lookupOrder"),
        tool_context(),
    );
    let input = input(&schema);
    assert_eq!(
        input["properties"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        vec!["orderId"]
    );
    let parameters = serde_json::to_value(&schema.parameters).unwrap();
    assert_eq!(
        parameters[1]["hostSupplied"],
        json!({ "module": AGENT_MODULE, "name": "ToolContext" })
    );
    assert_eq!(
        parameters[1]["typeRef"],
        json!({ "module": "libraries/chat-link/ChatLink.nx", "name": "ChatToolContext" })
    );
    assert_eq!(parameters[1]["type"], "ChatToolContext");
}

#[test]
fn a_sequence_of_a_listed_type_is_not_host_supplied() {
    let artifact =
        source("import \"@nx/agent\"\nlet many(contexts:ToolContext+): string = { \"\" }");
    let schema = function_with(&artifact, DeclarationRef::entry("many"), tool_context());
    assert!(schema.parameters[0].host_supplied.is_none());
    assert!(schema.input_schema.is_none());
}

/// The schema of `name` in a program whose host library declares two subtypes of `ToolContext`,
/// with `ToolContext` listed as host-supplied.
fn held_in(main: &str, name: &str) -> FunctionSchema {
    held_in_listing(main, name, tool_context())
}

/// The schema of `name` in that program, with the host-supplied types of `options`.
fn held_in_listing(main: &str, name: &str, options: FunctionSchemaOptions) -> FunctionSchema {
    let registry = LibraryRegistry::new();
    registry
        .load_library_from_sources(&NxLibrarySource::new(
            "libraries/chat-link",
            vec![NxLibraryModule::new(
                "ChatLink.nx",
                "import \"@nx/agent\"\nexport type ChatToolContext extends ToolContext = { conversationId:string }\nexport type AuditToolContext extends ToolContext = { actor:string }",
            )],
        ))
        .expect("library loads");
    let artifact = build_in(
        &[(
            "main.nx",
            &format!("import \"@nx/agent\"\nimport \"libraries/chat-link\"\n{main}"),
        )],
        &registry.build_context(),
    );
    function_with(&artifact, DeclarationRef::entry(name), options)
}

/// The listed types each parameter's type holds, as the answer serializes them: `null` for a
/// parameter whose entry has no such member.
fn held(schema: &FunctionSchema) -> Vec<Value> {
    serde_json::to_value(&schema.parameters)
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|parameter| parameter["hostSuppliedWithin"].clone())
        .collect()
}

fn the_tool_context() -> Value {
    json!([{ "module": AGENT_MODULE, "name": "ToolContext" }])
}

#[test]
fn a_listed_type_under_an_occurrence_is_named_and_the_parameter_is_kept() {
    let schema = held_in(
        "let many(orderId:string, contexts:ChatToolContext+): string = { orderId }",
        "many",
    );
    assert!(schema.parameters[1].host_supplied.is_none());
    assert_eq!(held(&schema), vec![Value::Null, the_tool_context()]);
    // Nothing else about the answer changes: the parameter is still an argument.
    let input = input(&schema);
    assert_eq!(input["required"], json!(["orderId", "contexts"]));
    assert_eq!(input["properties"]["contexts"]["type"], "array");
    assert!(schema.diagnostics.is_empty());
}

#[test]
fn a_listed_type_held_as_a_field_is_named_at_any_depth() {
    let schema = held_in(
        "type Request = { orderId:string context:ChatToolContext }\ntype Batch = { requests:Request+ }\nlet send(request:Request, batch:Batch, note:string): string = { note }",
        "send",
    );
    assert_eq!(
        held(&schema),
        vec![the_tool_context(), the_tool_context(), Value::Null]
    );
}

#[test]
fn two_parameters_of_one_record_are_both_named() {
    // The schema describes `Request` once and refers to it for the second parameter; what a
    // parameter holds is found for each on its own.
    let schema = held_in(
        "type Request = { context:ChatToolContext }\nlet both(first:Request, second:Request): string = { \"\" }",
        "both",
    );
    assert_eq!(held(&schema), vec![the_tool_context(), the_tool_context()]);
}

#[test]
fn a_parameter_that_holds_no_listed_type_says_nothing() {
    let schema = held_in(
        "type Plan = { name:string seats:int }\nlet price(orderId:string, plan:Plan, plans?:Plan+): int = { 1 }",
        "price",
    );
    assert_eq!(held(&schema), vec![Value::Null, Value::Null, Value::Null]);
    assert!(schema
        .parameters
        .iter()
        .all(|parameter| parameter.host_supplied_within.is_empty()));
}

#[test]
fn a_record_that_holds_itself_ends_and_is_named_once() {
    let schema = held_in(
        "type Node = { next?:Node context?:ChatToolContext }\nlet walk(node:Node): string = { \"\" }",
        "walk",
    );
    assert_eq!(held(&schema), vec![the_tool_context()]);
}

#[test]
fn a_listed_type_in_a_union_case_through_an_alias_and_as_a_type_argument_is_named() {
    let schema = held_in(
        "type Target =\n  | nobody\n  | chat { context:ChatToolContext }\ntype Contexts = ChatToolContext+\ntype Page = { T:type items:T+ }\ntype Plan = { name:string }\nlet reach(target:Target, alias:Contexts, page:<Page T=ChatToolContext />, plans:<Page T=Plan />): string = { \"\" }",
        "reach",
    );
    assert_eq!(
        held(&schema),
        vec![
            the_tool_context(),
            the_tool_context(),
            the_tool_context(),
            Value::Null
        ]
    );
}

#[test]
fn an_alias_of_a_listed_type_is_host_supplied_as_the_type_it_denotes() {
    // An alias denotes its target everywhere else: in the schema, and to a runtime.
    let schema = held_in(
        "type Context = ChatToolContext\ntype Again = Context\ntype Base = ToolContext\ntype Order = { id:string }\ntype OrderAlias = Order\nlet lookup(orderId:string, context:Context, again:Again, base:Base, order:OrderAlias): string = { orderId }",
        "lookup",
    );
    let parameters = serde_json::to_value(&schema.parameters).unwrap();
    let chat = json!({ "module": "libraries/chat-link/ChatLink.nx", "name": "ChatToolContext" });
    let listed = json!({ "module": AGENT_MODULE, "name": "ToolContext" });
    for index in [1, 2] {
        assert_eq!(parameters[index]["hostSupplied"], listed);
        assert_eq!(parameters[index]["typeRef"], chat);
        assert!(parameters[index].get("hostSuppliedWithin").is_none());
    }
    // The spelling is still the author's.
    assert_eq!(parameters[1]["type"], "Context");
    assert_eq!(parameters[3]["hostSupplied"], listed);
    assert_eq!(parameters[3]["typeRef"], listed);
    // An alias of any record names the record it denotes, host-supplied or not.
    assert!(parameters[4].get("hostSupplied").is_none());
    assert_eq!(
        parameters[4]["typeRef"],
        json!({ "module": "main.nx", "name": "Order" })
    );
    let input = input(&schema);
    assert_eq!(
        input["properties"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        vec!["order", "orderId"]
    );
    assert_eq!(input["required"], json!(["orderId", "order"]));
}

#[test]
fn an_alias_of_an_occurrence_of_a_listed_type_is_not_host_supplied() {
    let schema = held_in(
        "type Contexts = ChatToolContext+\ntype Names = string+\nlet many(contexts:Contexts, names:Names): string = { \"\" }",
        "many",
    );
    assert!(schema.parameters[0].host_supplied.is_none());
    assert!(schema.parameters[0].type_ref.is_none());
    assert!(schema.parameters[1].type_ref.is_none());
    assert_eq!(held(&schema), vec![the_tool_context(), Value::Null]);
}

#[test]
fn a_record_that_is_two_listed_types_is_named_for_each() {
    let main = "abstract type SignalContext extends ToolContext = { origin:string }\ntype Signal extends SignalContext = | ping | pong { n:int }\ntype Request = { orderId:string context:ChatToolContext }\nlet many(contexts:ChatToolContext+, request:Request, audits:AuditToolContext+, signal:Signal, context:ChatToolContext): string = { \"\" }";
    let base = || DeclarationRef::in_module(AGENT_MODULE, "ToolContext");
    let chat = || DeclarationRef::in_module("libraries/chat-link/ChatLink.nx", "ChatToolContext");
    let signal = || DeclarationRef::entry("SignalContext");
    let base_name = json!({ "module": AGENT_MODULE, "name": "ToolContext" });
    let chat_name =
        json!({ "module": "libraries/chat-link/ChatLink.nx", "name": "ChatToolContext" });
    let signal_name = json!({ "module": "main.nx", "name": "SignalContext" });
    let listing = |host_supplied_types| {
        held_in_listing(
            main,
            "many",
            FunctionSchemaOptions {
                host_supplied_types,
            },
        )
    };

    // A `ChatToolContext` is a `ToolContext` too, and a case of `Signal` is its base and what
    // that extends: each is named, in the order listed.
    let schema = listing(vec![base(), chat(), signal()]);
    assert_eq!(
        held(&schema),
        vec![
            json!([base_name, chat_name]),
            json!([base_name, chat_name]),
            json!([base_name]),
            json!([base_name, signal_name]),
            Value::Null,
        ]
    );
    let schema = listing(vec![signal(), chat(), base()]);
    assert_eq!(
        held(&schema),
        vec![
            json!([chat_name, base_name]),
            json!([chat_name, base_name]),
            json!([base_name]),
            json!([signal_name, base_name]),
            Value::Null,
        ]
    );
    // A parameter the host fills in is filled in as one listed type: the first it is.
    assert_eq!(
        serde_json::to_value(&schema.parameters[4].host_supplied).unwrap(),
        chat_name
    );

    // A type listed twice is one listed type, at its first place.
    let schema = listing(vec![base(), chat(), base(), chat()]);
    assert_eq!(held(&schema)[0], json!([base_name, chat_name]));
}

#[test]
fn a_derived_type_of_a_listed_type_holds_none_of_it() {
    // A value of `ChatToolContext.Update` is not a `ChatToolContext`, and no host fills one in:
    // a parameter typed by it is an argument like any other. The update record of a record that
    // holds a context still reaches the field.
    let schema = held_in(
        "type Request = { orderId:string context:ChatToolContext }\nlet patch(own:ChatToolContext.Update, base:ToolContext.Update, property:ChatToolContext.Property, holder:Request.Update): string = { \"\" }",
        "patch",
    );
    for index in [0, 1, 2] {
        assert!(schema.parameters[index].host_supplied.is_none());
    }
    assert_eq!(
        held(&schema),
        vec![Value::Null, Value::Null, Value::Null, the_tool_context()]
    );
    let input = input(&schema);
    assert_eq!(
        input["required"],
        json!(["own", "base", "property", "holder"])
    );
}

#[test]
fn a_record_that_applies_itself_to_a_growing_argument_ends() {
    let schema = held_in(
        "type Grow = { T:type value?:T inner?:<Grow T=<Grow T=T />/> }\nlet grow(plain:<Grow T=string />, held:<Grow T=ChatToolContext />): string = { \"\" }",
        "grow",
    );
    assert_eq!(held(&schema), vec![Value::Null, the_tool_context()]);
}

#[test]
fn the_abstract_listed_type_and_a_subtype_the_host_does_not_use_are_named() {
    let schema = held_in(
        "let any(contexts:ToolContext+, audits:AuditToolContext+): string = { \"\" }",
        "any",
    );
    assert_eq!(held(&schema), vec![the_tool_context(), the_tool_context()]);
}

#[test]
fn a_host_supplied_parameter_names_nothing_it_holds() {
    let schema = held_in(
        "let lookup(orderId:string, context:ChatToolContext, base:ToolContext): string = { orderId }",
        "lookup",
    );
    assert!(schema.parameters[1].host_supplied.is_some());
    assert!(schema.parameters[2].host_supplied.is_some());
    assert_eq!(held(&schema), vec![Value::Null, Value::Null, Value::Null]);
}

#[test]
fn listed_types_are_named_in_the_order_listed_and_each_once() {
    let artifact = source(
        "type A = { a:string }\ntype B = { b:string }\ntype Holder = { first:A second:B again:A+ }\nlet hold(holder:Holder, only:B+): string = { \"\" }",
    );
    let schema = function_with(
        &artifact,
        DeclarationRef::entry("hold"),
        FunctionSchemaOptions {
            host_supplied_types: vec![DeclarationRef::entry("B"), DeclarationRef::entry("A")],
        },
    );
    assert_eq!(
        held(&schema),
        vec![
            json!([{ "module": "main.nx", "name": "B" }, { "module": "main.nx", "name": "A" }]),
            json!([{ "module": "main.nx", "name": "B" }]),
        ]
    );
}

#[test]
fn with_no_listed_types_nothing_is_named() {
    let artifact = source(
        "import \"@nx/agent\"\ntype Chat extends ToolContext = { id:string }\nlet many(contexts:Chat+): string = { \"\" }",
    );
    let schema = function(&artifact, "many");
    assert_eq!(held(&schema), vec![Value::Null]);
}

#[test]
fn a_listed_type_the_program_does_not_declare_matches_nothing() {
    let artifact = source("let add(a:int, b:int): int = { a + b }");
    let schema = function_with(&artifact, DeclarationRef::entry("add"), tool_context());
    assert!(schema.diagnostics.is_empty());
    assert_eq!(input(&schema)["required"], json!(["a", "b"]));
}

#[test]
fn a_site_typed_by_the_agent_librarys_tool_has_no_json_form() {
    let artifact =
        source("import \"@nx/agent\"\nlet pick(): Tool = { <WebSearchTool name=\"search\" /> }");
    let schema = function(&artifact, "pick");
    assert!(schema.output_schema.is_none());
    let message = messages(&schema.diagnostics);
    assert!(message.contains("`FunctionTool.function`"), "{message}");
    assert!(message.contains("`HttpTool.arguments`"), "{message}");
    assert_eq!(
        codes(&schema.diagnostics),
        vec!["schema-inexpressible-type", "schema-inexpressible-type"]
    );
}

// ---- The shared corpus -------------------------------------------------------------------------

/// The schema corpus the SDKs' tests read too: one NX source per mapping, with the documents the
/// export answers for each of its functions.
fn corpus_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../bindings/wasm/test/fixtures/schema")
}

/// The golden document of one corpus module: for each function, in declaration order, its input
/// and output schemas and the codes of its diagnostics.
fn corpus_golden(identity: &str, source_text: &str) -> String {
    let artifact = build(&[(identity, source_text)]);
    let module = artifact
        .root_modules
        .iter()
        .find(|module| module.file_name == identity)
        .and_then(|module| module.lowered_module.clone())
        .expect("the corpus module lowers");
    let mut functions = Vec::new();
    for item in module.items() {
        let nx_hir::Item::Function(declared) = item else {
            continue;
        };
        let schema = function(&artifact, declared.name.as_str());
        let mut entry = Vec::new();
        if let Some(input) = schema.input_schema {
            entry.push(("inputSchema".to_string(), input));
        }
        if let Some(output) = schema.output_schema {
            entry.push(("outputSchema".to_string(), output));
        }
        entry.push((
            "diagnostics".to_string(),
            SchemaValue::Array(
                codes(&schema.diagnostics)
                    .into_iter()
                    .map(|code| SchemaValue::String(code.to_string()))
                    .collect(),
            ),
        ));
        functions.push((declared.name.to_string(), SchemaValue::Object(entry)));
    }
    serde_json::to_string_pretty(&SchemaValue::Object(functions)).expect("golden serializes") + "\n"
}

#[test]
fn the_corpus_answers_its_golden_documents() {
    let update = std::env::var_os("NX_UPDATE_SCHEMA_CORPUS").is_some();
    let mut sources = std::fs::read_dir(corpus_dir())
        .expect("the corpus directory")
        .map(|entry| entry.expect("a corpus entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "nx"))
        .collect::<Vec<_>>();
    sources.sort();
    assert!(!sources.is_empty());
    let mut stale = Vec::new();
    for path in sources {
        let identity = path.file_name().unwrap().to_str().unwrap();
        let source_text = std::fs::read_to_string(&path).expect("a corpus source");
        let golden = corpus_golden(identity, &source_text);
        let golden_path = path.with_extension("schema.json");
        if update {
            std::fs::write(&golden_path, &golden).expect("the golden document is written");
        } else if std::fs::read_to_string(&golden_path).ok().as_deref() != Some(golden.as_str()) {
            stale.push(identity.to_string());
        }
    }
    assert!(
        stale.is_empty(),
        "golden documents differ for {stale:?}; review and rerun with NX_UPDATE_SCHEMA_CORPUS=1"
    );
}

// ---- Review follow-ups -------------------------------------------------------------------------

#[test]
fn object_admits_any_value_but_null() {
    // A runtime reads `null` as no value and refuses it where exactly one value is expected, while
    // it takes an array or an object at an `object` site whole.
    let artifact = source("let keep(value:object, many:object+): object = { value }");
    let schema = function(&artifact, "keep");
    let not_null = json!({ "not": { "type": "null" } });
    assert_eq!(input(&schema)["properties"]["value"], not_null);
    assert_eq!(
        input(&schema)["properties"]["many"],
        json!({ "type": "array", "items": not_null, "minItems": 1 })
    );
    assert_eq!(
        output(&schema),
        json!({ "$schema": SCHEMA, "not": { "type": "null" } })
    );

    // An update record still spells a cleared `object` field as `null`.
    let update = source("type Bag = { data?:object }");
    let document = document(&output_type(&update, "Bag.Update"));
    assert_eq!(
        document["$defs"]["Bag.Update"]["properties"]["data"],
        json!({ "anyOf": [not_null, { "type": "null" }] })
    );
}

#[test]
fn a_member_access_default_is_a_constant_case_only_through_the_union() {
    let artifact = source(
        "type Status = active | paused\ntype Cfg = { paused:Status }\nlet pick(c:Cfg, s:Status = { c.paused }, t:Status = {Status.paused}): Status = { s }",
    );
    let input = input(&function(&artifact, "pick"));
    assert_eq!(
        input["properties"]["s"],
        json!({ "$ref": "#/$defs/Status" })
    );
    assert_eq!(
        input["properties"]["t"],
        json!({ "$ref": "#/$defs/Status", "default": "paused" })
    );
}

#[test]
fn markup_is_reported_as_markup() {
    let artifact = source("let card() = { <div /> }\nlet node(): Element = { <div /> }");
    for name in ["card", "node"] {
        let schema = function(&artifact, name);
        assert!(schema.output_schema.is_none());
        assert_eq!(
            codes(&schema.diagnostics),
            vec!["schema-inexpressible-type"]
        );
        let message = messages(&schema.diagnostics);
        assert!(message.contains("is markup"), "{message}");
        assert!(!message.contains("does not resolve"), "{message}");
    }
}

#[test]
fn a_query_sent_as_json_is_answered_as_json_and_refuses_unknown_fields() {
    use crate::{
        program_artifact_function_schema_json, program_artifact_type_schema_json, SchemaQueryError,
    };
    let artifact = source(LOOKUP);
    let answer = program_artifact_function_schema_json(
        &artifact,
        r#"{"reference":{"name":"lookupOrder"},"options":{"hostSuppliedTypes":[{"module":"@nx/agent/agent.nx","name":"ToolContext"}]}}"#,
    )
    .unwrap();
    let answer: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(
        answer["parameters"][1]["hostSupplied"],
        json!({ "module": AGENT_MODULE, "name": "ToolContext" })
    );
    assert_eq!(answer["inputSchema"]["required"], json!(["orderId"]));

    for request in [
        r#"{"reference":{"name":"lookupOrder"},"options":{"hostSupplied":[]}}"#,
        r#"{"reference":{"name":"lookupOrder","modul":"main.nx"}}"#,
        r#"{"ref":{"name":"lookupOrder"}}"#,
    ] {
        assert!(
            matches!(
                program_artifact_function_schema_json(&artifact, request),
                Err(SchemaQueryError::Request(_))
            ),
            "{request}"
        );
    }
    assert!(matches!(
        program_artifact_type_schema_json(
            &artifact,
            r#"{"reference":{"name":"HttpArguments"},"options":{"direction":"sideways"}}"#
        ),
        Err(SchemaQueryError::Request(_))
    ));
    assert!(matches!(
        program_artifact_function_schema_json(&artifact, r#"{"reference":{"name":"missing"}}"#),
        Err(SchemaQueryError::Declaration(_))
    ));
}

/// A workspace whose entry takes an abstract `Base`, with a subtype in a module the entry imports
/// but never references and one in a module nothing imports.
const SUBTYPE_MODULES: [(&str, &str); 4] = [
    (
        "main.nx",
        "import \"./base.nx\"\nimport \"./x.nx\"\nlet f(s:Base): string = { \"ok\" }",
    ),
    (
        "base.nx",
        "export abstract type Base = { id:int }\nexport type A extends Base = { a:string }",
    ),
    (
        "x.nx",
        "import \"./base.nx\"\nexport type X extends Base = { x:string }",
    ),
    (
        "y.nx",
        "import \"./base.nx\"\nexport type Y extends Base = { y:string }",
    ),
];

#[test]
fn the_modules_declaring_boundary_subtypes_are_the_ones_the_schemas_list() {
    use crate::program_artifact_boundary_subtype_modules;
    let artifact = build(&SUBTYPE_MODULES);
    assert_eq!(
        program_artifact_boundary_subtype_modules(&artifact),
        vec!["base.nx", "x.nx", "y.nx"]
    );
    assert_eq!(
        input(&function(&artifact, "f"))["properties"]["s"]["anyOf"],
        json!([
            { "$ref": "#/$defs/A" },
            { "$ref": "#/$defs/X" },
            { "$ref": "#/$defs/Y" }
        ])
    );

    // An abstract record reached only through a field counts, and one no function reaches does not.
    let nested = build(&[
        (
            "main.nx",
            "import \"./shapes.nx\"\ntype Scene = { shape:Shape }\nlet draw(scene:Scene): int = { 1 }",
        ),
        (
            "shapes.nx",
            "export abstract type Shape = { label?:string }\nexport type Dot extends Shape = { r:int }",
        ),
        (
            "unused.nx",
            "abstract type Lonely = { id:int }\ntype Only extends Lonely = { n:int }",
        ),
    ]);
    assert_eq!(
        program_artifact_boundary_subtype_modules(&nested),
        vec!["shapes.nx"]
    );

    // A program with no abstract record at a function's boundary lists nothing.
    assert!(program_artifact_boundary_subtype_modules(&source(
        "type Plan = { name:string }\nlet plan(): Plan = { <Plan name=\"a\" /> }"
    ))
    .is_empty());
}

#[test]
fn a_record_applied_to_itself_without_end_is_described_and_walked_in_finite_steps() {
    use crate::program_artifact_boundary_subtype_modules;
    let artifact = source(
        "type Box = { T:type v:T inner?:<Box T=<Box T=T /> /> }\nlet f(b:<Box T=int />): int = { 1 }",
    );
    assert!(program_artifact_boundary_subtype_modules(&artifact).is_empty());
    let schema = function(&artifact, "f");
    let text = text(schema.input_schema.as_ref().unwrap());
    let input = input(&schema);
    assert_eq!(
        input["properties"]["b"],
        json!({ "$ref": "#/$defs/Box_int" })
    );
    // One entry per depth until the arguments apply `Box` twice; from there the record named with
    // no arguments, whose `v` takes the `object` schema, closes the cycle.
    let keys = [
        "Box_int",
        "Box_Box_int",
        "Box",
        "Box_Box_object",
        "Box_object",
    ];
    for key in keys {
        assert!(input["$defs"].get(key).is_some(), "{key} in {text}");
    }
    assert_eq!(
        input["$defs"].as_object().unwrap().len(),
        keys.len(),
        "{text}"
    );
    assert_eq!(
        input["$defs"]["Box_Box_int"]["properties"]["inner"],
        json!({ "$ref": "#/$defs/Box" })
    );
    assert_eq!(
        input["$defs"]["Box"]["properties"]["v"],
        json!({ "not": { "type": "null" } })
    );
}

#[test]
fn a_record_whose_argument_grows_through_another_record_stays_finite() {
    use crate::program_artifact_boundary_subtype_modules;
    let artifact = source(
        "type Pair = { L:type R:type l:L r:R }\ntype Box = { T:type v:T inner?:<Box T=<Pair L=T R=T /> /> }\nlet f(b:<Box T=int />): int = { 1 }",
    );
    assert!(program_artifact_boundary_subtype_modules(&artifact).is_empty());
    let boxed = input(&function(&artifact, "f"));
    let defs = boxed["$defs"].as_object().unwrap();
    assert!(defs.len() < 20, "{} entries", defs.len());
    assert!(
        defs.contains_key("Box"),
        "{:?}",
        defs.keys().collect::<Vec<_>>()
    );

    // Mutual recursion through two records ends too.
    let mutual = source(
        "type A = { T:type v:T b?:<B T=<A T=T /> /> }\ntype B = { T:type w:T a?:<A T=<B T=T /> /> }\nlet g(x:<A T=int />): int = { 1 }",
    );
    assert!(function(&mutual, "g").input_schema.is_some());

    // A nested type written out within the bound keeps its arguments.
    let nested = source(&format!(
        "{PAGE}\nlet deep(p:<Page T=<Page T=Plan /> />): int = {{ 1 }}"
    ));
    let deep = input(&function(&nested, "deep"));
    assert_eq!(
        deep["properties"]["p"],
        json!({ "$ref": "#/$defs/Page_Page_Plan" })
    );
}

#[test]
fn an_abstract_record_reached_through_a_type_argument_counts() {
    use crate::program_artifact_boundary_subtype_modules;
    let artifact = build(&[
        (
            "main.nx",
            "import \"./shapes.nx\"\ntype Page = { T:type items:T+ }\nlet show(page:<Page T=Shape />): int = { 1 }",
        ),
        (
            "shapes.nx",
            "export abstract type Shape = { label?:string }\nexport type Dot extends Shape = { r:int }",
        ),
    ]);
    assert_eq!(
        program_artifact_boundary_subtype_modules(&artifact),
        vec!["shapes.nx"]
    );
}

#[test]
fn only_functions_the_entry_reaches_through_imports_count() {
    use crate::program_artifact_boundary_subtype_modules;
    // `tools.nx` takes `Base`, but nothing imports it, so nothing links it and nothing calls it.
    let artifact = build(&[
        ("main.nx", "let root() = { 1 }"),
        (
            "tools.nx",
            "import \"./base.nx\"\nlet use(s:Base): int = { 1 }",
        ),
        (
            "base.nx",
            "export abstract type Base = { id:int }\nexport type A extends Base = { a:string }",
        ),
    ]);
    assert!(program_artifact_boundary_subtype_modules(&artifact).is_empty());

    // Imported from the entry, through another module, it counts.
    let reached = build(&[
        ("main.nx", "import \"./middle.nx\"\nlet root() = { 1 }"),
        ("middle.nx", "import \"./tools.nx\"\nexport let m() = { 1 }"),
        (
            "tools.nx",
            "import \"./base.nx\"\nexport let use(s:Base): int = { 1 }",
        ),
        (
            "base.nx",
            "export abstract type Base = { id:int }\nexport type A extends Base = { a:string }",
        ),
    ]);
    assert_eq!(
        program_artifact_boundary_subtype_modules(&reached),
        vec!["base.nx"]
    );
}
