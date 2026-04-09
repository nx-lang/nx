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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedFile {
    pub relative_path: PathBuf,
    pub content: String,
}

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

pub fn generate_types(
    module: &LoweredModule,
    source_path: &Path,
    opts: &GenerateTypesOptions,
) -> Result<String, String> {
    let graph = model::ExportedTypeGraph::from_module(module, source_path)?;

    match opts.language {
        TargetLanguage::TypeScript => languages::typescript::emit_single_file(&graph, opts),
        TargetLanguage::CSharp => {
            let namespace = opts.csharp_namespace.as_deref().unwrap_or("Nx.Generated");
            languages::csharp::emit_single_file(&graph, namespace, opts)
        }
    }
}

pub fn generate_library_types(
    library: &LibraryArtifact,
    opts: &GenerateTypesOptions,
) -> Result<Vec<GeneratedFile>, String> {
    let graph = model::ExportedTypeGraph::from_library(library)?;

    match opts.language {
        TargetLanguage::TypeScript => languages::typescript::emit_library(&graph, opts),
        TargetLanguage::CSharp => {
            let namespace = opts.csharp_namespace.as_deref().unwrap_or("Nx.Generated");
            languages::csharp::emit_library(&graph, namespace, opts)
        }
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
    fn generates_csharp_record_fields_without_colliding_with_type_discriminator() {
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

        assert!(output.contains("public string __NxType { get; set; } = \"Payload\";"));
        assert!(output.contains("public string NxType { get; set; } = default!;"));
    }

    #[test]
    fn generates_csharp_abstract_record_discriminators_for_concrete_descendants() {
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

        assert!(output.contains("public abstract class Question"));
        assert!(output.contains("public abstract string __NxType { get; set; }"));
        assert!(output.contains("public sealed class ShortTextQuestion : Question"));
        assert!(output
            .contains("public override string __NxType { get; set; } = \"ShortTextQuestion\";"));
    }

    #[test]
    fn generates_csharp_multi_level_abstract_record_discriminators() {
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

        assert!(output.contains("public abstract class Question"));
        assert!(output.contains("public abstract string __NxType { get; set; }"));
        assert!(output.contains("public abstract class TextQuestion : Question"));
        assert!(output.contains("public sealed class ShortTextQuestion : TextQuestion"));
        assert!(output
            .contains("public override string __NxType { get; set; } = \"ShortTextQuestion\";"));

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
}
