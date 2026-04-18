mod editorconfig;
mod languages;
mod model;
pub mod options;
mod writer;

use crate::codegen::options::FormatOptions;
use nx_api::LibraryArtifact;
use nx_hir::LoweredModule;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetLanguage {
    CSharp,
    TypeScript,
}

#[derive(Clone, Debug)]
pub struct GenerateTypesOptions {
    pub language: TargetLanguage,
    pub csharp_namespace: Option<String>,
    pub format: FormatOptions,
}

impl GenerateTypesOptions {
    fn csharp_namespace_or_default(&self) -> &str {
        self.csharp_namespace
            .as_deref()
            .unwrap_or(DEFAULT_CSHARP_NAMESPACE)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedFile {
    pub relative_path: PathBuf,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedOutput<T> {
    pub value: T,
    pub warnings: Vec<String>,
}

const DEFAULT_CSHARP_NAMESPACE: &str = "Nx.Generated";

pub fn format_options_from_editorconfig(
    language: TargetLanguage,
    editorconfig_path: &Path,
    target_file_name: &str,
) -> Result<FormatOptions, String> {
    let config = editorconfig::EditorConfig::parse_file(editorconfig_path)?;
    let mut opts = FormatOptions::defaults_for(language);
    config.apply_to(&mut opts, language, target_file_name);
    Ok(opts)
}

pub fn default_single_file_name(language: TargetLanguage) -> &'static str {
    match language {
        TargetLanguage::TypeScript => "types.ts",
        TargetLanguage::CSharp => "Types.g.cs",
    }
}

pub fn default_library_target_name(language: TargetLanguage) -> &'static str {
    match language {
        TargetLanguage::TypeScript => "index.ts",
        TargetLanguage::CSharp => "Types.g.cs",
    }
}

#[allow(dead_code)]
pub fn generate_types(
    module: &LoweredModule,
    source_path: &Path,
    opts: &GenerateTypesOptions,
) -> Result<String, String> {
    Ok(generate_types_with_warnings(module, source_path, opts)?.value)
}

pub fn generate_types_with_warnings(
    module: &LoweredModule,
    source_path: &Path,
    opts: &GenerateTypesOptions,
) -> Result<GeneratedOutput<String>, String> {
    let build = model::ExportedTypeGraph::from_module_with_warnings(module, source_path)?;
    let graph = &build.graph;

    let value = match opts.language {
        TargetLanguage::TypeScript => languages::typescript::emit_single_file(&graph, opts),
        TargetLanguage::CSharp => {
            languages::csharp::emit_single_file(&graph, opts.csharp_namespace_or_default(), opts)
        }
    }?;

    let mut warnings = build.warnings;
    warnings.extend(collect_language_warnings(graph, opts));

    Ok(GeneratedOutput { value, warnings })
}

#[allow(dead_code)]
pub fn generate_library_types(
    library: &LibraryArtifact,
    opts: &GenerateTypesOptions,
) -> Result<Vec<GeneratedFile>, String> {
    Ok(generate_library_types_with_warnings(library, opts)?.value)
}

pub fn generate_library_types_with_warnings(
    library: &LibraryArtifact,
    opts: &GenerateTypesOptions,
) -> Result<GeneratedOutput<Vec<GeneratedFile>>, String> {
    let build = model::ExportedTypeGraph::from_library_with_warnings(library)?;
    let graph = &build.graph;

    let value = match opts.language {
        TargetLanguage::TypeScript => languages::typescript::emit_library(&graph, opts),
        TargetLanguage::CSharp => {
            languages::csharp::emit_library(&graph, opts.csharp_namespace_or_default(), opts)
        }
    }?;

    let mut warnings = build.warnings;
    warnings.extend(collect_language_warnings(graph, opts));

    Ok(GeneratedOutput { value, warnings })
}

fn collect_language_warnings(
    graph: &model::ExportedTypeGraph,
    opts: &GenerateTypesOptions,
) -> Vec<String> {
    match opts.language {
        TargetLanguage::CSharp => {
            languages::csharp::collect_warnings(graph, opts.csharp_namespace_or_default())
        }
        TargetLanguage::TypeScript => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nx_api::build_library_artifact_from_directory;
    use nx_hir::{lower, SourceId};
    use nx_syntax::parse_str;
    use std::fs;
    use tempfile::TempDir;

    fn lower_module(source: &str, file_name: &str) -> LoweredModule {
        let parse_result = parse_str(source, file_name);
        let tree = parse_result.tree.expect("expected parse tree");
        lower(tree.root(), SourceId::new(0))
    }

    #[test]
    fn generates_typescript_exported_aliases_and_action_records_only() {
        let source = r#"
            type Hidden = string
            export type Theme = string
            export enum Direction = | north | south
            export action SearchRequested = { query:string }
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(output.contains("export type Theme = string;"));
        assert!(output.contains("export type Direction = \"north\" | \"south\";"));
        assert!(output.contains("export interface NxRecord<TType extends string = string>"));
        assert!(output
            .contains("export interface SearchRequested extends NxRecord<\"SearchRequested\">"));
        assert!(!output.contains("Hidden"));
    }

    #[test]
    fn generates_typescript_external_component_state_contracts_without_discriminator() {
        let source = r#"
            export external component <SearchBox /> = {
              state { query:string }
            }
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(output.contains("export interface SearchBox_state"));
        assert!(output.contains("query: string;"));
        assert!(!output.contains("NxRecord<\"SearchBox_state\">"));
        let state_block = output
            .split("export interface SearchBox_state")
            .nth(1)
            .and_then(|tail| tail.split("}").next())
            .expect("SearchBox_state block");
        assert!(!state_block.contains("$type"));
    }

    #[test]
    fn generates_typescript_external_component_props_with_discriminators() {
        let source = r#"
            export abstract external component <Question label:string />
            export external component <ShortTextQuestion extends Question placeholder:string? />
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(output.contains("export interface QuestionBase {"));
        assert!(output.contains("label: string;"));
        assert!(output.contains("export type Question = ShortTextQuestion;"));
        assert!(output
            .contains("export interface ShortTextQuestion extends QuestionBase, NxRecord<\"ShortTextQuestion\">"));
        assert!(output.contains("placeholder: string | null;"));
    }

    #[test]
    fn generates_typescript_concrete_root_records_with_discriminators() {
        let source = r#"
            export type Payload = { data:string }
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(output.contains("export interface NxRecord<TType extends string = string>"));
        assert!(output.contains("export interface Payload extends NxRecord<\"Payload\">"));
        assert!(output.contains("data: string;"));
    }

    #[test]
    fn generates_typescript_composed_list_and_nullable_types() {
        let source = r#"
            export type Matrix = string[][]
            export type MaybeNames = string[]?
            export type Payload = {
              aliases:string?[]
              maybeNames:string[]?
              matrix:string[][]
            }
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(output.contains("export type Matrix = string[][];"));
        assert!(output.contains("export type MaybeNames = string[] | null;"));
        assert!(output.contains("aliases: (string | null)[];"));
        assert!(output.contains("maybeNames: string[] | null;"));
        assert!(output.contains("matrix: string[][];"));
    }

    #[test]
    fn generates_typescript_abstract_record_runtime_unions() {
        let source = r#"
            export abstract type Question = { label:string }
            export type ShortTextQuestion extends Question = { placeholder:string? }
            export type LongTextQuestion extends Question = { wordLimit:int? }
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(output.contains("export interface QuestionBase {"));
        assert!(output.contains("export type Question = LongTextQuestion | ShortTextQuestion;"));
        assert!(output
            .contains("export interface ShortTextQuestion extends QuestionBase, NxRecord<\"ShortTextQuestion\">"));
        assert!(output
            .contains("export interface LongTextQuestion extends QuestionBase, NxRecord<\"LongTextQuestion\">"));
    }

    #[test]
    fn generates_typescript_abstract_action_runtime_unions() {
        let source = r#"
            export abstract action SearchAction = { source:string }
            export action SearchRequested extends SearchAction = { query:string }
            export action SearchSubmitted extends SearchAction = { submittedAt:string }
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(output.contains("export interface SearchActionBase {"));
        assert!(output.contains("export type SearchAction = SearchRequested | SearchSubmitted;"));
        assert!(output
            .contains("export interface SearchRequested extends SearchActionBase, NxRecord<\"SearchRequested\">"));
        assert!(output
            .contains("export interface SearchSubmitted extends SearchActionBase, NxRecord<\"SearchSubmitted\">"));
    }

    #[test]
    fn generates_csharp_global_aliases() {
        let source = r#"
            export type Count = int
            export type Name = string
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(output.contains("global using Count = long;"));
        assert!(output.contains("global using Name = string;"));
    }

    #[test]
    fn generates_csharp_global_aliases_before_non_global_usings() {
        let source = r#"
            export type Count = int
            export type Payload = { count: Count }
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();
        let global_using_index = output.find("global using Count = long;").unwrap();
        let using_system_index = output.find("using System;").unwrap();

        assert!(global_using_index < using_system_index);
    }

    #[test]
    fn omits_non_exported_external_component_state_contracts() {
        let source = r#"
            external component <SearchBox /> = {
              state { query:string }
            }
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(!output.contains("SearchBox_state"));
    }

    #[test]
    fn omits_non_exported_external_component_props_contracts() {
        let source = r#"
            external component <SearchBox placeholder:string />
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(!output.contains("SearchBox"));
    }

    #[test]
    fn generates_typescript_library_files_with_cross_module_imports() {
        let temp_dir = TempDir::new().expect("temp dir");
        let library_dir = temp_dir.path().join("ui");
        fs::create_dir_all(&library_dir).expect("library dir");
        fs::write(
            library_dir.join("theme.nx"),
            "export enum ThemeMode = | light | dark",
        )
        .expect("theme file");
        fs::write(
            library_dir.join("forms.nx"),
            "export type FormTheme = ThemeMode",
        )
        .expect("forms file");

        let artifact = build_library_artifact_from_directory(&library_dir).expect("library build");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let files = generate_library_types(&artifact, &opts).unwrap();
        assert_eq!(files.len(), 3);

        let forms = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("forms.ts"))
            .expect("forms.ts");
        assert!(forms
            .content
            .contains("import type { ThemeMode } from \"./theme\";"));
        assert!(forms.content.contains("export type FormTheme = ThemeMode;"));

        let index = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("index.ts"))
            .expect("index.ts");
        assert!(!index.content.contains("NxRecord"));
        assert!(index.content.contains("export * from \"./forms\";"));
        assert!(index.content.contains("export * from \"./theme\";"));
    }

    #[test]
    fn generates_typescript_library_files_for_external_component_state_contracts() {
        let temp_dir = TempDir::new().expect("temp dir");
        let library_dir = temp_dir.path().join("ui");
        fs::create_dir_all(&library_dir).expect("library dir");
        fs::write(
            library_dir.join("theme.nx"),
            "export enum ThemeMode = | light | dark",
        )
        .expect("theme file");
        fs::write(
            library_dir.join("search-box.nx"),
            r#"export external component <SearchBox /> = {
  state { theme:ThemeMode }
}"#,
        )
        .expect("search-box file");

        let artifact = build_library_artifact_from_directory(&library_dir).expect("library build");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let files = generate_library_types(&artifact, &opts).unwrap();
        let search_box = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("search-box.ts"))
            .expect("search-box.ts");
        assert!(search_box
            .content
            .contains("import type { ThemeMode } from \"./theme\";"));
        assert!(search_box
            .content
            .contains("export interface SearchBox_state"));
        assert!(search_box.content.contains("theme: ThemeMode;"));
        assert!(!search_box.content.contains("$type"));
    }

    #[test]
    fn generates_csharp_library_files_for_external_component_state_contracts() {
        let temp_dir = TempDir::new().expect("temp dir");
        let library_dir = temp_dir.path().join("ui");
        fs::create_dir_all(&library_dir).expect("library dir");
        fs::write(
            library_dir.join("theme.nx"),
            "export enum ThemeMode = | light | dark",
        )
        .expect("theme file");
        fs::write(
            library_dir.join("search-box.nx"),
            r#"export external component <SearchBox /> = {
  state { theme:ThemeMode }
}"#,
        )
        .expect("search-box file");

        let artifact = build_library_artifact_from_directory(&library_dir).expect("library build");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let files = generate_library_types(&artifact, &opts).unwrap();
        let search_box = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("search-box.g.cs"))
            .expect("search-box.g.cs");
        assert!(search_box.content.contains("namespace Test.Models"));
        assert!(search_box
            .content
            .contains("public sealed class SearchBox_state"));
        assert!(search_box
            .content
            .contains("public ThemeMode Theme { get; set; }"));
        let state_block = search_box
            .content
            .split("public sealed class SearchBox_state")
            .nth(1)
            .and_then(|tail| tail.split("}").next())
            .expect("SearchBox_state block");
        assert!(!state_block.contains("__NxType"));

        let theme = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("theme.g.cs"))
            .expect("theme.g.cs");
        assert!(theme.content.contains("public enum ThemeMode"));
    }

    #[test]
    fn generates_csharp_external_component_props_without_generated_discriminator_member() {
        let source = r#"
            export abstract external component <Question label:string />
            export external component <ShortTextQuestion extends Question placeholder:string? />
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(output.contains("public abstract class Question"));
        assert!(output.contains("[JsonPolymorphic(TypeDiscriminatorPropertyName = \"$type\")]"));
        assert!(output.contains("[Key(\"label\")]"));
        assert!(output.contains("public string Label { get; set; } = default!;"));
        assert!(output.contains("public sealed class ShortTextQuestion : Question"));
        assert!(!output.contains("__NxType"));
        assert!(output.contains("public string? Placeholder { get; set; }"));
    }

    #[test]
    fn generates_csharp_enums_with_shared_runtime_enum_serialization_helpers() {
        let source = r#"
            export enum DealStage = | draft | pending_review | closed_won
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(output.contains("using System.Text.Json.Serialization;"));
        assert!(output.contains("using NxLang.Nx.Serialization;"));
        assert!(output.contains(
            "[JsonConverter(typeof(NxEnumJsonConverter<DealStage, DealStageWireFormat>))]"
        ));
        assert!(output.contains(
            "[MessagePackFormatter(typeof(NxEnumMessagePackFormatter<DealStage, DealStageWireFormat>))]"
        ));
        assert!(output
            .contains("internal sealed class DealStageWireFormat : INxEnumWireFormat<DealStage>"));
        assert!(output.contains("public static string Format(DealStage value) =>"));
        assert!(output.contains("public static DealStage Parse(string value) =>"));
        assert!(output.contains("DealStage.PendingReview => \"pending_review\","));
        assert!(output.contains("\"pending_review\" => DealStage.PendingReview,"));
        assert!(!output.contains("public sealed class DealStageJsonConverter"));
        assert!(!output.contains("public sealed class DealStageMessagePackFormatter"));
        assert!(!output.contains("MessagePackType.Map"));
        assert!(!output.contains("$variant"));
        assert!(!output.contains("Expected string or map for NX enum."));
    }

    #[test]
    fn generates_csharp_enum_wire_mappings_when_clr_member_names_are_normalized() {
        let source = r#"
            export enum BuildTarget = | web_api | ios_app
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(output.contains("WebApi,"));
        assert!(output.contains("IosApp"));
        assert!(output.contains("BuildTarget.WebApi => \"web_api\","));
        assert!(output.contains("BuildTarget.IosApp => \"ios_app\","));
        assert!(output.contains("\"web_api\" => BuildTarget.WebApi,"));
        assert!(output.contains("\"ios_app\" => BuildTarget.IosApp,"));
    }

    #[test]
    fn generates_typescript_library_files_for_nested_modules() {
        let temp_dir = TempDir::new().expect("temp dir");
        let library_dir = temp_dir.path().join("ui");
        fs::create_dir_all(library_dir.join("components")).expect("components dir");
        fs::write(
            library_dir.join("theme.nx"),
            "export enum ThemeMode = | light | dark",
        )
        .expect("theme file");
        fs::write(
            library_dir.join("components").join("button.nx"),
            "export type ButtonTheme = ThemeMode",
        )
        .expect("button file");

        let artifact = build_library_artifact_from_directory(&library_dir).expect("library build");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let files = generate_library_types(&artifact, &opts).unwrap();
        let button = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("components/button.ts"))
            .expect("components/button.ts");
        assert!(button
            .content
            .contains("import type { ThemeMode } from \"../theme\";"));

        let index = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("index.ts"))
            .expect("index.ts");
        assert!(!index.content.contains("NxRecord"));
        assert!(index
            .content
            .contains("export * from \"./components/button\";"));
        assert!(index.content.contains("export * from \"./theme\";"));
    }

    #[test]
    fn generates_typescript_library_files_for_cross_module_abstract_record_families() {
        let temp_dir = TempDir::new().expect("temp dir");
        let library_dir = temp_dir.path().join("ui");
        fs::create_dir_all(&library_dir).expect("library dir");
        fs::write(
            library_dir.join("base.nx"),
            "export abstract type Question = { label:string }",
        )
        .expect("base file");
        fs::write(
            library_dir.join("short-text.nx"),
            "export type ShortTextQuestion extends Question = { placeholder:string? }",
        )
        .expect("short text file");

        let artifact = build_library_artifact_from_directory(&library_dir).expect("library build");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let files = generate_library_types(&artifact, &opts).unwrap();
        let base = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("base.ts"))
            .expect("base.ts");
        assert!(!base.content.contains("import type { NxRecord }"));
        assert!(base
            .content
            .contains("import type { ShortTextQuestion } from \"./short-text\";"));
        assert!(base.content.contains("export interface QuestionBase {"));
        assert!(base
            .content
            .contains("export type Question = ShortTextQuestion;"));

        let short_text = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("short-text.ts"))
            .expect("short-text.ts");
        assert!(short_text
            .content
            .contains("import type { NxRecord } from \"./_nx\";"));
        assert!(short_text
            .content
            .contains("import type { QuestionBase } from \"./base\";"));
        assert!(short_text
            .content
            .contains("export interface ShortTextQuestion extends QuestionBase, NxRecord<\"ShortTextQuestion\">"));

        let index = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("index.ts"))
            .expect("index.ts");
        assert!(index
            .content
            .contains("export type { NxRecord } from \"./_nx\";"));
    }

    #[test]
    fn generates_typescript_library_files_for_cross_module_abstract_action_families() {
        let temp_dir = TempDir::new().expect("temp dir");
        let library_dir = temp_dir.path().join("ui");
        fs::create_dir_all(&library_dir).expect("library dir");
        fs::write(
            library_dir.join("base.nx"),
            "export abstract action SearchAction = { source:string }",
        )
        .expect("base file");
        fs::write(
            library_dir.join("requested.nx"),
            "export action SearchRequested extends SearchAction = { query:string }",
        )
        .expect("requested file");

        let artifact = build_library_artifact_from_directory(&library_dir).expect("library build");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let files = generate_library_types(&artifact, &opts).unwrap();
        let base = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("base.ts"))
            .expect("base.ts");
        assert!(!base.content.contains("import type { NxRecord }"));
        assert!(base
            .content
            .contains("import type { SearchRequested } from \"./requested\";"));
        assert!(base.content.contains("export interface SearchActionBase {"));
        assert!(base
            .content
            .contains("export type SearchAction = SearchRequested;"));

        let requested = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("requested.ts"))
            .expect("requested.ts");
        assert!(requested
            .content
            .contains("import type { NxRecord } from \"./_nx\";"));
        assert!(requested
            .content
            .contains("import type { SearchActionBase } from \"./base\";"));
        assert!(requested
            .content
            .contains("export interface SearchRequested extends SearchActionBase, NxRecord<\"SearchRequested\">"));

        let index = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("index.ts"))
            .expect("index.ts");
        assert!(index
            .content
            .contains("export type { NxRecord } from \"./_nx\";"));
    }

    #[test]
    fn generates_typescript_library_files_when_source_module_matches_helper_name() {
        let temp_dir = TempDir::new().expect("temp dir");
        let library_dir = temp_dir.path().join("ui");
        fs::create_dir_all(&library_dir).expect("library dir");
        fs::write(
            library_dir.join("_nx.nx"),
            "export type Payload = { data:string }",
        )
        .expect("_nx source file");

        let artifact = build_library_artifact_from_directory(&library_dir).expect("library build");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let files = generate_library_types(&artifact, &opts).unwrap();
        assert_eq!(files.len(), 3);

        let payload_module = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("_nx.ts"))
            .expect("_nx.ts source output");
        assert!(payload_module
            .content
            .contains("import type { NxRecord } from \"./_nx1\";"));
        assert!(payload_module
            .content
            .contains("export interface Payload extends NxRecord<\"Payload\">"));

        let helper_module = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("_nx1.ts"))
            .expect("_nx1.ts helper output");
        assert!(helper_module
            .content
            .contains("export interface NxRecord<TType extends string = string>"));

        let index = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("index.ts"))
            .expect("index.ts");
        assert!(index
            .content
            .contains("export type { NxRecord } from \"./_nx1\";"));
        assert!(index.content.contains("export * from \"./_nx\";"));
    }

    #[test]
    fn generates_typescript_library_files_when_source_modules_match_nested_helper_names() {
        let temp_dir = TempDir::new().expect("temp dir");
        let library_dir = temp_dir.path().join("ui");
        fs::create_dir_all(&library_dir).expect("library dir");
        fs::write(
            library_dir.join("_nx.nx"),
            "export type Payload = { data:string }",
        )
        .expect("_nx source file");
        fs::write(
            library_dir.join("_nx1.nx"),
            "export type PayloadExtra = { flag:bool }",
        )
        .expect("_nx1 source file");

        let artifact = build_library_artifact_from_directory(&library_dir).expect("library build");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let files = generate_library_types(&artifact, &opts).unwrap();
        assert_eq!(files.len(), 4);

        let first_payload_module = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("_nx.ts"))
            .expect("_nx.ts source output");
        assert!(first_payload_module
            .content
            .contains("import type { NxRecord } from \"./_nx2\";"));
        assert!(first_payload_module
            .content
            .contains("export interface Payload extends NxRecord<\"Payload\">"));

        let second_payload_module = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("_nx1.ts"))
            .expect("_nx1.ts source output");
        assert!(second_payload_module
            .content
            .contains("import type { NxRecord } from \"./_nx2\";"));
        assert!(second_payload_module
            .content
            .contains("export interface PayloadExtra extends NxRecord<\"PayloadExtra\">"));

        let helper_module = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("_nx2.ts"))
            .expect("_nx2.ts helper output");
        assert!(helper_module
            .content
            .contains("export interface NxRecord<TType extends string = string>"));

        let index = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("index.ts"))
            .expect("index.ts");
        assert!(index
            .content
            .contains("export type { NxRecord } from \"./_nx2\";"));
        assert!(index.content.contains("export * from \"./_nx\";"));
        assert!(index.content.contains("export * from \"./_nx1\";"));
    }

    #[test]
    fn generates_csharp_external_component_state_contracts_without_discriminator() {
        let source = r#"
            export external component <SearchBox /> = {
              state { query:string theme:string? }
            }
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(output.contains("using System.Text.Json.Serialization;"));
        assert!(output.contains("[MessagePackObject]"));
        assert!(output.contains("public sealed class SearchBox_state"));
        let nl = opts.format.newline_str();
        let expected = [
            "public sealed class SearchBox_state",
            "    {",
            "        [Key(\"query\")]",
            "        [JsonPropertyName(\"query\")]",
            "        public string Query { get; set; } = default!;",
            "",
            "        [Key(\"theme\")]",
            "        [JsonPropertyName(\"theme\")]",
            "        public string? Theme { get; set; }",
            "    }",
        ]
        .join(nl);
        assert!(output.contains(&expected));
        let state_block = output
            .split("public sealed class SearchBox_state")
            .nth(1)
            .and_then(|tail| tail.split("}").next())
            .expect("SearchBox_state block");
        assert!(!state_block.contains("__NxType"));
        assert!(!state_block.contains("[Key(\"$type\")]"));
        assert!(!state_block.contains("[JsonPropertyName(\"$type\")]"));
    }

    #[test]
    fn generates_csharp_record_fields_without_synthetic_discriminator_collision() {
        let source = r#"
            export type Payload = { nx_type: string }
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(!output.contains("[JsonPropertyName(\"$type\")]"));
        assert!(!output.contains("__NxType"));
        assert!(output.contains("[JsonPropertyName(\"nx_type\")]"));
        assert!(output.contains("public string NxType { get; set; } = default!;"));
    }

    #[test]
    fn generates_csharp_minimal_concrete_record_without_discriminator_member() {
        let source = r#"
            export type ShortTextQuestion = { label:string }
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        let nl = opts.format.newline_str();
        let expected = [
            "public sealed class ShortTextQuestion",
            "    {",
            "        [Key(\"label\")]",
            "        [JsonPropertyName(\"label\")]",
            "        public string Label { get; set; } = default!;",
            "    }",
        ]
        .join(nl);

        assert!(output.contains(&expected));
        assert!(!output.contains("__NxType"));
        assert!(!output.contains("[Key(\"$type\")]"));
        assert!(!output.contains("[JsonPropertyName(\"$type\")]"));
    }

    #[test]
    fn generates_csharp_concrete_record_and_action_fields_with_dual_annotations() {
        let source = r#"
            export type ShortTextQuestion = { label:string placeholder:string? }
            export action SearchRequested = { query:string }
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(output.contains("using System.Text.Json.Serialization;"));
        assert!(output.contains("public sealed class ShortTextQuestion"));
        let nl = opts.format.newline_str();
        let expected = [
            "public sealed class ShortTextQuestion",
            "    {",
            "        [Key(\"label\")]",
            "        [JsonPropertyName(\"label\")]",
            "        public string Label { get; set; } = default!;",
            "",
            "        [Key(\"placeholder\")]",
            "        [JsonPropertyName(\"placeholder\")]",
            "        public string? Placeholder { get; set; }",
            "    }",
        ]
        .join(nl);
        assert!(output.contains(&expected));
        assert!(output.contains("public sealed class SearchRequested"));
        assert!(output.contains("[Key(\"query\")]"));
        assert!(output.contains("[JsonPropertyName(\"query\")]"));
        assert!(output.contains("public string Query { get; set; } = default!;"));
        assert!(!output.contains("__NxType"));
    }

    #[test]
    fn generate_types_warns_and_skips_conflicting_external_component_state_name() {
        let source = r#"
            export type SearchBox_state = string
            export external component <SearchBox /> = {
              state { query:string }
            }
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let output = generate_types_with_warnings(&module, Path::new("types.nx"), &opts)
            .expect("generation output");

        assert!(output
            .value
            .contains("export type SearchBox_state = string;"));
        assert!(!output.value.contains("export interface SearchBox_state"));
        assert_eq!(output.warnings.len(), 1);
        assert!(output.warnings[0].contains("SearchBox_state"));
    }

    #[test]
    fn generate_types_warns_when_csharp_abstract_root_has_no_concrete_descendants() {
        let source = r#"
            export abstract type Question = { label:string }
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types_with_warnings(&module, Path::new("types.nx"), &opts)
            .expect("generation output");

        assert!(output.value.contains("public abstract class Question"));
        assert!(!output.value.contains("[JsonPolymorphic("));
        assert!(!output.value.contains("[JsonDerivedType("));
        assert!(output.value.contains(
            "// No JsonPolymorphic metadata was generated because this abstract type had"
        ));
        assert!(output
            .value
            .contains("// no concrete exported descendants at code-generation time."));
        assert_eq!(output.warnings.len(), 1);
        assert!(output.warnings[0].contains("Question"));
        assert!(output.warnings[0].contains("no concrete exported descendants"));
    }

    #[test]
    fn generate_types_warns_when_imported_library_cannot_be_resolved() {
        let temp_dir = TempDir::new().expect("temp dir");
        let source_path = temp_dir.path().join("chat-link.nx");
        let module = lower_module(
            r#"import "../question-flow"

export type QuestionFlowInitialExperience = {
  questionFlow: QuestionFlow
}
"#,
            source_path.to_str().expect("source path"),
        );
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models.ChatLink".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output =
            generate_types_with_warnings(&module, &source_path, &opts).expect("generation output");

        assert_eq!(output.warnings.len(), 1);
        assert!(output.warnings[0].contains("../question-flow"));
        assert!(output
            .value
            .contains("public QuestionFlow QuestionFlow { get; set; } = default!;"));
    }

    #[test]
    fn generates_csharp_abstract_record_polymorphism_metadata_for_concrete_descendants() {
        let source = r#"
            export abstract type Question = { label:string }
            export type ShortTextQuestion extends Question = { placeholder:string? }
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(output.contains("[JsonPolymorphic(TypeDiscriminatorPropertyName = \"$type\")]"));
        assert!(
            output.contains("[JsonDerivedType(typeof(ShortTextQuestion), \"ShortTextQuestion\")]")
        );
        assert!(output.contains("[Union(0, typeof(ShortTextQuestion))]"));
        assert!(output.contains("public abstract class Question"));
        assert!(output.contains("public sealed class ShortTextQuestion : Question"));
        assert!(!output.contains("__NxType"));
        assert!(output.contains("[JsonPropertyName(\"placeholder\")]"));
    }

    #[test]
    fn generates_csharp_abstract_action_polymorphism_metadata_for_concrete_descendants() {
        let source = r#"
            export abstract action SearchAction = { source:string }
            export action SearchRequested extends SearchAction = { query:string }
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(output.contains("[JsonPolymorphic(TypeDiscriminatorPropertyName = \"$type\")]"));
        assert!(output.contains("[JsonDerivedType(typeof(SearchRequested), \"SearchRequested\")]"));
        assert!(output.contains("[Union(0, typeof(SearchRequested))]"));
        assert!(output.contains("public abstract class SearchAction"));
        assert!(output.contains("[JsonPropertyName(\"source\")]"));
        assert!(output.contains("public string Source { get; set; } = default!;"));
        assert!(output.contains("public sealed class SearchRequested : SearchAction"));
        assert!(!output.contains("__NxType"));
        assert!(output.contains("[JsonPropertyName(\"query\")]"));
    }

    #[test]
    fn generates_csharp_composed_list_and_nullable_field_types() {
        let source = r#"
            export type Payload = {
              matrix:string[][]
              maybeNames:string[]?
              aliases:string?[]
            }
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(output.contains("public string[][] Matrix { get; set; } = default!;"));
        assert!(output.contains("public string[]? MaybeNames { get; set; }"));
        assert!(output.contains("public string?[] Aliases { get; set; } = default!;"));
    }

    #[test]
    fn generates_csharp_multi_level_abstract_record_polymorphism_metadata() {
        let source = r#"
            export abstract type Question = { label:string }
            export abstract type TextQuestion extends Question = { placeholder:string? }
            export type ShortTextQuestion extends TextQuestion = { maxLength:int? }
        "#;
        let module = lower_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(output.contains("[JsonPolymorphic(TypeDiscriminatorPropertyName = \"$type\")]"));
        assert!(
            output.contains("[JsonDerivedType(typeof(ShortTextQuestion), \"ShortTextQuestion\")]")
        );
        assert!(output.contains("[Union(0, typeof(ShortTextQuestion))]"));
        assert!(output.contains("public abstract class Question"));
        assert!(output.contains("public abstract class TextQuestion : Question"));
        assert!(output.contains("public sealed class ShortTextQuestion : TextQuestion"));
        assert!(!output.contains("__NxType"));

        let text_question_block = output
            .split("public abstract class TextQuestion : Question")
            .nth(1)
            .and_then(|tail| {
                tail.split("public sealed class ShortTextQuestion : TextQuestion")
                    .next()
            })
            .expect("TextQuestion block");
        assert!(
            !text_question_block.contains("__NxType"),
            "intermediate abstract records should inherit the root discriminator without redeclaring it"
        );
        assert!(
            !text_question_block.contains("[JsonPolymorphic("),
            "intermediate abstract records should inherit the root polymorphism contract"
        );
    }

    #[test]
    fn generates_csharp_library_files_for_nested_modules() {
        let temp_dir = TempDir::new().expect("temp dir");
        let library_dir = temp_dir.path().join("ui");
        fs::create_dir_all(library_dir.join("components")).expect("components dir");
        fs::write(
            library_dir.join("theme.nx"),
            "export enum ThemeMode = | light | dark",
        )
        .expect("theme file");
        fs::write(
            library_dir.join("components").join("button.nx"),
            "export type ButtonState = { theme: ThemeMode }",
        )
        .expect("button file");

        let artifact = build_library_artifact_from_directory(&library_dir).expect("library build");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let files = generate_library_types(&artifact, &opts).unwrap();
        let button = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("components/button.g.cs"))
            .expect("components/button.g.cs");
        assert!(button.content.contains("namespace Test.Models"));
        assert!(button
            .content
            .contains("public ThemeMode Theme { get; set; }"));
    }

    #[test]
    fn generates_csharp_library_aliases_with_global_qualified_cross_module_types() {
        let temp_dir = TempDir::new().expect("temp dir");
        let library_dir = temp_dir.path().join("ui");
        fs::create_dir_all(&library_dir).expect("library dir");
        fs::write(
            library_dir.join("theme.nx"),
            "export enum ThemeMode = | light | dark",
        )
        .expect("theme file");
        fs::write(
            library_dir.join("aliases.nx"),
            "export type ThemeAlias = ThemeMode",
        )
        .expect("alias file");

        let artifact = build_library_artifact_from_directory(&library_dir).expect("library build");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let files = generate_library_types(&artifact, &opts).unwrap();
        let aliases = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("aliases.g.cs"))
            .expect("aliases.g.cs");
        assert!(aliases
            .content
            .contains("global using ThemeAlias = global::Test.Models.ThemeMode;"));
    }

    #[test]
    fn generates_csharp_library_files_with_dependency_namespace_usings() {
        let temp_dir = TempDir::new().expect("temp dir");
        let question_flow_dir = temp_dir.path().join("question-flow");
        let chat_link_dir = temp_dir.path().join("chat-link");
        fs::create_dir_all(&question_flow_dir).expect("question-flow dir");
        fs::create_dir_all(&chat_link_dir).expect("chat-link dir");

        fs::write(
            question_flow_dir.join("QuestionFlow.nx"),
            "export type QuestionFlow = { id:string }",
        )
        .expect("question-flow file");
        fs::write(
            chat_link_dir.join("ChatLinkConfig.nx"),
            r#"import "../question-flow"

export type QuestionFlowInitialExperience = {
  questionFlow: QuestionFlow
}
"#,
        )
        .expect("chat-link file");

        let artifact =
            build_library_artifact_from_directory(&chat_link_dir).expect("library build");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models.ChatLink".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_library_types_with_warnings(&artifact, &opts).unwrap();
        let chat_link = output
            .value
            .iter()
            .find(|file| file.relative_path == PathBuf::from("ChatLinkConfig.g.cs"))
            .expect("ChatLinkConfig.g.cs");

        assert_eq!(output.warnings.len(), 1);
        assert!(output.warnings[0].contains("question-flow"));
        assert!(output.warnings[0].contains("Test.Models.QuestionFlow"));
        assert!(chat_link
            .content
            .contains("using Test.Models.QuestionFlow;"));
        assert!(chat_link
            .content
            .contains(
                "public global::Test.Models.QuestionFlow.QuestionFlow QuestionFlow { get; set; } = default!;"
            ));
    }

    #[test]
    fn csharp_dependency_namespace_warning_matches_emitted_namespace_for_digit_prefixed_library() {
        let temp_dir = TempDir::new().expect("temp dir");
        let dependency_dir = temp_dir.path().join("123dep");
        let chat_link_dir = temp_dir.path().join("chat-link");
        fs::create_dir_all(&dependency_dir).expect("dependency dir");
        fs::create_dir_all(&chat_link_dir).expect("chat-link dir");

        fs::write(
            dependency_dir.join("QuestionFlow.nx"),
            "export type QuestionFlow = { id:string }",
        )
        .expect("dependency file");
        fs::write(
            chat_link_dir.join("ChatLinkConfig.nx"),
            r#"import "../123dep"

export type QuestionFlowInitialExperience = {
  questionFlow: QuestionFlow
}
"#,
        )
        .expect("chat-link file");

        let artifact =
            build_library_artifact_from_directory(&chat_link_dir).expect("library build");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models.ChatLink".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_library_types_with_warnings(&artifact, &opts).unwrap();
        let chat_link = output
            .value
            .iter()
            .find(|file| file.relative_path == PathBuf::from("ChatLinkConfig.g.cs"))
            .expect("ChatLinkConfig.g.cs");
        let dependency_using = chat_link
            .content
            .lines()
            .find(|line| line.starts_with("using Test.Models."))
            .expect("dependency using");
        let expected_namespace = dependency_using
            .strip_prefix("using ")
            .and_then(|line| line.strip_suffix(';'))
            .expect("dependency namespace");

        assert_eq!(output.warnings.len(), 1);
        assert!(output.warnings[0].contains(&expected_namespace));
        assert!(chat_link.content.contains(&format!(
            "public global::{expected_namespace}.QuestionFlow QuestionFlow {{ get; set; }} = default!;"
        )));
    }
}
