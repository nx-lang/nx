use crate::typegen::model::{
    erase_field_type_parameters, update_companion_type_params, ExportedExternalState,
    ExportedFieldDefault, ExportedLiteralDefault, ExportedModule, ExportedPolymorphicDescendant,
    ExportedRecord, ExportedRecordField, ExportedType, ExportedTypeGraph, ExportedUnion,
    ExportedUnionCase, ExportedUpdate, ImportedType, ImportedTypeKind, ModuleTypes,
};
use crate::typegen::writer::CodeWriter;
use crate::typegen::{GenerateTypesOptions, GeneratedFile};
use nx_hir::ast::TypeRef;
use rustc_hash::FxHashMap;
use std::borrow::Cow;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The basename of the file a library's closed update formatters are declared in.
const CLOSED_FORMATTER_FILE_BASENAME: &str = "_NxFormatters";

/// Where the closed update formatters a module's members name are declared.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ClosedFormatterPlacement {
    /// Single-file output: the one module declares the formatters its own members name.
    Inline,
    /// Library output: every formatter is declared once in the shared formatter file.
    Shared,
}

pub fn emit_single_file(
    graph: &ExportedTypeGraph,
    namespace: &str,
    opts: &GenerateTypesOptions,
) -> Result<String, String> {
    let module = graph
        .modules
        .first()
        .ok_or_else(|| "Single-file generation requires one source module".to_string())?;
    Ok(render_module(
        graph,
        module,
        namespace,
        opts,
        ClosedFormatterPlacement::Inline,
    ))
}

pub fn emit_library(
    graph: &ExportedTypeGraph,
    namespace: &str,
    opts: &GenerateTypesOptions,
) -> Result<Vec<GeneratedFile>, String> {
    let mut files = Vec::new();

    // Every module of a library is generated into the one namespace, whatever its directory, so a
    // formatter declared per module is declared twice as soon as two modules type a member by the
    // same instantiation. They go in a file of their own instead — the move TypeScript makes with
    // its helper module — and each module names them across the namespace.
    let shared_formatters = collect_library_closed_update_formatters(graph, namespace);
    if !shared_formatters.formatters.is_empty() {
        files.push(GeneratedFile {
            relative_path: closed_formatter_file_path(graph),
            content: render_closed_formatter_file(&shared_formatters, namespace, opts),
        });
    }

    for module in &graph.modules {
        files.push(GeneratedFile {
            relative_path: module_output_path(&module.module_path),
            content: render_module(
                graph,
                module,
                namespace,
                opts,
                ClosedFormatterPlacement::Shared,
            ),
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

        // MessagePack's source generator cannot close the open generic a generic update companion
        // carries, so a member typed by one needs the generator to reach a formatter some other way.
        // A companion the generating library declares is reached through the declaration itself: the
        // generator has it in the compilation and closes the companion's own shim per instantiation,
        // wherever in the member's type it sits. A companion from another assembly is reached only
        // through a closed formatter the *member* names, which needs the member's own type to be the
        // instantiation — below that there is nowhere to put the attribute, and the open generic on
        // the companion is `MsgPack006` from `CSC` itself, which no file-level pragma reaches. That
        // last shape is the one named here rather than compiled into a file that does not build.
        let imported_type_lookup = module
            .imported_types
            .iter()
            .cloned()
            .map(|imported_type| (imported_type.visible_name.clone(), imported_type))
            .collect::<FxHashMap<_, _>>();
        let context = CSharpRenderContext {
            namespace,
            types: ModuleTypes::for_module(graph, module),
            imported_types_by_visible_name: &imported_type_lookup,
            qualify_generated_types: false,
        };
        for declaration in &module.declarations {
            for (owner, field) in declaration_fields(&declaration.item) {
                if closed_update_formatter_for(&field.ty, &context).is_some() {
                    continue;
                }
                for name in nx_hir::type_ref_names(&field.ty) {
                    if !generic_update_companion(name.as_str(), &context) {
                        continue;
                    }
                    if context.graph().declaration(name.as_str()).is_some() {
                        continue;
                    }
                    warnings.push(format!(
                        "Generated C# field '{}.{}' reaches the generic update companion '{}' below its own type, and that companion is declared in another assembly, so neither the member nor the companion can name a formatter MessagePack's source generator resolves. Type the member as '{}' itself, or patch '{}' in NX and expose the record across the host boundary instead.",
                        owner, field.name, name, name, name
                    ));
                }
            }
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
    placement: ClosedFormatterPlacement,
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
        types: ModuleTypes::for_module(graph, module),
        imported_types_by_visible_name: &imported_type_lookup,
        qualify_generated_types: false,
    };

    // A member typed by one instantiation of a companion this file does not declare names a
    // formatter of that closed type, which has to be declared where the member can see it: beside
    // the contracts in single-file output, in the library's shared formatter file otherwise.
    let closed_update_formatters = match placement {
        ClosedFormatterPlacement::Inline => {
            collect_closed_update_formatters(module, &namespace_context)
        }
        ClosedFormatterPlacement::Shared => Vec::new(),
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

    // Only a generic companion declares a formatter of its own, and only that needs the formatter
    // interface in scope.
    let needs_formatter_interface = !closed_update_formatters.is_empty()
        || module.declarations.iter().any(|declaration| {
            matches!(&declaration.item, ExportedType::Update(update)
                if !update_companion_type_params(update, namespace_context.types).is_empty())
        });

    writer.line("using System;");
    writer.line("using System.Text.Json.Serialization;");
    writer.line("using MessagePack;");
    if needs_formatter_interface {
        writer.line("using MessagePack.Formatters;");
    }
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
            for (index, formatter) in closed_update_formatters.iter().enumerate() {
                emit_closed_update_formatter(writer, formatter);
                if index + 1 != closed_update_formatters.len() || !body_items.is_empty() {
                    writer.blank_line();
                }
            }
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
    // MsgPack006 reads `[MessagePackFormatter(typeof(Foo_updateFormatter<>))]` — the open generic
    // MessagePack documents for a generic type — as a formatter that implements no formatter
    // interface, because an unbound generic implements none until it is closed. The generator
    // closes it and the round trip works; the diagnostic only fires once some member is typed by a
    // generic companion, which is exactly when a correct file would stop compiling.
    writer.line("#pragma warning disable MsgPack006");
    // MsgPack009 counts formatters against the *open* generic, so two closed formatters over two
    // instantiations of one companion — `NxRange_update<long>` and `NxRange_update<double>` in one
    // contract — read as two formatters for `NxRange_update<T>`. They are not: the source generator
    // resolves a member by the closed type its attribute names, and each formatter serializes only
    // the instantiation it is declared over. A generated file declares a formatter only where the
    // emitter put one, so a genuine duplicate here would be a generator defect, not a source one.
    writer.line("#pragma warning disable MsgPack009");
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
/// `NxOptional<T>` accessor pair over the base's map, so `new User_update { Email = null }` means
/// "clear email" — legal because `email` is optional in `User`. The schema-driven SDK converter
/// and formatter named on the class write `$type` and only the set fields, so no member needs a
/// wire attribute.</para>
fn emit_update(
    writer: &mut CodeWriter,
    update: &ExportedUpdate,
    context: &CSharpRenderContext<'_>,
) {
    // A generic record's companion declares the record's parameters and keeps its field types, so
    // a `Range<long>` is patched by a `Range_update<long>` whose `Start` is a `long`. Everything
    // else erases: a state field typed by the *component's* type parameter becomes `object`,
    // because the host patches state as data and never names the instantiation an NX use site
    // fixed. A companion with no plain target has no generic surface to declare parameters on, so
    // it erases too.
    let type_params = update_companion_type_params(update, context.types);
    let plain_target = update_plain_target(update, context, type_params);
    let type_params: &[String] = if plain_target.is_some() {
        type_params
    } else {
        &[]
    };
    let erased;
    let update = if type_params.is_empty() {
        match erase_field_type_parameters(&update.fields, &update.type_params) {
            Cow::Borrowed(_) => update,
            Cow::Owned(fields) => {
                erased = ExportedUpdate {
                    fields,
                    ..update.clone()
                };
                &erased
            }
        }
    } else {
        update
    };
    let class_name = sanitize_csharp_identifier(&update.name);
    let generics = if type_params.is_empty() {
        String::new()
    } else {
        format!("<{}>", type_params.join(", "))
    };
    // The companion's own instantiation, for the members that name it: its `Diff` return, and the
    // formatter closed over it.
    let class_type = format!("{class_name}{generics}");
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

    // A generic companion cannot name its own converter or formatter in an attribute: an
    // attribute argument cannot use type parameters (CS0416). It names the SDK's factory for JSON,
    // and for MessagePack an open generic shim emitted below, which the reflecting resolver closes
    // over the same arguments the companion is closed over.
    let formatter_name = format!("{class_name}Formatter");
    let emit_open_formatter_shim = !type_params.is_empty();
    if type_params.is_empty() {
        writer.line(&format!(
            "[JsonConverter(typeof(NxUpdateRecordJsonConverter<{class_name}>))]"
        ));
        writer.line(&format!(
            "[MessagePackFormatter(typeof(NxUpdateRecordMessagePackFormatter<{class_name}>))]"
        ));
    } else {
        writer.line("[JsonConverter(typeof(NxUpdateRecordJsonConverterFactory))]");
        writer.line(&format!(
            "[MessagePackFormatter(typeof({formatter_name}<{}>))]",
            ",".repeat(type_params.len() - 1)
        ));
    }
    let base = match &plain_target {
        Some(target) => format!("NxUpdate<{}>", target.type_name),
        None => "NxUpdateRecord".to_string(),
    };
    writer.block(
        &format!("public sealed class {class_type} : {base}"),
        |writer| {
            let mut schema_entries = vec![format!("\"{discriminator}\"")];
            for field in &update.fields {
                schema_entries.push(match &plain_target {
                    Some(target) => format!(
                        "{}.{}",
                        target.properties_table,
                        sanitize_csharp_member_name(&field.name)
                    ),
                    // A field is clearable only where the target declares it optional.
                    None => format!(
                        "new NxField(\"{}\", typeof({}){})",
                        escape_csharp_string_literal(&field.name),
                        csharp_typeof_operand(&csharp_field_type(field, context)),
                        if field.optional { "" } else { ", clearable: false" }
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

            // A clearable field — one the target declares `name?:T` — is `NxOptional<T?>`, so a
            // host writes `null` to clear it; a non-clearable one is `NxOptional<T>`, so it cannot.
            for field in &update.fields {
                let field_type = csharp_field_type(field, context);
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
                    "public static {class_type} {diff_member}({target_type} before, {target_type} after) => {base}.Diff<{class_type}>(before, after);"
                ));
            }
        },
    );

    if emit_open_formatter_shim {
        writer.blank_line();
        emit_update_formatter_shim(writer, &formatter_name, &class_type, &generics);
    }

    if let Some(target) = &plain_target {
        writer.blank_line();
        emit_property_table(
            writer,
            update,
            target,
            property_enum.as_ref(),
            context,
            &generics,
        );
    }
}

/// Emits the open generic formatter a generic companion names in its `[MessagePackFormatter]`.
///
/// <para>The attribute resolver closes the formatter type over the annotated type's own arguments,
/// so the shim must take the same parameters the companion does and close
/// `NxUpdateRecordMessagePackFormatter` over the companion itself. Naming
/// `NxUpdateRecordMessagePackFormatter<>` directly would close it over `long` rather than over
/// `Range_update<long>`.</para>
fn emit_update_formatter_shim(
    writer: &mut CodeWriter,
    formatter_name: &str,
    class_type: &str,
    generics: &str,
) {
    // Annotated nullable, which is what the analyzer asks of a formatter over a reference type and
    // what the inner formatter actually does: it writes a null value as nil and reads nil back as
    // null, through a signature that does not say so.
    writer.block(
        &format!(
            "public sealed class {formatter_name}{generics} : IMessagePackFormatter<{class_type}?>"
        ),
        |writer| {
            writer.line(&format!(
                "private static readonly NxUpdateRecordMessagePackFormatter<{class_type}> Inner = new();"
            ));
            writer.blank_line();
            writer.line(&format!(
                "public void Serialize(ref MessagePackWriter writer, {class_type}? value, MessagePackSerializerOptions options) =>"
            ));
            writer.indent();
            writer.line("Inner.Serialize(ref writer, value!, options);");
            writer.dedent();
            writer.blank_line();
            writer.line(&format!(
                "public {class_type}? Deserialize(ref MessagePackReader reader, MessagePackSerializerOptions options) =>"
            ));
            writer.indent();
            writer.line("Inner.Deserialize(ref reader, options);");
            writer.dedent();
        },
    );
}

/// The non-generic members of `NxUpdateRecord`, and the one `NxUpdate<TRecord>` adds. A field
/// accessor that takes one of these names hides the base member and says so with `new`. The
/// generic members (`Get`, `Set`, `Merge`, `Diff`) are not hidden by a property — the compiler
/// rejects `new` there as unneeded — and stay callable beside an accessor of the same name.
const UPDATE_RECORD_MEMBERS: &[&str] = &["Fields", "Schema", "IsSet", "Unset", "ChangedNames"];
const NX_UPDATE_MEMBERS: &[&str] = &["Apply"];

/// The plain generated type an update companion patches, and the key table emitted beside it.
struct UpdatePlainTarget {
    /// The C# reference to the plain type, instantiated where the target is generic.
    type_name: String,
    /// The C# reference to the key table, instantiated where the target is generic.
    properties_table: String,
    /// The key table's own identifier, for its declaration, without its parameter list.
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
    type_params: &[String],
) -> Option<UpdatePlainTarget> {
    let candidate = update_plain_target_candidate(update, context.graph())?;
    if context
        .graph()
        .declaration(&candidate.properties_table)
        .is_some()
    {
        return None;
    }
    // A generic record is emitted as `Range<T>` and its companion patches that same generic, under
    // the companion's own parameters: `Range_update<T>` extends `NxUpdate<Range<T>>` and its key
    // table is `RangeProperties<T>`. Where the companion declares no parameters — a component's
    // state, or a target whose key table's name is taken — `type_params` is empty and the target
    // is the plain name, as it always was.
    let arguments = if type_params.is_empty() {
        String::new()
    } else {
        format!("<{}>", type_params.join(", "))
    };
    Some(UpdatePlainTarget {
        type_name: format!(
            "{}{arguments}",
            csharp_type_name(&candidate.type_name, context).text
        ),
        properties_table: format!(
            "{}{arguments}",
            generated_type_name(
                &candidate.properties_table,
                context.namespace,
                context.qualify_generated_types,
            )
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
    let ExportedType::Union(union_def) = &context.graph().declaration(&companion_name)?.item else {
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
    generics: &str,
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

    writer.block(
        &format!("public static class {table_name}{generics}"),
        |writer| {
            for (index, field) in update.fields.iter().enumerate() {
                if index > 0 {
                    writer.blank_line();
                }
                let member = sanitize_csharp_member_name(&field.name);
                let field_type = csharp_field_type(field, context);
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
                // A field is clearable only where the target declares it optional, which the
                // CLR type cannot say for a reference type.
                if field.optional {
                    writer.line(&format!("(record, value) => record.{member} = value);"));
                } else {
                    writer.line(&format!("(record, value) => record.{member} = value,"));
                    writer.line("clearable: false);");
                }
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
        },
    );
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
    // A C# host receives a *component* value by its discriminator and cannot pick a generic
    // instantiation from data, so a component contract's type parameter is erased to `object` and
    // the contract declares no generic parameter of its own. A declared generic record is the
    // other way round: the host names the concrete instantiation at its own deserialization site,
    // so it is emitted as a real generic.
    let erased;
    let record = if record.is_component_contract {
        match erase_field_type_parameters(&record.fields, &record.type_params) {
            Cow::Borrowed(_) => record,
            Cow::Owned(fields) => {
                erased = ExportedRecord {
                    fields,
                    ..record.clone()
                };
                &erased
            }
        }
    } else {
        record
    };
    let generics = if record.is_component_contract || record.type_params.is_empty() {
        String::new()
    } else {
        format!("<{}>", record.type_params.join(", "))
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
            "{} {}{} : {}",
            class_modifier,
            sanitize_csharp_identifier(&record.name),
            generics,
            csharp_type_name(base, context).text
        )
    } else {
        format!(
            "{} {}{}",
            class_modifier,
            sanitize_csharp_identifier(&record.name),
            generics
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
    for descendant in context.graph().polymorphic_descendants(&record.name) {
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
            .graph()
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
        current = context.graph().resolved_record_base(current)?;
    }
}

fn should_emit_missing_polymorphism_hint(
    record: &ExportedRecord,
    context: &CSharpRenderContext<'_>,
) -> bool {
    record.is_abstract
        && !graph_resolves_record_base(context, record)
        && context
            .graph()
            .polymorphic_descendants(&record.name)
            .is_empty()
}

/// Every field a declaration puts on the wire, paired with the declaration's name.
///
/// <para>An update companion's fields are left out: they carry no `[Key]`, so MessagePack resolves
/// them by their runtime type through the host's resolver rather than statically.</para>
fn declaration_fields(item: &ExportedType) -> Vec<(&str, &ExportedRecordField)> {
    match item {
        ExportedType::Record(record) => record
            .fields
            .iter()
            .map(|field| (record.name.as_str(), field))
            .collect(),
        ExportedType::ExternalState(state) => state
            .fields
            .iter()
            .map(|field| (state.name.as_str(), field))
            .collect(),
        ExportedType::Union(union_def) => union_def
            .cases
            .iter()
            .flat_map(|case| {
                case.fields
                    .iter()
                    .map(|field| (union_def.name.as_str(), field))
            })
            .collect(),
        ExportedType::Update(_) | ExportedType::Alias(_) => Vec::new(),
    }
}

/// One closed formatter a module declares: the instantiation it formats and the class name.
struct ClosedUpdateFormatter {
    /// The rendered C# type, such as `global::NxLang.Nx.NxRange_update<long>`.
    rendered_type: String,
    /// The generated class, such as `NxRange_updateOfLongFormatter`.
    class_name: String,
}

/// The closed formatters `module` needs: one per instantiation of a generic update companion that a
/// member is typed by, in a stable order.
///
/// <para>A generic companion carries `[MessagePackFormatter(typeof(Foo_updateFormatter&lt;&gt;))]`,
/// which is the form MessagePack documents and which its reflecting resolver closes at run time.
/// MessagePack's source generator cannot: asked for a formatter of `Foo_update&lt;long&gt;` it
/// rejects the unbound generic, so a member typed by one would not compile. A closed formatter
/// beside the member, named by the member's own attribute, is what the generator can resolve.</para>
///
/// <para>This reaches only a companion another assembly declares — the prelude's, or a
/// dependency's. A companion this file declares already carries its open generic shim, and a
/// second formatter of the same type in one compilation is `MsgPack009`; there is no form that
/// satisfies both, so such a field is reported by `collect_warnings` instead.</para>
fn collect_closed_update_formatters(
    module: &ExportedModule,
    context: &CSharpRenderContext<'_>,
) -> Vec<ClosedUpdateFormatter> {
    let mut formatters: Vec<ClosedUpdateFormatter> = Vec::new();
    let mut add = |ty: &TypeRef| {
        let Some(formatter) = closed_update_formatter_for(ty, context) else {
            return;
        };
        if !formatters
            .iter()
            .any(|existing| existing.class_name == formatter.class_name)
        {
            formatters.push(formatter);
        }
    };

    for declaration in &module.declarations {
        match &declaration.item {
            ExportedType::Record(record) => {
                for field in &record.fields {
                    add(&field.ty);
                }
            }
            ExportedType::ExternalState(state) => {
                for field in &state.fields {
                    add(&field.ty);
                }
            }
            ExportedType::Union(union_def) => {
                for case in &union_def.cases {
                    for field in &case.fields {
                        add(&field.ty);
                    }
                }
            }
            // An update companion's own fields are `NxOptional<…>` properties with no `[Key]`, so
            // MessagePack never resolves a formatter for them statically: the update formatter
            // serializes each set field by its runtime type through the host's resolver.
            ExportedType::Update(_) | ExportedType::Alias(_) => {}
        }
    }

    formatters.sort_by(|left, right| left.class_name.cmp(&right.class_name));
    formatters
}

/// Every closed update formatter a library's modules name, with the dependency namespaces the
/// modules that name them bring into scope.
///
/// <para>Deduplicated by class name across the whole generation: two modules that type a member by
/// the same instantiation name one formatter, and the shared file declares it once.</para>
struct LibraryClosedFormatters {
    formatters: Vec<ClosedUpdateFormatter>,
    dependency_namespaces: BTreeSet<String>,
}

fn collect_library_closed_update_formatters(
    graph: &ExportedTypeGraph,
    namespace: &str,
) -> LibraryClosedFormatters {
    let mut collected = LibraryClosedFormatters {
        formatters: Vec::new(),
        dependency_namespaces: BTreeSet::new(),
    };

    for module in &graph.modules {
        let imported_type_lookup = module
            .imported_types
            .iter()
            .cloned()
            .map(|imported_type| (imported_type.visible_name.clone(), imported_type))
            .collect::<FxHashMap<_, _>>();
        let context = CSharpRenderContext {
            namespace,
            types: ModuleTypes::for_module(graph, module),
            imported_types_by_visible_name: &imported_type_lookup,
            qualify_generated_types: false,
        };

        let formatters = collect_closed_update_formatters(module, &context);
        if formatters.is_empty() {
            continue;
        }

        // A formatter over a dependency's companion renders that type unqualified, so the shared
        // file needs the usings the module rendering it would have carried.
        collected
            .dependency_namespaces
            .extend(collect_dependency_namespaces(
                &module.imported_types,
                namespace,
            ));

        for formatter in formatters {
            if !collected
                .formatters
                .iter()
                .any(|existing| existing.class_name == formatter.class_name)
            {
                collected.formatters.push(formatter);
            }
        }
    }

    collected
        .formatters
        .sort_by(|left, right| left.class_name.cmp(&right.class_name));
    collected
}

/// The path of the shared formatter file: a name no module of the library takes.
fn closed_formatter_file_path(graph: &ExportedTypeGraph) -> PathBuf {
    for suffix in 0usize.. {
        let stem = if suffix == 0 {
            CLOSED_FORMATTER_FILE_BASENAME.to_string()
        } else {
            format!("{CLOSED_FORMATTER_FILE_BASENAME}{suffix}")
        };
        let candidate = PathBuf::from(stem);
        if graph
            .modules
            .iter()
            .all(|module| module.module_path != candidate)
        {
            return module_output_path(&candidate);
        }
    }

    unreachable!("formatter file name search should always terminate")
}

/// Renders the library's shared formatter file: every closed formatter, once, in the one namespace.
fn render_closed_formatter_file(
    shared: &LibraryClosedFormatters,
    namespace: &str,
    opts: &GenerateTypesOptions,
) -> String {
    let mut writer = CodeWriter::new(opts.format.clone());
    write_header(&mut writer);

    writer.line("using MessagePack;");
    writer.line("using MessagePack.Formatters;");
    writer.line("using NxLang.Nx.Serialization;");
    for dependency_namespace in &shared.dependency_namespaces {
        writer.line(&format!(
            "using {};",
            sanitize_csharp_qualified_name(dependency_namespace)
        ));
    }
    writer.blank_line();

    writer.block(
        &format!("namespace {}", sanitize_csharp_qualified_name(namespace)),
        |writer| {
            for (index, formatter) in shared.formatters.iter().enumerate() {
                emit_closed_update_formatter(writer, formatter);
                if index + 1 != shared.formatters.len() {
                    writer.blank_line();
                }
            }
        },
    );

    writer.finish()
}

/// The closed formatter a member typed `ty` names, or `None` when `ty` is not one instantiation of
/// a generic update companion.
///
/// <para>A nullable one is the same CLR type, so it answers the same formatter. Any other position
/// — a list of patches, say — is left alone and reported by `collect_warnings`, because the
/// attribute a member carries cannot reach inside its own type.</para>
fn closed_update_formatter_for(
    ty: &TypeRef,
    context: &CSharpRenderContext<'_>,
) -> Option<ClosedUpdateFormatter> {
    let inner = match ty {
        TypeRef::Seq { inner, occ } if !occ.admits_many() => inner.as_ref(),
        other => other,
    };
    let TypeRef::Applied { name, .. } = inner else {
        return None;
    };
    if !generic_update_companion(name.as_str(), context)
        || context.graph().declaration(name.as_str()).is_some()
    {
        return None;
    }
    let rendered = csharp_type(inner, context);
    Some(ClosedUpdateFormatter {
        class_name: closed_update_formatter_name(&rendered.text),
        rendered_type: rendered.text,
    })
}

/// Whether `type_name` is a generic update companion, wherever it is declared.
fn generic_update_companion(type_name: &str, context: &CSharpRenderContext<'_>) -> bool {
    let declaration = context
        .graph()
        .declaration(type_name)
        .or_else(|| context.types.prelude_declaration(type_name));
    match declaration.map(|declaration| &declaration.item) {
        Some(ExportedType::Update(update)) => {
            !update_companion_type_params(update, context.types).is_empty()
        }
        _ => false,
    }
}

/// The class name of the closed formatter of `rendered`: the companion, `Of`, and the arguments.
///
/// <para>Built from the whole rendered type, so two instantiations of one companion cannot collide
/// and neither can the prelude's `NxRange_update` with a library's own `Range_update`. Everything
/// that is not a letter or a digit becomes a word boundary, so a qualified argument such as
/// `global::Other.Thing` still reduces to one identifier.</para>
fn closed_update_formatter_name(rendered: &str) -> String {
    let (head, arguments) = rendered.split_once('<').unwrap_or((rendered, ""));
    let companion = head.rsplit(['.', ':']).next().unwrap_or(head);
    format!(
        "{}Of{}Formatter",
        companion,
        pascal_case_identifier(arguments)
    )
}

/// `text` reduced to one PascalCase identifier, with every run of other characters a word boundary.
fn pascal_case_identifier(text: &str) -> String {
    let mut identifier = String::new();
    let mut at_boundary = true;
    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            if at_boundary {
                identifier.extend(character.to_uppercase());
            } else {
                identifier.push(character);
            }
            at_boundary = false;
        } else {
            at_boundary = true;
        }
    }
    identifier
}

/// Emits one closed formatter, which delegates to the SDK's update-record formatter.
fn emit_closed_update_formatter(writer: &mut CodeWriter, formatter: &ClosedUpdateFormatter) {
    let ClosedUpdateFormatter {
        rendered_type,
        class_name,
    } = formatter;
    writer.block(
        &format!(
            "public sealed class {class_name} : IMessagePackFormatter<{rendered_type}?>"
        ),
        |writer| {
            writer.line(&format!(
                "private static readonly NxUpdateRecordMessagePackFormatter<{rendered_type}> Inner = new();"
            ));
            writer.blank_line();
            writer.line(&format!(
                "public void Serialize(ref MessagePackWriter writer, {rendered_type}? value, MessagePackSerializerOptions options) =>"
            ));
            writer.indent();
            writer.line("Inner.Serialize(ref writer, value!, options);");
            writer.dedent();
            writer.blank_line();
            writer.line(&format!(
                "public {rendered_type}? Deserialize(ref MessagePackReader reader, MessagePackSerializerOptions options) =>"
            ));
            writer.indent();
            writer.line("Inner.Deserialize(ref reader, options);");
            writer.dedent();
        },
    );
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
        let field_type = csharp_field_type(field, context);
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
            closed_update_formatter_for(&field.ty, context).as_ref(),
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
    closed_update_formatter: Option<&ClosedUpdateFormatter>,
) {
    if has_emitted_property {
        writer.blank_line();
    }

    emit_dual_wire_name_attributes(writer, wire_name);
    // A member typed by one instantiation of a generic companion names its formatter here, because
    // the open generic on the companion itself is one MessagePack's source generator cannot close.
    if let Some(formatter) = closed_update_formatter {
        writer.line(&format!(
            "[MessagePackFormatter(typeof({}))]",
            formatter.class_name
        ));
    }
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
    /// The graph as the module being rendered resolves names against it, so an import written in
    /// one module does not hide the prelude's `Range` from its siblings.
    types: ModuleTypes<'a>,
    imported_types_by_visible_name: &'a FxHashMap<String, ImportedType>,
    qualify_generated_types: bool,
}

impl<'a> CSharpRenderContext<'a> {
    fn graph(&self) -> &'a ExportedTypeGraph {
        self.types.graph()
    }
}

fn graph_resolves_record_base(context: &CSharpRenderContext<'_>, record: &ExportedRecord) -> bool {
    context.graph().resolved_record_base(record).is_some()
}

/// The C# type of a field: its declared type, made nullable when the field carries the `?` mark.
///
/// <para>`name?:T` is a nullable `T`, and `name?:T+` is a nullable array, `T[]?`: the property may
/// be absent, which the host reads as `null`. The mark is on the field rather than in its type, so
/// the declared type is never itself a `?` type and the `?` is added exactly once.</para>
fn csharp_field_type(field: &ExportedRecordField, context: &CSharpRenderContext<'_>) -> CSharpType {
    let ty = csharp_type(&field.ty, context);
    if field.optional {
        csharp_nullable(ty)
    } else {
        ty
    }
}

/// `ty` as a nullable type, unless it already is one.
fn csharp_nullable(mut ty: CSharpType) -> CSharpType {
    if !ty.is_nullable {
        ty.text = format!("{}?", ty.text);
        ty.is_nullable = true;
    }
    ty
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
        // `T?` is the item's type made nullable; `T+` and `T*` are both the array, since C# has no
        // static spelling for non-emptiness — the runtime checks that at the NX boundary.
        TypeRef::Seq { inner, occ } if !occ.admits_many() => {
            csharp_nullable(csharp_type_inner(inner, context, seen_aliases))
        }
        TypeRef::Seq { inner, .. } => {
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
        // An applied type is the instantiation of its record, with arguments in the record's
        // declaration order whatever order the source wrote them in. A generic record is
        // concrete and outside every polymorphic hierarchy, so no union attribute applies to it.
        TypeRef::Applied { name, args } => {
            let base = csharp_type_name_inner(name.as_str(), context, seen_aliases);
            let rendered = match context.types.record_type_params(name.as_str()) {
                Some(order) => order
                    .iter()
                    .filter_map(|param| {
                        args.iter()
                            .find(|(arg, _)| arg.as_str() == param)
                            .map(|(_, ty)| csharp_type_inner(ty, context, seen_aliases).text)
                    })
                    .collect::<Vec<_>>(),
                // The declaration could not be reached, so there is no order to sort into and the
                // source order is the only one there is. It is still rendered: the bare name would
                // be an open generic, which is CS0305, and a checked program wrote the arguments
                // against a declaration that does exist.
                None => args
                    .iter()
                    .map(|(_, ty)| csharp_type_inner(ty, context, seen_aliases).text)
                    .collect(),
            };
            if rendered.is_empty() {
                return base;
            }
            CSharpType {
                text: format!("{}<{}>", base.text, rendered.join(", ")),
                is_reference: true,
                is_nullable: false,
            }
        }
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
            // A prelude type is a hand-written SDK type, as a function value is `NxFunctionRef`:
            // generated C# opens with `using System;`, so `Range` would be `System.Range`, and two
            // generated libraries in one namespace would each declare their own copy. Its derived
            // companions are SDK types for the same reason, under the same `Nx` prefix, so
            // `Range_update` is `NxRange_update`. The module's own declarations answer first, so a
            // module that declares its own `Range` generates it and its companions.
            if context.types.prelude_declaration(other).is_some() {
                return CSharpType {
                    text: format!("global::NxLang.Nx.Nx{other}"),
                    is_reference: true,
                    is_nullable: false,
                };
            }
            if let Some(declaration) = context.graph().declaration(other) {
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
        } => csharp_imported_alias_target_type(
            target,
            &dependency_namespace,
            *target_is_reference,
            context,
        ),
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
    context: &CSharpRenderContext<'_>,
) -> CSharpType {
    match ty {
        TypeRef::Seq { inner, occ } if !occ.admits_many() => {
            csharp_nullable(csharp_imported_alias_target_type(
                inner,
                dependency_namespace,
                target_is_reference,
                context,
            ))
        }
        TypeRef::Seq { inner, .. } => {
            let inner = csharp_imported_alias_target_type(
                inner,
                dependency_namespace,
                target_is_reference,
                context,
            );
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
        // An imported alias whose target is an applied type is that instantiation: the alias
        // itself is not generated, so dropping the arguments here would leave an open generic at
        // every use of the alias. The arguments are names the dependency wrote, so they render in
        // the dependency's namespace like the tag does.
        TypeRef::Applied { name, args } => {
            let base = csharp_imported_alias_target_name(
                name.as_str(),
                dependency_namespace,
                target_is_reference,
                context,
            );
            let render = |ty: &TypeRef| {
                csharp_imported_alias_target_type(ty, dependency_namespace, true, context).text
            };
            let rendered = match context.types.record_type_params(name.as_str()) {
                Some(order) => order
                    .iter()
                    .filter_map(|param| {
                        args.iter()
                            .find(|(arg, _)| arg.as_str() == param)
                            .map(|(_, ty)| render(ty))
                    })
                    .collect::<Vec<_>>(),
                // The dependency's own declaration of the tag is not reachable from here — the
                // importing module named the alias, not the record. The source order is then the
                // only ordering there is, and is still better than an open generic.
                None => args.iter().map(|(_, ty)| render(ty)).collect(),
            };
            if rendered.is_empty() {
                base
            } else {
                CSharpType {
                    text: format!("{}<{}>", base.text, rendered.join(", ")),
                    is_reference: true,
                    is_nullable: false,
                }
            }
        }
        TypeRef::Name(name) => csharp_imported_alias_target_name(
            name.as_str(),
            dependency_namespace,
            target_is_reference,
            context,
        ),
    }
}

fn csharp_imported_alias_target_name(
    name: &str,
    dependency_namespace: &str,
    target_is_reference: bool,
    context: &CSharpRenderContext<'_>,
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
            // A prelude name the dependency did not declare itself is the prelude's here too, and
            // resolves to the SDK type: the dependency generates no `Range` of its own, so naming
            // one in its namespace would reference a type that does not exist. A `Range` the
            // dependency does declare is imported under that name, which `prelude_record` sees.
            if context.types.prelude_declaration(other).is_some() {
                return CSharpType {
                    text: format!("global::NxLang.Nx.Nx{other}"),
                    is_reference: true,
                    is_nullable: false,
                };
            }
            CSharpType {
                text: generated_type_name(other, dependency_namespace, true),
                is_reference: target_is_reference,
                is_nullable: false,
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
        TypeRef::Seq { inner, .. } => imported_alias_target_uses_dependency_namespace(inner),
        TypeRef::Function { .. } => false,
        TypeRef::Applied { name, .. } | TypeRef::Name(name) => !matches!(
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
