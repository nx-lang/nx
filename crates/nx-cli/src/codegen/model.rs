use nx_hir::{ast::TypeRef, EnumDef, Item, Module, RecordDef};

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
    pub fields: Vec<ExportedRecordField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedTypes {
    pub enums: Vec<ExportedEnum>,
    pub records: Vec<ExportedRecord>,
}

pub fn collect_exported_types(module: &Module) -> ExportedTypes {
    let mut enums = Vec::new();
    let mut records = Vec::new();

    for item in module.items() {
        match item {
            Item::Enum(def) => enums.push(export_enum(def)),
            Item::Record(def) => records.push(export_record(def)),
            _ => {}
        }
    }

    ExportedTypes { enums, records }
}

fn export_enum(def: &EnumDef) -> ExportedEnum {
    ExportedEnum {
        name: def.name.as_str().to_string(),
        members: def
            .members
            .iter()
            .map(|m| m.name.as_str().to_string())
            .collect(),
    }
}

fn export_record(def: &RecordDef) -> ExportedRecord {
    ExportedRecord {
        name: def.name.as_str().to_string(),
        fields: def
            .properties
            .iter()
            .map(|f| ExportedRecordField {
                name: f.name.as_str().to_string(),
                ty: f.ty.clone(),
                has_default: f.default.is_some(),
            })
            .collect(),
    }
}
