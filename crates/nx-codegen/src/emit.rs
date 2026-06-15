use crate::builder::build_codegen_program;
use crate::model::{
    CodegenDeclaration, CodegenDeclarationKind, CodegenElement, CodegenExpression,
    CodegenExpressionKind, CodegenModule, CodegenModuleProvenance, CodegenProgram, CodegenProperty,
    CodegenRecordField, CodegenReference, CodegenStatement, CodegenUnionCase,
};
use crate::options::{CodegenError, CodegenOptions, CodegenOutput, CodegenTarget, GeneratedFile};
use crate::runtime::runtime_helper_source;
use nx_api::ProgramArtifact;
use nx_hir::ast::{BinOp, Literal, TypeRef, UnOp};
use nx_interpreter::{ResolvedItemKind, RuntimeModuleId};
use nx_types::{Primitive, Type};
use rustc_hash::{FxHashMap, FxHashSet};
use std::path::{Path, PathBuf};

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

struct EmitContext {
    module_files: FxHashMap<RuntimeModuleId, String>,
    declaration_names: FxHashMap<ReferenceKey, String>,
    import_aliases: FxHashMap<ReferenceKey, String>,
}

impl EmitContext {
    fn new(program: &CodegenProgram, target: CodegenTarget) -> Self {
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

        let mut declaration_names = FxHashMap::default();
        for module in &program.modules {
            let mut used_names = FxHashSet::default();
            for declaration in &module.declarations {
                let name = unique_identifier(
                    safe_identifier(&declaration.reference.name),
                    &mut used_names,
                );
                declaration_names.insert(ReferenceKey::new(&declaration.reference), name);
            }
        }

        let mut import_aliases = FxHashMap::default();
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

        Self {
            module_files,
            declaration_names,
            import_aliases,
        }
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
        if reference.module_id == current_module_id {
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
    let runtime_helpers = collect_module_runtime_helpers(module);
    if !runtime_helpers.is_empty() {
        out.push_str(&format!(
            "import {{ {} }} from \"./{}\";\n",
            runtime_helpers.join(", "),
            runtime_import_file(target)
        ));
    }

    let value_imports = collect_module_value_references(module);
    for reference in &value_imports {
        let alias = context.reference_name(module.id, reference);
        let source_file = context.module_file(reference.module_id);
        out.push_str(&format!(
            "import {{ {} as {} }} from \"./{}\";\n",
            context.declaration_name(reference),
            alias,
            import_file(source_file, target)
        ));
    }
    if target.is_typescript() {
        let value_keys = value_imports
            .iter()
            .map(ReferenceKey::new)
            .collect::<FxHashSet<_>>();
        for reference in collect_module_type_references(module)
            .into_iter()
            .filter(|reference| !value_keys.contains(&ReferenceKey::new(reference)))
        {
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
        emit_declaration(module, declaration, context, target, &mut out);
        out.push('\n');
    }

    out
}

fn emit_declaration(
    module: &CodegenModule,
    declaration: &CodegenDeclaration,
    context: &EmitContext,
    target: CodegenTarget,
    out: &mut String,
) {
    let name = context.declaration_name(&declaration.reference);
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
                    "export function {}({}): {} {{\n",
                    name,
                    params,
                    return_type
                        .as_ref()
                        .map(|ty| emit_type(module.id, ty, module, context))
                        .unwrap_or_else(|| "unknown".to_string())
                ));
            } else {
                out.push_str(&format!("export function {}({}) {{\n", name, params));
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
                    "export const {}: {} = {};\n",
                    name,
                    ty.as_ref()
                        .map(|ty| emit_type(module.id, ty, module, context))
                        .unwrap_or_else(|| "unknown".to_string()),
                    emit_expression(module.id, value, context)
                ));
            } else {
                out.push_str(&format!(
                    "export const {} = {};\n",
                    name,
                    emit_expression(module.id, value, context)
                ));
            }
        }
        CodegenDeclarationKind::Enum { members } => {
            if target.is_typescript() {
                out.push_str(&format!("export const {} = {{\n", name));
                for member in members {
                    out.push_str(&format!(
                        "  {}: {},\n",
                        safe_object_key(member),
                        js_string(member)
                    ));
                }
                out.push_str("} as const;\n\n");
                out.push_str(&format!(
                    "export type {} = typeof {}[keyof typeof {}];\n",
                    name, name, name
                ));
            } else {
                out.push_str(&format!("export const {} = Object.freeze({{\n", name));
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
                    out,
                );
                out.push('\n');
            }
        }
        CodegenDeclarationKind::Union { cases } => {
            if target.is_typescript() {
                emit_union_type(
                    &name,
                    &declaration.reference.name,
                    cases,
                    module,
                    context,
                    out,
                );
                out.push('\n');
            }
        }
        CodegenDeclarationKind::TypeAlias => {}
        CodegenDeclarationKind::Unsupported(unsupported) => {
            out.push_str(&format!(
                "export const {} = nxRuntimeError({});\n",
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
    out: &mut String,
) {
    out.push_str(&format!("export type {} = {{\n", name));
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
            out,
        );
        out.push('\n');
    }
    let union = if case_type_names.is_empty() {
        "never".to_string()
    } else {
        case_type_names.join(" | ")
    };
    out.push_str(&format!("export type {} = {};\n", name, union));
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
    module: &CodegenModule,
    context: &EmitContext,
) -> String {
    match name {
        "i32" | "i64" | "int" | "f32" | "f64" | "float" => "number".to_string(),
        "string" => "string".to_string(),
        "bool" => "boolean".to_string(),
        "void" => "void".to_string(),
        _ => module
            .imports
            .iter()
            .find(|reference| reference.name == name && is_type_reference_kind(reference.kind))
            .map(|reference| context.reference_name(current_module_id, reference))
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
        CodegenDeclarationKind::Enum { .. }
        | CodegenDeclarationKind::TypeAlias
        | CodegenDeclarationKind::Unsupported(_) => {}
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

fn collect_module_runtime_helpers(module: &CodegenModule) -> Vec<&'static str> {
    let mut helpers = FxHashSet::default();
    for declaration in &module.declarations {
        collect_declaration_runtime_helpers(declaration, &mut helpers);
    }
    let mut output = ["nxElement", "nxRuntimeError"]
        .into_iter()
        .filter(|helper| helpers.contains(helper))
        .collect::<Vec<_>>();
    output.sort();
    output
}

fn collect_declaration_runtime_helpers(
    declaration: &CodegenDeclaration,
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
        CodegenDeclarationKind::Enum { .. }
        | CodegenDeclarationKind::Record { .. }
        | CodegenDeclarationKind::Union { .. }
        | CodegenDeclarationKind::TypeAlias => {}
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
    }
}

fn should_import_value_reference(kind: ResolvedItemKind) -> bool {
    matches!(
        kind,
        ResolvedItemKind::Function | ResolvedItemKind::Value | ResolvedItemKind::Enum
    )
}

fn is_type_reference_kind(kind: ResolvedItemKind) -> bool {
    matches!(
        kind,
        ResolvedItemKind::Enum
            | ResolvedItemKind::Record
            | ResolvedItemKind::Union
            | ResolvedItemKind::TypeAlias
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
