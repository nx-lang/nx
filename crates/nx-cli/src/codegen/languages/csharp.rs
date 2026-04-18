use crate::codegen::model::{
    ExportedAlias, ExportedEnum, ExportedExternalState, ExportedModule, ExportedRecord,
    ExportedRecordField, ExportedType, ExportedTypeGraph, ImportedType,
};
use crate::codegen::writer::CodeWriter;
use crate::codegen::{GenerateTypesOptions, GeneratedFile};
use nx_hir::ast::TypeRef;
use rustc_hash::FxHashMap;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub fn emit_single_file(
    graph: &ExportedTypeGraph,
    namespace: &str,
    opts: &GenerateTypesOptions,
) -> Result<String, String> {
    let module = graph
        .modules
        .first()
        .ok_or_else(|| "Single-file generation requires one source module".to_string())?;
    Ok(render_module(graph, module, namespace, opts))
}

pub fn emit_library(
    graph: &ExportedTypeGraph,
    namespace: &str,
    opts: &GenerateTypesOptions,
) -> Result<Vec<GeneratedFile>, String> {
    let mut files = Vec::new();

    for module in &graph.modules {
        files.push(GeneratedFile {
            relative_path: module_output_path(&module.module_path),
            content: render_module(graph, module, namespace, opts),
        });
    }

    Ok(files)
}

pub(crate) fn collect_warnings(graph: &ExportedTypeGraph, namespace: &str) -> Vec<String> {
    let mut warnings = Vec::new();
    let mut warned_dependency_namespaces = BTreeSet::new();

    for module in &graph.modules {
        for imported_type in &module.imported_types {
            let assumed_namespace =
                assumed_dependency_namespace_for_library(namespace, &imported_type.library_name);
            if warned_dependency_namespaces.insert((
                imported_type.library_name.clone(),
                assumed_namespace.clone(),
            )) {
                warnings.push(format!(
                    "Generated C# cross-library references for dependency '{}' assume namespace '{}' derived from the dependency directory name. If that library was generated with a different --csharp-namespace, regenerate with matching namespaces or update the generated namespace manually.",
                    imported_type.library_name, assumed_namespace
                ));
            }
        }

        for declaration in &module.declarations {
            let ExportedType::Record(record) = &declaration.item else {
                continue;
            };

            if !record.is_abstract || graph.resolved_record_base(record).is_some() {
                continue;
            }

            if !graph.concrete_descendants(&record.name).is_empty() {
                continue;
            }

            warnings.push(format!(
                "Generated C# abstract type '{}' has no concrete exported descendants; omitting JsonPolymorphic metadata because System.Text.Json requires at least one derived type registration.",
                record.name
            ));
        }
    }

    warnings
}

fn render_module(
    graph: &ExportedTypeGraph,
    module: &ExportedModule,
    namespace: &str,
    opts: &GenerateTypesOptions,
) -> String {
    let mut writer = CodeWriter::new(opts.format.clone());
    write_header(&mut writer);

    if module.declarations.is_empty() {
        return writer.finish();
    }

    let aliases = module
        .declarations
        .iter()
        .filter_map(|declaration| match &declaration.item {
            ExportedType::Alias(alias) => Some(alias),
            _ => None,
        })
        .collect::<Vec<_>>();
    let imported_type_lookup = module
        .imported_types
        .iter()
        .cloned()
        .map(|imported_type| (imported_type.visible_name.clone(), imported_type))
        .collect::<FxHashMap<_, _>>();
    let dependency_usings = collect_dependency_namespaces(&module.imported_types, namespace);

    let global_context = CSharpRenderContext {
        namespace,
        graph,
        imported_types_by_visible_name: &imported_type_lookup,
        qualify_generated_types: true,
    };
    let namespace_context = CSharpRenderContext {
        namespace,
        graph,
        imported_types_by_visible_name: &imported_type_lookup,
        qualify_generated_types: false,
    };

    for alias in &aliases {
        emit_alias(&mut writer, alias, &global_context);
    }

    if !aliases.is_empty() {
        writer.blank_line();
    }

    let needs_json_serialization = module
        .declarations
        .iter()
        .any(|declaration| !matches!(declaration.item, ExportedType::Alias(_)));
    let needs_enum_serialization_helpers = module
        .declarations
        .iter()
        .any(|declaration| matches!(declaration.item, ExportedType::Enum(_)));

    writer.line("using System;");
    if needs_json_serialization {
        writer.line("using System.Text.Json.Serialization;");
    }
    writer.line("using MessagePack;");
    if needs_enum_serialization_helpers {
        writer.line("using NxLang.Nx.Serialization;");
    }

    for dependency_namespace in &dependency_usings {
        writer.line(&format!(
            "using {};",
            sanitize_csharp_qualified_name(dependency_namespace)
        ));
    }

    writer.blank_line();

    let body_items = module
        .declarations
        .iter()
        .filter(|declaration| !matches!(declaration.item, ExportedType::Alias(_)))
        .collect::<Vec<_>>();

    if !body_items.is_empty() {
        writer.block(
            &format!("namespace {}", sanitize_csharp_qualified_name(namespace)),
            |writer| {
                for (index, declaration) in body_items.iter().enumerate() {
                    emit_declaration(writer, &declaration.item, &namespace_context);
                    if index + 1 != body_items.len() {
                        writer.blank_line();
                    }
                }
            },
        );
    }

    writer.finish()
}

fn write_header(writer: &mut CodeWriter) {
    writer.line("// <auto-generated/>");
    writer.line("// Generated by nxlang");
    writer.line("#nullable enable");
    writer.line("#pragma warning disable MsgPack005");
    writer.blank_line();
}

fn emit_alias(writer: &mut CodeWriter, alias: &ExportedAlias, context: &CSharpRenderContext<'_>) {
    let alias_name = sanitize_csharp_identifier(&alias.name);
    let target_type = csharp_type(&alias.target, context);
    writer.line(&format!(
        "global using {alias_name} = {};",
        target_type.text
    ));
}

fn emit_declaration(
    writer: &mut CodeWriter,
    declaration: &ExportedType,
    context: &CSharpRenderContext<'_>,
) {
    match declaration {
        ExportedType::Alias(_) => {}
        ExportedType::Enum(enum_def) => emit_enum(writer, enum_def),
        ExportedType::Record(record) => emit_record(writer, record, context),
        ExportedType::ExternalState(state) => emit_external_state(writer, state, context),
    }
}

fn emit_enum(writer: &mut CodeWriter, enum_def: &ExportedEnum) {
    let enum_name = sanitize_csharp_identifier(&enum_def.name);
    writer.line(&format!(
        "[JsonConverter(typeof(NxEnumJsonConverter<{enum_name}, {enum_name}WireFormat>))]"
    ));
    writer.line(&format!(
        "[MessagePackFormatter(typeof(NxEnumMessagePackFormatter<{enum_name}, {enum_name}WireFormat>))]"
    ));
    writer.block(&format!("public enum {enum_name}"), |writer| {
        for (index, member) in enum_def.members.iter().enumerate() {
            let comma = if index + 1 == enum_def.members.len() {
                ""
            } else {
                ","
            };
            writer.line(&format!("{}{}", sanitize_csharp_member_name(member), comma));
        }
    });

    writer.blank_line();
    emit_enum_wire_format(writer, enum_def);
}

fn emit_record(
    writer: &mut CodeWriter,
    record: &ExportedRecord,
    context: &CSharpRenderContext<'_>,
) {
    emit_record_json_polymorphism_attributes(writer, record, context);

    if record.is_abstract {
        for (index, descendant) in context
            .graph
            .concrete_descendants(&record.name)
            .iter()
            .enumerate()
        {
            writer.line(&format!(
                "[Union({index}, typeof({}))]",
                sanitize_csharp_identifier(&descendant.name)
            ));
        }
    }

    if should_emit_missing_polymorphism_hint(record, context) {
        writer.line("// No JsonPolymorphic metadata was generated because this abstract type had");
        writer.line("// no concrete exported descendants at code-generation time.");
    }

    writer.line("[MessagePackObject]");
    let class_modifier = if record.is_abstract {
        "public abstract class"
    } else {
        "public sealed class"
    };
    let header = if let Some(base) = &record.base {
        format!(
            "{} {} : {}",
            class_modifier,
            sanitize_csharp_identifier(&record.name),
            csharp_type_name(base, context).text
        )
    } else {
        format!(
            "{} {}",
            class_modifier,
            sanitize_csharp_identifier(&record.name)
        )
    };

    writer.block(&header, |writer| {
        emit_record_fields(writer, &record.fields, context);
    });
}

fn emit_external_state(
    writer: &mut CodeWriter,
    state: &ExportedExternalState,
    context: &CSharpRenderContext<'_>,
) {
    writer.line("[MessagePackObject]");
    writer.block(
        &format!(
            "public sealed class {}",
            sanitize_csharp_identifier(&state.name)
        ),
        |writer| {
            emit_record_fields(writer, &state.fields, context);
        },
    );
}

fn emit_record_json_polymorphism_attributes(
    writer: &mut CodeWriter,
    record: &ExportedRecord,
    context: &CSharpRenderContext<'_>,
) {
    if !should_emit_json_polymorphism_attributes(record, context) {
        return;
    }

    writer.line("[JsonPolymorphic(TypeDiscriminatorPropertyName = \"$type\")]");
    for descendant in context.graph.concrete_descendants(&record.name) {
        writer.line(&format!(
            "[JsonDerivedType(typeof({}), \"{}\")]",
            sanitize_csharp_identifier(&descendant.name),
            escape_csharp_string_literal(&descendant.name)
        ));
    }
}

fn should_emit_json_polymorphism_attributes(
    record: &ExportedRecord,
    context: &CSharpRenderContext<'_>,
) -> bool {
    record.is_abstract
        && !graph_resolves_record_base(context, record)
        && !context.graph.concrete_descendants(&record.name).is_empty()
}

fn should_emit_missing_polymorphism_hint(
    record: &ExportedRecord,
    context: &CSharpRenderContext<'_>,
) -> bool {
    record.is_abstract
        && !graph_resolves_record_base(context, record)
        && context.graph.concrete_descendants(&record.name).is_empty()
}

fn emit_dual_wire_name_attributes(writer: &mut CodeWriter, name: &str) {
    let escaped_name = escape_csharp_string_literal(name);
    writer.line(&format!("[Key(\"{escaped_name}\")]"));
    writer.line(&format!("[JsonPropertyName(\"{escaped_name}\")]"));
}

fn emit_record_fields(
    writer: &mut CodeWriter,
    fields: &[ExportedRecordField],
    context: &CSharpRenderContext<'_>,
) {
    let mut needs_leading_blank_line = false;

    for field in fields {
        let field_type = csharp_type(&field.ty, context);
        let field_name = sanitize_csharp_member_name(&field.name);

        let property_declaration = if field_type.is_reference && !field_type.is_nullable {
            format!(
                "public {} {} {{ get; set; }} = default!;",
                field_type.text, field_name
            )
        } else {
            format!("public {} {} {{ get; set; }}", field_type.text, field_name)
        };

        emit_dual_annotated_auto_property(
            writer,
            &field.name,
            &property_declaration,
            needs_leading_blank_line,
        );
        needs_leading_blank_line = true;
    }
}

fn emit_dual_annotated_auto_property(
    writer: &mut CodeWriter,
    wire_name: &str,
    declaration: &str,
    has_emitted_property: bool,
) {
    if has_emitted_property {
        writer.blank_line();
    }

    emit_dual_wire_name_attributes(writer, wire_name);
    writer.line(declaration);
}

fn emit_enum_wire_format(writer: &mut CodeWriter, enum_def: &ExportedEnum) {
    let enum_name = sanitize_csharp_identifier(&enum_def.name);
    writer.block(
        &format!("internal sealed class {enum_name}WireFormat : INxEnumWireFormat<{enum_name}>"),
        |writer| {
            writer.line(&format!(
                "public static string Format({enum_name} value) =>"
            ));
            writer.indent();
            writer.line("value switch");
            writer.line("{");
            writer.indent();
            for member in &enum_def.members {
                let member_literal = escape_csharp_string_literal(member);
                let member_ident = sanitize_csharp_member_name(member);
                writer.line(&format!(
                    "{enum_name}.{member_ident} => \"{member_literal}\","
                ));
            }
            writer.line("_ => throw new FormatException(\"Unknown NX enum value.\"),");
            writer.dedent();
            writer.line("};");
            writer.dedent();

            writer.blank_line();

            writer.line(&format!("public static {enum_name} Parse(string value) =>"));
            writer.indent();
            writer.line("value switch");
            writer.line("{");
            writer.indent();
            for member in &enum_def.members {
                let member_literal = escape_csharp_string_literal(member);
                let member_ident = sanitize_csharp_member_name(member);
                writer.line(&format!(
                    "\"{member_literal}\" => {enum_name}.{member_ident},"
                ));
            }
            writer.line("_ => throw new FormatException(\"Unknown NX enum member.\"),");
            writer.dedent();
            writer.line("};");
            writer.dedent();
        },
    );
}

#[derive(Clone, Debug)]
struct CSharpType {
    text: String,
    is_reference: bool,
    is_nullable: bool,
}

struct CSharpRenderContext<'a> {
    namespace: &'a str,
    graph: &'a ExportedTypeGraph,
    imported_types_by_visible_name: &'a FxHashMap<String, ImportedType>,
    qualify_generated_types: bool,
}

fn graph_resolves_record_base(context: &CSharpRenderContext<'_>, record: &ExportedRecord) -> bool {
    context.graph.resolved_record_base(record).is_some()
}

fn csharp_type(ty: &TypeRef, context: &CSharpRenderContext<'_>) -> CSharpType {
    let mut seen_aliases = BTreeSet::new();
    csharp_type_inner(ty, context, &mut seen_aliases)
}

fn csharp_type_inner(
    ty: &TypeRef,
    context: &CSharpRenderContext<'_>,
    seen_aliases: &mut BTreeSet<String>,
) -> CSharpType {
    match ty {
        TypeRef::Nullable(inner) => {
            let mut inner = csharp_type_inner(inner, context, seen_aliases);
            inner.text = format!("{}?", inner.text);
            inner.is_nullable = true;
            inner
        }
        TypeRef::Array(inner) => {
            let inner = csharp_type_inner(inner, context, seen_aliases);
            CSharpType {
                text: format!("{}[]", inner.text),
                is_reference: true,
                is_nullable: false,
            }
        }
        TypeRef::Function { .. } => CSharpType {
            text: "global::System.Delegate".to_string(),
            is_reference: true,
            is_nullable: false,
        },
        TypeRef::Name(name) => csharp_type_name_inner(name.as_str(), context, seen_aliases),
    }
}

fn csharp_type_name(name: &str, context: &CSharpRenderContext<'_>) -> CSharpType {
    let mut seen_aliases = BTreeSet::new();
    csharp_type_name_inner(name, context, &mut seen_aliases)
}

fn csharp_type_name_inner(
    name: &str,
    context: &CSharpRenderContext<'_>,
    seen_aliases: &mut BTreeSet<String>,
) -> CSharpType {
    match name {
        "string" => CSharpType {
            text: "string".to_string(),
            is_reference: true,
            is_nullable: false,
        },
        "i32" => CSharpType {
            text: "int".to_string(),
            is_reference: false,
            is_nullable: false,
        },
        "i64" | "int" => CSharpType {
            text: "long".to_string(),
            is_reference: false,
            is_nullable: false,
        },
        "f32" => CSharpType {
            text: "float".to_string(),
            is_reference: false,
            is_nullable: false,
        },
        "f64" | "float" => CSharpType {
            text: "double".to_string(),
            is_reference: false,
            is_nullable: false,
        },
        "bool" => CSharpType {
            text: "bool".to_string(),
            is_reference: false,
            is_nullable: false,
        },
        "void" => CSharpType {
            text: "void".to_string(),
            is_reference: false,
            is_nullable: false,
        },
        "object" | "unknown" | "error" => CSharpType {
            text: "object".to_string(),
            is_reference: true,
            is_nullable: false,
        },
        other => {
            if let Some(declaration) = context.graph.declaration(other) {
                match &declaration.item {
                    ExportedType::Alias(alias) => {
                        if !seen_aliases.insert(other.to_string()) {
                            return CSharpType {
                                text: sanitize_csharp_identifier(other),
                                is_reference: true,
                                is_nullable: false,
                            };
                        }

                        let mut alias_type =
                            csharp_type_inner(&alias.target, context, seen_aliases);
                        seen_aliases.remove(other);
                        alias_type.text = sanitize_csharp_identifier(other);
                        alias_type
                    }
                    ExportedType::Enum(_) => CSharpType {
                        text: generated_type_name(
                            other,
                            context.namespace,
                            context.qualify_generated_types,
                        ),
                        is_reference: false,
                        is_nullable: false,
                    },
                    ExportedType::Record(_) => CSharpType {
                        text: generated_type_name(
                            other,
                            context.namespace,
                            context.qualify_generated_types,
                        ),
                        is_reference: true,
                        is_nullable: false,
                    },
                    ExportedType::ExternalState(_) => CSharpType {
                        text: generated_type_name(
                            other,
                            context.namespace,
                            context.qualify_generated_types,
                        ),
                        is_reference: true,
                        is_nullable: false,
                    },
                }
            } else if let Some(imported_type) = context.imported_types_by_visible_name.get(other) {
                let dependency_namespace = assumed_dependency_namespace_for_library(
                    context.namespace,
                    &imported_type.library_name,
                );
                CSharpType {
                    text: generated_type_name(
                        &imported_type.exported_name,
                        &dependency_namespace,
                        true,
                    ),
                    is_reference: imported_type.is_reference,
                    is_nullable: false,
                }
            } else {
                CSharpType {
                    text: sanitize_csharp_qualified_name(other),
                    is_reference: true,
                    is_nullable: false,
                }
            }
        }
    }
}

fn generated_type_name(name: &str, namespace: &str, qualify: bool) -> String {
    let identifier = sanitize_csharp_identifier(name);
    if qualify {
        format!(
            "global::{}.{}",
            sanitize_csharp_qualified_name(namespace),
            identifier
        )
    } else {
        identifier
    }
}

fn collect_dependency_namespaces(
    imported_types: &[ImportedType],
    current_namespace: &str,
) -> BTreeSet<String> {
    imported_types
        .iter()
        .map(|imported_type| {
            assumed_dependency_namespace_for_library(current_namespace, &imported_type.library_name)
        })
        .filter(|dependency_namespace| dependency_namespace != current_namespace)
        .collect()
}

// Cross-library C# references currently assume sibling namespaces derived from dependency
// directory names because nx modules do not publish an explicit external namespace mapping yet.
fn assumed_dependency_namespace_for_library(current_namespace: &str, library_name: &str) -> String {
    let dependency_segment = sanitize_csharp_member_name(library_name);
    let mut namespace_parts = current_namespace
        .split('.')
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();

    if !namespace_parts.is_empty() {
        namespace_parts.pop();
    }

    namespace_parts.push(dependency_segment);
    namespace_parts.join(".")
}

fn module_output_path(module_path: &Path) -> PathBuf {
    module_path.with_extension("g.cs")
}

fn sanitize_csharp_qualified_name(name: &str) -> String {
    name.split('.')
        .filter(|part| !part.is_empty())
        .map(sanitize_csharp_identifier)
        .collect::<Vec<_>>()
        .join(".")
}

fn sanitize_csharp_identifier(name: &str) -> String {
    let mut out = String::new();

    for (index, ch) in name.chars().enumerate() {
        let valid = if index == 0 {
            ch == '_' || ch.is_ascii_alphabetic()
        } else {
            ch == '_' || ch.is_ascii_alphanumeric()
        };
        out.push(if valid { ch } else { '_' });
    }

    if out.is_empty() {
        "_".to_string()
    } else if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        format!("_{out}")
    } else {
        out
    }
}

fn sanitize_csharp_member_name(name: &str) -> String {
    let mut out = String::new();
    let mut capitalize_next = true;

    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            if capitalize_next {
                out.push(ch.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                out.push(ch);
            }
        } else {
            capitalize_next = true;
        }
    }

    sanitize_csharp_identifier(&out)
}

fn escape_csharp_string_literal(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\"', "\\\"")
}
