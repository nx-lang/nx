use crate::builder::build_codegen_program;
use crate::model::{
    CodegenComponent, CodegenComponentDescriptor, CodegenComponentField,
    CodegenComponentTargetKind, CodegenDeclaration, CodegenDeclarationKind, CodegenElement,
    CodegenExpression, CodegenExpressionKind, CodegenModule, CodegenModuleProvenance,
    CodegenProgram, CodegenProperty, CodegenRecordField, CodegenReference, CodegenStatement,
    CodegenUnionCase,
};
use crate::options::{
    CodegenError, CodegenOptions, CodegenOutput, CodegenTarget, GeneratedFile,
    GeneratedJsProgramModule, GeneratedJsProgramModuleComponentExport,
    GeneratedJsProgramModuleFunctionExport, JsProgramModuleOptions, NX_JS_RUNTIME_ABI,
};
use crate::runtime::runtime_helper_source;
use nx_api::ProgramArtifact;
use nx_diagnostics::{Diagnostic, Label};
use nx_hir::ast::{BinOp, Literal, TypeRef, UnOp};
use nx_interpreter::{ResolvedItemKind, RuntimeModuleId};
use nx_types::{Primitive, Type};
use rustc_hash::{FxHashMap, FxHashSet};
use std::path::{Path, PathBuf};

const JS_PROGRAM_MODULE_MANIFEST_EXPORT_NAME: &str = "nxProgramModuleManifest";
const JS_PROGRAM_MODULE_RESERVED_RUNTIME_NAMES: &[&str] = &[
    "NxResult",
    "NxValue",
    "nxAssertRecord",
    "nxAnySchema",
    "nxArraySchema",
    "nxBooleanSchema",
    "nxComponentSchema",
    "nxDiagnosticsFromError",
    "nxElement",
    "nxEnumSchema",
    "nxExternalComponentSchema",
    "nxField",
    "nxMissingField",
    "nxNamedRecordSchema",
    "nxNormalizeValue",
    "nxNullableSchema",
    "nxNumberSchema",
    "nxRejectUnknownFields",
    "nxRecordSchema",
    "nxRuntimeError",
    "nxStringSchema",
    "nxUnionSchema",
];

/// Builds and emits executable files directly from a program artifact.
pub fn emit_program(
    artifact: &ProgramArtifact,
    options: &CodegenOptions,
) -> Result<CodegenOutput, CodegenError> {
    let program = build_codegen_program(artifact)?;
    emit_codegen_program(&program, options)
}

/// Emits executable files from an already-built codegen program.
pub fn emit_codegen_program(
    program: &CodegenProgram,
    options: &CodegenOptions,
) -> Result<CodegenOutput, CodegenError> {
    validate_source_codegen_program(program)?;

    let context = EmitContext::new(program, options.target);
    let mut files = Vec::new();
    files.push(GeneratedFile {
        relative_path: PathBuf::from(format!("nx-runtime.{}", options.target.extension())),
        content: runtime_helper_source(options.target),
    });

    for module in &program.modules {
        files.push(GeneratedFile {
            relative_path: PathBuf::from(context.module_file(module.id)),
            content: emit_module(program, module, &context, options.target),
        });
    }

    files.push(GeneratedFile {
        relative_path: PathBuf::from(format!("index.{}", options.target.extension())),
        content: emit_index(program, &context, options.target),
    });

    files.sort_by(|lhs, rhs| lhs.relative_path.cmp(&rhs.relative_path));
    Ok(CodegenOutput {
        files,
        warnings: Vec::new(),
        diagnostics: Vec::new(),
    })
}

/// Builds and emits a host-neutral JavaScript program module directly from a program artifact.
pub fn emit_js_program_module(
    artifact: &ProgramArtifact,
    options: &JsProgramModuleOptions,
) -> Result<GeneratedJsProgramModule, CodegenError> {
    let program = build_codegen_program(artifact)?;
    emit_codegen_js_program_module(&program, options)
}

/// Emits a host-neutral JavaScript program module from an already-built codegen program.
pub fn emit_codegen_js_program_module(
    program: &CodegenProgram,
    options: &JsProgramModuleOptions,
) -> Result<GeneratedJsProgramModule, CodegenError> {
    validate_source_codegen_program(program)?;

    let context = EmitContext::new_js_program_module(program);
    let source_text = emit_js_program_module_source(program, &context, options);

    Ok(GeneratedJsProgramModule {
        source_text,
        logical_module_name: options.logical_module_name.clone(),
        runtime_import_specifier: options.runtime_import_specifier.clone(),
        runtime_abi: NX_JS_RUNTIME_ABI.to_string(),
        program_fingerprint: program.fingerprint,
        function_exports: collect_js_program_module_function_exports(program, &context),
        component_exports: collect_js_program_module_component_exports(program, &context),
    })
}

fn validate_source_codegen_program(program: &CodegenProgram) -> Result<(), CodegenError> {
    let mut diagnostics = Vec::new();
    for module in &program.modules {
        for declaration in &module.declarations {
            collect_source_codegen_diagnostics(module, declaration, &mut diagnostics);
        }
    }

    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(CodegenError::new(diagnostics))
    }
}

fn collect_source_codegen_diagnostics(
    module: &CodegenModule,
    declaration: &CodegenDeclaration,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match &declaration.kind {
        CodegenDeclarationKind::Function { body, .. } => {
            collect_expression_source_codegen_diagnostics(module, body, diagnostics);
        }
        CodegenDeclarationKind::Value { value, .. } => {
            collect_expression_source_codegen_diagnostics(module, value, diagnostics);
        }
        CodegenDeclarationKind::Record { fields } => {
            collect_record_field_source_codegen_diagnostics(module, fields, diagnostics);
        }
        CodegenDeclarationKind::Component(component) => {
            collect_component_field_source_codegen_diagnostics(
                module,
                &component.props,
                diagnostics,
            );
            collect_component_field_source_codegen_diagnostics(
                module,
                &component.state,
                diagnostics,
            );
            if let Some(body) = component.body.as_ref() {
                collect_expression_source_codegen_diagnostics(module, body, diagnostics);
            }
        }
        CodegenDeclarationKind::Union { cases } => {
            for case in cases {
                collect_record_field_source_codegen_diagnostics(module, &case.fields, diagnostics);
            }
        }
        CodegenDeclarationKind::Enum { .. }
        | CodegenDeclarationKind::TypeAlias
        | CodegenDeclarationKind::Unsupported(_) => {}
    }
}

fn collect_component_field_source_codegen_diagnostics(
    module: &CodegenModule,
    fields: &[CodegenComponentField],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for field in fields {
        if let Some(default) = field.default.as_ref() {
            collect_expression_source_codegen_diagnostics(module, default, diagnostics);
        }
    }
}

fn collect_record_field_source_codegen_diagnostics(
    module: &CodegenModule,
    fields: &[CodegenRecordField],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for field in fields {
        if let Some(default) = field.default.as_ref() {
            collect_expression_source_codegen_diagnostics(module, default, diagnostics);
        }
    }
}

fn collect_expression_source_codegen_diagnostics(
    module: &CodegenModule,
    expression: &CodegenExpression,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match &expression.kind {
        CodegenExpressionKind::Match { .. } => {
            diagnostics.push(source_codegen_unsupported_diagnostic(
                module,
                expression.span,
                "match expressions are not supported by executable source codegen yet",
            ));
        }
        CodegenExpressionKind::Binary { lhs, rhs, .. } => {
            collect_expression_source_codegen_diagnostics(module, lhs, diagnostics);
            collect_expression_source_codegen_diagnostics(module, rhs, diagnostics);
        }
        CodegenExpressionKind::Unary { expr, .. } => {
            collect_expression_source_codegen_diagnostics(module, expr, diagnostics);
        }
        CodegenExpressionKind::Call { callee, args } => {
            collect_expression_source_codegen_diagnostics(module, callee, diagnostics);
            for arg in args {
                collect_expression_source_codegen_diagnostics(module, arg, diagnostics);
            }
        }
        CodegenExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_expression_source_codegen_diagnostics(module, condition, diagnostics);
            collect_expression_source_codegen_diagnostics(module, then_branch, diagnostics);
            if let Some(else_branch) = else_branch {
                collect_expression_source_codegen_diagnostics(module, else_branch, diagnostics);
            }
        }
        CodegenExpressionKind::Let { value, body, .. } => {
            collect_expression_source_codegen_diagnostics(module, value, diagnostics);
            collect_expression_source_codegen_diagnostics(module, body, diagnostics);
        }
        CodegenExpressionKind::Block {
            statements,
            expression,
        } => {
            for statement in statements {
                match statement {
                    CodegenStatement::Let { init, .. } => {
                        collect_expression_source_codegen_diagnostics(module, init, diagnostics);
                    }
                    CodegenStatement::Expr(expr) => {
                        collect_expression_source_codegen_diagnostics(module, expr, diagnostics);
                    }
                }
            }
            if let Some(expression) = expression {
                collect_expression_source_codegen_diagnostics(module, expression, diagnostics);
            }
        }
        CodegenExpressionKind::Array(elements) => {
            for element in elements {
                collect_expression_source_codegen_diagnostics(module, element, diagnostics);
            }
        }
        CodegenExpressionKind::For { iterable, body, .. } => {
            collect_expression_source_codegen_diagnostics(module, iterable, diagnostics);
            collect_expression_source_codegen_diagnostics(module, body, diagnostics);
        }
        CodegenExpressionKind::Index { base, index } => {
            collect_expression_source_codegen_diagnostics(module, base, diagnostics);
            collect_expression_source_codegen_diagnostics(module, index, diagnostics);
        }
        CodegenExpressionKind::Member { base, .. } => {
            collect_expression_source_codegen_diagnostics(module, base, diagnostics);
        }
        CodegenExpressionKind::UnionCase {
            fields,
            properties,
            content,
            ..
        } => {
            collect_record_field_source_codegen_diagnostics(module, fields, diagnostics);
            for property in properties {
                collect_expression_source_codegen_diagnostics(module, &property.value, diagnostics);
            }
            for item in content {
                collect_expression_source_codegen_diagnostics(module, item, diagnostics);
            }
        }
        CodegenExpressionKind::Record {
            fields, properties, ..
        } => {
            collect_record_field_source_codegen_diagnostics(module, fields, diagnostics);
            for property in properties {
                collect_expression_source_codegen_diagnostics(module, &property.value, diagnostics);
            }
        }
        CodegenExpressionKind::ComponentDescriptor(descriptor) => {
            for property in &descriptor.properties {
                collect_expression_source_codegen_diagnostics(module, &property.value, diagnostics);
            }
            for item in &descriptor.content {
                collect_expression_source_codegen_diagnostics(module, item, diagnostics);
            }
        }
        CodegenExpressionKind::Element(element) => {
            for property in &element.properties {
                collect_expression_source_codegen_diagnostics(module, &property.value, diagnostics);
            }
            for item in &element.content {
                collect_expression_source_codegen_diagnostics(module, item, diagnostics);
            }
        }
        CodegenExpressionKind::Literal(_)
        | CodegenExpressionKind::Identifier { .. }
        | CodegenExpressionKind::EnumMember { .. }
        | CodegenExpressionKind::Unsupported(_) => {}
    }
}

fn source_codegen_unsupported_diagnostic(
    module: &CodegenModule,
    span: nx_diagnostics::TextSpan,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::error("codegen-unsupported-construct")
        .with_message(message)
        .with_label(Label::primary(module_diagnostic_identity(module), span))
        .build()
}

fn module_diagnostic_identity(module: &CodegenModule) -> String {
    match &module.provenance {
        CodegenModuleProvenance::SourceProvider { identity } => identity.clone(),
        CodegenModuleProvenance::Library { module_path, .. } => module_path.display().to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EmitMode {
    Files,
    JsProgramModule,
}

#[derive(Debug, Clone, Copy)]
enum ExportPolicy<'a> {
    All,
    Names(&'a FxHashSet<String>),
}

impl ExportPolicy<'_> {
    fn should_export(self, name: &str) -> bool {
        match self {
            Self::All => true,
            Self::Names(names) => names.contains(name),
        }
    }

    fn prefix(self, name: &str) -> &'static str {
        if self.should_export(name) {
            "export "
        } else {
            ""
        }
    }
}

struct EmitContext {
    mode: EmitMode,
    module_files: FxHashMap<RuntimeModuleId, String>,
    modules: FxHashMap<RuntimeModuleId, CodegenModule>,
    declaration_names: FxHashMap<ReferenceKey, String>,
    component_names: FxHashMap<ReferenceKey, ComponentGeneratedNames>,
    schema_declarations: FxHashMap<ReferenceKey, SchemaDeclaration>,
    import_aliases: FxHashMap<ReferenceKey, String>,
}

impl EmitContext {
    fn new(program: &CodegenProgram, target: CodegenTarget) -> Self {
        Self::with_mode(program, target, EmitMode::Files)
    }

    fn new_js_program_module(program: &CodegenProgram) -> Self {
        Self::with_mode(
            program,
            CodegenTarget::JavaScript,
            EmitMode::JsProgramModule,
        )
    }

    fn with_mode(program: &CodegenProgram, target: CodegenTarget, mode: EmitMode) -> Self {
        let mut module_files = FxHashMap::default();
        let mut used_files = FxHashSet::default();
        for module in &program.modules {
            let stem = module_file_stem(module);
            let mut file_name = format!("{}.{}", stem, target.extension());
            let mut disambiguator = 2;
            while !used_files.insert(file_name.clone()) {
                file_name = format!("{}_{}.{}", stem, disambiguator, target.extension());
                disambiguator += 1;
            }
            module_files.insert(module.id, file_name);
        }

        let modules = program
            .modules
            .iter()
            .map(|module| (module.id, module.clone()))
            .collect::<FxHashMap<_, _>>();

        let mut declaration_names = FxHashMap::default();
        let mut component_names = FxHashMap::default();
        let mut schema_declarations = FxHashMap::default();
        let mut global_used_names = FxHashSet::default();
        if mode == EmitMode::JsProgramModule {
            reserve_js_program_module_names(&mut global_used_names);
        }
        for module in &program.modules {
            let mut module_used_names = FxHashSet::default();
            let used_names = match mode {
                EmitMode::Files => &mut module_used_names,
                EmitMode::JsProgramModule => &mut global_used_names,
            };
            for declaration in &module.declarations {
                let name =
                    unique_identifier(safe_identifier(&declaration.reference.name), used_names);
                declaration_names.insert(ReferenceKey::new(&declaration.reference), name);
                if let CodegenDeclarationKind::Component(component) = &declaration.kind {
                    let generated_names = ComponentGeneratedNames::new(
                        declaration_names
                            .get(&ReferenceKey::new(&declaration.reference))
                            .expect("component declaration name should be planned"),
                        component,
                        used_names,
                    );
                    component_names
                        .insert(ReferenceKey::new(&declaration.reference), generated_names);
                }
                if let Some(schema) = SchemaDeclaration::from_declaration(declaration) {
                    schema_declarations.insert(ReferenceKey::new(&declaration.reference), schema);
                }
            }
        }

        let mut import_aliases = FxHashMap::default();
        if mode == EmitMode::Files {
            for module in &program.modules {
                let references = collect_module_import_references(module, target);
                for reference in references {
                    import_aliases
                        .entry(ReferenceKey::new(&reference))
                        .or_insert_with(|| {
                            format!(
                                "{}_{}",
                                module_prefix(reference.module_id),
                                safe_identifier(&reference.name)
                            )
                        });
                }
            }
        }

        Self {
            mode,
            module_files,
            modules,
            declaration_names,
            component_names,
            schema_declarations,
            import_aliases,
        }
    }

    fn module(&self, module_id: RuntimeModuleId) -> Option<&CodegenModule> {
        self.modules.get(&module_id)
    }

    fn module_file(&self, module_id: RuntimeModuleId) -> &str {
        self.module_files
            .get(&module_id)
            .map(String::as_str)
            .expect("module should have an output file")
    }

    fn reference_name(
        &self,
        current_module_id: RuntimeModuleId,
        reference: &CodegenReference,
    ) -> String {
        if reference.module_id == current_module_id || self.mode == EmitMode::JsProgramModule {
            self.declaration_name(reference)
        } else {
            self.import_aliases
                .get(&ReferenceKey::new(reference))
                .cloned()
                .unwrap_or_else(|| {
                    format!(
                        "{}_{}",
                        module_prefix(reference.module_id),
                        safe_identifier(&reference.name)
                    )
                })
        }
    }

    fn declaration_name(&self, reference: &CodegenReference) -> String {
        self.declaration_names
            .get(&ReferenceKey::new(reference))
            .cloned()
            .unwrap_or_else(|| safe_identifier(&reference.name))
    }

    fn component_names(&self, reference: &CodegenReference) -> &ComponentGeneratedNames {
        self.component_names
            .get(&ReferenceKey::new(reference))
            .expect("component should have generated names")
    }

    fn component(&self, reference: &CodegenReference) -> Option<&CodegenComponent> {
        self.module(reference.module_id)?
            .declarations
            .iter()
            .find(|declaration| {
                ReferenceKey::new(&declaration.reference) == ReferenceKey::new(reference)
            })
            .and_then(|declaration| match &declaration.kind {
                CodegenDeclarationKind::Component(component) => Some(component),
                _ => None,
            })
    }

    fn type_reference(
        &self,
        current_module_id: RuntimeModuleId,
        name: &str,
    ) -> Option<CodegenReference> {
        let module = self.module(current_module_id)?;
        module
            .imports
            .iter()
            .find(|reference| reference.name == name && is_type_reference_kind(reference.kind))
            .cloned()
            .or_else(|| {
                module
                    .declarations
                    .iter()
                    .find(|declaration| {
                        declaration.reference.name == name
                            && is_type_reference_kind(declaration.reference.kind)
                    })
                    .map(|declaration| declaration.reference.clone())
            })
    }

    fn generated_component_name(
        &self,
        current_module_id: RuntimeModuleId,
        reference: &CodegenReference,
        role: ComponentNameRole,
    ) -> String {
        let exported_name = self.component_names(reference).name(role);
        if reference.module_id == current_module_id || self.mode == EmitMode::JsProgramModule {
            exported_name.to_string()
        } else {
            format!("{}_{}", module_prefix(reference.module_id), exported_name)
        }
    }
}

fn reserve_js_program_module_names(used_names: &mut FxHashSet<String>) {
    used_names.insert(JS_PROGRAM_MODULE_MANIFEST_EXPORT_NAME.to_string());
    for name in JS_PROGRAM_MODULE_RESERVED_RUNTIME_NAMES {
        used_names.insert((*name).to_string());
    }
}

#[derive(Debug, Clone)]
struct ComponentGeneratedNames {
    props_type: String,
    resolved_props_type: String,
    element_type: Option<String>,
    state_type: Option<String>,
    schema_value: String,
    resolve_props_function: String,
    initial_state_function: Option<String>,
    render_function: Option<String>,
}

impl ComponentGeneratedNames {
    fn new(
        component_name: &str,
        component: &CodegenComponent,
        used_names: &mut FxHashSet<String>,
    ) -> Self {
        let props_type = unique_identifier(format!("{}Props", component_name), used_names);
        let resolved_props_type =
            unique_identifier(format!("{}ResolvedProps", component_name), used_names);
        let element_type = (!component.is_abstract)
            .then(|| unique_identifier(format!("{}Element", component_name), used_names));
        let state_type = (!component.state.is_empty())
            .then(|| unique_identifier(format!("{}State", component_name), used_names));
        let schema_value = unique_identifier(format!("{}Schema", component_name), used_names);
        let resolve_props_function =
            unique_identifier(format!("resolve{}Props", component_name), used_names);
        let initial_state_function = (!component.state.is_empty())
            .then(|| unique_identifier(format!("initial{}State", component_name), used_names));
        let render_function = (!component.is_external && !component.is_abstract)
            .then(|| unique_identifier(format!("render{}", component_name), used_names));

        Self {
            props_type,
            resolved_props_type,
            element_type,
            state_type,
            schema_value,
            resolve_props_function,
            initial_state_function,
            render_function,
        }
    }

    fn name(&self, role: ComponentNameRole) -> &str {
        match role {
            ComponentNameRole::Props => &self.props_type,
            ComponentNameRole::ResolvedProps => &self.resolved_props_type,
            ComponentNameRole::Element => self
                .element_type
                .as_deref()
                .expect("external component should have an element type"),
            ComponentNameRole::State => self
                .state_type
                .as_deref()
                .expect("stateful component should have a state type"),
            ComponentNameRole::Schema => &self.schema_value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ComponentNameRole {
    Props,
    ResolvedProps,
    Element,
    State,
    Schema,
}

#[derive(Debug, Clone)]
struct SchemaDeclaration {
    reference: CodegenReference,
    kind: SchemaDeclarationKind,
}

impl SchemaDeclaration {
    fn from_declaration(declaration: &CodegenDeclaration) -> Option<Self> {
        let kind = match &declaration.kind {
            CodegenDeclarationKind::Enum { members } => SchemaDeclarationKind::Enum {
                members: members.clone(),
            },
            CodegenDeclarationKind::Record { fields } => SchemaDeclarationKind::Record {
                fields: fields.clone(),
            },
            CodegenDeclarationKind::Union { cases } => SchemaDeclarationKind::Union {
                cases: cases.clone(),
            },
            CodegenDeclarationKind::Component(component) => SchemaDeclarationKind::Component {
                fields: component.props.clone(),
                is_abstract: component.is_abstract,
            },
            CodegenDeclarationKind::Function { .. }
            | CodegenDeclarationKind::Value { .. }
            | CodegenDeclarationKind::TypeAlias
            | CodegenDeclarationKind::Unsupported(_) => return None,
        };

        Some(Self {
            reference: declaration.reference.clone(),
            kind,
        })
    }
}

#[derive(Debug, Clone)]
enum SchemaDeclarationKind {
    Enum {
        members: Vec<String>,
    },
    Record {
        fields: Vec<CodegenRecordField>,
    },
    Union {
        cases: Vec<CodegenUnionCase>,
    },
    Component {
        fields: Vec<CodegenComponentField>,
        is_abstract: bool,
    },
}

fn emit_module(
    program: &CodegenProgram,
    module: &CodegenModule,
    context: &EmitContext,
    target: CodegenTarget,
) -> String {
    let mut out = String::new();
    out.push_str("// <auto-generated/>\n");
    out.push_str(&format!("// nx-fingerprint: {}\n", program.fingerprint));
    match &module.provenance {
        CodegenModuleProvenance::SourceProvider { identity } => {
            out.push_str(&format!("// nx-source: {}\n", identity));
        }
        CodegenModuleProvenance::Library { module_path, .. } => {
            out.push_str(&format!("// nx-source: {}\n", module_path.display()));
        }
    }
    out.push('\n');
    let runtime_helpers = collect_module_runtime_helpers(module, target);
    if !runtime_helpers.is_empty() {
        out.push_str(&format!(
            "import {{ {} }} from \"./{}\";\n",
            runtime_helpers.join(", "),
            runtime_import_file(target)
        ));
    }

    let value_imports = collect_module_value_references(module);
    for reference in &value_imports {
        if reference.kind == ResolvedItemKind::Component
            && context
                .component(reference)
                .is_some_and(|component| component.is_abstract)
        {
            continue;
        }
        let alias = context.reference_name(module.id, reference);
        let source_file = context.module_file(reference.module_id);
        if reference.kind == ResolvedItemKind::Component
            && context
                .component(reference)
                .is_some_and(|component| !component.is_abstract)
        {
            out.push_str(&format!(
                "import {{ {} as {}, {} as {} }} from \"./{}\";\n",
                context.declaration_name(reference),
                alias,
                context
                    .component_names(reference)
                    .name(ComponentNameRole::Schema),
                context.generated_component_name(module.id, reference, ComponentNameRole::Schema),
                import_file(source_file, target)
            ));
        } else {
            out.push_str(&format!(
                "import {{ {} as {} }} from \"./{}\";\n",
                context.declaration_name(reference),
                alias,
                import_file(source_file, target)
            ));
        }
    }
    if target.is_typescript() {
        let value_keys = value_imports
            .iter()
            .map(ReferenceKey::new)
            .collect::<FxHashSet<_>>();
        for reference in collect_module_type_references(module) {
            if reference.kind == ResolvedItemKind::Component {
                let Some(component) = context.component(&reference) else {
                    continue;
                };
                if component.is_abstract || reference.module_id == module.id {
                    continue;
                }
                let alias = context.generated_component_name(
                    module.id,
                    &reference,
                    ComponentNameRole::Element,
                );
                let source_file = context.module_file(reference.module_id);
                out.push_str(&format!(
                    "import type {{ {} as {} }} from \"./{}\";\n",
                    context
                        .component_names(&reference)
                        .name(ComponentNameRole::Element),
                    alias,
                    import_file(source_file, target)
                ));
                continue;
            }
            if value_keys.contains(&ReferenceKey::new(&reference)) {
                continue;
            }
            let alias = context.reference_name(module.id, &reference);
            let source_file = context.module_file(reference.module_id);
            out.push_str(&format!(
                "import type {{ {} as {} }} from \"./{}\";\n",
                context.declaration_name(&reference),
                alias,
                import_file(source_file, target)
            ));
        }
    }
    out.push('\n');

    for declaration in &module.declarations {
        emit_declaration(
            module,
            declaration,
            context,
            target,
            ExportPolicy::All,
            &mut out,
        );
        out.push('\n');
    }

    out
}

fn emit_js_program_module_source(
    program: &CodegenProgram,
    context: &EmitContext,
    options: &JsProgramModuleOptions,
) -> String {
    let mut out = String::new();
    out.push_str("// <auto-generated/>\n");
    out.push_str(&format!(
        "// nx-program-module: {}\n",
        options.logical_module_name
    ));
    out.push_str(&format!("// nx-fingerprint: {}\n", program.fingerprint));
    out.push_str(&format!("// nx-runtime-abi: {}\n\n", NX_JS_RUNTIME_ABI));

    let runtime_helpers = collect_js_program_module_runtime_helpers(program);
    if !runtime_helpers.is_empty() {
        out.push_str(&format!(
            "import {{ {} }} from {};\n\n",
            runtime_helpers.join(", "),
            js_string(&options.runtime_import_specifier)
        ));
    }

    let exported_names = collect_js_program_module_public_names(program, context);
    let export_policy = ExportPolicy::Names(&exported_names);

    for module in js_program_module_ordered_modules(program) {
        out.push_str(&format!("// nx-source: {}\n", module_source_label(module)));
        for declaration in &module.declarations {
            emit_declaration(
                module,
                declaration,
                context,
                CodegenTarget::JavaScript,
                export_policy,
                &mut out,
            );
            out.push('\n');
        }
    }

    emit_js_program_module_manifest(program, context, options, &mut out);
    out
}

fn js_program_module_ordered_modules(program: &CodegenProgram) -> Vec<&CodegenModule> {
    let mut ordered_ids = Vec::new();
    let mut visited = FxHashSet::default();
    let mut visiting = FxHashSet::default();
    for module in &program.modules {
        push_module_after_dependencies(
            program,
            module.id,
            &mut ordered_ids,
            &mut visited,
            &mut visiting,
        );
    }

    ordered_ids
        .into_iter()
        .filter_map(|module_id| program.module(module_id))
        .collect()
}

fn push_module_after_dependencies(
    program: &CodegenProgram,
    module_id: RuntimeModuleId,
    ordered_ids: &mut Vec<RuntimeModuleId>,
    visited: &mut FxHashSet<RuntimeModuleId>,
    visiting: &mut FxHashSet<RuntimeModuleId>,
) {
    if visited.contains(&module_id) || !visiting.insert(module_id) {
        return;
    }

    if let Some(module) = program.module(module_id) {
        let mut dependency_ids = module
            .imports
            .iter()
            .map(|reference| reference.module_id)
            .filter(|dependency_id| *dependency_id != module_id)
            .collect::<Vec<_>>();
        dependency_ids.sort_by_key(RuntimeModuleId::as_u32);
        dependency_ids.dedup();
        for dependency_id in dependency_ids {
            push_module_after_dependencies(program, dependency_id, ordered_ids, visited, visiting);
        }
    }

    visiting.remove(&module_id);
    visited.insert(module_id);
    ordered_ids.push(module_id);
}

fn emit_js_program_module_manifest(
    program: &CodegenProgram,
    context: &EmitContext,
    options: &JsProgramModuleOptions,
    out: &mut String,
) {
    out.push_str(&format!(
        "export const {} = Object.freeze({{\n",
        JS_PROGRAM_MODULE_MANIFEST_EXPORT_NAME
    ));
    out.push_str(&format!(
        "  logicalModuleName: {},\n",
        js_string(&options.logical_module_name)
    ));
    out.push_str(&format!(
        "  runtimeImportSpecifier: {},\n",
        js_string(&options.runtime_import_specifier)
    ));
    out.push_str(&format!(
        "  runtimeAbi: {},\n",
        js_string(NX_JS_RUNTIME_ABI)
    ));
    out.push_str(&format!(
        "  programFingerprint: {},\n",
        js_string(&program.fingerprint.to_string())
    ));
    out.push_str("  functionExports: Object.freeze([\n");
    for export in collect_js_program_module_function_exports(program, context) {
        out.push_str(&format!(
            "    Object.freeze({{ entrypointName: {}, exportName: {} }}),\n",
            js_string(&export.entrypoint_name),
            js_string(&export.export_name)
        ));
    }
    out.push_str("  ]),\n");
    out.push_str("  componentExports: Object.freeze([\n");
    for export in collect_js_program_module_component_exports(program, context) {
        out.push_str(&format!(
            "    Object.freeze({{ componentName: {}, componentExportName: {}, schemaExportName: {}",
            js_string(&export.component_name),
            js_string(&export.component_export_name),
            js_string(&export.schema_export_name)
        ));
        if let Some(initial_state_export_name) = export.initial_state_export_name.as_deref() {
            out.push_str(&format!(
                ", initialStateExportName: {}",
                js_string(initial_state_export_name)
            ));
        }
        if let Some(render_export_name) = export.render_export_name.as_deref() {
            out.push_str(&format!(
                ", renderExportName: {}",
                js_string(render_export_name)
            ));
        }
        out.push_str(" }),\n");
    }
    out.push_str("  ]),\n");
    out.push_str("});\n");
}

fn collect_js_program_module_public_names(
    program: &CodegenProgram,
    context: &EmitContext,
) -> FxHashSet<String> {
    let mut names = FxHashSet::default();
    names.insert(JS_PROGRAM_MODULE_MANIFEST_EXPORT_NAME.to_string());
    for export in collect_js_program_module_function_exports(program, context) {
        names.insert(export.export_name);
    }
    for export in collect_js_program_module_component_exports(program, context) {
        names.insert(export.component_export_name);
        names.insert(export.schema_export_name);
        if let Some(initial_state_export_name) = export.initial_state_export_name {
            names.insert(initial_state_export_name);
        }
        if let Some(render_export_name) = export.render_export_name {
            names.insert(render_export_name);
        }
    }
    names
}

fn module_source_label(module: &CodegenModule) -> String {
    match &module.provenance {
        CodegenModuleProvenance::SourceProvider { identity } => identity.clone(),
        CodegenModuleProvenance::Library { module_path, .. } => module_path.display().to_string(),
    }
}

fn collect_js_program_module_function_exports(
    program: &CodegenProgram,
    context: &EmitContext,
) -> Vec<GeneratedJsProgramModuleFunctionExport> {
    program
        .entrypoints
        .iter()
        .map(|entrypoint| GeneratedJsProgramModuleFunctionExport {
            entrypoint_name: entrypoint.name.clone(),
            export_name: context.declaration_name(&entrypoint.reference),
        })
        .collect()
}

fn collect_js_program_module_component_exports(
    program: &CodegenProgram,
    context: &EmitContext,
) -> Vec<GeneratedJsProgramModuleComponentExport> {
    let mut exports = Vec::new();
    for module in &program.modules {
        for declaration in &module.declarations {
            let CodegenDeclarationKind::Component(component) = &declaration.kind else {
                continue;
            };
            if component.is_abstract {
                continue;
            }

            let names = context.component_names(&declaration.reference);
            let has_state_exports = !component.is_external && !component.state.is_empty();
            exports.push(GeneratedJsProgramModuleComponentExport {
                component_name: declaration.reference.name.clone(),
                component_export_name: context.declaration_name(&declaration.reference),
                schema_export_name: names.name(ComponentNameRole::Schema).to_string(),
                initial_state_export_name: has_state_exports.then(|| {
                    names
                        .initial_state_function
                        .as_deref()
                        .expect("stateful component should have an initial-state helper")
                        .to_string()
                }),
                render_export_name: has_state_exports.then(|| {
                    names
                        .render_function
                        .as_deref()
                        .expect("stateful component should have a render helper")
                        .to_string()
                }),
            });
        }
    }
    exports
}

fn emit_declaration(
    module: &CodegenModule,
    declaration: &CodegenDeclaration,
    context: &EmitContext,
    target: CodegenTarget,
    export_policy: ExportPolicy<'_>,
    out: &mut String,
) {
    let name = context.declaration_name(&declaration.reference);
    let export_prefix = export_policy.prefix(&name);
    match &declaration.kind {
        CodegenDeclarationKind::Function {
            params,
            body,
            return_type,
        } => {
            let params = params
                .iter()
                .map(|param| {
                    if target.is_typescript() {
                        format!(
                            "{}: {}",
                            safe_identifier(&param.name),
                            emit_type_ref(module.id, &param.ty, module, context)
                        )
                    } else {
                        safe_identifier(&param.name)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            if target.is_typescript() {
                out.push_str(&format!(
                    "{}function {}({}): {} {{\n",
                    export_prefix,
                    name,
                    params,
                    return_type
                        .as_ref()
                        .map(|ty| emit_type(module.id, ty, module, context))
                        .unwrap_or_else(|| "unknown".to_string())
                ));
            } else {
                out.push_str(&format!(
                    "{}function {}({}) {{\n",
                    export_prefix, name, params
                ));
            }
            out.push_str(&format!(
                "  return {};\n",
                emit_expression(module.id, body, context)
            ));
            out.push_str("}\n");
        }
        CodegenDeclarationKind::Value { value, ty } => {
            if target.is_typescript() {
                out.push_str(&format!(
                    "{}const {}: {} = {};\n",
                    export_prefix,
                    name,
                    ty.as_ref()
                        .map(|ty| emit_type(module.id, ty, module, context))
                        .unwrap_or_else(|| "unknown".to_string()),
                    emit_expression(module.id, value, context)
                ));
            } else {
                out.push_str(&format!(
                    "{}const {} = {};\n",
                    export_prefix,
                    name,
                    emit_expression(module.id, value, context)
                ));
            }
        }
        CodegenDeclarationKind::Enum { members } => {
            if target.is_typescript() {
                out.push_str(&format!("{}const {} = {{\n", export_prefix, name));
                for member in members {
                    out.push_str(&format!(
                        "  {}: {},\n",
                        safe_object_key(member),
                        js_string(member)
                    ));
                }
                out.push_str("} as const;\n\n");
                out.push_str(&format!(
                    "{}type {} = typeof {}[keyof typeof {}];\n",
                    export_prefix, name, name, name
                ));
            } else {
                out.push_str(&format!(
                    "{}const {} = Object.freeze({{\n",
                    export_prefix, name
                ));
                for member in members {
                    out.push_str(&format!(
                        "  {}: {},\n",
                        safe_object_key(member),
                        js_string(member)
                    ));
                }
                out.push_str("});\n");
            }
        }
        CodegenDeclarationKind::Record { fields } => {
            if target.is_typescript() {
                emit_record_type(
                    &name,
                    &declaration.reference.name,
                    fields,
                    module,
                    context,
                    export_policy,
                    out,
                );
                out.push('\n');
            }
        }
        CodegenDeclarationKind::Component(component) => {
            emit_component_declaration(
                module,
                &declaration.reference,
                &name,
                &declaration.reference.name,
                component,
                context,
                target,
                export_policy,
                out,
            );
        }
        CodegenDeclarationKind::Union { cases } => {
            if target.is_typescript() {
                emit_union_type(
                    &name,
                    &declaration.reference.name,
                    cases,
                    module,
                    context,
                    export_policy,
                    out,
                );
                out.push('\n');
            }
        }
        CodegenDeclarationKind::TypeAlias => {}
        CodegenDeclarationKind::Unsupported(unsupported) => {
            out.push_str(&format!(
                "{}const {} = nxRuntimeError({});\n",
                export_prefix,
                name,
                js_string(&unsupported.message)
            ));
        }
    }
}

fn emit_record_type(
    name: &str,
    runtime_name: &str,
    fields: &[CodegenRecordField],
    module: &CodegenModule,
    context: &EmitContext,
    export_policy: ExportPolicy<'_>,
    out: &mut String,
) {
    out.push_str(&format!(
        "{}type {} = {{\n",
        export_policy.prefix(name),
        name
    ));
    out.push_str(&format!("  readonly $type: {};\n", js_string(runtime_name)));
    for field in fields {
        let optional = if field.is_required { "" } else { "?" };
        out.push_str(&format!(
            "  readonly {}{}: {};\n",
            safe_object_key(&field.name),
            optional,
            emit_type_ref(module.id, &field.ty, module, context)
        ));
    }
    out.push_str("};\n");
}

fn emit_union_type(
    name: &str,
    runtime_name: &str,
    cases: &[CodegenUnionCase],
    module: &CodegenModule,
    context: &EmitContext,
    export_policy: ExportPolicy<'_>,
    out: &mut String,
) {
    let case_type_names = cases
        .iter()
        .map(|case| union_case_type_name(name, &case.name))
        .collect::<Vec<_>>();
    for (case, case_type_name) in cases.iter().zip(case_type_names.iter()) {
        emit_record_type(
            case_type_name,
            &format!("{}.{}", runtime_name, case.name),
            &case.fields,
            module,
            context,
            export_policy,
            out,
        );
        out.push('\n');
    }
    let union = if case_type_names.is_empty() {
        "never".to_string()
    } else {
        case_type_names.join(" | ")
    };
    out.push_str(&format!(
        "{}type {} = {};\n",
        export_policy.prefix(name),
        name,
        union
    ));
}

fn emit_component_declaration(
    module: &CodegenModule,
    reference: &CodegenReference,
    name: &str,
    runtime_name: &str,
    component: &CodegenComponent,
    context: &EmitContext,
    target: CodegenTarget,
    export_policy: ExportPolicy<'_>,
    out: &mut String,
) {
    if target.is_typescript() {
        emit_component_props_type(module, reference, component, context, export_policy, out);
        out.push('\n');
        emit_component_resolved_props_type(module, reference, component, context, out);
        out.push('\n');
        if !component.is_abstract {
            emit_component_element_type(
                module,
                reference,
                runtime_name,
                component,
                context,
                export_policy,
                out,
            );
            out.push('\n');
        }
        if !component.state.is_empty() {
            emit_component_state_type(module, reference, component, context, export_policy, out);
            out.push('\n');
        }
    }

    if component.is_abstract {
        return;
    }

    emit_component_props_resolver(module, reference, component, context, target, out);
    out.push('\n');

    if component.is_external {
        emit_component_descriptor_factory(
            module,
            reference,
            name,
            runtime_name,
            component,
            context,
            target,
            export_policy,
            out,
        );
        out.push('\n');
        emit_external_component_schema(
            module,
            reference,
            runtime_name,
            component,
            context,
            target,
            export_policy,
            out,
        );
        return;
    }

    if !component.state.is_empty() {
        emit_component_initial_state(
            module,
            reference,
            runtime_name,
            component,
            context,
            target,
            export_policy,
            out,
        );
        out.push('\n');
    }
    emit_component_descriptor_factory(
        module,
        reference,
        name,
        runtime_name,
        component,
        context,
        target,
        export_policy,
        out,
    );
    out.push('\n');
    emit_normal_component_render_function(
        module,
        reference,
        component,
        context,
        target,
        export_policy,
        out,
    );
    out.push('\n');
    emit_normal_component_schema(
        module,
        reference,
        runtime_name,
        component,
        context,
        target,
        export_policy,
        out,
    );
}

fn emit_component_props_type(
    module: &CodegenModule,
    reference: &CodegenReference,
    component: &CodegenComponent,
    context: &EmitContext,
    export_policy: ExportPolicy<'_>,
    out: &mut String,
) {
    let props_type = context
        .component_names(reference)
        .name(ComponentNameRole::Props);
    let export_prefix = export_policy.prefix(props_type);
    if component.props.is_empty() {
        out.push_str(&format!(
            "{}type {} = Record<string, never>;\n",
            export_prefix, props_type
        ));
        return;
    }

    out.push_str(&format!("{}type {} = {{\n", export_prefix, props_type));
    for field in &component.props {
        let optional = if field.is_required { "" } else { "?" };
        out.push_str(&format!(
            "  {}{}: {};\n",
            safe_object_key(&field.name),
            optional,
            emit_type_ref(module.id, &field.ty, module, context)
        ));
    }
    out.push_str("};\n");
}

fn emit_component_resolved_props_type(
    module: &CodegenModule,
    reference: &CodegenReference,
    component: &CodegenComponent,
    context: &EmitContext,
    out: &mut String,
) {
    let resolved_props_type = context
        .component_names(reference)
        .name(ComponentNameRole::ResolvedProps);
    if component.props.is_empty() {
        out.push_str(&format!(
            "type {} = Record<string, never>;\n",
            resolved_props_type
        ));
        return;
    }

    out.push_str(&format!("type {} = {{\n", resolved_props_type));
    for field in &component.props {
        out.push_str(&format!(
            "  readonly {}: {};\n",
            safe_object_key(&field.name),
            emit_type_ref(module.id, &field.ty, module, context)
        ));
    }
    out.push_str("};\n");
}

fn emit_component_element_type(
    module: &CodegenModule,
    reference: &CodegenReference,
    runtime_name: &str,
    component: &CodegenComponent,
    context: &EmitContext,
    export_policy: ExportPolicy<'_>,
    out: &mut String,
) {
    let element_type = context
        .component_names(reference)
        .name(ComponentNameRole::Element);
    out.push_str(&format!(
        "{}type {} = {{\n",
        export_policy.prefix(element_type),
        element_type
    ));
    out.push_str(&format!("  readonly $type: {};\n", js_string(runtime_name)));
    for field in &component.props {
        out.push_str(&format!(
            "  readonly {}: {};\n",
            safe_object_key(&field.name),
            emit_type_ref(module.id, &field.ty, module, context)
        ));
    }
    out.push_str("};\n");
}

fn emit_component_state_type(
    module: &CodegenModule,
    reference: &CodegenReference,
    component: &CodegenComponent,
    context: &EmitContext,
    export_policy: ExportPolicy<'_>,
    out: &mut String,
) {
    let state_type = context
        .component_names(reference)
        .name(ComponentNameRole::State);
    out.push_str(&format!(
        "{}type {} = {{\n",
        export_policy.prefix(state_type),
        state_type
    ));
    for field in &component.state {
        out.push_str(&format!(
            "  readonly {}: {};\n",
            safe_object_key(&field.name),
            emit_type_ref(module.id, &field.ty, module, context)
        ));
    }
    out.push_str("};\n");
}

fn emit_component_props_resolver(
    module: &CodegenModule,
    reference: &CodegenReference,
    component: &CodegenComponent,
    context: &EmitContext,
    target: CodegenTarget,
    out: &mut String,
) {
    let names = context.component_names(reference);
    let default_value = component_props_can_default(component)
        .then_some(" = {}")
        .unwrap_or("");
    if target.is_typescript() {
        out.push_str(&format!(
            "function {}(props: {}{}): {} {{\n",
            names.resolve_props_function,
            names.name(ComponentNameRole::Props),
            default_value,
            names.name(ComponentNameRole::ResolvedProps)
        ));
    } else {
        out.push_str(&format!(
            "function {}(props{}) {{\n",
            names.resolve_props_function, default_value
        ));
    }

    emit_typed_field_initializers(
        module.id,
        &component.props,
        "props",
        &FxHashSet::default(),
        context,
        out,
    );
    emit_field_object_return(&component.props, "  ", out);
    out.push_str("}\n");
}

fn emit_component_descriptor_factory(
    _module: &CodegenModule,
    reference: &CodegenReference,
    name: &str,
    runtime_name: &str,
    component: &CodegenComponent,
    context: &EmitContext,
    target: CodegenTarget,
    export_policy: ExportPolicy<'_>,
    out: &mut String,
) {
    let names = context.component_names(reference);
    let default_value = component_props_can_default(component)
        .then_some(" = {}")
        .unwrap_or("");
    let export_prefix = export_policy.prefix(name);
    if target.is_typescript() {
        out.push_str(&format!(
            "{}function {}(props: {}{}): {} {{\n",
            export_prefix,
            name,
            names.name(ComponentNameRole::Props),
            default_value,
            names.name(ComponentNameRole::Element)
        ));
    } else {
        out.push_str(&format!(
            "{}function {}(props{}) {{\n",
            export_prefix, name, default_value
        ));
    }
    out.push_str(&format!(
        "  const resolvedProps = {}(props);\n",
        names.resolve_props_function
    ));
    out.push_str(&format!("  return {{ $type: {}", js_string(runtime_name)));
    for field in &component.props {
        out.push_str(&format!(
            ", {}: resolvedProps{}",
            safe_object_key(&field.name),
            member_access(&field.name)
        ));
    }
    out.push_str(" };\n");
    out.push_str("}\n");
}

fn emit_component_initial_state(
    module: &CodegenModule,
    reference: &CodegenReference,
    runtime_name: &str,
    component: &CodegenComponent,
    context: &EmitContext,
    target: CodegenTarget,
    export_policy: ExportPolicy<'_>,
    out: &mut String,
) {
    let names = context.component_names(reference);
    let function_name = names
        .initial_state_function
        .as_deref()
        .expect("stateful component should have an initial state helper");
    let default_value = component_props_can_default(component)
        .then_some(" = {}")
        .unwrap_or("");
    let export_prefix = export_policy.prefix(function_name);
    if target.is_typescript() {
        out.push_str(&format!(
            "{}function {}(props: {}{}): {} {{\n",
            export_prefix,
            function_name,
            names.name(ComponentNameRole::Props),
            default_value,
            names.name(ComponentNameRole::State)
        ));
    } else {
        out.push_str(&format!(
            "{}function {}(props{}) {{\n",
            export_prefix, function_name, default_value
        ));
    }
    let mut predeclared = FxHashSet::default();
    for prop in &component.props {
        predeclared.insert(safe_identifier(&prop.name));
    }

    let mut generated_locals = predeclared.clone();
    let resolved_props_name = unique_identifier("resolvedProps".to_string(), &mut generated_locals);
    out.push_str(&format!(
        "  const {} = {}(props);\n",
        resolved_props_name, names.resolve_props_function
    ));

    for prop in &component.props {
        out.push_str(&format!(
            "  let {} = {}{};\n",
            safe_identifier(&prop.name),
            resolved_props_name,
            member_access(&prop.name)
        ));
    }
    emit_initial_state_field_initializers(
        module.id,
        runtime_name,
        component,
        &predeclared,
        context,
        out,
    );
    emit_field_object_return(&component.state, "  ", out);
    out.push_str("}\n");
}

fn emit_normal_component_render_function(
    module: &CodegenModule,
    reference: &CodegenReference,
    component: &CodegenComponent,
    context: &EmitContext,
    target: CodegenTarget,
    export_policy: ExportPolicy<'_>,
    out: &mut String,
) {
    let names = context.component_names(reference);
    let render_function = names
        .render_function
        .as_deref()
        .expect("normal component should have a render helper");
    let export_prefix =
        if component.state.is_empty() || !export_policy.should_export(render_function) {
            ""
        } else {
            "export "
        };
    let return_type = component_render_return_type(module, component, context);
    if target.is_typescript() {
        if component.state.is_empty() {
            out.push_str(&format!(
                "{}function {}(props: {}): {} {{\n",
                export_prefix,
                render_function,
                names.name(ComponentNameRole::ResolvedProps),
                return_type
            ));
        } else {
            out.push_str(&format!(
                "{}function {}(props: {}, state: {}): {} {{\n",
                export_prefix,
                render_function,
                names.name(ComponentNameRole::ResolvedProps),
                names.name(ComponentNameRole::State),
                return_type
            ));
        }
    } else if component.state.is_empty() {
        out.push_str(&format!(
            "{}function {}(props) {{\n",
            export_prefix, render_function
        ));
    } else {
        out.push_str(&format!(
            "{}function {}(props, state) {{\n",
            export_prefix, render_function
        ));
    }

    let mut predeclared = FxHashSet::default();
    for prop in &component.props {
        predeclared.insert(safe_identifier(&prop.name));
        out.push_str(&format!(
            "  let {} = props{};\n",
            safe_identifier(&prop.name),
            member_access(&prop.name)
        ));
    }
    for field in &component.state {
        let local_name = safe_identifier(&field.name);
        if predeclared.contains(&local_name) {
            out.push_str(&format!(
                "  {} = state{};\n",
                local_name,
                member_access(&field.name)
            ));
        } else {
            out.push_str(&format!(
                "  const {} = state{};\n",
                local_name,
                member_access(&field.name)
            ));
        }
    }
    match component.body.as_ref() {
        Some(body) => {
            out.push_str(&format!(
                "  return {};\n",
                emit_expression(module.id, body, context)
            ));
        }
        None => {
            out.push_str("  return null;\n");
        }
    }
    out.push_str("}\n");
}

fn emit_external_component_schema(
    module: &CodegenModule,
    reference: &CodegenReference,
    runtime_name: &str,
    component: &CodegenComponent,
    context: &EmitContext,
    target: CodegenTarget,
    export_policy: ExportPolicy<'_>,
    out: &mut String,
) {
    let names = context.component_names(reference);
    let schema_name = names.name(ComponentNameRole::Schema);
    let export_prefix = export_policy.prefix(schema_name);
    if target.is_typescript() {
        out.push_str(&format!(
            "{}const {} = nxExternalComponentSchema<{}, {}>({{\n",
            export_prefix,
            schema_name,
            names.name(ComponentNameRole::Props),
            names.name(ComponentNameRole::Element)
        ));
    } else {
        out.push_str(&format!(
            "{}const {} = nxExternalComponentSchema({{\n",
            export_prefix, schema_name
        ));
    }
    out.push_str(&format!("  name: {},\n", js_string(runtime_name)));
    out.push_str(&format!(
        "  props: {},\n",
        emit_component_boundary_schema(module.id, &component.props, false, context)
    ));
    out.push_str(&format!(
        "  create: {},\n",
        context.declaration_name(reference)
    ));
    out.push_str("});\n");
}

fn emit_normal_component_schema(
    module: &CodegenModule,
    reference: &CodegenReference,
    runtime_name: &str,
    component: &CodegenComponent,
    context: &EmitContext,
    target: CodegenTarget,
    export_policy: ExportPolicy<'_>,
    out: &mut String,
) {
    let names = context.component_names(reference);
    let schema_name = names.name(ComponentNameRole::Schema);
    let export_prefix = export_policy.prefix(schema_name);
    let state_type = names
        .state_type
        .as_deref()
        .unwrap_or("Record<string, never>");
    let return_type = component_render_return_type(module, component, context);
    if target.is_typescript() {
        out.push_str(&format!(
            "{}const {} = nxComponentSchema<{}, {}, {}>({{\n",
            export_prefix,
            schema_name,
            names.name(ComponentNameRole::Props),
            state_type,
            return_type
        ));
    } else {
        out.push_str(&format!(
            "{}const {} = nxComponentSchema({{\n",
            export_prefix, schema_name
        ));
    }
    out.push_str(&format!("  name: {},\n", js_string(runtime_name)));
    out.push_str(&format!(
        "  props: {},\n",
        emit_component_boundary_schema(module.id, &component.props, false, context)
    ));
    if !component.state.is_empty() {
        out.push_str(&format!(
            "  state: {},\n",
            emit_component_boundary_schema(module.id, &component.state, true, context)
        ));
    }
    if component.state.is_empty() {
        let render_function = names
            .render_function
            .as_deref()
            .expect("normal component should have a render helper");
        out.push_str(&format!(
            "  initialize: (props) => {{\n    const resolvedProps = {}(props);\n    return {{ rendered: {}(resolvedProps), state: {{}} }};\n  }},\n",
            names.resolve_props_function, render_function
        ));
        out.push_str(&format!(
            "  evaluate: (props) => {{\n    const resolvedProps = {}(props);\n    return {}(resolvedProps);\n  }},\n",
            names.resolve_props_function, render_function
        ));
    } else {
        let initial_state_function = names
            .initial_state_function
            .as_deref()
            .expect("stateful component should have an initial state helper");
        let render_function = names
            .render_function
            .as_deref()
            .expect("normal component should have a render helper");
        out.push_str("  initialize: (props) => {\n");
        out.push_str(&format!(
            "    const resolvedProps = {}(props);\n",
            names.resolve_props_function
        ));
        out.push_str(&format!(
            "    const initialState = {}(resolvedProps);\n",
            initial_state_function
        ));
        out.push_str(&format!(
            "    return {{ rendered: {}(resolvedProps, initialState), state: initialState }};\n",
            render_function
        ));
        out.push_str("  },\n");
        out.push_str("  evaluate: (props, state) => {\n");
        out.push_str(&format!(
            "    const resolvedProps = {}(props);\n",
            names.resolve_props_function
        ));
        out.push_str(&format!(
            "    const resolvedState = state ?? {}(resolvedProps);\n",
            initial_state_function
        ));
        out.push_str(&format!(
            "    return {}(resolvedProps, resolvedState);\n",
            render_function
        ));
        out.push_str("  },\n");
    }
    out.push_str("});\n");
}

fn component_props_can_default(component: &CodegenComponent) -> bool {
    component.props.iter().all(|field| !field.is_required)
}

fn component_render_return_type(
    module: &CodegenModule,
    component: &CodegenComponent,
    context: &EmitContext,
) -> String {
    let mut visiting = FxHashSet::default();
    component
        .body
        .as_ref()
        .and_then(|body| {
            expression_render_return_type(module.id, body, module, context, &mut visiting)
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn expression_render_return_type(
    current_module_id: RuntimeModuleId,
    expression: &CodegenExpression,
    module: &CodegenModule,
    context: &EmitContext,
    visiting: &mut FxHashSet<ReferenceKey>,
) -> Option<String> {
    if let Some(ty) = expression
        .ty
        .as_ref()
        .filter(|ty| !matches!(ty, Type::Unknown | Type::Error))
        .and_then(|ty| emit_known_type(current_module_id, ty, module, context))
    {
        return Some(ty);
    }

    match &expression.kind {
        CodegenExpressionKind::ComponentDescriptor(descriptor) => {
            component_descriptor_render_return_type(
                current_module_id,
                descriptor,
                context,
                visiting,
            )
        }
        CodegenExpressionKind::Call { .. } => referenced_function_return_type(expression, context)
            .as_ref()
            .and_then(|ty| emit_known_type(current_module_id, ty, module, context)),
        CodegenExpressionKind::If {
            then_branch,
            else_branch: Some(else_branch),
            ..
        } => {
            let then_type = expression_render_return_type(
                current_module_id,
                then_branch,
                module,
                context,
                visiting,
            )?;
            let else_type = expression_render_return_type(
                current_module_id,
                else_branch,
                module,
                context,
                visiting,
            )?;
            if then_type == else_type {
                Some(then_type)
            } else {
                None
            }
        }
        CodegenExpressionKind::Match {
            arms,
            else_branch: Some(else_branch),
            ..
        } => {
            let mut expected = None;
            for arm in arms {
                let branch_type = expression_render_return_type(
                    current_module_id,
                    &arm.body,
                    module,
                    context,
                    visiting,
                )?;
                match &expected {
                    Some(expected) if expected != &branch_type => return None,
                    None => expected = Some(branch_type),
                    _ => {}
                }
            }
            let else_type = expression_render_return_type(
                current_module_id,
                else_branch,
                module,
                context,
                visiting,
            )?;
            match expected {
                Some(expected) if expected == else_type => Some(expected),
                None => Some(else_type),
                _ => None,
            }
        }
        CodegenExpressionKind::Let { body, .. } => {
            expression_render_return_type(current_module_id, body, module, context, visiting)
        }
        CodegenExpressionKind::Block {
            expression: Some(expression),
            ..
        } => {
            expression_render_return_type(current_module_id, expression, module, context, visiting)
        }
        _ => None,
    }
}

fn emit_known_type(
    current_module_id: RuntimeModuleId,
    ty: &Type,
    module: &CodegenModule,
    context: &EmitContext,
) -> Option<String> {
    let emitted = emit_type(current_module_id, ty, module, context);
    (emitted != "unknown").then_some(emitted)
}

fn component_descriptor_render_return_type(
    current_module_id: RuntimeModuleId,
    descriptor: &CodegenComponentDescriptor,
    context: &EmitContext,
    _visiting: &mut FxHashSet<ReferenceKey>,
) -> Option<String> {
    match descriptor.target_kind {
        CodegenComponentTargetKind::External | CodegenComponentTargetKind::Normal => {
            Some(context.generated_component_name(
                current_module_id,
                &descriptor.component,
                ComponentNameRole::Element,
            ))
        }
    }
}

fn referenced_function_return_type(
    expression: &CodegenExpression,
    context: &EmitContext,
) -> Option<Type> {
    let CodegenExpressionKind::Call { callee, .. } = &expression.kind else {
        return None;
    };
    let CodegenExpressionKind::Identifier {
        reference: Some(reference),
        ..
    } = &callee.kind
    else {
        return None;
    };
    let module = context.module(reference.module_id)?;
    module
        .declarations
        .iter()
        .find(|declaration| {
            ReferenceKey::new(&declaration.reference) == ReferenceKey::new(reference)
        })
        .and_then(|declaration| match &declaration.kind {
            CodegenDeclarationKind::Function {
                return_type: Some(return_type),
                ..
            } => Some(return_type.clone()),
            _ => None,
        })
}

fn emit_typed_field_initializers(
    current_module_id: RuntimeModuleId,
    fields: &[CodegenComponentField],
    input_name: &str,
    predeclared_locals: &FxHashSet<String>,
    context: &EmitContext,
    out: &mut String,
) {
    for (index, field) in fields.iter().enumerate() {
        let has_name = format!("__nx_has_{}", index);
        let field_name = format!("__nx_field_{}", index);
        out.push_str(&format!(
            "  const {} = Object.prototype.hasOwnProperty.call({}, {});\n",
            has_name,
            input_name,
            js_string(&field.name)
        ));
        let fallback = typed_field_fallback(current_module_id, field, context);
        out.push_str(&format!(
            "  const {} = {} ? {}{} : {};\n",
            field_name,
            has_name,
            input_name,
            member_access(&field.name),
            fallback
        ));
        let local_name = safe_identifier(&field.name);
        if predeclared_locals.contains(&local_name) {
            out.push_str(&format!("  {} = {};\n", local_name, field_name));
        } else {
            out.push_str(&format!("  const {} = {};\n", local_name, field_name));
        }
    }
}

fn emit_initial_state_field_initializers(
    current_module_id: RuntimeModuleId,
    runtime_name: &str,
    component: &CodegenComponent,
    predeclared_locals: &FxHashSet<String>,
    context: &EmitContext,
    out: &mut String,
) {
    for (index, field) in component.state.iter().enumerate() {
        let field_name = format!("__nx_field_{}", index);
        let fallback = field
            .default
            .as_ref()
            .map(|default| emit_expression(current_module_id, default, context))
            .or_else(|| is_nullable_type(&field.ty).then(|| "null".to_string()))
            .unwrap_or_else(|| {
                format!(
                    "nxMissingField({}, {})",
                    js_string(&format!("{} state.{}", runtime_name, field.name)),
                    js_string(&format!("{} state", runtime_name))
                )
            });
        out.push_str(&format!("  const {} = {};\n", field_name, fallback));
        let local_name = safe_identifier(&field.name);
        if predeclared_locals.contains(&local_name) {
            out.push_str(&format!("  {} = {};\n", local_name, field_name));
        } else {
            out.push_str(&format!("  const {} = {};\n", local_name, field_name));
        }
    }
}

fn typed_field_fallback(
    current_module_id: RuntimeModuleId,
    field: &CodegenComponentField,
    context: &EmitContext,
) -> String {
    field
        .default
        .as_ref()
        .map(|default| emit_expression(current_module_id, default, context))
        .or_else(|| is_nullable_type(&field.ty).then(|| "null".to_string()))
        .unwrap_or_else(|| format!("{}{}", "props", member_access(&field.name)))
}

fn emit_field_object_return(fields: &[CodegenComponentField], indent: &str, out: &mut String) {
    out.push_str(indent);
    out.push_str("return {");
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            out.push_str(",");
        }
        out.push_str(&format!(
            " {}: __nx_field_{}",
            safe_object_key(&field.name),
            index
        ));
    }
    out.push_str(" };\n");
}

fn emit_component_boundary_schema(
    current_module_id: RuntimeModuleId,
    fields: &[CodegenComponentField],
    require_defaulted_fields: bool,
    context: &EmitContext,
) -> String {
    let fields = fields
        .iter()
        .map(|field| {
            let is_required = field.is_required
                || (require_defaulted_fields
                    && field.default.is_some()
                    && !is_nullable_type(&field.ty));
            let options = if is_required {
                String::new()
            } else {
                ", { required: false }".to_string()
            };
            format!(
                "{}: nxField({}{})",
                safe_object_key(&field.name),
                emit_type_schema(current_module_id, field.owner_module_id, &field.ty, context),
                options
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("nxRecordSchema({{ {} }})", fields)
}

fn emit_type_schema(
    current_module_id: RuntimeModuleId,
    schema_module_id: RuntimeModuleId,
    ty: &TypeRef,
    context: &EmitContext,
) -> String {
    let mut seen = FxHashSet::default();
    emit_type_schema_inner(current_module_id, schema_module_id, ty, context, &mut seen)
}

fn emit_type_schema_inner(
    current_module_id: RuntimeModuleId,
    schema_module_id: RuntimeModuleId,
    ty: &TypeRef,
    context: &EmitContext,
    seen: &mut FxHashSet<ReferenceKey>,
) -> String {
    match ty {
        TypeRef::Name(name) => emit_named_type_schema(
            current_module_id,
            schema_module_id,
            name.as_str(),
            context,
            seen,
        ),
        TypeRef::Array(inner) => format!(
            "nxArraySchema({})",
            emit_type_schema_inner(current_module_id, schema_module_id, inner, context, seen)
        ),
        TypeRef::Nullable(inner) => format!(
            "nxNullableSchema({})",
            emit_type_schema_inner(current_module_id, schema_module_id, inner, context, seen)
        ),
        TypeRef::Function { .. } => "nxAnySchema".to_string(),
    }
}

fn emit_named_type_schema(
    current_module_id: RuntimeModuleId,
    schema_module_id: RuntimeModuleId,
    name: &str,
    context: &EmitContext,
    seen: &mut FxHashSet<ReferenceKey>,
) -> String {
    match name {
        "i32" | "i64" | "int" | "f32" | "f64" | "float" => {
            return "nxNumberSchema".to_string();
        }
        "string" => return "nxStringSchema".to_string(),
        "bool" => return "nxBooleanSchema".to_string(),
        _ => {}
    }

    let Some(reference) = resolve_schema_reference(schema_module_id, name, context) else {
        return js_string("any");
    };
    let key = ReferenceKey::new(&reference);
    if !seen.insert(key) {
        return js_string("any");
    }

    let schema = context
        .schema_declarations
        .get(&key)
        .map(|declaration| match &declaration.kind {
            SchemaDeclarationKind::Enum { members } => emit_enum_schema(members),
            SchemaDeclarationKind::Record { fields } => emit_record_schema(
                current_module_id,
                declaration.reference.module_id,
                &declaration.reference.name,
                fields,
                context,
                seen,
            ),
            SchemaDeclarationKind::Union { cases } => emit_union_schema(
                current_module_id,
                declaration.reference.module_id,
                &declaration.reference.name,
                cases,
                context,
                seen,
            ),
            SchemaDeclarationKind::Component {
                fields,
                is_abstract,
                ..
            } => {
                if !*is_abstract && declaration.reference.module_id != current_module_id {
                    format!(
                        "{}.element",
                        context.generated_component_name(
                            current_module_id,
                            &declaration.reference,
                            ComponentNameRole::Schema,
                        )
                    )
                } else {
                    emit_component_schema(
                        current_module_id,
                        &declaration.reference.name,
                        fields,
                        !*is_abstract,
                        context,
                        seen,
                    )
                }
            }
        })
        .unwrap_or_else(|| js_string("any"));

    seen.remove(&key);
    schema
}

fn resolve_schema_reference(
    schema_module_id: RuntimeModuleId,
    name: &str,
    context: &EmitContext,
) -> Option<CodegenReference> {
    let module = context.module(schema_module_id)?;
    module
        .imports
        .iter()
        .find(|reference| reference.name == name && is_type_reference_kind(reference.kind))
        .cloned()
        .or_else(|| {
            module
                .declarations
                .iter()
                .find(|declaration| declaration.reference.name == name)
                .map(|declaration| declaration.reference.clone())
        })
}

fn emit_enum_schema(members: &[String]) -> String {
    let members = members
        .iter()
        .map(|member| js_string(member))
        .collect::<Vec<_>>()
        .join(", ");
    format!("nxEnumSchema([{}])", members)
}

fn emit_record_schema(
    current_module_id: RuntimeModuleId,
    schema_module_id: RuntimeModuleId,
    runtime_name: &str,
    fields: &[CodegenRecordField],
    context: &EmitContext,
    seen: &mut FxHashSet<ReferenceKey>,
) -> String {
    let fields = fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let default_metadata = field.default.as_ref().map(|default| {
                emit_schema_field_default(current_module_id, &fields[..index], default, context)
            });
            emit_schema_field(
                current_module_id,
                schema_module_id,
                &field.name,
                &field.ty,
                field.is_required,
                default_metadata,
                context,
                seen,
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "nxNamedRecordSchema({}, [{}])",
        js_string(runtime_name),
        fields
    )
}

fn emit_union_schema(
    current_module_id: RuntimeModuleId,
    schema_module_id: RuntimeModuleId,
    runtime_name: &str,
    cases: &[CodegenUnionCase],
    context: &EmitContext,
    seen: &mut FxHashSet<ReferenceKey>,
) -> String {
    let cases = cases
        .iter()
        .map(|case| {
            emit_record_schema(
                current_module_id,
                schema_module_id,
                &format!("{}.{}", runtime_name, case.name),
                &case.fields,
                context,
                seen,
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("nxUnionSchema([{}])", cases)
}

fn emit_component_schema(
    current_module_id: RuntimeModuleId,
    runtime_name: &str,
    fields: &[CodegenComponentField],
    require_all_fields: bool,
    context: &EmitContext,
    seen: &mut FxHashSet<ReferenceKey>,
) -> String {
    let fields = fields
        .iter()
        .map(|field| {
            emit_schema_field(
                current_module_id,
                field.owner_module_id,
                &field.name,
                &field.ty,
                field.is_required || require_all_fields,
                None,
                context,
                seen,
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "nxNamedRecordSchema({}, [{}])",
        js_string(runtime_name),
        fields
    )
}

fn emit_schema_field(
    current_module_id: RuntimeModuleId,
    schema_module_id: RuntimeModuleId,
    name: &str,
    ty: &TypeRef,
    is_required: bool,
    default_metadata: Option<String>,
    context: &EmitContext,
    seen: &mut FxHashSet<ReferenceKey>,
) -> String {
    let default = default_metadata
        .map(|metadata| format!(", {}", metadata))
        .unwrap_or_default();
    format!(
        "{{ name: {}, schema: {}, required: {}{} }}",
        js_string(name),
        emit_type_schema_inner(current_module_id, schema_module_id, ty, context, seen),
        is_required,
        default
    )
}

fn emit_schema_field_default(
    current_module_id: RuntimeModuleId,
    available_fields: &[CodegenRecordField],
    default: &CodegenExpression,
    context: &EmitContext,
) -> String {
    if let Some(value) = emit_schema_literal_default(default) {
        return format!("hasDefault: true, defaultValue: {}", value);
    }

    format!(
        "hasDefault: true, defaultFactory: {}",
        emit_schema_default_factory(current_module_id, available_fields, default, context)
    )
}

fn emit_schema_literal_default(default: &CodegenExpression) -> Option<String> {
    match &default.kind {
        CodegenExpressionKind::Literal(literal) => Some(emit_literal(literal)),
        CodegenExpressionKind::EnumMember { member, .. } => Some(js_string(member)),
        CodegenExpressionKind::Array(elements) => elements
            .iter()
            .map(emit_schema_literal_default)
            .collect::<Option<Vec<_>>>()
            .map(|values| format!("[{}]", values.join(", "))),
        _ => None,
    }
}

fn emit_schema_default_factory(
    current_module_id: RuntimeModuleId,
    available_fields: &[CodegenRecordField],
    default: &CodegenExpression,
    context: &EmitContext,
) -> String {
    let bindings = available_fields
        .iter()
        .map(|field| {
            format!(
                "const {} = __nx_record{};",
                safe_identifier(&field.name),
                member_access(&field.name)
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "(__nx_record) => {{ {} return {}; }}",
        bindings,
        emit_expression(current_module_id, default, context)
    )
}

fn is_nullable_type(ty: &TypeRef) -> bool {
    matches!(ty, TypeRef::Nullable(_))
}

fn emit_type_ref(
    current_module_id: RuntimeModuleId,
    ty: &TypeRef,
    module: &CodegenModule,
    context: &EmitContext,
) -> String {
    match ty {
        TypeRef::Name(name) => emit_named_type(current_module_id, name.as_str(), module, context),
        TypeRef::Array(inner) => {
            emit_array_type(emit_type_ref(current_module_id, inner, module, context))
        }
        TypeRef::Nullable(inner) => format!(
            "{} | null",
            emit_type_ref(current_module_id, inner, module, context)
        ),
        TypeRef::Function {
            params,
            return_type,
        } => {
            let params = params
                .iter()
                .enumerate()
                .map(|(index, param)| {
                    format!(
                        "arg{}: {}",
                        index,
                        emit_type_ref(current_module_id, param, module, context)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "({}) => {}",
                params,
                emit_type_ref(current_module_id, return_type, module, context)
            )
        }
    }
}

fn emit_type(
    current_module_id: RuntimeModuleId,
    ty: &Type,
    module: &CodegenModule,
    context: &EmitContext,
) -> String {
    match ty {
        Type::Primitive(primitive) => emit_primitive_type(*primitive),
        Type::Array(inner) => emit_array_type(emit_type(current_module_id, inner, module, context)),
        Type::Nullable(inner) => {
            format!(
                "{} | null",
                emit_type(current_module_id, inner, module, context)
            )
        }
        Type::Function { params, ret } => {
            let params = params
                .iter()
                .enumerate()
                .map(|(index, param)| {
                    format!(
                        "arg{}: {}",
                        index,
                        emit_type(current_module_id, param, module, context)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "({}) => {}",
                params,
                emit_type(current_module_id, ret, module, context)
            )
        }
        Type::Named(name) => emit_named_type(current_module_id, name.as_str(), module, context),
        Type::Enum(enum_ty) => {
            emit_named_type(current_module_id, enum_ty.name.as_str(), module, context)
        }
        Type::Union(union_ty) => {
            emit_named_type(current_module_id, union_ty.name.as_str(), module, context)
        }
        Type::UnionCase(case_ty) => {
            let union_name =
                emit_named_type(current_module_id, case_ty.union.as_str(), module, context);
            union_case_type_name(&union_name, case_ty.case.as_str())
        }
        Type::Variable(_) | Type::Unknown | Type::Error => "unknown".to_string(),
    }
}

fn emit_array_type(element: String) -> String {
    if should_parenthesize_array_element_type(&element) {
        format!("readonly ({})[]", element)
    } else {
        format!("readonly {}[]", element)
    }
}

fn should_parenthesize_array_element_type(element: &str) -> bool {
    element.contains(" | ") || element.starts_with("readonly ") || element.contains("=>")
}

fn emit_named_type(
    current_module_id: RuntimeModuleId,
    name: &str,
    _module: &CodegenModule,
    context: &EmitContext,
) -> String {
    match name {
        "i32" | "i64" | "int" | "f32" | "f64" | "float" => "number".to_string(),
        "string" => "string".to_string(),
        "bool" => "boolean".to_string(),
        "void" => "void".to_string(),
        _ => context
            .type_reference(current_module_id, name)
            .map(|reference| {
                if reference.kind == ResolvedItemKind::Component {
                    match context.component(&reference) {
                        Some(component) if !component.is_abstract => context
                            .generated_component_name(
                                current_module_id,
                                &reference,
                                ComponentNameRole::Element,
                            ),
                        Some(_) => "unknown".to_string(),
                        None => context.reference_name(current_module_id, &reference),
                    }
                } else {
                    context.reference_name(current_module_id, &reference)
                }
            })
            .unwrap_or_else(|| safe_identifier(name)),
    }
}

fn emit_primitive_type(primitive: Primitive) -> String {
    match primitive {
        Primitive::I32
        | Primitive::I64
        | Primitive::Int
        | Primitive::F32
        | Primitive::F64
        | Primitive::Float => "number".to_string(),
        Primitive::String => "string".to_string(),
        Primitive::Bool => "boolean".to_string(),
        Primitive::Void => "void".to_string(),
    }
}

fn emit_expression(
    current_module_id: RuntimeModuleId,
    expression: &CodegenExpression,
    context: &EmitContext,
) -> String {
    match &expression.kind {
        CodegenExpressionKind::Literal(literal) => emit_literal(literal),
        CodegenExpressionKind::Identifier { name, reference } => reference
            .as_ref()
            .map(|reference| context.reference_name(current_module_id, reference))
            .unwrap_or_else(|| safe_identifier(name)),
        CodegenExpressionKind::Binary { lhs, op, rhs } => format!(
            "({} {} {})",
            emit_expression(current_module_id, lhs, context),
            binop_text(*op),
            emit_expression(current_module_id, rhs, context)
        ),
        CodegenExpressionKind::Unary { op, expr } => format!(
            "({}{})",
            unop_text(*op),
            emit_expression(current_module_id, expr, context)
        ),
        CodegenExpressionKind::Call { callee, args } => format!(
            "{}({})",
            emit_expression(current_module_id, callee, context),
            args.iter()
                .map(|arg| emit_expression(current_module_id, arg, context))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        CodegenExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => format!(
            "({} ? {} : {})",
            emit_expression(current_module_id, condition, context),
            emit_expression(current_module_id, then_branch, context),
            else_branch
                .as_ref()
                .map(|expr| emit_expression(current_module_id, expr, context))
                .unwrap_or_else(|| "null".to_string())
        ),
        CodegenExpressionKind::Match { .. } => {
            "nxRuntimeError(\"match expressions are not supported by executable source codegen yet\")"
                .to_string()
        }
        CodegenExpressionKind::Let { name, value, body } => format!(
            "(() => {{ const {} = {}; return {}; }})()",
            safe_identifier(name),
            emit_expression(current_module_id, value, context),
            emit_expression(current_module_id, body, context)
        ),
        CodegenExpressionKind::Block {
            statements,
            expression,
        } => emit_block(
            current_module_id,
            statements,
            expression.as_deref(),
            context,
        ),
        CodegenExpressionKind::Array(elements) => format!(
            "[{}]",
            elements
                .iter()
                .map(|element| emit_expression(current_module_id, element, context))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        CodegenExpressionKind::For {
            item,
            index,
            iterable,
            body,
        } => {
            let index_name = index.as_deref().unwrap_or("_index");
            format!(
                "Array.from({}).map(({}, {}) => {})",
                emit_expression(current_module_id, iterable, context),
                safe_identifier(item),
                safe_identifier(index_name),
                emit_expression(current_module_id, body, context)
            )
        }
        CodegenExpressionKind::Index { base, index } => format!(
            "{}[{}]",
            emit_expression(current_module_id, base, context),
            emit_expression(current_module_id, index, context)
        ),
        CodegenExpressionKind::Member {
            base,
            member,
            reference,
        } => reference
            .as_ref()
            .map(|reference| context.reference_name(current_module_id, reference))
            .unwrap_or_else(|| {
                format!(
                    "{}[{}]",
                    emit_expression(current_module_id, base, context),
                    js_string(member)
                )
            }),
        CodegenExpressionKind::EnumMember {
            enum_reference,
            member,
        } => {
            let enum_name = context.reference_name(current_module_id, enum_reference);
            format!("{}{}", enum_name, member_access(member))
        }
        CodegenExpressionKind::UnionCase {
            union_reference,
            case_name,
            fields,
            properties,
            content_field,
            content,
        } => emit_union_case_object(
            current_module_id,
            union_reference,
            case_name,
            fields,
            properties,
            content_field.as_deref(),
            content,
            context,
        ),
        CodegenExpressionKind::Record {
            name,
            fields,
            properties,
        } => emit_record_object(current_module_id, name, fields, properties, context),
        CodegenExpressionKind::ComponentDescriptor(descriptor) => {
            emit_component_descriptor(current_module_id, descriptor, context)
        }
        CodegenExpressionKind::Element(element) => {
            emit_element(current_module_id, element, context)
        }
        CodegenExpressionKind::Unsupported(unsupported) => {
            format!("nxRuntimeError({})", js_string(&unsupported.message))
        }
    }
}

fn emit_record_object(
    current_module_id: RuntimeModuleId,
    name: &str,
    fields: &[CodegenRecordField],
    properties: &[CodegenProperty],
    context: &EmitContext,
) -> String {
    emit_materialized_record_object(
        current_module_id,
        name,
        fields,
        properties,
        Vec::new(),
        context,
    )
}

fn emit_component_descriptor(
    current_module_id: RuntimeModuleId,
    descriptor: &CodegenComponentDescriptor,
    context: &EmitContext,
) -> String {
    let mut extra_properties = Vec::new();
    if let Some(content_field) = descriptor.content_field.as_deref() {
        if !descriptor.content.is_empty()
            && !descriptor
                .properties
                .iter()
                .any(|property| property.name == content_field)
        {
            extra_properties.push((
                content_field.to_string(),
                emit_content_value(current_module_id, &descriptor.content, context),
            ));
        }
    }
    let mut explicit_properties = explicit_property_values(
        current_module_id,
        &descriptor.properties,
        extra_properties,
        context,
    );
    explicit_properties.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));

    let properties = explicit_properties
        .iter()
        .map(|(name, value)| format!("{}: {}", safe_object_key(name), value))
        .collect::<Vec<_>>()
        .join(", ");
    let component_name = context.reference_name(current_module_id, &descriptor.component);
    // Concrete component functions construct atomic descriptors; schemas/render helpers are the
    // explicit component-entry path that evaluates normal component bodies.
    format!("{}({{ {} }})", component_name, properties)
}

fn emit_union_case_object(
    current_module_id: RuntimeModuleId,
    union_reference: &CodegenReference,
    case_name: &str,
    fields: &[CodegenRecordField],
    properties: &[CodegenProperty],
    content_field: Option<&str>,
    content: &[CodegenExpression],
    context: &EmitContext,
) -> String {
    let mut extra_properties = Vec::new();
    if let Some(content_field) = content_field {
        if !content.is_empty()
            && !properties
                .iter()
                .any(|property| property.name == content_field)
        {
            extra_properties.push((
                content_field.to_string(),
                emit_content_value(current_module_id, content, context),
            ));
        }
    }
    emit_materialized_record_object(
        current_module_id,
        &format!("{}.{}", union_reference.name, case_name),
        fields,
        properties,
        extra_properties,
        context,
    )
}

fn emit_materialized_record_object(
    current_module_id: RuntimeModuleId,
    type_name: &str,
    fields: &[CodegenRecordField],
    properties: &[CodegenProperty],
    extra_properties: Vec<(String, String)>,
    context: &EmitContext,
) -> String {
    let explicit_properties =
        explicit_property_values(current_module_id, properties, extra_properties, context);
    let needs_materialization = fields.iter().any(|field| {
        !explicit_properties
            .iter()
            .any(|(name, _)| name == &field.name)
            && field.default.is_some()
    });

    if needs_materialization {
        return emit_materialized_record_iife(
            current_module_id,
            type_name,
            fields,
            &explicit_properties,
            context,
        );
    }

    let mut emitted = FxHashSet::default();
    let mut entries = Vec::new();
    entries.push(format!("$type: {}", js_string(type_name)));

    for field in fields {
        if let Some((_, value)) = explicit_properties
            .iter()
            .find(|(name, _)| name == &field.name)
        {
            entries.push(format!("{}: {}", safe_object_key(&field.name), value));
            emitted.insert(field.name.clone());
        } else if let Some(default) = field.default.as_ref() {
            entries.push(format!(
                "{}: {}",
                safe_object_key(&field.name),
                emit_expression(current_module_id, default, context)
            ));
            emitted.insert(field.name.clone());
        } else {
            entries.push(format!("{}: null", safe_object_key(&field.name)));
            emitted.insert(field.name.clone());
        }
    }

    let mut remaining = explicit_properties
        .iter()
        .filter(|(name, _)| !emitted.contains(name))
        .collect::<Vec<_>>();
    remaining.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
    for (name, value) in remaining {
        entries.push(format!("{}: {}", safe_object_key(name), value));
    }

    format!("({{ {} }})", entries.join(", "))
}

fn explicit_property_values(
    current_module_id: RuntimeModuleId,
    properties: &[CodegenProperty],
    extra_properties: Vec<(String, String)>,
    context: &EmitContext,
) -> Vec<(String, String)> {
    let mut values = properties
        .iter()
        .map(|property| {
            (
                property.name.clone(),
                emit_expression(current_module_id, &property.value, context),
            )
        })
        .collect::<Vec<_>>();
    values.extend(extra_properties);
    values
}

fn emit_materialized_record_iife(
    current_module_id: RuntimeModuleId,
    type_name: &str,
    fields: &[CodegenRecordField],
    explicit_properties: &[(String, String)],
    context: &EmitContext,
) -> String {
    let mut out = "(() => { ".to_string();
    let mut property_temps = FxHashMap::default();
    for (index, (name, value)) in explicit_properties.iter().enumerate() {
        let temp_name = format!("__nx_prop_{}", index);
        property_temps.insert(name.clone(), temp_name.clone());
        out.push_str(&format!("const {} = {}; ", temp_name, value));
    }

    let mut emitted = FxHashSet::default();
    let mut field_temps = Vec::new();
    for field in fields {
        let field_temp_name = format!("__nx_field_{}", field_temps.len());
        let value = property_temps
            .get(&field.name)
            .cloned()
            .or_else(|| {
                field
                    .default
                    .as_ref()
                    .map(|default| emit_expression(current_module_id, default, context))
            })
            .unwrap_or_else(|| "null".to_string());
        out.push_str(&format!("const {} = {}; ", field_temp_name, value));
        field_temps.push((field.name.clone(), field_temp_name));
        emitted.insert(field.name.clone());
    }

    let mut entries = Vec::new();
    entries.push(format!("$type: {}", js_string(type_name)));
    for (field_name, field_temp_name) in &field_temps {
        entries.push(format!(
            "{}: {}",
            safe_object_key(field_name),
            field_temp_name
        ));
    }

    let mut remaining = explicit_properties
        .iter()
        .filter(|(name, _)| !emitted.contains(name))
        .collect::<Vec<_>>();
    remaining.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
    for (name, _) in remaining {
        if let Some(temp_name) = property_temps.get(name) {
            entries.push(format!("{}: {}", safe_object_key(name), temp_name));
        }
    }

    out.push_str(&format!("return {{ {} }}; ", entries.join(", ")));
    out.push_str("})()");
    out
}

fn emit_content_value(
    current_module_id: RuntimeModuleId,
    content: &[CodegenExpression],
    context: &EmitContext,
) -> String {
    if content.len() == 1 {
        emit_expression(current_module_id, &content[0], context)
    } else {
        format!(
            "[{}]",
            content
                .iter()
                .map(|expr| emit_expression(current_module_id, expr, context))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn member_access(member: &str) -> String {
    let identifier = safe_identifier(member);
    if identifier == member && !is_reserved_word(member) {
        format!(".{}", identifier)
    } else {
        format!("[{}]", js_string(member))
    }
}

fn emit_block(
    current_module_id: RuntimeModuleId,
    statements: &[CodegenStatement],
    expression: Option<&CodegenExpression>,
    context: &EmitContext,
) -> String {
    let mut out = "(() => { ".to_string();
    for statement in statements {
        match statement {
            CodegenStatement::Let { name, init, .. } => {
                out.push_str(&format!(
                    "const {} = {}; ",
                    safe_identifier(name),
                    emit_expression(current_module_id, init, context)
                ));
            }
            CodegenStatement::Expr(expr) => {
                out.push_str(&format!(
                    "{}; ",
                    emit_expression(current_module_id, expr, context)
                ));
            }
        }
    }
    match expression {
        Some(expression) => out.push_str(&format!(
            "return {}; ",
            emit_expression(current_module_id, expression, context)
        )),
        None => out.push_str("return null; "),
    }
    out.push_str("})()");
    out
}

fn emit_element(
    current_module_id: RuntimeModuleId,
    element: &CodegenElement,
    context: &EmitContext,
) -> String {
    let properties = element
        .properties
        .iter()
        .map(|property| {
            format!(
                "{}: {}",
                safe_object_key(&property.name),
                emit_expression(current_module_id, &property.value, context)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let content = element
        .content
        .iter()
        .map(|expr| emit_expression(current_module_id, expr, context))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "nxElement({}, {{ {} }}, [{}])",
        js_string(&element.tag),
        properties,
        content
    )
}

fn emit_index(program: &CodegenProgram, context: &EmitContext, target: CodegenTarget) -> String {
    let mut out = String::new();
    out.push_str("// <auto-generated/>\n");
    out.push_str(&format!("// nx-fingerprint: {}\n\n", program.fingerprint));
    for entrypoint in &program.entrypoints {
        out.push_str(&format!(
            "export {{ {} }} from \"./{}\";\n",
            context.declaration_name(&entrypoint.reference),
            import_file(context.module_file(entrypoint.reference.module_id), target)
        ));
    }
    for entrypoint in &program.component_entrypoints {
        if context
            .component(&entrypoint.reference)
            .is_some_and(|component| component.is_abstract)
        {
            continue;
        }
        let component_name = context.declaration_name(&entrypoint.reference);
        let schema_name = context
            .component_names(&entrypoint.reference)
            .name(ComponentNameRole::Schema);
        let names = context.component_names(&entrypoint.reference);
        let component = context.component(&entrypoint.reference);
        let mut value_exports = vec![component_name.to_string(), schema_name.to_string()];
        if component.is_some_and(|component| !component.state.is_empty()) {
            value_exports.push(
                names
                    .initial_state_function
                    .as_deref()
                    .expect("stateful component should have an initial-state helper")
                    .to_string(),
            );
            value_exports.push(
                names
                    .render_function
                    .as_deref()
                    .expect("stateful component should have a render helper")
                    .to_string(),
            );
        }
        out.push_str(&format!(
            "export {{ {} }} from \"./{}\";\n",
            value_exports.join(", "),
            import_file(context.module_file(entrypoint.reference.module_id), target)
        ));
        if target.is_typescript() {
            let mut type_exports = vec![names.name(ComponentNameRole::Props).to_string()];
            if component.is_some_and(|component| !component.is_abstract) {
                type_exports.push(names.name(ComponentNameRole::Element).to_string());
            }
            if component.is_some_and(|component| !component.state.is_empty()) {
                type_exports.push(names.name(ComponentNameRole::State).to_string());
            }
            out.push_str(&format!(
                "export type {{ {} }} from \"./{}\";\n",
                type_exports.join(", "),
                import_file(context.module_file(entrypoint.reference.module_id), target)
            ));
        }
    }
    out
}

fn collect_module_import_references(
    module: &CodegenModule,
    target: CodegenTarget,
) -> Vec<CodegenReference> {
    let mut references = Vec::new();
    references.extend(collect_module_value_references(module));
    if target.is_typescript() {
        references.extend(collect_module_type_references(module));
    }
    sort_and_dedup_references(&mut references);
    references
}

fn collect_module_value_references(module: &CodegenModule) -> Vec<CodegenReference> {
    let mut references = Vec::new();
    for declaration in &module.declarations {
        collect_declaration_value_references(module, declaration, &mut references);
    }
    sort_and_dedup_references(&mut references);
    references
}

fn collect_module_type_references(module: &CodegenModule) -> Vec<CodegenReference> {
    let mut references = Vec::new();
    for declaration in &module.declarations {
        collect_declaration_type_references(module, declaration, &mut references);
    }
    sort_and_dedup_references(&mut references);
    references
}

fn sort_and_dedup_references(references: &mut Vec<CodegenReference>) {
    references.sort_by(|lhs, rhs| {
        lhs.module_id
            .as_u32()
            .cmp(&rhs.module_id.as_u32())
            .then_with(|| lhs.definition_id.index().cmp(&rhs.definition_id.index()))
            .then_with(|| lhs.name.cmp(&rhs.name))
    });
    references.dedup_by(|lhs, rhs| ReferenceKey::new(lhs) == ReferenceKey::new(rhs));
}

fn collect_declaration_value_references(
    module: &CodegenModule,
    declaration: &CodegenDeclaration,
    output: &mut Vec<CodegenReference>,
) {
    match &declaration.kind {
        CodegenDeclarationKind::Function { body, .. } => {
            collect_expression_value_references(module.id, body, output);
        }
        CodegenDeclarationKind::Value { value, .. } => {
            collect_expression_value_references(module.id, value, output);
        }
        CodegenDeclarationKind::Component(component) => {
            collect_component_value_references(module, component, output);
        }
        CodegenDeclarationKind::Unsupported(_) => {}
        CodegenDeclarationKind::Enum { .. }
        | CodegenDeclarationKind::Record { .. }
        | CodegenDeclarationKind::Union { .. }
        | CodegenDeclarationKind::TypeAlias => {}
    }
}

fn collect_declaration_type_references(
    module: &CodegenModule,
    declaration: &CodegenDeclaration,
    output: &mut Vec<CodegenReference>,
) {
    match &declaration.kind {
        CodegenDeclarationKind::Function {
            params,
            return_type,
            ..
        } => {
            for param in params {
                collect_type_ref_references(module, &param.ty, output);
            }
            if let Some(return_type) = return_type {
                collect_type_references(module, return_type, output);
            }
        }
        CodegenDeclarationKind::Value { ty, .. } => {
            if let Some(ty) = ty {
                collect_type_references(module, ty, output);
            }
        }
        CodegenDeclarationKind::Record { fields } => {
            for field in fields {
                collect_type_ref_references(module, &field.ty, output);
            }
        }
        CodegenDeclarationKind::Union { cases } => {
            for case in cases {
                for field in &case.fields {
                    collect_type_ref_references(module, &field.ty, output);
                }
            }
        }
        CodegenDeclarationKind::Component(component) => {
            for field in &component.props {
                collect_type_ref_references(module, &field.ty, output);
            }
            for field in &component.state {
                collect_type_ref_references(module, &field.ty, output);
            }
            if let Some(body) = component.body.as_ref() {
                if let Some(ty) = body.ty.as_ref() {
                    collect_type_references(module, ty, output);
                }
                collect_expression_render_type_references(module.id, body, output);
            }
        }
        CodegenDeclarationKind::Enum { .. }
        | CodegenDeclarationKind::TypeAlias
        | CodegenDeclarationKind::Unsupported(_) => {}
    }
}

fn collect_component_value_references(
    module: &CodegenModule,
    component: &CodegenComponent,
    output: &mut Vec<CodegenReference>,
) {
    for field in &component.props {
        collect_type_ref_schema_value_references(module, &field.ty, output);
        if let Some(default) = field.default.as_ref() {
            collect_expression_value_references(module.id, default, output);
        }
    }
    for field in &component.state {
        collect_type_ref_schema_value_references(module, &field.ty, output);
        if let Some(default) = field.default.as_ref() {
            collect_expression_value_references(module.id, default, output);
        }
    }
    if let Some(body) = component.body.as_ref() {
        collect_expression_value_references(module.id, body, output);
    }
}

fn collect_type_ref_schema_value_references(
    module: &CodegenModule,
    ty: &TypeRef,
    output: &mut Vec<CodegenReference>,
) {
    match ty {
        TypeRef::Name(name) => {
            if let Some(reference) = module.imports.iter().find(|reference| {
                reference.name == name.as_str() && reference.kind == ResolvedItemKind::Component
            }) {
                output.push(reference.clone());
            }
        }
        TypeRef::Array(inner) | TypeRef::Nullable(inner) => {
            collect_type_ref_schema_value_references(module, inner, output);
        }
        TypeRef::Function {
            params,
            return_type,
        } => {
            for param in params {
                collect_type_ref_schema_value_references(module, param, output);
            }
            collect_type_ref_schema_value_references(module, return_type, output);
        }
    }
}

fn collect_type_ref_references(
    module: &CodegenModule,
    ty: &TypeRef,
    output: &mut Vec<CodegenReference>,
) {
    match ty {
        TypeRef::Name(name) => collect_named_type_reference(module, name.as_str(), output),
        TypeRef::Array(inner) | TypeRef::Nullable(inner) => {
            collect_type_ref_references(module, inner, output);
        }
        TypeRef::Function {
            params,
            return_type,
        } => {
            for param in params {
                collect_type_ref_references(module, param, output);
            }
            collect_type_ref_references(module, return_type, output);
        }
    }
}

fn collect_type_references(module: &CodegenModule, ty: &Type, output: &mut Vec<CodegenReference>) {
    match ty {
        Type::Array(inner) | Type::Nullable(inner) => {
            collect_type_references(module, inner, output);
        }
        Type::Function { params, ret } => {
            for param in params {
                collect_type_references(module, param, output);
            }
            collect_type_references(module, ret, output);
        }
        Type::Named(name) => collect_named_type_reference(module, name.as_str(), output),
        Type::Enum(enum_ty) => collect_named_type_reference(module, enum_ty.name.as_str(), output),
        Type::Union(union_ty) => {
            collect_named_type_reference(module, union_ty.name.as_str(), output)
        }
        Type::UnionCase(case_ty) => {
            collect_named_type_reference(module, case_ty.union.as_str(), output);
        }
        Type::Primitive(_) | Type::Variable(_) | Type::Unknown | Type::Error => {}
    }
}

fn collect_expression_render_type_references(
    current_module_id: RuntimeModuleId,
    expression: &CodegenExpression,
    output: &mut Vec<CodegenReference>,
) {
    match &expression.kind {
        CodegenExpressionKind::ComponentDescriptor(descriptor) => {
            if descriptor.component.module_id != current_module_id {
                output.push(descriptor.component.clone());
            }
        }
        CodegenExpressionKind::If {
            then_branch,
            else_branch: Some(else_branch),
            ..
        } => {
            collect_expression_render_type_references(current_module_id, then_branch, output);
            collect_expression_render_type_references(current_module_id, else_branch, output);
        }
        CodegenExpressionKind::Match {
            arms, else_branch, ..
        } => {
            for arm in arms {
                collect_expression_render_type_references(current_module_id, &arm.body, output);
            }
            if let Some(else_branch) = else_branch {
                collect_expression_render_type_references(current_module_id, else_branch, output);
            }
        }
        CodegenExpressionKind::Let { body, .. } => {
            collect_expression_render_type_references(current_module_id, body, output);
        }
        CodegenExpressionKind::Block {
            expression: Some(expression),
            ..
        } => {
            collect_expression_render_type_references(current_module_id, expression, output);
        }
        _ => {}
    }
}

fn collect_named_type_reference(
    module: &CodegenModule,
    name: &str,
    output: &mut Vec<CodegenReference>,
) {
    if let Some(reference) = module
        .imports
        .iter()
        .find(|reference| reference.name == name && is_type_reference_kind(reference.kind))
    {
        output.push(reference.clone());
    }
}

fn collect_expression_value_references(
    current_module_id: RuntimeModuleId,
    expression: &CodegenExpression,
    output: &mut Vec<CodegenReference>,
) {
    match &expression.kind {
        CodegenExpressionKind::Identifier {
            reference: Some(reference),
            ..
        }
        | CodegenExpressionKind::Member {
            reference: Some(reference),
            ..
        } => {
            if reference.module_id != current_module_id
                && should_import_value_reference(reference.kind)
            {
                output.push(reference.clone());
            }
        }
        CodegenExpressionKind::EnumMember { enum_reference, .. } => {
            if enum_reference.module_id != current_module_id {
                output.push(enum_reference.clone());
            }
        }
        CodegenExpressionKind::Binary { lhs, rhs, .. } => {
            collect_expression_value_references(current_module_id, lhs, output);
            collect_expression_value_references(current_module_id, rhs, output);
        }
        CodegenExpressionKind::Unary { expr, .. } => {
            collect_expression_value_references(current_module_id, expr, output);
        }
        CodegenExpressionKind::Call { callee, args } => {
            collect_expression_value_references(current_module_id, callee, output);
            for arg in args {
                collect_expression_value_references(current_module_id, arg, output);
            }
        }
        CodegenExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_expression_value_references(current_module_id, condition, output);
            collect_expression_value_references(current_module_id, then_branch, output);
            if let Some(else_branch) = else_branch {
                collect_expression_value_references(current_module_id, else_branch, output);
            }
        }
        CodegenExpressionKind::Match {
            scrutinee,
            arms,
            else_branch,
        } => {
            collect_expression_value_references(current_module_id, scrutinee, output);
            for arm in arms {
                for pattern in &arm.patterns {
                    collect_expression_value_references(current_module_id, pattern, output);
                }
                collect_expression_value_references(current_module_id, &arm.body, output);
            }
            if let Some(else_branch) = else_branch {
                collect_expression_value_references(current_module_id, else_branch, output);
            }
        }
        CodegenExpressionKind::Let { value, body, .. } => {
            collect_expression_value_references(current_module_id, value, output);
            collect_expression_value_references(current_module_id, body, output);
        }
        CodegenExpressionKind::Block {
            statements,
            expression,
        } => {
            for statement in statements {
                match statement {
                    CodegenStatement::Let { init, .. } => {
                        collect_expression_value_references(current_module_id, init, output);
                    }
                    CodegenStatement::Expr(expr) => {
                        collect_expression_value_references(current_module_id, expr, output);
                    }
                }
            }
            if let Some(expression) = expression {
                collect_expression_value_references(current_module_id, expression, output);
            }
        }
        CodegenExpressionKind::Array(elements) => {
            for element in elements {
                collect_expression_value_references(current_module_id, element, output);
            }
        }
        CodegenExpressionKind::For { iterable, body, .. } => {
            collect_expression_value_references(current_module_id, iterable, output);
            collect_expression_value_references(current_module_id, body, output);
        }
        CodegenExpressionKind::Index { base, index } => {
            collect_expression_value_references(current_module_id, base, output);
            collect_expression_value_references(current_module_id, index, output);
        }
        CodegenExpressionKind::Member { base, .. } => {
            collect_expression_value_references(current_module_id, base, output);
        }
        CodegenExpressionKind::Record {
            fields, properties, ..
        } => {
            for field in fields {
                if let Some(default) = field.default.as_ref() {
                    collect_expression_value_references(current_module_id, default, output);
                }
            }
            for property in properties {
                collect_expression_value_references(current_module_id, &property.value, output);
            }
        }
        CodegenExpressionKind::ComponentDescriptor(descriptor) => {
            if descriptor.component.module_id != current_module_id {
                output.push(descriptor.component.clone());
            }
            for property in &descriptor.properties {
                collect_expression_value_references(current_module_id, &property.value, output);
            }
            for content in &descriptor.content {
                collect_expression_value_references(current_module_id, content, output);
            }
        }
        CodegenExpressionKind::UnionCase {
            fields,
            properties,
            content,
            ..
        } => {
            for field in fields {
                if let Some(default) = field.default.as_ref() {
                    collect_expression_value_references(current_module_id, default, output);
                }
            }
            for property in properties {
                collect_expression_value_references(current_module_id, &property.value, output);
            }
            for expr in content {
                collect_expression_value_references(current_module_id, expr, output);
            }
        }
        CodegenExpressionKind::Element(element) => {
            for property in &element.properties {
                collect_expression_value_references(current_module_id, &property.value, output);
            }
            for content in &element.content {
                collect_expression_value_references(current_module_id, content, output);
            }
        }
        CodegenExpressionKind::Literal(_)
        | CodegenExpressionKind::Identifier {
            reference: None, ..
        }
        | CodegenExpressionKind::Unsupported(_) => {}
    }
}

fn collect_module_runtime_helpers(
    module: &CodegenModule,
    target: CodegenTarget,
) -> Vec<&'static str> {
    let mut helpers = FxHashSet::default();
    for declaration in &module.declarations {
        collect_declaration_runtime_helpers(declaration, target, &mut helpers);
    }
    let mut output = JS_PROGRAM_MODULE_RESERVED_RUNTIME_NAMES
        .into_iter()
        .copied()
        .filter(|helper| target.is_typescript() || !matches!(*helper, "NxResult" | "NxValue"))
        .filter(|helper| helpers.contains(*helper))
        .collect::<Vec<_>>();
    output.sort();
    output
}

fn collect_js_program_module_runtime_helpers(program: &CodegenProgram) -> Vec<&'static str> {
    let mut helpers = FxHashSet::default();
    for module in &program.modules {
        for declaration in &module.declarations {
            collect_declaration_runtime_helpers(
                declaration,
                CodegenTarget::JavaScript,
                &mut helpers,
            );
        }
    }
    let mut output = helpers.into_iter().collect::<Vec<_>>();
    output.sort();
    output
}

fn collect_declaration_runtime_helpers(
    declaration: &CodegenDeclaration,
    _target: CodegenTarget,
    output: &mut FxHashSet<&'static str>,
) {
    match &declaration.kind {
        CodegenDeclarationKind::Function { body, .. } => {
            collect_expression_runtime_helpers(body, output);
        }
        CodegenDeclarationKind::Value { value, .. } => {
            collect_expression_runtime_helpers(value, output);
        }
        CodegenDeclarationKind::Unsupported(_) => {
            output.insert("nxRuntimeError");
        }
        CodegenDeclarationKind::Component(component) => {
            if !component.is_abstract {
                if component.is_external {
                    output.insert("nxExternalComponentSchema");
                } else {
                    output.insert("nxComponentSchema");
                }
                output.insert("nxField");
                output.insert("nxRecordSchema");
                collect_component_schema_runtime_helpers(component, output);
            }
            if component.state.iter().any(|field| {
                field.default.is_none()
                    && !is_nullable_type(&field.ty)
                    && !component.is_external
                    && !component.is_abstract
            }) {
                output.insert("nxMissingField");
            }
            if let Some(body) = component.body.as_ref() {
                collect_expression_runtime_helpers(body, output);
            }
        }
        CodegenDeclarationKind::Enum { .. }
        | CodegenDeclarationKind::Record { .. }
        | CodegenDeclarationKind::Union { .. }
        | CodegenDeclarationKind::TypeAlias => {}
    }
}

fn collect_component_schema_runtime_helpers(
    component: &CodegenComponent,
    output: &mut FxHashSet<&'static str>,
) {
    for field in &component.props {
        collect_type_ref_schema_runtime_helpers(&field.ty, output);
    }
    for field in &component.state {
        collect_type_ref_schema_runtime_helpers(&field.ty, output);
    }
}

fn collect_type_ref_schema_runtime_helpers(ty: &TypeRef, output: &mut FxHashSet<&'static str>) {
    match ty {
        TypeRef::Name(name) => match name.as_str() {
            "i32" | "i64" | "int" | "f32" | "f64" | "float" => {
                output.insert("nxNumberSchema");
            }
            "string" => {
                output.insert("nxStringSchema");
            }
            "bool" => {
                output.insert("nxBooleanSchema");
            }
            _ => {
                output.insert("nxNamedRecordSchema");
                output.insert("nxEnumSchema");
                output.insert("nxUnionSchema");
                output.insert("nxAnySchema");
                output.insert("nxArraySchema");
                output.insert("nxBooleanSchema");
                output.insert("nxNullableSchema");
                output.insert("nxNumberSchema");
                output.insert("nxStringSchema");
            }
        },
        TypeRef::Array(inner) => {
            output.insert("nxArraySchema");
            collect_type_ref_schema_runtime_helpers(inner, output);
        }
        TypeRef::Nullable(inner) => {
            output.insert("nxNullableSchema");
            collect_type_ref_schema_runtime_helpers(inner, output);
        }
        TypeRef::Function { .. } => {
            output.insert("nxAnySchema");
        }
    }
}

fn collect_expression_runtime_helpers(
    expression: &CodegenExpression,
    output: &mut FxHashSet<&'static str>,
) {
    match &expression.kind {
        CodegenExpressionKind::Array(elements) => {
            for element in elements {
                collect_expression_runtime_helpers(element, output);
            }
        }
        CodegenExpressionKind::For { iterable, body, .. } => {
            collect_expression_runtime_helpers(iterable, output);
            collect_expression_runtime_helpers(body, output);
        }
        CodegenExpressionKind::Element(_) => {
            output.insert("nxElement");
        }
        CodegenExpressionKind::Unsupported(_) => {
            output.insert("nxRuntimeError");
        }
        CodegenExpressionKind::Literal(_)
        | CodegenExpressionKind::Identifier { .. }
        | CodegenExpressionKind::EnumMember { .. } => {}
        CodegenExpressionKind::Binary { lhs, rhs, .. } => {
            collect_expression_runtime_helpers(lhs, output);
            collect_expression_runtime_helpers(rhs, output);
        }
        CodegenExpressionKind::Unary { expr, .. } => {
            collect_expression_runtime_helpers(expr, output);
        }
        CodegenExpressionKind::Call { callee, args } => {
            collect_expression_runtime_helpers(callee, output);
            for arg in args {
                collect_expression_runtime_helpers(arg, output);
            }
        }
        CodegenExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_expression_runtime_helpers(condition, output);
            collect_expression_runtime_helpers(then_branch, output);
            if let Some(else_branch) = else_branch {
                collect_expression_runtime_helpers(else_branch, output);
            }
        }
        CodegenExpressionKind::Match {
            scrutinee,
            arms,
            else_branch,
        } => {
            collect_expression_runtime_helpers(scrutinee, output);
            for arm in arms {
                for pattern in &arm.patterns {
                    collect_expression_runtime_helpers(pattern, output);
                }
                collect_expression_runtime_helpers(&arm.body, output);
            }
            if let Some(else_branch) = else_branch {
                collect_expression_runtime_helpers(else_branch, output);
            }
        }
        CodegenExpressionKind::Let { value, body, .. } => {
            collect_expression_runtime_helpers(value, output);
            collect_expression_runtime_helpers(body, output);
        }
        CodegenExpressionKind::Block {
            statements,
            expression,
        } => {
            for statement in statements {
                match statement {
                    CodegenStatement::Let { init, .. } => {
                        collect_expression_runtime_helpers(init, output);
                    }
                    CodegenStatement::Expr(expr) => {
                        collect_expression_runtime_helpers(expr, output);
                    }
                }
            }
            if let Some(expression) = expression {
                collect_expression_runtime_helpers(expression, output);
            }
        }
        CodegenExpressionKind::Index { base, index } => {
            collect_expression_runtime_helpers(base, output);
            collect_expression_runtime_helpers(index, output);
        }
        CodegenExpressionKind::Member { base, .. } => {
            collect_expression_runtime_helpers(base, output);
        }
        CodegenExpressionKind::Record {
            fields, properties, ..
        } => {
            for field in fields {
                if let Some(default) = field.default.as_ref() {
                    collect_expression_runtime_helpers(default, output);
                }
            }
            for property in properties {
                collect_expression_runtime_helpers(&property.value, output);
            }
        }
        CodegenExpressionKind::UnionCase {
            fields,
            properties,
            content,
            ..
        } => {
            for field in fields {
                if let Some(default) = field.default.as_ref() {
                    collect_expression_runtime_helpers(default, output);
                }
            }
            for property in properties {
                collect_expression_runtime_helpers(&property.value, output);
            }
            for expr in content {
                collect_expression_runtime_helpers(expr, output);
            }
        }
        CodegenExpressionKind::ComponentDescriptor(descriptor) => {
            for property in &descriptor.properties {
                collect_expression_runtime_helpers(&property.value, output);
            }
            for content in &descriptor.content {
                collect_expression_runtime_helpers(content, output);
            }
        }
    }
}

fn should_import_value_reference(kind: ResolvedItemKind) -> bool {
    matches!(
        kind,
        ResolvedItemKind::Function
            | ResolvedItemKind::Value
            | ResolvedItemKind::Enum
            | ResolvedItemKind::Component
    )
}

fn is_type_reference_kind(kind: ResolvedItemKind) -> bool {
    matches!(
        kind,
        ResolvedItemKind::Enum
            | ResolvedItemKind::Record
            | ResolvedItemKind::Union
            | ResolvedItemKind::TypeAlias
            | ResolvedItemKind::Component
    )
}

fn emit_literal(literal: &Literal) -> String {
    match literal {
        Literal::String(value) => js_string(value.as_str()),
        Literal::Int(value) => value.to_string(),
        Literal::Float(value) => {
            if value.0.is_finite() {
                value.0.to_string()
            } else {
                "null".to_string()
            }
        }
        Literal::Boolean(value) => value.to_string(),
        Literal::Null => "null".to_string(),
    }
}

fn binop_text(op: BinOp) -> &'static str {
    match op {
        BinOp::Add | BinOp::Concat => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Eq => "===",
        BinOp::Ne => "!==",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
    }
}

fn unop_text(op: UnOp) -> &'static str {
    match op {
        UnOp::Neg => "-",
        UnOp::Not => "!",
    }
}

fn module_file_stem(module: &CodegenModule) -> String {
    let source_hint = match &module.provenance {
        CodegenModuleProvenance::SourceProvider { identity } => Path::new(identity)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(identity),
        CodegenModuleProvenance::Library { module_path, .. } => module_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("library"),
    };
    format!(
        "{}_{}",
        module_prefix(module.id),
        safe_identifier(source_hint)
    )
}

fn runtime_import_file(target: CodegenTarget) -> String {
    import_file(&format!("nx-runtime.{}", target.extension()), target)
}

fn import_file(file_name: &str, target: CodegenTarget) -> String {
    if target.is_typescript() && file_name.ends_with(".ts") {
        format!("{}.js", file_name.trim_end_matches(".ts"))
    } else {
        file_name.to_string()
    }
}

fn module_prefix(module_id: RuntimeModuleId) -> String {
    format!("m{}", module_id.as_u32())
}

fn unique_identifier(base: String, used_names: &mut FxHashSet<String>) -> String {
    if used_names.insert(base.clone()) {
        return base;
    }

    let mut disambiguator = 2;
    loop {
        let candidate = format!("{}_{}", base, disambiguator);
        if used_names.insert(candidate.clone()) {
            return candidate;
        }
        disambiguator += 1;
    }
}

fn union_case_type_name(union_name: &str, case_name: &str) -> String {
    format!(
        "{}_{}",
        safe_identifier(union_name),
        safe_identifier(case_name)
    )
}

fn safe_identifier(name: &str) -> String {
    let mut identifier = String::new();
    for (index, ch) in name.chars().enumerate() {
        if (index == 0 && (ch.is_ascii_alphabetic() || ch == '_'))
            || (index > 0 && (ch.is_ascii_alphanumeric() || ch == '_'))
        {
            identifier.push(ch);
        } else if ch == '.' || ch == '-' || ch == '/' || ch == '\\' || ch.is_whitespace() {
            identifier.push('_');
        }
    }

    if identifier.is_empty() {
        identifier.push('_');
    }
    if identifier
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_digit())
    {
        identifier.insert(0, '_');
    }
    if is_reserved_word(&identifier) {
        identifier.push('_');
    }
    identifier
}

fn safe_object_key(name: &str) -> String {
    let identifier = safe_identifier(name);
    if identifier == name && !is_reserved_word(name) {
        identifier
    } else {
        js_string(name)
    }
}

fn is_reserved_word(identifier: &str) -> bool {
    matches!(
        identifier,
        "await"
            | "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "export"
            | "extends"
            | "finally"
            | "for"
            | "function"
            | "if"
            | "import"
            | "in"
            | "instanceof"
            | "let"
            | "new"
            | "return"
            | "super"
            | "switch"
            | "this"
            | "throw"
            | "try"
            | "typeof"
            | "var"
            | "void"
            | "while"
            | "with"
            | "yield"
    )
}

fn js_string(value: &str) -> String {
    format!("{:?}", value)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ReferenceKey {
    module_id: RuntimeModuleId,
    definition_index: usize,
}

impl ReferenceKey {
    fn new(reference: &CodegenReference) -> Self {
        Self {
            module_id: reference.module_id,
            definition_index: reference.definition_id.index(),
        }
    }
}
