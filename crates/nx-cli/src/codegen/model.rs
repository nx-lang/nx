use nx_api::LibraryArtifact;
use nx_hir::{
    ast::TypeRef, EnumDef, Item, LoweredModule, RecordDef, RecordKind, TypeAlias, Visibility,
};
use rustc_hash::FxHashMap;
use std::collections::{BTreeMap, BTreeSet};
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

    fn collect_referenced_type_names(&self, out: &mut BTreeSet<String>) {
        match self {
            Self::Alias(alias) => collect_type_ref_names(&alias.target, out),
            Self::Enum(_) => {}
            Self::Record(record) => {
                if let Some(base) = &record.base {
                    out.insert(base.clone());
                }

                for field in &record.fields {
                    collect_type_ref_names(&field.ty, out);
                }
            }
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
pub struct ModuleDependency {
    pub module_path: PathBuf,
    pub type_names: Vec<String>,
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

    pub fn module_dependencies(&self, module: &ExportedModule) -> Vec<ModuleDependency> {
        let mut dependencies = BTreeMap::<PathBuf, BTreeSet<String>>::new();

        for declaration in &module.declarations {
            let mut referenced_names = BTreeSet::new();
            declaration
                .item
                .collect_referenced_type_names(&mut referenced_names);

            for referenced_name in referenced_names {
                let Some(owner_path) = self.owner_module(&referenced_name) else {
                    continue;
                };

                if owner_path == module.module_path.as_path() {
                    continue;
                }

                dependencies
                    .entry(owner_path.to_path_buf())
                    .or_default()
                    .insert(referenced_name);
            }
        }

        dependencies
            .into_iter()
            .map(|(module_path, type_names)| ModuleDependency {
                module_path,
                type_names: type_names.into_iter().collect(),
            })
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

fn collect_type_ref_names(ty: &TypeRef, out: &mut BTreeSet<String>) {
    match ty {
        TypeRef::Name(name) => {
            out.insert(name.as_str().to_string());
        }
        TypeRef::Array(inner) | TypeRef::Nullable(inner) => collect_type_ref_names(inner, out),
        TypeRef::Function {
            params,
            return_type,
        } => {
            for param in params {
                collect_type_ref_names(param, out);
            }
            collect_type_ref_names(return_type, out);
        }
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
