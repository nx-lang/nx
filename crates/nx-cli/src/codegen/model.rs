use nx_api::LibraryArtifact;
use nx_hir::{
    ast::TypeRef, EnumDef, Item, LoweredModule, RecordDef, RecordKind, TypeAlias, Visibility,
};
use rustc_hash::FxHashMap;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedAlias {
    pub name: String,
    pub target: TypeRef,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedEnum {
    pub name: String,
    pub members: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedRecordField {
    pub name: String,
    pub ty: TypeRef,
    pub has_default: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedRecord {
    pub name: String,
    pub kind: RecordKind,
    pub is_abstract: bool,
    pub base: Option<String>,
    pub fields: Vec<ExportedRecordField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExportedType {
    Alias(ExportedAlias),
    Enum(ExportedEnum),
    Record(ExportedRecord),
}

impl ExportedType {
    pub fn name(&self) -> &str {
        match self {
            Self::Alias(alias) => &alias.name,
            Self::Enum(enum_def) => &enum_def.name,
            Self::Record(record) => &record.name,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedTypeDecl {
    pub visibility: Visibility,
    pub item: ExportedType,
}

impl ExportedTypeDecl {
    pub fn name(&self) -> &str {
        self.item.name()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedModule {
    pub module_path: PathBuf,
    pub declarations: Vec<ExportedTypeDecl>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedTypeGraph {
    pub modules: Vec<ExportedModule>,
    owners: FxHashMap<String, PathBuf>,
}

impl ExportedTypeGraph {
    pub fn from_module(module: &LoweredModule, source_path: &Path) -> Result<Self, String> {
        let file_name = source_path
            .file_name()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("types.nx"));
        let module_path = module_output_stem(&file_name)?;
        Ok(Self::from_exported_modules(vec![ExportedModule {
            module_path,
            declarations: collect_exported_declarations(module),
        }]))
    }

    pub fn from_library(library: &LibraryArtifact) -> Result<Self, String> {
        let mut modules = Vec::new();

        for artifact in &library.modules {
            let Some(module) = artifact.lowered_module.as_ref() else {
                continue;
            };

            let declarations = collect_exported_declarations(module);
            if declarations.is_empty() {
                continue;
            }

            let module_file = Path::new(&artifact.file_name);
            let relative_path = module_file.strip_prefix(&library.root_path).map_err(|_| {
                format!(
                    "Module '{}' is not located under library root '{}'",
                    module_file.display(),
                    library.root_path.display()
                )
            })?;

            modules.push(ExportedModule {
                module_path: module_output_stem(relative_path)?,
                declarations,
            });
        }

        modules.sort_by(|left, right| left.module_path.cmp(&right.module_path));
        Ok(Self::from_exported_modules(modules))
    }

    pub fn owner_module(&self, type_name: &str) -> Option<&Path> {
        self.owners.get(type_name).map(PathBuf::as_path)
    }

    pub fn declaration(&self, type_name: &str) -> Option<&ExportedTypeDecl> {
        self.modules
            .iter()
            .flat_map(|module| module.declarations.iter())
            .find(|declaration| declaration.name() == type_name)
    }

    pub fn record(&self, type_name: &str) -> Option<&ExportedRecord> {
        match &self.declaration(type_name)?.item {
            ExportedType::Record(record) => Some(record),
            _ => None,
        }
    }

    pub fn resolve_record(&self, type_name: &str) -> Option<&ExportedRecord> {
        match &self
            .resolve_named_type_declaration(type_name, &mut BTreeSet::new())?
            .item
        {
            ExportedType::Record(record) => Some(record),
            _ => None,
        }
    }

    pub fn resolved_record_base<'a>(
        &'a self,
        record: &ExportedRecord,
    ) -> Option<&'a ExportedRecord> {
        let base_name = record.base.as_deref()?;
        self.resolve_record(base_name)
    }

    pub fn concrete_descendants<'a>(&'a self, type_name: &str) -> Vec<&'a ExportedRecord> {
        let Some(record) = self.record(type_name) else {
            return Vec::new();
        };
        if !record.is_abstract {
            return Vec::new();
        }

        let mut names = BTreeSet::new();
        self.collect_concrete_descendant_names(type_name, &mut names);
        names
            .into_iter()
            .filter_map(|name| self.record(&name))
            .collect()
    }

    fn from_exported_modules(modules: Vec<ExportedModule>) -> Self {
        let mut owners = FxHashMap::default();

        for module in &modules {
            for declaration in &module.declarations {
                owners.insert(declaration.name().to_string(), module.module_path.clone());
            }
        }

        Self { modules, owners }
    }

    fn resolve_named_type_declaration<'a>(
        &'a self,
        type_name: &str,
        seen_aliases: &mut BTreeSet<String>,
    ) -> Option<&'a ExportedTypeDecl> {
        let declaration = self.declaration(type_name)?;
        match &declaration.item {
            ExportedType::Alias(alias) => {
                let TypeRef::Name(target_name) = &alias.target else {
                    return Some(declaration);
                };

                if !seen_aliases.insert(type_name.to_string()) {
                    return None;
                }

                let resolved =
                    self.resolve_named_type_declaration(target_name.as_str(), seen_aliases);
                seen_aliases.remove(type_name);
                resolved
            }
            _ => Some(declaration),
        }
    }

    fn collect_concrete_descendant_names(&self, type_name: &str, out: &mut BTreeSet<String>) {
        for record in self.records() {
            let Some(base_record) = self.resolved_record_base(record) else {
                continue;
            };

            if base_record.name != type_name {
                continue;
            }

            if record.is_abstract {
                self.collect_concrete_descendant_names(&record.name, out);
            } else {
                out.insert(record.name.clone());
            }
        }
    }

    fn records(&self) -> impl Iterator<Item = &ExportedRecord> {
        self.modules
            .iter()
            .flat_map(|module| module.declarations.iter())
            .filter_map(|declaration| match &declaration.item {
                ExportedType::Record(record) => Some(record),
                _ => None,
            })
    }
}

fn collect_exported_declarations(module: &LoweredModule) -> Vec<ExportedTypeDecl> {
    let mut declarations = Vec::new();

    for item in module.items() {
        if item.visibility() != Visibility::Export {
            continue;
        }

        let declaration = match item {
            Item::TypeAlias(alias) => Some(ExportedTypeDecl {
                visibility: alias.visibility,
                item: ExportedType::Alias(export_alias(alias)),
            }),
            Item::Enum(enum_def) => Some(ExportedTypeDecl {
                visibility: enum_def.visibility,
                item: ExportedType::Enum(export_enum(enum_def)),
            }),
            Item::Record(record) => Some(ExportedTypeDecl {
                visibility: record.visibility,
                item: ExportedType::Record(export_record(record)),
            }),
            _ => None,
        };

        if let Some(declaration) = declaration {
            declarations.push(declaration);
        }
    }

    declarations
}

fn export_alias(def: &TypeAlias) -> ExportedAlias {
    ExportedAlias {
        name: def.name.as_str().to_string(),
        target: def.ty.clone(),
    }
}

fn export_enum(def: &EnumDef) -> ExportedEnum {
    ExportedEnum {
        name: def.name.as_str().to_string(),
        members: def
            .members
            .iter()
            .map(|member| member.name.as_str().to_string())
            .collect(),
    }
}

fn export_record(def: &RecordDef) -> ExportedRecord {
    ExportedRecord {
        name: def.name.as_str().to_string(),
        kind: def.kind,
        is_abstract: def.is_abstract,
        base: def.base.as_ref().map(|name| name.as_str().to_string()),
        fields: def
            .properties
            .iter()
            .map(|field| ExportedRecordField {
                name: field.name.as_str().to_string(),
                ty: field.ty.clone(),
                has_default: field.default.is_some(),
            })
            .collect(),
    }
}

fn module_output_stem(path: &Path) -> Result<PathBuf, String> {
    let stem = path.with_extension("");
    if stem.as_os_str().is_empty() {
        return Err(format!(
            "Could not derive a module output path from '{}'",
            path.display()
        ));
    }

    Ok(stem)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nx_api::build_library_artifact_from_directory;
    use nx_hir::{lower, LoweredModule, SourceId};
    use nx_syntax::parse_str;
    use std::fs;
    use tempfile::TempDir;

    fn lower_module(source: &str, file_name: &str) -> LoweredModule {
        let parse_result = parse_str(source, file_name);
        let tree = parse_result.tree.expect("expected parse tree");
        lower(tree.root(), SourceId::new(0))
    }

    #[test]
    fn resolves_abstract_record_alias_bases() {
        let source = r#"
            export abstract type Question = { label:string }
            export type QuestionBaseAlias = Question
            export type ShortTextQuestion extends QuestionBaseAlias = { placeholder:string? }
        "#;
        let module = lower_module(source, "types.nx");
        let graph = ExportedTypeGraph::from_module(&module, Path::new("types.nx")).unwrap();

        let short_text = graph
            .record("ShortTextQuestion")
            .expect("short text record");
        let base = graph
            .resolved_record_base(short_text)
            .expect("resolved abstract base");
        assert_eq!(base.name, "Question");
        assert!(base.is_abstract);
    }

    #[test]
    fn collects_transitive_concrete_descendants_across_modules() {
        let temp_dir = TempDir::new().expect("temp dir");
        let library_dir = temp_dir.path().join("ui");
        fs::create_dir_all(&library_dir).expect("library dir");
        fs::write(
            library_dir.join("base.nx"),
            "export abstract type Question = { label:string }",
        )
        .expect("base file");
        fs::write(
            library_dir.join("derived.nx"),
            "export abstract type TextQuestion extends Question = { placeholder:string? }",
        )
        .expect("derived file");
        fs::write(
            library_dir.join("short-text.nx"),
            "export type ShortTextQuestion extends TextQuestion = { maxLength:int? }",
        )
        .expect("short text file");

        let artifact = build_library_artifact_from_directory(&library_dir).expect("library build");
        let graph = ExportedTypeGraph::from_library(&artifact).unwrap();

        let descendants = graph
            .concrete_descendants("Question")
            .into_iter()
            .map(|record| record.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(descendants, vec!["ShortTextQuestion"]);
        assert_eq!(
            graph
                .owner_module("ShortTextQuestion")
                .expect("short text module"),
            Path::new("short-text")
        );
    }
}
