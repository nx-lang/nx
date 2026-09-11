mod editorconfig;
mod languages;
mod model;
pub mod options;
mod writer;

use crate::typegen::options::FormatOptions;
use nx_api::LibraryArtifact;
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
    pub typescript_package_prefix: Option<String>,
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
    source: &str,
    source_path: &Path,
    opts: &GenerateTypesOptions,
) -> Result<String, String> {
    Ok(generate_types_with_warnings(source, source_path, opts)?.value)
}

/// Generates types for one source file, resolving the libraries it imports from `source_path`.
pub fn generate_types_with_warnings(
    source: &str,
    source_path: &Path,
    opts: &GenerateTypesOptions,
) -> Result<GeneratedOutput<String>, String> {
    let build = model::ExportedTypeGraph::from_source_with_warnings(source, source_path)?;
    let graph = &build.graph;

    let value = match opts.language {
        TargetLanguage::TypeScript => languages::typescript::emit_single_file(&graph, opts),
        TargetLanguage::CSharp => {
            languages::csharp::emit_single_file(&graph, opts.csharp_namespace_or_default(), opts)
        }
    }?;

    let mut warnings = build.warnings;
    warnings.extend(collect_language_warnings(graph, opts, false));

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
    warnings.extend(collect_language_warnings(graph, opts, true));

    Ok(GeneratedOutput { value, warnings })
}

fn collect_language_warnings(
    graph: &model::ExportedTypeGraph,
    opts: &GenerateTypesOptions,
    include_imports: bool,
) -> Vec<String> {
    match opts.language {
        TargetLanguage::CSharp => {
            languages::csharp::collect_warnings(graph, opts.csharp_namespace_or_default())
        }
        TargetLanguage::TypeScript => languages::typescript::collect_warnings(
            graph,
            opts.typescript_package_prefix.as_deref(),
            include_imports,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nx_api::build_library_artifact_from_directory;
    use std::fs;
    use tempfile::TempDir;

    /// The source a test generates from; generation lowers and analyzes it itself.
    fn source_module(source: &str, _file_name: &str) -> String {
        source.to_string()
    }

    fn generate_for(source: &str, language: TargetLanguage) -> String {
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language,
            csharp_namespace: None,
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(language),
        };
        generate_types(&module, Path::new("types.nx"), &opts).unwrap()
    }

    /// `void` is no longer a primitive, so a user may declare a type with that name. The
    /// primitive-name-to-host-type maps must not intercept it: the reference has to resolve to the
    /// declaration, as it does for every other user type name.
    ///
    /// <para>What the name then renders as is a separate, pre-existing question — a user type named
    /// after a host keyword (`class`, `void`) emits an unescaped identifier today, and did so for
    /// `class` before this change. That defect is recorded in the proposal's "Not in this change".
    /// This test pins the part the change is responsible for: the declaration is generated and the
    /// field refers to it, rather than the field silently becoming the host's `void`.</para>
    #[test]
    fn a_user_declared_void_type_is_not_mapped_to_the_host_void_type() {
        let source = "export type void = { value:int }\nexport type Holder = { n:void }\n";

        let csharp = generate_for(source, TargetLanguage::CSharp);
        assert!(
            csharp.contains("class void"),
            "the user's record should be generated:\n{csharp}"
        );
        assert!(
            csharp.contains("[Key(\"value\")]"),
            "the user's record should carry its own field:\n{csharp}"
        );

        let typescript = generate_for(source, TargetLanguage::TypeScript);
        assert!(
            typescript.contains("interface void"),
            "the user's record should be generated:\n{typescript}"
        );
        // The reference reads `n: void`, which is what the keyword collision makes unavoidable
        // without escaping. What this asserts is the part in scope: the name reached the
        // declaration lookup, which is why an interface exists for it at all.
        assert!(
            typescript.contains("value: number"),
            "the user's record should carry its own field:\n{typescript}"
        );
    }

    #[test]
    fn generates_the_same_defaults_for_both_spellings_of_a_whole_float() {
        // This path reads a lowered module and never type checks, so the HIR conversion has not
        // run: the spelling has to be settled from the field's own type or the two forms diverge.
        for language in [TargetLanguage::CSharp, TargetLanguage::TypeScript] {
            let written_as_int = generate_for(
                "export type Opts = { x:float64 = 0 y:float32 = 1 }",
                language,
            );
            let written_as_float = generate_for(
                "export type Opts = { x:float64 = 0.0 y:float32 = 1.0 }",
                language,
            );

            assert_eq!(
                written_as_int, written_as_float,
                "{:?} output should not depend on which spelling was written",
                language
            );
        }
    }

    #[test]
    fn generates_csharp_float_defaults_with_a_floating_point_spelling() {
        let output = generate_for(
            "export type Opts = { x:float64 = 0 y:float32 = 1 }",
            TargetLanguage::CSharp,
        );

        assert!(
            output.contains("= 0.0;"),
            "a double default should read as a double: {}",
            output
        );
        assert!(
            output.contains("= 1.0f;"),
            "a float default should carry the float suffix: {}",
            output
        );
        // An integer field keeps its integer spelling.
        let integers = generate_for("export type Opts = { n:int = 0 }", TargetLanguage::CSharp);
        assert!(integers.contains("= 0;"), "{}", integers);
        assert!(!integers.contains("= 0.0;"), "{}", integers);
    }

    #[test]
    fn generates_typescript_exported_aliases_and_action_records_only() {
        let source = r#"
            type Hidden = string
            export type Theme = string
            export type Direction = north | south
            export action SearchRequested = { query:string }
        "#;
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            typescript_package_prefix: None,
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
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            typescript_package_prefix: None,
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
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            typescript_package_prefix: None,
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
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            typescript_package_prefix: None,
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
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            typescript_package_prefix: None,
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
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            typescript_package_prefix: None,
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
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            typescript_package_prefix: None,
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
    fn generates_typescript_discriminated_unions() {
        let source = r#"
            export abstract type EventBase = { source:string }
            export type LoadState =
              | idle
              | failed { message:string retryable:boolean = true }
            export type UiEvent extends EventBase =
              | clicked { x:int }
              | closed
        "#;
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        // `idle` declares no fields and `LoadState` declares no base, so it is a constant case:
        // a bare string on the wire, and a string literal in the type (design D3).
        assert!(
            !output.contains("LoadStateIdle"),
            "a constant case needs no record type: {output}"
        );
        assert!(output
            .contains("export interface LoadStateFailed extends NxRecord<\"LoadState.failed\">"));
        assert!(output.contains("message: string;"));
        assert!(output.contains("retryable: boolean;"));
        assert!(output.contains("export type LoadState = \"idle\" | LoadStateFailed;"));
        assert!(output.contains("export interface EventBaseBase {"));
        assert!(output.contains("source: string;"));
        assert!(output.contains(
            "export interface UiEventClicked extends EventBaseBase, NxRecord<\"UiEvent.clicked\">"
        ));
        assert!(output.contains(
            "export interface UiEventClosed extends EventBaseBase, NxRecord<\"UiEvent.closed\">"
        ));
        assert!(output.contains("export type EventBase = UiEventClicked | UiEventClosed;"));
        assert!(output.contains("export type UiEvent = UiEventClicked | UiEventClosed;"));
    }

    /// A fieldless case of a union that extends a base is not constant: it carries the base's
    /// fields at runtime, so it keeps the record representation (design D3). `UiEvent.closed`
    /// above is that case, and it still generates an interface.
    #[test]
    fn a_fieldless_case_of_a_based_union_is_not_a_constant_case() {
        let source = r#"
            export abstract type EventBase = { source:string }
            export type UiEvent extends EventBase = clicked { x:int } | closed
        "#;
        let module = source_module(source, "types.nx");
        let output = generate_types(
            &module,
            Path::new("types.nx"),
            &GenerateTypesOptions {
                language: TargetLanguage::TypeScript,
                csharp_namespace: None,
                typescript_package_prefix: None,
                format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
            },
        )
        .unwrap();

        assert!(
            output.contains("export interface UiEventClosed"),
            "a fieldless case of a based union keeps its record type: {output}"
        );
        assert!(
            !output.contains("\"closed\""),
            "and is not emitted as a bare string: {output}"
        );
    }

    /// Generated C# and TypeScript type definitions for a module whose only difference is how its
    /// closed set of constants is declared.
    fn constant_set_types(declaration: &str) -> (String, String) {
        let source = format!("{declaration}\nexport type Settings = {{ fit: Fit }}\n");
        let module = source_module(&source, "types.nx");
        let csharp = generate_types(
            &module,
            Path::new("types.nx"),
            &GenerateTypesOptions {
                language: TargetLanguage::CSharp,
                csharp_namespace: Some("Test.Models".to_string()),
                typescript_package_prefix: None,
                format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
            },
        )
        .unwrap();
        let typescript = generate_types(
            &module,
            Path::new("types.nx"),
            &GenerateTypesOptions {
                language: TargetLanguage::TypeScript,
                csharp_namespace: None,
                typescript_package_prefix: None,
                format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
            },
        )
        .unwrap();

        (csharp, typescript)
    }

    /// The optional leading `|` is purely syntactic: both spellings of the same constant union
    /// generate identical types.
    ///
    /// This began as the guard that an `enum` and the equivalent `type` generate the same thing.
    /// With `enum` removed there is one keyword left, so what remains to guard is D2's two
    /// case-list spellings.
    #[test]
    fn both_case_list_spellings_generate_identical_csharp_and_typescript_types() {
        let (bare_csharp, bare_typescript) =
            constant_set_types("export type Fit = fill | contain | cover");
        let (piped_csharp, piped_typescript) =
            constant_set_types("export type Fit = fill | contain | cover");

        assert_eq!(bare_csharp, piped_csharp, "generated C#");
        assert_eq!(bare_typescript, piped_typescript, "generated TypeScript");
    }

    /// Pins the host shape each kind of union generates, which is what byte identity with the enum
    /// form actually means for this generator.
    ///
    /// CLI type generation emits a constant union as the union of its authored string literals —
    /// not as the `as const` value object, which is executable codegen's form for the same
    /// declaration. The two emitters differ deliberately: this one emits a type surface that erases
    /// completely, so it exports no runtime values at all (design D4).
    #[test]
    fn constant_and_mixed_unions_generate_their_respective_host_shapes() {
        let (constant_csharp, constant_typescript) =
            constant_set_types("export type Fit = fill | cover");

        assert!(
            constant_csharp.contains("public enum Fit"),
            "a constant union generates a C# enum, got:\n{constant_csharp}"
        );
        assert!(
            constant_typescript.contains("export type Fit = \"fill\" | \"cover\";"),
            "a constant union generates a TypeScript string-literal union, got:\n{constant_typescript}"
        );
        for runtime_export in [
            "export const",
            "export function",
            "export class",
            "export enum",
        ] {
            assert!(
                !constant_typescript.contains(runtime_export),
                "generated type declarations are a pure type surface, but found `{runtime_export}` \
                 in:\n{constant_typescript}"
            );
        }

        let (mixed_csharp, mixed_typescript) =
            constant_set_types("export type Fit = fill | scaled { factor:real }");

        assert!(
            mixed_csharp.contains("abstract class Fit"),
            "a mixed union generates the polymorphic C# shape, got:\n{mixed_csharp}"
        );
        assert!(
            mixed_typescript.contains("\"fill\""),
            "a constant case contributes its string literal to the TypeScript union, got:\n{mixed_typescript}"
        );
    }

    #[test]
    fn generates_csharp_alias_only_output_without_global_usings() {
        let source = r#"
            export type Count = int
            export type Name = string
        "#;
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(!output.contains("global using"));
        assert!(!output.contains("namespace Test.Models"));
    }

    #[test]
    fn generates_csharp_alias_references_transparently() {
        let source = r#"
            export type Count = int
            export type Payload = { count: Count }
        "#;
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(!output.contains("global using Count"));
        assert!(output.contains("public long Count { get; set; }"));
    }

    #[test]
    fn omits_non_exported_external_component_state_contracts() {
        let source = r#"
            external component <SearchBox /> = {
              state { query:string }
            }
        "#;
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            typescript_package_prefix: None,
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
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            typescript_package_prefix: None,
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
            "export type ThemeMode = light | dark",
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
            typescript_package_prefix: None,
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
    fn generates_typescript_library_files_for_cross_module_union_fields() {
        let temp_dir = TempDir::new().expect("temp dir");
        let library_dir = temp_dir.path().join("ui");
        fs::create_dir_all(&library_dir).expect("library dir");
        fs::write(
            library_dir.join("items.nx"),
            "export type Item = { name:string }",
        )
        .expect("items file");
        fs::write(
            library_dir.join("state.nx"),
            "export type LoadState = | loaded { items:Item[] }",
        )
        .expect("state file");

        let artifact = build_library_artifact_from_directory(&library_dir).expect("library build");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let files = generate_library_types(&artifact, &opts).unwrap();
        let state = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("state.ts"))
            .expect("state.ts");

        assert!(state
            .content
            .contains("import type { Item } from \"./items\";"));
        assert!(state.content.contains("items: Item[];"));
        assert!(state
            .content
            .contains("export type LoadState = LoadStateLoaded;"));
    }

    #[test]
    fn generates_typescript_library_files_for_external_component_state_contracts() {
        let temp_dir = TempDir::new().expect("temp dir");
        let library_dir = temp_dir.path().join("ui");
        fs::create_dir_all(&library_dir).expect("library dir");
        fs::write(
            library_dir.join("theme.nx"),
            "export type ThemeMode = light | dark",
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
            typescript_package_prefix: None,
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
        let (state, update) = search_box
            .content
            .split_once("export interface SearchBox_update")
            .expect("the state contract and its update companion");
        assert!(!state.contains("$type"));
        assert!(update.contains("$type: \"SearchBox.Update\";"));
        assert!(update.contains("theme?: ThemeMode;"));
    }

    #[test]
    fn generates_csharp_library_files_for_external_component_state_contracts() {
        let temp_dir = TempDir::new().expect("temp dir");
        let library_dir = temp_dir.path().join("ui");
        fs::create_dir_all(&library_dir).expect("library dir");
        fs::write(
            library_dir.join("theme.nx"),
            "export type ThemeMode = light | dark",
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
            typescript_package_prefix: None,
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
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            typescript_package_prefix: None,
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
    fn capitalized_primitive_spellings_are_not_mapped_to_host_primitives() {
        // Primitive names are case-sensitive. `INT64` and friends are ordinary named types, so
        // they must never be emitted as C# primitives.
        let source = r#"
            export type Weird = {
              a:INT64
              b:Boolean
              c:String
            }
        "#;
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(!output.contains("public long A"), "got: {}", output);
        assert!(!output.contains("public bool B"), "got: {}", output);
        assert!(!output.contains("public string C"), "got: {}", output);
    }

    #[test]
    fn generates_csharp_numeric_widths_including_int() {
        let source = r#"
            export type Sizes = {
              count:int
              narrow:int32
              wide:int64
              ratio:float32
              precise:float64
            }
        "#;
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        // `int` is exact over +/-(2^53-1), which does not fit a C# `int`, so it maps to `long`.
        assert!(
            output.contains("public long Count { get; set; }"),
            "{}",
            output
        );
        assert!(
            output.contains("public int Narrow { get; set; }"),
            "{}",
            output
        );
        assert!(
            output.contains("public long Wide { get; set; }"),
            "{}",
            output
        );
        assert!(
            output.contains("public float Ratio { get; set; }"),
            "{}",
            output
        );
        assert!(
            output.contains("public double Precise { get; set; }"),
            "{}",
            output
        );
    }

    #[test]
    fn generates_typescript_numeric_widths_including_int() {
        let source = r#"
            export type Sizes = {
              count:int
              narrow:int32
              wide:int64
              ratio:float32
              precise:float64
            }
        "#;
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        // Every numeric primitive is a TypeScript `number`, `int64` included. Carrying `int64`
        // as `bigint` is a separate change; today it is still emitted as `number`.
        for field in ["count", "narrow", "wide", "ratio", "precise"] {
            assert!(
                output.contains(&format!("{}: number", field)),
                "expected `{}: number` in: {}",
                field,
                output
            );
        }
    }

    #[test]
    fn generates_csharp_record_field_literal_default_initializers() {
        let source = r#"
            export type Settings = {
              enabled:boolean = true
              count:int = 42
              title:string = "hello"
              maybe:string? = null
              ratio:float64 = 0.25
              small:float32 = 0.5
              maybeSmall:float32? = 0.5
            }
        "#;
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(output.contains("public bool Enabled { get; set; } = true;"));
        assert!(output.contains("public long Count { get; set; } = 42;"));
        assert!(output.contains("public string Title { get; set; } = \"hello\";"));
        assert!(output.contains("public string? Maybe { get; set; } = null;"));
        assert!(output.contains("public double Ratio { get; set; } = 0.25;"));
        assert!(output.contains("public float Small { get; set; } = 0.5f;"));
        assert!(output.contains("public float? MaybeSmall { get; set; } = 0.5f;"));
        assert!(!output.contains("public string Title { get; set; } = default!;"));
    }

    #[test]
    fn generates_csharp_union_and_external_component_literal_default_initializers() {
        let source = r#"
            export type LoadState =
              | failed { retryable:boolean = true }
            export external component <Toggle selected:boolean = true label:string = "On" />
        "#;
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(output.contains("public sealed class LoadStateFailed : LoadState"));
        assert!(output.contains("public bool Retryable { get; set; } = true;"));
        assert!(output.contains("public sealed class Toggle"));
        assert!(output.contains("public bool Selected { get; set; } = true;"));
        assert!(output.contains("public string Label { get; set; } = \"On\";"));
        assert!(!output.contains("public string Label { get; set; } = default!;"));
    }

    #[test]
    fn warns_when_csharp_literal_default_initializer_cannot_be_preserved() {
        let source = r#"
            export type Settings = {
              enabled:boolean = { !false }
              title:string = { "hello" + "world" }
            }
        "#;
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types_with_warnings(&module, Path::new("types.nx"), &opts).unwrap();

        assert_eq!(output.warnings.len(), 2);
        assert!(output.warnings[0].contains("Settings.enabled"));
        assert!(output.warnings[0].contains("omitted default"));
        assert!(output.warnings[1].contains("Settings.title"));
        assert!(output.warnings[1].contains("omitted default"));
        assert!(output.value.contains("public bool Enabled { get; set; }"));
        assert!(!output.value.contains("public bool Enabled { get; set; } ="));
        assert!(output
            .value
            .contains("public string Title { get; set; } = default!;"));
    }

    #[test]
    fn generates_csharp_enums_with_shared_runtime_enum_serialization_helpers() {
        let source = r#"
            export type DealStage = draft | pending_review | closed_won
        "#;
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            typescript_package_prefix: None,
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
            export type BuildTarget = web_api | ios_app
        "#;
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            typescript_package_prefix: None,
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
    fn generates_csharp_discriminated_unions() {
        let source = r#"
            export type CardSortMode = closed | open
            export abstract type EventBase = { source:string }
            export type LoadState =
              | idle
              | failed { message:string retryable:boolean = true }
            export type UiEvent extends EventBase =
              | clicked { x:int }
              | closed
        "#;
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(output.contains(
            "[JsonConverter(typeof(NxEnumJsonConverter<CardSortMode, CardSortModeWireFormat>))]"
        ));
        // `LoadState` mixes a constant case with a payload case, so it has two wire shapes. It
        // carries the converter and `[NxUnionCase]` registrations rather than `[JsonPolymorphic]`,
        // which System.Text.Json will not accept alongside a converter on the same type.
        assert!(output.contains("[JsonConverter(typeof(NxPolymorphicJsonConverter<LoadState>))]"));
        assert!(output.contains("[NxUnionCase(typeof(LoadStateIdle), \"LoadState.idle\")]"));
        assert!(output.contains("[NxUnionCase(typeof(LoadStateFailed), \"LoadState.failed\")]"));
        assert!(output.contains(
            "[MessagePackFormatter(typeof(NxPolymorphicMessagePackFormatter<LoadState>))]"
        ));
        assert!(output.contains("public abstract class LoadState"));
        assert!(output.contains("public sealed class LoadStateIdle : LoadState"));
        // The constant case gets its marker and its singleton (design D4).
        assert!(output.contains("[NxConstantCase(\"idle\")]"));
        assert!(output.contains("public static readonly LoadStateIdle Instance = new();"));
        assert!(output.contains("public sealed class LoadStateFailed : LoadState"));
        assert!(output.contains("[Key(\"message\")]"));
        assert!(output.contains("public string Message { get; set; } = default!;"));
        // `UiEvent` extends a base, so its fieldless `closed` case is *not* constant — it carries
        // the base's fields. The union therefore has one wire shape and keeps `[JsonPolymorphic]`.
        assert!(output.contains("[JsonPolymorphic(TypeDiscriminatorPropertyName = \"$type\")]"));
        assert!(!output.contains("[NxConstantCase(\"closed\")]"));
        let normalized = output.replace("\r\n", "\n");
        assert!(normalized.contains(
            "[JsonDerivedType(typeof(UiEventClicked), \"UiEvent.clicked\")]\n    [JsonDerivedType(typeof(UiEventClosed), \"UiEvent.closed\")]\n    [MessagePackFormatter(typeof(NxPolymorphicMessagePackFormatter<EventBase>))]\n    public abstract class EventBase"
        ));
        assert!(output.contains("public abstract class UiEvent : EventBase"));
        assert!(output.contains("public sealed class UiEventClicked : UiEvent"));
        assert!(output.contains("public sealed class UiEventClosed : UiEvent"));
        assert!(output.contains("[Key(\"source\")]"));
        assert!(output.contains("public string Source { get; set; } = default!;"));
    }

    #[test]
    fn generates_typescript_library_files_for_nested_modules() {
        let temp_dir = TempDir::new().expect("temp dir");
        let library_dir = temp_dir.path().join("ui");
        fs::create_dir_all(library_dir.join("components")).expect("components dir");
        fs::write(
            library_dir.join("theme.nx"),
            "export type ThemeMode = light | dark",
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
            typescript_package_prefix: None,
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
            typescript_package_prefix: None,
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
            typescript_package_prefix: None,
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
            typescript_package_prefix: None,
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
            "export type PayloadExtra = { flag:boolean }",
        )
        .expect("_nx1 source file");

        let artifact = build_library_artifact_from_directory(&library_dir).expect("library build");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            typescript_package_prefix: None,
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
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            typescript_package_prefix: None,
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
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();
        let (record, update) = output
            .split_once("[MessagePackFormatter(typeof(NxUpdateRecordMessagePackFormatter")
            .expect("the record and its update companion");

        assert!(!record.contains("[JsonPropertyName(\"$type\")]"));
        assert!(!output.contains("__NxType"));
        assert!(record.contains("[JsonPropertyName(\"nx_type\")]"));
        assert!(record.contains("public string NxType { get; set; } = default!;"));
        // The companion's own discriminator steps around the field it would collide with.
        assert!(update.contains("public string NxType_ => \"Payload.Update\";"));
        assert!(update.contains("public NxOptional<string> NxType { get; set; }"));
    }

    #[test]
    fn generates_csharp_minimal_concrete_record_without_discriminator_member() {
        let source = r#"
            export type ShortTextQuestion = { label:string }
        "#;
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            typescript_package_prefix: None,
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

        let record = output
            .split("[MessagePackFormatter(typeof(NxUpdateRecordMessagePackFormatter")
            .next()
            .expect("the record");

        assert!(output.contains(&expected));
        assert!(!output.contains("__NxType"));
        assert!(!record.contains("[Key(\"$type\")]"));
        assert!(!record.contains("[JsonPropertyName(\"$type\")]"));
    }

    #[test]
    fn generates_csharp_concrete_record_and_action_fields_with_dual_annotations() {
        let source = r#"
            export type ShortTextQuestion = { label:string placeholder:string? }
            export action SearchRequested = { query:string }
        "#;
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            typescript_package_prefix: None,
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
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            typescript_package_prefix: None,
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
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types_with_warnings(&module, Path::new("types.nx"), &opts)
            .expect("generation output");

        assert!(output.value.contains("public abstract class Question"));
        assert!(!output.value.contains("[JsonPolymorphic("));
        assert!(!output.value.contains("[JsonDerivedType("));
        assert!(!output
            .value
            .contains("NxPolymorphicMessagePackFormatter<Question>"));
        assert!(!output
            .value
            .contains("NxPolymorphicConcreteMessagePackFormatter"));
        assert!(output.value.contains(
            "// No polymorphism metadata (JSON or MessagePack) was generated because this abstract type had"
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
        let module = source_module(
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
            typescript_package_prefix: None,
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
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(output.contains("[JsonPolymorphic(TypeDiscriminatorPropertyName = \"$type\")]"));
        assert!(
            output.contains("[JsonDerivedType(typeof(ShortTextQuestion), \"ShortTextQuestion\")]")
        );
        assert!(output.contains(
            "[MessagePackFormatter(typeof(NxPolymorphicMessagePackFormatter<Question>))]"
        ));
        assert!(output.contains(
            "[MessagePackFormatter(typeof(NxPolymorphicConcreteMessagePackFormatter<Question, ShortTextQuestion>))]"
        ));
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
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(output.contains("[JsonPolymorphic(TypeDiscriminatorPropertyName = \"$type\")]"));
        assert!(output.contains("[JsonDerivedType(typeof(SearchRequested), \"SearchRequested\")]"));
        assert!(output.contains(
            "[MessagePackFormatter(typeof(NxPolymorphicMessagePackFormatter<SearchAction>))]"
        ));
        assert!(output.contains(
            "[MessagePackFormatter(typeof(NxPolymorphicConcreteMessagePackFormatter<SearchAction, SearchRequested>))]"
        ));
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
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            typescript_package_prefix: None,
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
        let module = source_module(source, "types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_types(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(output.contains("[JsonPolymorphic(TypeDiscriminatorPropertyName = \"$type\")]"));
        assert!(
            output.contains("[JsonDerivedType(typeof(ShortTextQuestion), \"ShortTextQuestion\")]")
        );
        assert!(output.contains(
            "[MessagePackFormatter(typeof(NxPolymorphicMessagePackFormatter<Question>))]"
        ));
        assert!(output.contains(
            "[MessagePackFormatter(typeof(NxPolymorphicConcreteMessagePackFormatter<Question, TextQuestion>))]"
        ));
        assert!(output.contains(
            "[MessagePackFormatter(typeof(NxPolymorphicConcreteMessagePackFormatter<Question, ShortTextQuestion>))]"
        ));
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
            "export type ThemeMode = light | dark",
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
            typescript_package_prefix: None,
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
    fn generates_csharp_library_aliases_transparently_across_modules() {
        let temp_dir = TempDir::new().expect("temp dir");
        let library_dir = temp_dir.path().join("ui");
        fs::create_dir_all(&library_dir).expect("library dir");
        fs::write(
            library_dir.join("theme.nx"),
            "export type ThemeMode = light | dark",
        )
        .expect("theme file");
        fs::write(
            library_dir.join("aliases.nx"),
            "export type ThemeAlias = ThemeMode",
        )
        .expect("alias file");
        fs::write(
            library_dir.join("button.nx"),
            "export type Button = { theme: ThemeAlias }",
        )
        .expect("button file");

        let artifact = build_library_artifact_from_directory(&library_dir).expect("library build");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models".to_string()),
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let files = generate_library_types(&artifact, &opts).unwrap();
        let aliases = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("aliases.g.cs"))
            .expect("aliases.g.cs");
        let button = files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("button.g.cs"))
            .expect("button.g.cs");

        assert!(!aliases.content.contains("global using ThemeAlias"));
        assert!(button
            .content
            .contains("public ThemeMode Theme { get; set; }"));
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
            typescript_package_prefix: None,
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
    fn generates_csharp_dependency_primitive_aliases_transparently() {
        let temp_dir = TempDir::new().expect("temp dir");
        let question_flow_dir = temp_dir.path().join("question-flow");
        let chat_link_dir = temp_dir.path().join("chat-link");
        fs::create_dir_all(&question_flow_dir).expect("question-flow dir");
        fs::create_dir_all(&chat_link_dir).expect("chat-link dir");

        fs::write(
            question_flow_dir.join("QuestionFlowId.nx"),
            "export type QuestionFlowId = string",
        )
        .expect("question-flow file");
        fs::write(
            chat_link_dir.join("QuestionFlowInitialExperience.nx"),
            r#"import "../question-flow"

export type QuestionFlowInitialExperience = {
  questionFlowId: QuestionFlowId
}
"#,
        )
        .expect("chat-link file");

        let artifact =
            build_library_artifact_from_directory(&chat_link_dir).expect("library build");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models.ChatLink".to_string()),
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_library_types_with_warnings(&artifact, &opts).unwrap();
        let chat_link = output
            .value
            .iter()
            .find(|file| file.relative_path == PathBuf::from("QuestionFlowInitialExperience.g.cs"))
            .expect("QuestionFlowInitialExperience.g.cs");

        assert!(output.warnings.is_empty());
        assert!(!chat_link.content.contains("global using"));
        assert!(!chat_link.content.contains("public QuestionFlowId"));
        assert!(!chat_link.content.contains("Test.Models.QuestionFlow"));
        assert!(chat_link
            .content
            .contains("public string QuestionFlowId { get; set; } = default!;"));
    }

    #[test]
    fn generates_csharp_dependency_nominal_aliases_as_target_types() {
        let temp_dir = TempDir::new().expect("temp dir");
        let question_flow_dir = temp_dir.path().join("question-flow");
        let chat_link_dir = temp_dir.path().join("chat-link");
        fs::create_dir_all(&question_flow_dir).expect("question-flow dir");
        fs::create_dir_all(&chat_link_dir).expect("chat-link dir");

        fs::write(
            question_flow_dir.join("Question.nx"),
            r#"export type Question = { id:string }
export type PrimaryQuestion = Question
"#,
        )
        .expect("question-flow file");
        fs::write(
            chat_link_dir.join("QuestionFlowInitialExperience.nx"),
            r#"import "../question-flow"

export type QuestionFlowInitialExperience = {
  question: PrimaryQuestion
}
"#,
        )
        .expect("chat-link file");

        let artifact =
            build_library_artifact_from_directory(&chat_link_dir).expect("library build");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.Models.ChatLink".to_string()),
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_library_types_with_warnings(&artifact, &opts).unwrap();
        let chat_link = output
            .value
            .iter()
            .find(|file| file.relative_path == PathBuf::from("QuestionFlowInitialExperience.g.cs"))
            .expect("QuestionFlowInitialExperience.g.cs");

        assert_eq!(output.warnings.len(), 1);
        assert!(output.warnings[0].contains("question-flow"));
        assert!(!chat_link.content.contains("PrimaryQuestion"));
        assert!(chat_link
            .content
            .contains("using Test.Models.QuestionFlow;"));
        assert!(chat_link.content.contains(
            "public global::Test.Models.QuestionFlow.Question Question { get; set; } = default!;"
        ));
    }

    #[test]
    fn generates_typescript_library_files_with_dependency_package_imports() {
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
            chat_link_dir.join("QuestionFlowInitialExperience.nx"),
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
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            typescript_package_prefix: Some("@org/nx-".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let output = generate_library_types_with_warnings(&artifact, &opts).unwrap();
        let chat_link = output
            .value
            .iter()
            .find(|file| file.relative_path == PathBuf::from("QuestionFlowInitialExperience.ts"))
            .expect("QuestionFlowInitialExperience.ts");

        assert_eq!(output.warnings.len(), 1);
        assert!(output.warnings[0].contains("question-flow"));
        assert!(output.warnings[0].contains("@org/nx-question-flow"));
        assert!(chat_link
            .content
            .contains("import type { QuestionFlow } from \"@org/nx-question-flow\";"));
        assert!(chat_link.content.contains("questionFlow: QuestionFlow;"));
    }

    #[test]
    fn generates_typescript_library_files_with_dependency_alias_imports() {
        let temp_dir = TempDir::new().expect("temp dir");
        let question_flow_dir = temp_dir.path().join("question-flow");
        let chat_link_dir = temp_dir.path().join("chat-link");
        fs::create_dir_all(&question_flow_dir).expect("question-flow dir");
        fs::create_dir_all(&chat_link_dir).expect("chat-link dir");

        fs::write(
            question_flow_dir.join("QuestionFlowId.nx"),
            "export type QuestionFlowId = string",
        )
        .expect("question-flow file");
        fs::write(
            chat_link_dir.join("QuestionFlowInitialExperience.nx"),
            r#"import { QuestionFlowId as Flow.QuestionFlowId } from "../question-flow"

export type QuestionFlowInitialExperience = {
  questionFlowId: Flow.QuestionFlowId
}
"#,
        )
        .expect("chat-link file");

        let artifact =
            build_library_artifact_from_directory(&chat_link_dir).expect("library build");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            typescript_package_prefix: Some("@org/nx-".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let output = generate_library_types_with_warnings(&artifact, &opts).unwrap();
        let chat_link = output
            .value
            .iter()
            .find(|file| file.relative_path == PathBuf::from("QuestionFlowInitialExperience.ts"))
            .expect("QuestionFlowInitialExperience.ts");

        assert_eq!(output.warnings.len(), 1);
        assert!(output.warnings[0].contains("@org/nx-question-flow"));
        assert!(chat_link.content.contains(
            "import type { QuestionFlowId as Flow_QuestionFlowId } from \"@org/nx-question-flow\";"
        ));
        assert!(chat_link
            .content
            .contains("questionFlowId: Flow_QuestionFlowId;"));
    }

    #[test]
    fn generates_typescript_dependency_import_alias_for_qualified_selective_import() {
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
            chat_link_dir.join("QuestionFlowInitialExperience.nx"),
            r#"import { QuestionFlow as Flow.QuestionFlow } from "../question-flow"

export type QuestionFlowInitialExperience = {
  questionFlow: Flow.QuestionFlow
}
"#,
        )
        .expect("chat-link file");

        let artifact =
            build_library_artifact_from_directory(&chat_link_dir).expect("library build");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            typescript_package_prefix: Some("@org/nx-".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let output = generate_library_types_with_warnings(&artifact, &opts).unwrap();
        let chat_link = output
            .value
            .iter()
            .find(|file| file.relative_path == PathBuf::from("QuestionFlowInitialExperience.ts"))
            .expect("QuestionFlowInitialExperience.ts");

        assert!(chat_link.content.contains(
            "import type { QuestionFlow as Flow_QuestionFlow } from \"@org/nx-question-flow\";"
        ));
        assert!(chat_link
            .content
            .contains("questionFlow: Flow_QuestionFlow;"));
    }

    #[test]
    fn generates_typescript_dependency_import_warning_without_package_prefix() {
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
            chat_link_dir.join("QuestionFlowInitialExperience.nx"),
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
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let output = generate_library_types_with_warnings(&artifact, &opts).unwrap();
        let chat_link = output
            .value
            .iter()
            .find(|file| file.relative_path == PathBuf::from("QuestionFlowInitialExperience.ts"))
            .expect("QuestionFlowInitialExperience.ts");

        assert_eq!(output.warnings.len(), 1);
        assert!(output.warnings[0].contains("question-flow"));
        assert!(output.warnings[0].contains("package 'question-flow'"));
        assert!(chat_link
            .content
            .contains("import type { QuestionFlow } from \"question-flow\";"));
    }

    #[test]
    fn generates_no_typescript_dependency_warning_when_no_import_is_emitted() {
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
            chat_link_dir.join("ChatLink.nx"),
            r#"import "../question-flow"

export type ChatLink = {
  id: string
}
"#,
        )
        .expect("chat-link file");

        let artifact =
            build_library_artifact_from_directory(&chat_link_dir).expect("library build");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            typescript_package_prefix: Some("@org/nx-".to_string()),
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let output = generate_library_types_with_warnings(&artifact, &opts).unwrap();
        let chat_link = output
            .value
            .iter()
            .find(|file| file.relative_path == PathBuf::from("ChatLink.ts"))
            .expect("ChatLink.ts");

        assert!(
            output.warnings.is_empty(),
            "Expected no assumed package warning without an emitted dependency import, got {:?}",
            output.warnings
        );
        assert!(
            !chat_link.content.contains("@org/nx-question-flow"),
            "Generated source should not import an unused dependency package: {}",
            chat_link.content
        );
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
            typescript_package_prefix: None,
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

    #[test]
    fn generates_typescript_update_companions_with_optional_properties() {
        let output = generate_for(
            r#"
            export type User = { name:string email:string? }
            export component <Counter step:int /> = { state { count:int = 0 } <Label /> }
            "#,
            TargetLanguage::TypeScript,
        );

        let (_, user_update) = output
            .split_once("export interface User_update {")
            .expect("User_update companion");
        let user_update = user_update.split('}').next().expect("companion body");
        assert!(user_update.contains("$type: \"User.Update\";"));
        assert!(user_update.contains("name?: string;"));
        assert!(user_update.contains("email?: string | null;"));
        assert!(!user_update.contains("name: string;"));
        assert!(!user_update.contains("email: string | null;"));

        let (_, counter_update) = output
            .split_once("export interface Counter_update {")
            .expect("Counter_update companion");
        let counter_update = counter_update.split('}').next().expect("companion body");
        assert!(counter_update.contains("count?: number;"));
        assert!(!counter_update.contains("step"));
    }

    #[test]
    fn typescript_update_companion_yields_to_an_explicit_declaration() {
        let module = source_module(
            r#"
            export type User_update = string
            export type User = { name:string }
            "#,
            "types.nx",
        );
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };
        let generated =
            generate_types_with_warnings(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(generated
            .value
            .contains("export type User_update = string;"));
        assert!(!generated.value.contains("export interface User_update"));
        assert!(generated
            .warnings
            .iter()
            .any(|warning| warning.contains("User_update")));
    }

    #[test]
    fn generates_csharp_update_companions_with_optional_properties() {
        let output = generate_for(
            r#"
            export type User = { name:string email:string? }
            export action Saved = { note:string }
            export component <Counter step:int /> = { state { count:int = 0 } <Label /> }
            "#,
            TargetLanguage::CSharp,
        );

        let (_, counter) = output
            .split_once("public sealed class Counter_update")
            .expect("Counter_update companion");
        let counter = counter.split("public sealed class").next().expect("companion body");
        assert!(counter.contains("public string NxType => \"Counter.Update\";"), "{counter}");
        assert!(counter.contains("public NxOptional<long> Count { get; set; }"), "{counter}");
        assert!(!counter.contains("Step"), "{counter}");

        let (_, saved) = output
            .split_once("public sealed class Saved_update")
            .expect("Saved_update companion");
        let saved = saved.split("public sealed class").next().expect("companion body");
        assert!(saved.contains("public string NxType => \"Saved.Update\";"), "{saved}");
        assert!(saved.contains("public NxOptional<string> Note { get; set; }"), "{saved}");

        let (_, update) = output
            .split_once(
                "[MessagePackFormatter(typeof(NxUpdateRecordMessagePackFormatter<User_update>))]",
            )
            .expect("User_update companion");
        let nl = options::FormatOptions::defaults_for(TargetLanguage::CSharp).newline_str();
        let expected_name = [
            "        [Key(\"name\")]",
            "        [JsonPropertyName(\"name\")]",
            "        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingDefault)]",
            "        public NxOptional<string> Name { get; set; }",
        ]
        .join(nl);
        let expected_email = [
            "        [Key(\"email\")]",
            "        [JsonPropertyName(\"email\")]",
            "        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingDefault)]",
            "        public NxOptional<string?> Email { get; set; }",
        ]
        .join(nl);
        assert!(update.contains("public sealed class User_update"));
        assert!(update.contains("public string NxType => \"User.Update\";"));
        assert!(update.contains(&expected_name), "{update}");
        assert!(update.contains(&expected_email), "{update}");
        assert!(output.contains("using NxLang.Nx;"));
        assert!(output.contains("using NxLang.Nx.Serialization;"));
    }

    #[test]
    fn update_typed_fields_generate_as_the_companion_in_typescript() {
        let output = generate_for(
            r#"
            export type User = { name:string email:string? }
            export type Form = { pending:User.Update? drafts:User.Update[] }
            "#,
            TargetLanguage::TypeScript,
        );

        let (_, form) = output
            .split_once("export interface Form extends NxRecord<\"Form\"> {")
            .expect("Form record");
        let form = form.split('}').next().expect("record body");
        assert!(form.contains("pending: User_update | null;"), "{form}");
        assert!(form.contains("drafts: User_update[];"), "{form}");
        assert!(!output.contains("User_Update"), "{output}");
    }

    #[test]
    fn update_typed_fields_generate_as_the_companion_in_csharp() {
        let output = generate_for(
            r#"
            export type User = { name:string email:string? }
            export type Form = { pending:User.Update? drafts:User.Update[] }
            "#,
            TargetLanguage::CSharp,
        );

        let (_, form) = output
            .split_once("public sealed class Form")
            .expect("Form record");
        let form = form.split("public sealed class").next().expect("record body");
        assert!(form.contains("public User_update? Pending { get; set; }"), "{form}");
        assert!(form.contains("public User_update[] Drafts { get; set; }"), "{form}");
        assert!(!output.contains("User.Update?"), "{output}");
    }

    /// The companion has to carry the base's fields even when the base lives in another library,
    /// which only the compiler's own resolution knows.
    #[test]
    fn update_companion_carries_fields_inherited_from_an_imported_base() {
        let temp_dir = TempDir::new().expect("temp dir");
        let named_dir = temp_dir.path().join("named");
        fs::create_dir_all(&named_dir).expect("named dir");
        fs::write(
            named_dir.join("Named.nx"),
            "export abstract type Named = { name:string }",
        )
        .expect("named file");
        let source_path = temp_dir.path().join("types.nx");
        let source = r#"import "./named"

export type User extends Named = { email:string }
"#;
        fs::write(&source_path, source).expect("source file");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let generated = generate_types_with_warnings(source, &source_path, &opts).unwrap();

        let (_, user_update) = generated
            .value
            .split_once("export interface User_update {")
            .expect("User_update companion");
        let user_update = user_update.split('}').next().expect("companion body");
        assert!(user_update.contains("name?: string;"), "{user_update}");
        assert!(user_update.contains("email?: string;"), "{user_update}");
        assert!(
            !generated
                .warnings
                .iter()
                .any(|warning| warning.contains("User")),
            "{:?}",
            generated.warnings
        );
    }

    /// A copied field's type was written in the base's module; when this module cannot name it,
    /// the generated companion cannot either, and generation says so instead of miscompiling
    /// silently.
    #[test]
    fn update_companion_warns_when_an_inherited_field_type_is_not_imported() {
        let temp_dir = TempDir::new().expect("temp dir");
        let named_dir = temp_dir.path().join("named");
        fs::create_dir_all(&named_dir).expect("named dir");
        fs::write(
            named_dir.join("Named.nx"),
            "export type Tag = a | b\nexport abstract type Named = { name:string tag:Tag }",
        )
        .expect("named file");
        let source_path = temp_dir.path().join("types.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };

        let selective = r#"import { Named } from "./named"

export type User extends Named = { email:string }
"#;
        fs::write(&source_path, selective).expect("source file");
        let generated = generate_types_with_warnings(selective, &source_path, &opts).unwrap();
        assert!(
            generated.warnings.iter().any(|warning| {
                warning.contains("'User_update' inherits field 'tag' typed 'Tag'")
                    && warning.contains("does not import 'Tag'")
            }),
            "{:?}",
            generated.warnings
        );

        let wildcard = r#"import "./named"

export type User extends Named = { email:string }
"#;
        fs::write(&source_path, wildcard).expect("source file");
        let generated = generate_types_with_warnings(wildcard, &source_path, &opts).unwrap();
        assert!(
            !generated
                .warnings
                .iter()
                .any(|warning| warning.contains("User_update")),
            "{:?}",
            generated.warnings
        );
        assert!(generated.value.contains("tag?: Tag;"), "{}", generated.value);
    }

    /// The same holds for a library whose module extends a base from a library it imports.
    #[test]
    fn library_update_companion_carries_fields_inherited_from_a_dependency_base() {
        let temp_dir = TempDir::new().expect("temp dir");
        let named_dir = temp_dir.path().join("named");
        let people_dir = temp_dir.path().join("people");
        fs::create_dir_all(&named_dir).expect("named dir");
        fs::create_dir_all(&people_dir).expect("people dir");
        fs::write(
            named_dir.join("Named.nx"),
            "export abstract type Named = { name:string }",
        )
        .expect("named file");
        fs::write(
            people_dir.join("User.nx"),
            r#"import "../named"

export type User extends Named = { email:string }
"#,
        )
        .expect("people file");

        let artifact = build_library_artifact_from_directory(&people_dir).expect("library build");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("Test.People".to_string()),
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };

        let output = generate_library_types_with_warnings(&artifact, &opts).unwrap();
        let user = output
            .value
            .iter()
            .find(|file| file.relative_path == PathBuf::from("User.g.cs"))
            .expect("User.g.cs");
        let (_, user_update) = user
            .content
            .split_once("public sealed class User_update")
            .expect("User_update companion");
        let user_update = user_update.split("public sealed class").next().expect("body");
        assert!(user_update.contains("public NxOptional<string> Name { get; set; }"), "{user_update}");
        assert!(user_update.contains("public NxOptional<string> Email { get; set; }"), "{user_update}");
    }

    #[test]
    fn update_typed_field_without_a_companion_warns() {
        let module = source_module(
            r#"
            export type User_update = string
            export type User = { name:string }
            export type Form = { pending:User.Update }
            "#,
            "types.nx",
        );
        let opts = GenerateTypesOptions {
            language: TargetLanguage::TypeScript,
            csharp_namespace: None,
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::TypeScript),
        };
        let generated =
            generate_types_with_warnings(&module, Path::new("types.nx"), &opts).unwrap();

        assert!(generated.warnings.iter().any(|warning| {
            warning.contains("'User.Update' has no generated companion 'User_update'")
        }));
    }

    /// The .NET tests compile and exercise this generated file, so it has to be what typegen emits.
    #[test]
    fn checked_in_dotnet_update_record_fixture_matches_typegen_output() {
        let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../bindings/dotnet/tests/NxLang.Sdk.Tests/Generated");
        let source_path = fixture_dir.join("update-records.nx");
        let source = fs::read_to_string(&source_path).expect("fixture source");
        let module = source_module(&source, "update-records.nx");
        let opts = GenerateTypesOptions {
            language: TargetLanguage::CSharp,
            csharp_namespace: Some("NxLang.Sdk.Tests.Generated".to_string()),
            typescript_package_prefix: None,
            format: options::FormatOptions::defaults_for(TargetLanguage::CSharp),
        };
        let generated = generate_types(&module, &source_path, &opts).unwrap();
        let checked_in =
            fs::read_to_string(fixture_dir.join("UpdateRecords.g.cs")).expect("fixture output");

        assert_eq!(
            checked_in, generated,
            "Regenerate UpdateRecords.g.cs with the command in update-records.nx"
        );
    }
}
