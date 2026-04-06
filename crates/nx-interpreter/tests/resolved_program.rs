use nx_hir::{lower_source_module, LoweredModule, Name};
use nx_interpreter::{
    Interpreter, ModuleQualifiedItemRef, ResolvedItemKind, ResolvedModule, ResolvedProgram,
    RuntimeErrorKind, RuntimeModuleId, Value,
};
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

const ROOT_SOURCE: &str = r#"
    import { SearchSubmitted, DoSearch, calcDouble } from "../ui"

    component <SearchBox emits { SearchSubmitted } /> = {
      <TextInput />
    }

    let render(userId:string) = {
      <SearchBox onSearchSubmitted=<DoSearch userId={userId} search={action.searchString} /> />
    }

    let root() = { calcDouble(21) }
"#;

const LIBRARY_SOURCE: &str = r#"
    action SearchSubmitted = { searchString:string }
    action DoSearch = { userId:string search:string }

    let calcDouble(value:int) = { value * 2 }

    component <LibrarySearchBox placeholder:string = "Find docs" emits { SearchSubmitted } /> = {
      state { query:string = {placeholder} }
      <TextInput value={query} placeholder={placeholder} />
    }
"#;

fn lower_module(source: &str, path: &Path) -> Arc<LoweredModule> {
    let file_name = path.display().to_string();
    let module = lower_source_module(source, &file_name).unwrap_or_else(|diagnostics| {
        panic!("Expected {file_name} to lower, got {:?}", diagnostics)
    });
    Arc::new(module)
}

fn item_ref(
    module_id: RuntimeModuleId,
    item_name: &str,
    kind: ResolvedItemKind,
) -> ModuleQualifiedItemRef {
    ModuleQualifiedItemRef {
        module_id,
        item_name: item_name.to_string(),
        kind,
    }
}

fn empty_record() -> Value {
    Value::Record {
        type_name: Name::new("object"),
        fields: FxHashMap::default(),
    }
}

fn extract_field<'a>(value: &'a Value, field: &str) -> &'a Value {
    let Value::Record { fields, .. } = value else {
        panic!("Expected record value, got {:?}", value);
    };

    fields
        .get(field)
        .unwrap_or_else(|| panic!("Expected field '{field}'"))
}

fn build_resolved_program(
    fingerprint: u64,
) -> (ResolvedProgram, Arc<LoweredModule>, RuntimeModuleId) {
    let temp = TempDir::new().expect("temp dir");
    let app_dir = temp.path().join("app");
    let ui_dir = temp.path().join("ui");
    fs::create_dir_all(&app_dir).expect("app dir");
    fs::create_dir_all(&ui_dir).expect("ui dir");

    let root_path = app_dir.join("main.nx");
    let library_path = ui_dir.join("search-box.nx");
    fs::write(&root_path, ROOT_SOURCE).expect("root source");
    fs::write(&library_path, LIBRARY_SOURCE).expect("library source");

    let root_module = lower_module(ROOT_SOURCE, &root_path);
    let library_module = lower_module(LIBRARY_SOURCE, &library_path);
    let root_module_id = RuntimeModuleId::new(0);
    let library_module_id = RuntimeModuleId::new(1);

    let modules = vec![
        ResolvedModule {
            id: root_module_id,
            identity: "app/main.nx".to_string(),
            lowered_module: root_module.clone(),
        },
        ResolvedModule {
            id: library_module_id,
            identity: "ui/search-box.nx".to_string(),
            lowered_module: library_module,
        },
    ];

    let mut entry_functions = FxHashMap::default();
    entry_functions.insert(
        "render".to_string(),
        item_ref(root_module_id, "render", ResolvedItemKind::Function),
    );
    entry_functions.insert(
        "root".to_string(),
        item_ref(root_module_id, "root", ResolvedItemKind::Function),
    );
    entry_functions.insert(
        "calcDouble".to_string(),
        item_ref(library_module_id, "calcDouble", ResolvedItemKind::Function),
    );

    let mut entry_components = FxHashMap::default();
    entry_components.insert(
        "LibrarySearchBox".to_string(),
        item_ref(
            library_module_id,
            "LibrarySearchBox",
            ResolvedItemKind::Component,
        ),
    );

    let mut entry_records = FxHashMap::default();
    entry_records.insert(
        "DoSearch".to_string(),
        item_ref(library_module_id, "DoSearch", ResolvedItemKind::Record),
    );
    entry_records.insert(
        "SearchSubmitted".to_string(),
        item_ref(
            library_module_id,
            "SearchSubmitted",
            ResolvedItemKind::Record,
        ),
    );

    let mut imports = FxHashMap::default();
    let mut root_imports = FxHashMap::default();
    root_imports.insert(
        "SearchSubmitted".to_string(),
        item_ref(
            library_module_id,
            "SearchSubmitted",
            ResolvedItemKind::Record,
        ),
    );
    root_imports.insert(
        "DoSearch".to_string(),
        item_ref(library_module_id, "DoSearch", ResolvedItemKind::Record),
    );
    root_imports.insert(
        "calcDouble".to_string(),
        item_ref(library_module_id, "calcDouble", ResolvedItemKind::Function),
    );
    imports.insert(root_module_id, root_imports);

    (
        ResolvedProgram::new(
            fingerprint,
            vec![root_module_id],
            modules,
            entry_functions,
            entry_components,
            entry_records,
            FxHashMap::default(),
            imports,
        ),
        root_module,
        root_module_id,
    )
}

#[test]
fn resolved_program_executes_cross_module_entries_and_module_qualified_handlers() {
    let (program, root_module, root_module_id) = build_resolved_program(0xCAFE_BABE);
    let interpreter = Interpreter::from_resolved_program(program);

    let root_value = interpreter
        .execute_resolved_program_function("root", vec![])
        .expect("Expected cross-module root evaluation to succeed");
    assert_eq!(root_value, Value::Int(42));

    let rendered = interpreter
        .execute_resolved_program_function("render", vec![Value::String(SmolStr::new("u1"))])
        .expect("Expected render entrypoint to succeed");
    let handler = extract_field(&rendered, "onSearchSubmitted");

    match handler {
        Value::ActionHandler {
            module_id,
            component,
            emit,
            action_name,
            ..
        } => {
            assert_eq!(*module_id, root_module_id);
            assert_eq!(component.as_str(), "SearchBox");
            assert_eq!(emit.as_str(), "SearchSubmitted");
            assert_eq!(action_name.as_str(), "SearchSubmitted");
        }
        other => panic!("Expected action handler value, got {:?}", other),
    }

    let mut action_fields = FxHashMap::default();
    action_fields.insert(
        SmolStr::new("searchString"),
        Value::String(SmolStr::new("docs")),
    );
    let effects = interpreter
        .invoke_action_handler(
            root_module.as_ref(),
            handler,
            Value::Record {
                type_name: Name::new("SearchSubmitted"),
                fields: action_fields,
            },
        )
        .expect("Expected imported action handler to round-trip");

    assert_eq!(effects.len(), 1);
    match &effects[0] {
        Value::Record { type_name, fields } => {
            assert_eq!(type_name.as_str(), "DoSearch");
            assert_eq!(
                fields.get("userId"),
                Some(&Value::String(SmolStr::new("u1")))
            );
            assert_eq!(
                fields.get("search"),
                Some(&Value::String(SmolStr::new("docs")))
            );
        }
        other => panic!("Expected action record result, got {:?}", other),
    }
}

#[test]
fn resolved_program_component_snapshots_accept_matching_program_and_reject_mismatches() {
    let (program, root_module, _) = build_resolved_program(0xCAFE_BABE);
    let interpreter = Interpreter::from_resolved_program(program);

    let init = interpreter
        .initialize_resolved_component("LibrarySearchBox", empty_record())
        .expect("Expected imported component initialization to succeed");
    assert!(!init.state_snapshot.is_empty());

    match &init.rendered {
        Value::Record { type_name, fields } => {
            assert_eq!(type_name.as_str(), "TextInput");
            assert_eq!(
                fields.get("placeholder"),
                Some(&Value::String(SmolStr::new("Find docs")))
            );
            assert_eq!(
                fields.get("value"),
                Some(&Value::String(SmolStr::new("Find docs")))
            );
        }
        other => panic!("Expected rendered component record, got {:?}", other),
    }

    let dispatch = interpreter
        .dispatch_resolved_component_actions(&init.state_snapshot, vec![])
        .expect("Expected snapshot dispatch to accept matching program fingerprint");
    assert!(dispatch.effects.is_empty());
    assert!(!dispatch.state_snapshot.is_empty());

    let mismatch_interpreter =
        Interpreter::from_resolved_program(build_resolved_program(0xDEAD_BEEF).0);
    let mismatch_error = mismatch_interpreter
        .dispatch_resolved_component_actions(&init.state_snapshot, vec![])
        .expect_err("Expected snapshot dispatch to reject a different program fingerprint");
    assert!(matches!(
        mismatch_error.kind(),
        RuntimeErrorKind::InvalidComponentStateSnapshot { reason }
            if reason.contains("snapshot fingerprint")
    ));

    let bare_error = Interpreter::new()
        .dispatch_component_actions(root_module.as_ref(), &init.state_snapshot, vec![])
        .expect_err("Expected bare interpreter to reject program-stamped snapshots");
    assert!(matches!(
        bare_error.kind(),
        RuntimeErrorKind::InvalidComponentStateSnapshot { reason }
            if reason.contains("requires a resolved program runtime")
    ));
}
