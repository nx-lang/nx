use crate::typegen::model::{
    erase_field_type_parameters, ExportedExternalState, ExportedFieldDefault,
    ExportedLiteralDefault, ExportedModule, ExportedPolymorphicDescendant, ExportedRecord,
    ExportedRecordField, ExportedType, ExportedTypeGraph, ExportedUnion, ExportedUnionCase,
    ExportedUpdate, ImportedType, ImportedTypeKind,
};
use crate::typegen::writer::CodeWriter;
use crate::typegen::{GenerateTypesOptions, GeneratedFile};
use nx_hir::ast::TypeRef;
use rustc_hash::FxHashMap;
use std::borrow::Cow;
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
            if !imported_type_uses_dependency_namespace(imported_type) {
                continue;
            }

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

            if !graph.polymorphic_descendants(&record.name).is_empty() {
                continue;
            }

            warnings.push(format!(
                "Generated C# abstract type '{}' has no concrete exported descendants; omitting polymorphism metadata (JSON and MessagePack) because derived type registrations are required.",
                record.name
            ));
        }

        for declaration in &module.declarations {
            let ExportedType::Update(update) = &declaration.item else {
                continue;
            };
            let Some(candidate) = update_plain_target_candidate(update, graph) else {
                continue;
            };
            if graph.declaration(&candidate.properties_table).is_some() {
                warnings.push(format!(
                    "Generated C# property-key table '{}' for '{}' was omitted because it conflicts with exported declaration '{}'; '{}' derives from NxUpdateRecord without keys, Apply, or Diff.",
                    candidate.properties_table, update.target_name, candidate.properties_table, update.name
                ));
            }
        }

        for declaration in &module.declarations {
            match &declaration.item {
                ExportedType::Record(record) => collect_unsupported_default_warnings(
                    &mut warnings,
                    &record.name,
                    &record.fields,
                ),
                ExportedType::Union(union_def) => {
                    for case in &union_def.cases {
                        collect_unsupported_default_warnings(
                            &mut warnings,
                            &format!("{}.{}", union_def.name, case.name),
                            &case.fields,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    warnings
}

fn collect_unsupported_default_warnings(
    warnings: &mut Vec<String>,
    owner_name: &str,
    fields: &[ExportedRecordField],
) {
    for field in fields {
        if matches!(field.default_value, Some(ExportedFieldDefault::Unsupported)) {
            warnings.push(format!(
                "Generated C# omitted default for '{}.{}' because only literal defaults can be emitted as property initializers.",
                owner_name, field.name
            ));
        }
    }
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

    let body_items = module
        .declarations
        .iter()
        .filter(|declaration| !matches!(declaration.item, ExportedType::Alias(_)))
        .collect::<Vec<_>>();

    if body_items.is_empty() {
        return writer.finish();
    }

    let imported_type_lookup = module
        .imported_types
        .iter()
        .cloned()
        .map(|imported_type| (imported_type.visible_name.clone(), imported_type))
        .collect::<FxHashMap<_, _>>();
    let dependency_usings = collect_dependency_namespaces(&module.imported_types, namespace);

    let namespace_context = CSharpRenderContext {
        namespace,
        graph,
        imported_types_by_visible_name: &imported_type_lookup,
        qualify_generated_types: false,
    };

    let needs_enum_serialization_helpers = module
        .declarations
        .iter()
        .any(|declaration| {
            matches!(&declaration.item, ExportedType::Union(union_def) if union_def.is_constant())
        });
    let needs_polymorphic_serialization_helpers = module.declarations.iter().any(|declaration| {
        matches!(&declaration.item, ExportedType::Union(union_def) if !union_def.is_constant())
            || match &declaration.item {
                ExportedType::Record(record) => {
                    polymorphic_message_pack_root_name(record, &namespace_context).is_some()
                }
                _ => false,
            }
    });

    let needs_update_helpers = module
        .declarations
        .iter()
        .any(|declaration| matches!(&declaration.item, ExportedType::Update(_)));

    writer.line("using System;");
    writer.line("using System.Text.Json.Serialization;");
    writer.line("using MessagePack;");
    if needs_update_helpers {
        writer.line("using NxLang.Nx;");
    }
    if needs_enum_serialization_helpers
        || needs_polymorphic_serialization_helpers
        || needs_update_helpers
    {
        writer.line("using NxLang.Nx.Serialization;");
    }

    for dependency_namespace in &dependency_usings {
        writer.line(&format!(
            "using {};",
            sanitize_csharp_qualified_name(dependency_namespace)
        ));
    }

    writer.blank_line();

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

    writer.finish()
}

fn write_header(writer: &mut CodeWriter) {
    writer.line("// <auto-generated/>");
    writer.line("// Generated by nxlang");
    writer.line("#nullable enable");
    writer.line("#pragma warning disable MsgPack005");
    writer.blank_line();
}

fn emit_declaration(
    writer: &mut CodeWriter,
    declaration: &ExportedType,
    context: &CSharpRenderContext<'_>,
) {
    match declaration {
        ExportedType::Alias(_) => {}
        ExportedType::Union(union_def) => emit_union(writer, union_def, context),
        ExportedType::Record(record) => emit_record(writer, record, context),
        ExportedType::ExternalState(state) => emit_external_state(writer, state, context),
        ExportedType::Update(update) => emit_update(writer, update, context),
    }
}

/// Emits an update companion as a map-backed DTO over the SDK's `NxUpdateRecord`.
///
/// <para>The class derives from `NxUpdate<Target>` when a plain type is emitted for the target — a
/// record or an action that is not abstract — and from `NxUpdateRecord` otherwise, which is what a
/// component's state companion gets. Either way it passes its schema to the base: the target's
/// property keys from the `<Target>Properties` table, or a plain name→type table. Each field is an
/// `NxOptional<T>` accessor pair over the base's map, so `new User_update { Email = null }` still
/// means "set email to null". The schema-driven SDK converter and formatter named on the class
/// write `$type` and only the set fields, so no member needs a wire attribute.</para>
fn emit_update(
    writer: &mut CodeWriter,
    update: &ExportedUpdate,
    context: &CSharpRenderContext<'_>,
) {
    // A state field typed by the component's type parameter erases to `object`: the host patches
    // state as data and never names the instantiation, which an NX use site fixed.
    let erased;
    let update = match erase_field_type_parameters(&update.fields, &update.type_params) {
        Cow::Borrowed(_) => update,
        Cow::Owned(fields) => {
            erased = ExportedUpdate {
                fields,
                ..update.clone()
            };
            &erased
        }
    };
    let class_name = sanitize_csharp_identifier(&update.name);
    let plain_target = update_plain_target(update, context);
    let property_enum = update_property_enum(update, context);
    let field_members = update
        .fields
        .iter()
        .map(|field| sanitize_csharp_member_name(&field.name))
        .collect::<BTreeSet<_>>();
    // A field accessor keeps the record's property name, since that is the front door a host
    // writes. Every member the class introduces beside the accessors steps around the field names
    // instead, and an accessor that lands on a base-class member hides it explicitly with `new`;
    // the base surface stays reachable through an `NxUpdateRecord`-typed reference, and the
    // generated bodies reach it through `base.` and the base type name.
    let reserve_member = |preferred: &str| {
        let mut member = preferred.to_string();
        while field_members.contains(&member) || member == class_name {
            member.push('_');
        }
        member
    };
    let discriminator_member = reserve_member("NxType");
    let schema_member = reserve_member("FieldSchema");
    let is_set_member = reserve_member("IsSet");
    let unset_member = reserve_member("Unset");
    let changed_member = reserve_member("Changed");
    let diff_member = reserve_member("Diff");
    let discriminator = escape_csharp_string_literal(&update.discriminator);

    writer.line(&format!(
        "[JsonConverter(typeof(NxUpdateRecordJsonConverter<{class_name}>))]"
    ));
    writer.line(&format!(
        "[MessagePackFormatter(typeof(NxUpdateRecordMessagePackFormatter<{class_name}>))]"
    ));
    let base = match &plain_target {
        Some(target) => format!("NxUpdate<{}>", target.type_name),
        None => "NxUpdateRecord".to_string(),
    };
    writer.block(
        &format!("public sealed class {class_name} : {base}"),
        |writer| {
            let mut schema_entries = vec![format!("\"{discriminator}\"")];
            for field in &update.fields {
                schema_entries.push(match &plain_target {
                    Some(target) => format!(
                        "{}.{}",
                        target.properties_table,
                        sanitize_csharp_member_name(&field.name)
                    ),
                    None => format!(
                        "new NxField(\"{}\", typeof({}))",
                        escape_csharp_string_literal(&field.name),
                        csharp_typeof_operand(&csharp_type(&field.ty, context))
                    ),
                });
            }
            writer.line(&format!(
                "private static readonly NxUpdateSchema {schema_member} = new("
            ));
            writer.indent();
            for (index, entry) in schema_entries.iter().enumerate() {
                let terminator = if index + 1 == schema_entries.len() {
                    ");"
                } else {
                    ","
                };
                writer.line(&format!("{entry}{terminator}"));
            }
            writer.dedent();

            writer.blank_line();
            writer.line(&format!("public {class_name}()"));
            writer.indent();
            writer.line(&format!(": base({schema_member})"));
            writer.dedent();
            writer.line("{");
            writer.line("}");

            writer.blank_line();
            writer.line(&format!(
                "public string {discriminator_member} => \"{discriminator}\";"
            ));

            for field in &update.fields {
                let field_type = csharp_type(&field.ty, context);
                let wire_name = escape_csharp_string_literal(&field.name);
                let member = sanitize_csharp_member_name(&field.name);
                let hides_base_member = UPDATE_RECORD_MEMBERS.contains(&member.as_str())
                    || (plain_target.is_some() && NX_UPDATE_MEMBERS.contains(&member.as_str()));
                let modifier = if hides_base_member {
                    "public new"
                } else {
                    "public"
                };
                writer.blank_line();
                writer.block(
                    &format!("{modifier} NxOptional<{}> {member}", field_type.text),
                    |writer| {
                        writer.line(&format!(
                            "get => base.Get<{}>(\"{wire_name}\");",
                            field_type.text
                        ));
                        writer.line(&format!("set => base.Set(\"{wire_name}\", value);"));
                    },
                );
            }

            if let Some(property_enum) = &property_enum {
                let enum_name = &property_enum.type_name;
                let wire_format = &property_enum.wire_format;
                writer.blank_line();
                writer.line(&format!(
                    "public bool {is_set_member}({enum_name} property) => base.IsSet({wire_format}.Format(property));"
                ));
                writer.blank_line();
                writer.line(&format!(
                    "public void {unset_member}({enum_name} property) => base.Unset({wire_format}.Format(property));"
                ));
                writer.blank_line();
                writer.line(&format!(
                    "public {enum_name}[] {changed_member}() => Array.ConvertAll(base.ChangedNames(), {wire_format}.Parse);"
                ));
            }

            if let Some(target) = &plain_target {
                let target_type = &target.type_name;
                writer.blank_line();
                writer.line(&format!(
                    "public static {class_name} {diff_member}({target_type} before, {target_type} after) => {base}.Diff<{class_name}>(before, after);"
                ));
            }
        },
    );

    if let Some(target) = &plain_target {
        writer.blank_line();
        emit_property_table(writer, update, target, property_enum.as_ref(), context);
    }
}

/// The non-generic members of `NxUpdateRecord`, and the one `NxUpdate<TRecord>` adds. A field
/// accessor that takes one of these names hides the base member and says so with `new`. The
/// generic members (`Get`, `Set`, `Merge`, `Diff`) are not hidden by a property — the compiler
/// rejects `new` there as unneeded — and stay callable beside an accessor of the same name.
const UPDATE_RECORD_MEMBERS: &[&str] = &["Fields", "Schema", "IsSet", "Unset", "ChangedNames"];
const NX_UPDATE_MEMBERS: &[&str] = &["Apply"];

/// The plain generated type an update companion patches, and the key table emitted beside it.
struct UpdatePlainTarget {
    /// The C# reference to the plain type.
    type_name: String,
    /// The C# reference to the key table.
    properties_table: String,
    /// The key table's own identifier, for its declaration.
    properties_table_identifier: String,
}

/// The NX-level names behind an [`UpdatePlainTarget`], before the C# spelling and the collision
/// check are applied.
struct UpdatePlainTargetCandidate {
    type_name: String,
    properties_table: String,
}

/// The generated `<Target>_property` enum and its wire-format helper, when the companion exists.
struct UpdatePropertyEnum {
    type_name: String,
    wire_format: String,
    cases: Vec<String>,
}

/// Returns the instantiable plain type a companion's fields live on, when typegen emits one: the
/// target itself for a record or an action that is not abstract, and `<Name>_state` for an
/// external component with declared state. A non-external component's state has no plain type,
/// and an abstract record has none a host can instantiate to apply a patch to.
fn update_plain_target_candidate(
    update: &ExportedUpdate,
    graph: &ExportedTypeGraph,
) -> Option<UpdatePlainTargetCandidate> {
    let type_name = if update.target_is_component {
        let state_name = format!("{}_state", update.target_name);
        match &graph.declaration(&state_name)?.item {
            ExportedType::ExternalState(state) if state.component_name == update.target_name => {
                state_name
            }
            _ => return None,
        }
    } else {
        let record = graph.record(&update.target_name)?;
        if record.is_abstract {
            return None;
        }
        update.target_name.clone()
    };
    Some(UpdatePlainTargetCandidate {
        properties_table: format!("{type_name}Properties"),
        type_name,
    })
}

/// Returns the plain type a companion patches and its key table, unless an exported declaration
/// already claims the table's name, in which case `collect_warnings` reports the omission and the
/// companion falls back to the untyped base with a name→type schema.
fn update_plain_target(
    update: &ExportedUpdate,
    context: &CSharpRenderContext<'_>,
) -> Option<UpdatePlainTarget> {
    let candidate = update_plain_target_candidate(update, context.graph)?;
    if context
        .graph
        .declaration(&candidate.properties_table)
        .is_some()
    {
        return None;
    }
    Some(UpdatePlainTarget {
        type_name: csharp_type_name(&candidate.type_name, context).text,
        properties_table: generated_type_name(
            &candidate.properties_table,
            context.namespace,
            context.qualify_generated_types,
        ),
        properties_table_identifier: sanitize_csharp_identifier(&candidate.properties_table),
    })
}

/// Returns the `<Target>_property` companion of the update's target when typegen emits it as an
/// enum, which an explicit declaration of the same name can prevent.
fn update_property_enum(
    update: &ExportedUpdate,
    context: &CSharpRenderContext<'_>,
) -> Option<UpdatePropertyEnum> {
    let companion_name = format!("{}_property", update.target_name);
    let ExportedType::Union(union_def) = &context.graph.declaration(&companion_name)?.item else {
        return None;
    };
    if union_def.property_target.as_deref() != Some(update.target_name.as_str())
        || !union_def.is_constant()
    {
        return None;
    }
    Some(UpdatePropertyEnum {
        type_name: csharp_type_name(&union_def.name, context).text,
        wire_format: generated_type_name(
            &format!("{}WireFormat", sanitize_csharp_identifier(&union_def.name)),
            context.namespace,
            context.qualify_generated_types,
        ),
        cases: union_def
            .cases
            .iter()
            .map(|case| case.name.clone())
            .collect(),
    })
}

/// Emits the `<Target>Properties` key table: one `NxProperty<Target, TValue>` per field of the
/// companion, named through the `<Target>_property` wire format so the enum stays the only
/// spelling of a wire name, and `Of` mapping each enum case to its key.
fn emit_property_table(
    writer: &mut CodeWriter,
    update: &ExportedUpdate,
    target: &UpdatePlainTarget,
    property_enum: Option<&UpdatePropertyEnum>,
    context: &CSharpRenderContext<'_>,
) {
    let table_name = &target.properties_table_identifier;
    let target_type = &target.type_name;
    let field_members = update
        .fields
        .iter()
        .map(|field| sanitize_csharp_member_name(&field.name))
        .collect::<BTreeSet<_>>();
    let mut of_member = "Of".to_string();
    while field_members.contains(&of_member) {
        of_member.push('_');
    }
    let enum_case = |field_name: &str| {
        property_enum
            .filter(|property_enum| property_enum.cases.iter().any(|case| case == field_name))
    };

    writer.block(&format!("public static class {table_name}"), |writer| {
        for (index, field) in update.fields.iter().enumerate() {
            if index > 0 {
                writer.blank_line();
            }
            let member = sanitize_csharp_member_name(&field.name);
            let field_type = csharp_type(&field.ty, context);
            let wire_name = match enum_case(&field.name) {
                Some(property_enum) => format!(
                    "{}.Format({}.{})",
                    property_enum.wire_format, property_enum.type_name, member
                ),
                None => format!("\"{}\"", escape_csharp_string_literal(&field.name)),
            };
            writer.line(&format!(
                "public static readonly NxProperty<{target_type}, {}> {member} = new(",
                field_type.text
            ));
            writer.indent();
            writer.line(&format!("{wire_name},"));
            writer.line(&format!("record => record.{member},"));
            writer.line(&format!("(record, value) => record.{member} = value);"));
            writer.dedent();
        }

        // A `switch` statement rather than a switch expression: the arms differ in nullability
        // annotation (`NxProperty<User, string>` beside `NxProperty<User, string?>`), and the
        // expression would infer one of them as its natural type and warn about the other.
        if let Some(property_enum) = property_enum {
            let enum_name = &property_enum.type_name;
            writer.blank_line();
            writer.block(
                &format!(
                    "public static NxProperty<{target_type}> {of_member}({enum_name} property)"
                ),
                |writer| {
                    writer.block("switch (property)", |writer| {
                        for field in &update.fields {
                            if enum_case(&field.name).is_none() {
                                continue;
                            }
                            let member = sanitize_csharp_member_name(&field.name);
                            writer.line(&format!("case {enum_name}.{member}:"));
                            writer.indent();
                            writer.line(&format!("return {member};"));
                            writer.dedent();
                        }
                        writer.line("default:");
                        writer.indent();
                        writer.line("throw new ArgumentOutOfRangeException(nameof(property));");
                        writer.dedent();
                    });
                },
            );
        }
    });
}

/// The operand `typeof` takes for a field type: a nullable reference type drops its `?`, which
/// only annotates, while a nullable value type keeps it, since `Nullable<T>` is the runtime type.
fn csharp_typeof_operand(field_type: &CSharpType) -> String {
    if field_type.is_reference && field_type.is_nullable {
        field_type
            .text
            .strip_suffix('?')
            .unwrap_or(&field_type.text)
            .to_string()
    } else {
        field_type.text.clone()
    }
}

/// Emits a constant union as a CLR `enum` with its authored-string wire format (design D4).
fn emit_constant_union(writer: &mut CodeWriter, union: &ExportedUnion) {
    let enum_name = sanitize_csharp_identifier(&union.name);
    writer.line(&format!(
        "[JsonConverter(typeof(NxEnumJsonConverter<{enum_name}, {enum_name}WireFormat>))]"
    ));
    writer.line(&format!(
        "[MessagePackFormatter(typeof(NxEnumMessagePackFormatter<{enum_name}, {enum_name}WireFormat>))]"
    ));
    writer.block(&format!("public enum {enum_name}"), |writer| {
        for (index, case) in union.cases.iter().enumerate() {
            let comma = if index + 1 == union.cases.len() {
                ""
            } else {
                ","
            };
            writer.line(&format!(
                "{}{}",
                sanitize_csharp_member_name(&case.name),
                comma
            ));
        }
    });

    writer.blank_line();
    emit_constant_union_wire_format(writer, union);
}

fn emit_record(
    writer: &mut CodeWriter,
    record: &ExportedRecord,
    context: &CSharpRenderContext<'_>,
) {
    // A C# host receives a component value by its discriminator and cannot pick a generic
    // instantiation from data, so a type parameter is erased to `object` and the record declares
    // no generic parameter of its own.
    let erased;
    let record = match erase_field_type_parameters(&record.fields, &record.type_params) {
        Cow::Borrowed(_) => record,
        Cow::Owned(fields) => {
            erased = ExportedRecord {
                fields,
                ..record.clone()
            };
            &erased
        }
    };

    emit_record_json_polymorphism_attributes(writer, record, context);

    let polymorphic_root = polymorphic_message_pack_root_name(record, context);
    if let Some(root_name) = polymorphic_root.as_ref() {
        let safe_root = sanitize_csharp_identifier(root_name);
        let safe_name = sanitize_csharp_identifier(&record.name);
        if root_name == &record.name {
            writer.line(&format!(
                "[MessagePackFormatter(typeof(NxPolymorphicMessagePackFormatter<{safe_root}>))]"
            ));
        } else {
            writer.line(&format!(
                "[MessagePackFormatter(typeof(NxPolymorphicConcreteMessagePackFormatter<{safe_root}, {safe_name}>))]"
            ));
        }
    }

    if should_emit_missing_polymorphism_hint(record, context) {
        writer.line(
            "// No polymorphism metadata (JSON or MessagePack) was generated because this abstract type had",
        );
        writer.line("// no concrete exported descendants at code-generation time.");
    }

    if polymorphic_root.is_none() {
        writer.line("[MessagePackObject]");
    }
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

fn emit_union(
    writer: &mut CodeWriter,
    union_def: &ExportedUnion,
    context: &CSharpRenderContext<'_>,
) {
    if union_def.is_constant() {
        emit_constant_union(writer, union_def);
        return;
    }

    let union_name = sanitize_csharp_identifier(&union_def.name);

    // A union with a constant case beside a payload case has two wire shapes, so it needs a
    // converter (design D4). System.Text.Json will not accept a converter alongside
    // `[JsonPolymorphic]` metadata on the same type, so such a union registers its cases with
    // `[NxUnionCase]`, which both serializers read, instead of `[JsonDerivedType]`.
    let has_constant_case = union_def
        .cases
        .iter()
        .any(|case| union_def.is_constant_case(case));

    if has_constant_case {
        writer.line(&format!(
            "[JsonConverter(typeof(NxPolymorphicJsonConverter<{union_name}>))]"
        ));
        for case in &union_def.cases {
            writer.line(&format!(
                "[NxUnionCase(typeof({}), \"{}.{}\")]",
                csharp_union_case_type_name(&union_def.name, &case.name),
                escape_csharp_string_literal(&union_def.name),
                escape_csharp_string_literal(&case.name)
            ));
        }
    } else {
        writer.line("[JsonPolymorphic(TypeDiscriminatorPropertyName = \"$type\")]");
        for case in &union_def.cases {
            writer.line(&format!(
                "[JsonDerivedType(typeof({}), \"{}.{}\")]",
                csharp_union_case_type_name(&union_def.name, &case.name),
                escape_csharp_string_literal(&union_def.name),
                escape_csharp_string_literal(&case.name)
            ));
        }
    }
    writer.line(&format!(
        "[MessagePackFormatter(typeof(NxPolymorphicMessagePackFormatter<{union_name}>))]"
    ));

    let header = if let Some(base) = &union_def.base {
        format!(
            "public abstract class {union_name} : {}",
            csharp_type_name(base, context).text
        )
    } else {
        format!("public abstract class {union_name}")
    };

    writer.block(&header, |_| {});

    for case in &union_def.cases {
        writer.blank_line();
        emit_union_case(writer, union_def, case, context);
    }
}

fn emit_union_case(
    writer: &mut CodeWriter,
    union_def: &ExportedUnion,
    case: &ExportedUnionCase,
    context: &CSharpRenderContext<'_>,
) {
    let union_name = sanitize_csharp_identifier(&union_def.name);
    let case_type_name = csharp_union_case_type_name(&union_def.name, &case.name);
    let is_constant = union_def.is_constant_case(case);

    if is_constant {
        writer.line(&format!(
            "[NxConstantCase(\"{}\")]",
            escape_csharp_string_literal(&case.name)
        ));
    }
    writer.line(&format!(
        "[MessagePackFormatter(typeof(NxPolymorphicConcreteMessagePackFormatter<{union_name}, {case_type_name}>))]"
    ));
    writer.block(
        &format!("public sealed class {case_type_name} : {union_name}"),
        |writer| {
            if is_constant {
                // A constant case carries nothing, so every occurrence is the same value
                // (design D4).
                writer.line("/// <summary>The single instance of this constant case.</summary>");
                writer.line(&format!(
                    "public static readonly {case_type_name} Instance = new();"
                ));
            }
            emit_record_fields(writer, &case.fields, context);
        },
    );
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
            emit_record_fields(
                writer,
                &erase_field_type_parameters(&state.fields, &state.type_params),
                context,
            );
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
    for descendant in context.graph.polymorphic_descendants(&record.name) {
        let (type_name, discriminator) = csharp_polymorphic_descendant_metadata(&descendant);
        writer.line(&format!(
            "[JsonDerivedType(typeof({}), \"{}\")]",
            type_name,
            escape_csharp_string_literal(&discriminator)
        ));
    }
}

fn should_emit_json_polymorphism_attributes(
    record: &ExportedRecord,
    context: &CSharpRenderContext<'_>,
) -> bool {
    record.is_abstract
        && !graph_resolves_record_base(context, record)
        && !context
            .graph
            .polymorphic_descendants(&record.name)
            .is_empty()
}

/// Polymorphic MessagePack uses a root formatter plus per-declared-type wrappers so MessagePack can resolve
/// `IMessagePackFormatter<T>` for concrete and intermediate types, not only the abstract root.
fn polymorphic_message_pack_root_name(
    record: &ExportedRecord,
    context: &CSharpRenderContext<'_>,
) -> Option<String> {
    let mut current = record;
    loop {
        if should_emit_json_polymorphism_attributes(current, context) {
            return Some(current.name.clone());
        }
        current = context.graph.resolved_record_base(current)?;
    }
}

fn should_emit_missing_polymorphism_hint(
    record: &ExportedRecord,
    context: &CSharpRenderContext<'_>,
) -> bool {
    record.is_abstract
        && !graph_resolves_record_base(context, record)
        && context
            .graph
            .polymorphic_descendants(&record.name)
            .is_empty()
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

        let property_declaration = if let Some(initializer) =
            csharp_default_initializer(field.default_value.as_ref(), &field_type)
        {
            format!(
                "public {} {} {{ get; set; }} = {};",
                field_type.text, field_name, initializer
            )
        } else if field_type.is_reference && !field_type.is_nullable {
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

fn csharp_default_initializer(
    default_value: Option<&ExportedFieldDefault>,
    field_type: &CSharpType,
) -> Option<String> {
    let Some(ExportedFieldDefault::Literal(literal)) = default_value else {
        return None;
    };

    Some(match literal {
        ExportedLiteralDefault::String(value) => {
            format!("\"{}\"", escape_csharp_string_literal(value))
        }
        // Type generation reads a lowered module, so the conversion that turns an integer literal
        // at a float site into a float literal has not run here — that pass belongs to type
        // checking, which this path does not go through. `= 0` on a `double` compiles, but it
        // would leave the same declaration generating different text depending on which pipeline
        // produced it, so the spelling is settled from the field's own type instead.
        ExportedLiteralDefault::Int(value) if csharp_type_is_floating_point(field_type) => {
            csharp_float_literal(nx_hir::ast::OrderedFloat(*value as f64), field_type)
        }
        ExportedLiteralDefault::Int(value) => value.to_string(),
        ExportedLiteralDefault::Float(value) => csharp_float_literal(*value, field_type),
        ExportedLiteralDefault::Boolean(value) => value.to_string(),
        ExportedLiteralDefault::Null => "null".to_string(),
    })
}

/// Whether this generated type is one a float literal spelling belongs on.
fn csharp_type_is_floating_point(field_type: &CSharpType) -> bool {
    matches!(field_type.text.trim_end_matches('?'), "float" | "double")
}

fn csharp_float_literal(value: nx_hir::ast::OrderedFloat, field_type: &CSharpType) -> String {
    let type_name = field_type.text.trim_end_matches('?');
    let is_float = type_name == "float";
    let value = value.0;
    let mut literal = if value.is_nan() {
        if is_float {
            "float.NaN".to_string()
        } else {
            "double.NaN".to_string()
        }
    } else if value == f64::INFINITY {
        if is_float {
            "float.PositiveInfinity".to_string()
        } else {
            "double.PositiveInfinity".to_string()
        }
    } else if value == f64::NEG_INFINITY {
        if is_float {
            "float.NegativeInfinity".to_string()
        } else {
            "double.NegativeInfinity".to_string()
        }
    } else {
        let mut text = value.to_string();
        if !text.contains('.') && !text.contains('e') && !text.contains('E') {
            text.push_str(".0");
        }
        text
    };

    if is_float
        && !literal.starts_with("float.")
        && !literal.ends_with('f')
        && !literal.ends_with('F')
    {
        literal.push('f');
    }

    literal
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

fn emit_constant_union_wire_format(writer: &mut CodeWriter, union: &ExportedUnion) {
    let enum_name = sanitize_csharp_identifier(&union.name);
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
            for case in &union.cases {
                let member_literal = escape_csharp_string_literal(&case.name);
                let member_ident = sanitize_csharp_member_name(&case.name);
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
            for case in &union.cases {
                let member_literal = escape_csharp_string_literal(&case.name);
                let member_ident = sanitize_csharp_member_name(&case.name);
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
        // A function value is a name, not a callable the host can invoke: `System.Delegate` neither
        // serializes (MessagePack has no formatter for it, which breaks the whole containing type,
        // and `System.Text.Json` refuses it outright) nor carries the declaration. `NxFunctionRef`
        // is the `Function` record the runtime renders, as `NxActionHandlerRef` is for a handler.
        TypeRef::Function { .. } => CSharpType {
            text: "global::NxLang.Nx.NxFunctionRef".to_string(),
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
        // NX `int` is exact over +/-(2^53-1), which does not fit a C# `int`; `long` is the
        // narrowest C# type that holds the whole range.
        "int" => CSharpType {
            text: "long".to_string(),
            is_reference: false,
            is_nullable: false,
        },
        "int32" => CSharpType {
            text: "int".to_string(),
            is_reference: false,
            is_nullable: false,
        },
        "int64" => CSharpType {
            text: "long".to_string(),
            is_reference: false,
            is_nullable: false,
        },
        "float32" => CSharpType {
            text: "float".to_string(),
            is_reference: false,
            is_nullable: false,
        },
        "float64" => CSharpType {
            text: "double".to_string(),
            is_reference: false,
            is_nullable: false,
        },
        "boolean" => CSharpType {
            text: "bool".to_string(),
            is_reference: false,
            is_nullable: false,
        },
        // `void` is not an NX type name any more, so a `void` here is a user declaration
        // and must resolve to the type they declared rather than to the host's `void`.
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
                                text: "object".to_string(),
                                is_reference: true,
                                is_nullable: false,
                            };
                        }

                        let alias_type = csharp_type_inner(&alias.target, context, seen_aliases);
                        seen_aliases.remove(other);
                        alias_type
                    }
                    ExportedType::Union(union_def) if union_def.is_constant() => CSharpType {
                        text: generated_type_name(
                            other,
                            context.namespace,
                            context.qualify_generated_types,
                        ),
                        is_reference: false,
                        is_nullable: false,
                    },
                    ExportedType::Union(_) => CSharpType {
                        text: generated_type_name(
                            other,
                            context.namespace,
                            context.qualify_generated_types,
                        ),
                        is_reference: true,
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
                    ExportedType::ExternalState(_) | ExportedType::Update(_) => CSharpType {
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
                csharp_imported_type(imported_type, context)
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

fn csharp_imported_type(
    imported_type: &ImportedType,
    context: &CSharpRenderContext<'_>,
) -> CSharpType {
    let dependency_namespace =
        assumed_dependency_namespace_for_library(context.namespace, &imported_type.library_name);

    match &imported_type.kind {
        ImportedTypeKind::Alias {
            target,
            target_is_reference,
        } => csharp_imported_alias_target_type(target, &dependency_namespace, *target_is_reference),
        _ => CSharpType {
            text: generated_type_name(&imported_type.exported_name, &dependency_namespace, true),
            is_reference: imported_type.kind.is_reference(),
            is_nullable: false,
        },
    }
}

fn csharp_imported_alias_target_type(
    ty: &TypeRef,
    dependency_namespace: &str,
    target_is_reference: bool,
) -> CSharpType {
    match ty {
        TypeRef::Nullable(inner) => {
            let mut inner =
                csharp_imported_alias_target_type(inner, dependency_namespace, target_is_reference);
            inner.text = format!("{}?", inner.text);
            inner.is_nullable = true;
            inner
        }
        TypeRef::Array(inner) => {
            let inner =
                csharp_imported_alias_target_type(inner, dependency_namespace, target_is_reference);
            CSharpType {
                text: format!("{}[]", inner.text),
                is_reference: true,
                is_nullable: false,
            }
        }
        TypeRef::Function { .. } => CSharpType {
            text: "global::NxLang.Nx.NxFunctionRef".to_string(),
            is_reference: true,
            is_nullable: false,
        },
        TypeRef::Name(name) => csharp_imported_alias_target_name(
            name.as_str(),
            dependency_namespace,
            target_is_reference,
        ),
    }
}

fn csharp_imported_alias_target_name(
    name: &str,
    dependency_namespace: &str,
    target_is_reference: bool,
) -> CSharpType {
    match name {
        "string" => CSharpType {
            text: "string".to_string(),
            is_reference: true,
            is_nullable: false,
        },
        // NX `int` is exact over +/-(2^53-1), which does not fit a C# `int`; `long` is the
        // narrowest C# type that holds the whole range.
        "int" => CSharpType {
            text: "long".to_string(),
            is_reference: false,
            is_nullable: false,
        },
        "int32" => CSharpType {
            text: "int".to_string(),
            is_reference: false,
            is_nullable: false,
        },
        "int64" => CSharpType {
            text: "long".to_string(),
            is_reference: false,
            is_nullable: false,
        },
        "float32" => CSharpType {
            text: "float".to_string(),
            is_reference: false,
            is_nullable: false,
        },
        "float64" => CSharpType {
            text: "double".to_string(),
            is_reference: false,
            is_nullable: false,
        },
        "boolean" => CSharpType {
            text: "bool".to_string(),
            is_reference: false,
            is_nullable: false,
        },
        // `void` is not an NX type name any more, so a `void` here is a user declaration
        // and must resolve to the type they declared rather than to the host's `void`.
        "object" | "unknown" | "error" => CSharpType {
            text: "object".to_string(),
            is_reference: true,
            is_nullable: false,
        },
        other => CSharpType {
            text: generated_type_name(other, dependency_namespace, true),
            is_reference: target_is_reference,
            is_nullable: false,
        },
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

fn csharp_polymorphic_descendant_metadata(
    descendant: &ExportedPolymorphicDescendant,
) -> (String, String) {
    match descendant {
        ExportedPolymorphicDescendant::Record { name } => {
            (sanitize_csharp_identifier(name), name.clone())
        }
        ExportedPolymorphicDescendant::UnionCase {
            union_name,
            case_name,
        } => (
            csharp_union_case_type_name(union_name, case_name),
            format!("{union_name}.{case_name}"),
        ),
    }
}

fn csharp_union_case_type_name(union_name: &str, case_name: &str) -> String {
    sanitize_csharp_identifier(&format!(
        "{}{}",
        union_name,
        sanitize_csharp_member_name(case_name)
    ))
}

fn collect_dependency_namespaces(
    imported_types: &[ImportedType],
    current_namespace: &str,
) -> BTreeSet<String> {
    imported_types
        .iter()
        .filter(|imported_type| imported_type_uses_dependency_namespace(imported_type))
        .map(|imported_type| {
            assumed_dependency_namespace_for_library(current_namespace, &imported_type.library_name)
        })
        .filter(|dependency_namespace| dependency_namespace != current_namespace)
        .collect()
}

fn imported_type_uses_dependency_namespace(imported_type: &ImportedType) -> bool {
    match &imported_type.kind {
        ImportedTypeKind::Alias { target, .. } => {
            imported_alias_target_uses_dependency_namespace(target)
        }
        _ => true,
    }
}

fn imported_alias_target_uses_dependency_namespace(ty: &TypeRef) -> bool {
    match ty {
        TypeRef::Nullable(inner) | TypeRef::Array(inner) => {
            imported_alias_target_uses_dependency_namespace(inner)
        }
        TypeRef::Function { .. } => false,
        TypeRef::Name(name) => !matches!(
            name.as_str(),
            "string"
                | "int"
                | "int32"
                | "int64"
                | "float32"
                | "float64"
                | "boolean"
                | "object"
                | "unknown"
                | "error"
        ),
    }
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
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\0' => escaped.push_str("\\0"),
            ch if ch.is_control() => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => escaped.push(ch),
        }
    }
    escaped
}
