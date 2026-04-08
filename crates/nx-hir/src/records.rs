use crate::{
    ast, interface_record, interface_type_alias, InterfaceItemKind, Item, Name, PreparedModule,
    PreparedNamespace, RecordDef, RecordField, RecordKind, ResolvedPreparedItem,
};
use nx_diagnostics::TextSpan;
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveRecordShape {
    pub record: RecordDef,
    pub fields: Vec<RecordField>,
    pub ancestors: Vec<Name>,
}

impl EffectiveRecordShape {
    pub fn content_property(&self) -> Option<&RecordField> {
        self.fields.iter().find(|field| field.is_content)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidBaseReason {
    NotFound,
    NotRecord,
    AliasCycle,
    ActionRecord,
    ConcreteRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordResolutionError {
    InvalidBase {
        record: Name,
        base: Name,
        span: TextSpan,
        reason: InvalidBaseReason,
    },
    InheritanceCycle {
        record: Name,
        span: TextSpan,
        cycle: Vec<Name>,
    },
    DuplicateInheritedField {
        record: Name,
        field: Name,
        inherited_from: Name,
        span: TextSpan,
    },
    DuplicateContentProperty {
        record: Name,
        existing_field: Name,
        existing_owner: Name,
        field: Name,
        span: TextSpan,
    },
}

impl RecordResolutionError {
    pub fn code(&self) -> &'static str {
        match self {
            RecordResolutionError::InvalidBase { reason, .. } => match reason {
                InvalidBaseReason::NotFound => "record-base-not-found",
                InvalidBaseReason::NotRecord => "record-base-not-record",
                InvalidBaseReason::AliasCycle => "record-base-alias-cycle",
                InvalidBaseReason::ActionRecord => "record-base-action",
                InvalidBaseReason::ConcreteRecord => "record-base-not-abstract",
            },
            RecordResolutionError::InheritanceCycle { .. } => "record-inheritance-cycle",
            RecordResolutionError::DuplicateInheritedField { .. } => {
                "record-duplicate-inherited-field"
            }
            RecordResolutionError::DuplicateContentProperty { .. } => {
                "record-duplicate-content-property"
            }
        }
    }

    pub fn message(&self) -> String {
        match self {
            RecordResolutionError::InvalidBase {
                record,
                base,
                reason,
                ..
            } => match reason {
                InvalidBaseReason::NotFound => format!(
                    "Record '{}' extends '{}', but '{}' could not be resolved",
                    record, base, base
                ),
                InvalidBaseReason::NotRecord => format!(
                    "Record '{}' extends '{}', but '{}' does not resolve to an abstract record declaration",
                    record, base, base
                ),
                InvalidBaseReason::AliasCycle => format!(
                    "Record '{}' extends '{}', but resolving '{}' encountered a type alias cycle",
                    record, base, base
                ),
                InvalidBaseReason::ActionRecord => format!(
                    "Record '{}' extends '{}', but actions cannot be used as base records",
                    record, base
                ),
                InvalidBaseReason::ConcreteRecord => format!(
                    "Record '{}' extends '{}', but only abstract records may be extended",
                    record, base
                ),
            },
            RecordResolutionError::InheritanceCycle { cycle, .. } => {
                let chain = cycle
                    .iter()
                    .map(|name| name.as_str())
                    .collect::<Vec<_>>()
                    .join(" -> ");
                format!("Record inheritance cycle detected: {}", chain)
            }
            RecordResolutionError::DuplicateInheritedField {
                record,
                field,
                inherited_from,
                ..
            } => format!(
                "Record '{}' redeclares inherited field '{}' from '{}'",
                record, field, inherited_from
            ),
            RecordResolutionError::DuplicateContentProperty {
                record,
                existing_field,
                existing_owner,
                field,
                ..
            } => {
                if existing_owner == record {
                    format!(
                        "Record '{}' declares more than one content property: '{}' and '{}'",
                        record, existing_field, field
                    )
                } else {
                    format!(
                        "Record '{}' declares content property '{}' but already inherits content property '{}' from '{}'",
                        record, field, existing_field, existing_owner
                    )
                }
            }
        }
    }

    pub fn span(&self) -> TextSpan {
        match self {
            RecordResolutionError::InvalidBase { span, .. }
            | RecordResolutionError::InheritanceCycle { span, .. }
            | RecordResolutionError::DuplicateInheritedField { span, .. }
            | RecordResolutionError::DuplicateContentProperty { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OwnedRecordField {
    field: RecordField,
    owner: Name,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedRecordShape {
    record: RecordDef,
    fields: Vec<OwnedRecordField>,
    ancestors: Vec<Name>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecordValidationStatus {
    Valid,
    Invalid,
}

pub fn resolve_record_definition(module: &PreparedModule, name: &Name) -> Option<RecordDef> {
    resolve_record_definition_inner(module, name, &mut FxHashSet::default())
}

pub fn effective_record_shape_for_name(
    module: &PreparedModule,
    name: &Name,
) -> Result<Option<EffectiveRecordShape>, RecordResolutionError> {
    let Some(record) = resolve_record_definition(module, name) else {
        return Ok(None);
    };

    effective_record_shape(module, &record).map(Some)
}

pub fn effective_record_shape(
    module: &PreparedModule,
    record: &RecordDef,
) -> Result<EffectiveRecordShape, RecordResolutionError> {
    let resolved = resolve_record_shape_inner(module, record, &mut Vec::new())?;
    Ok(EffectiveRecordShape {
        record: resolved.record,
        fields: resolved
            .fields
            .into_iter()
            .map(|field| field.field)
            .collect(),
        ancestors: resolved.ancestors,
    })
}

pub fn is_record_subtype(
    module: &PreparedModule,
    actual: &Name,
    expected: &Name,
) -> Result<bool, RecordResolutionError> {
    let Some(actual_record) = resolve_record_definition(module, actual) else {
        return Ok(false);
    };
    let Some(expected_record) = resolve_record_definition(module, expected) else {
        return Ok(false);
    };

    if actual_record.name == expected_record.name {
        return Ok(true);
    }

    let actual_shape = effective_record_shape(module, &actual_record)?;
    Ok(actual_shape
        .ancestors
        .iter()
        .any(|ancestor| ancestor == &expected_record.name))
}

pub fn validate_record_definitions(module: &PreparedModule) -> Vec<RecordResolutionError> {
    let mut errors = Vec::new();
    let mut statuses = FxHashMap::default();
    let mut stack = Vec::new();

    for record in module
        .raw_module()
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::Record(record) if record.kind == RecordKind::Plain => Some(record),
            _ => None,
        })
    {
        validate_record_definition(module, record, &mut statuses, &mut stack, &mut errors);
    }

    errors
}

fn validate_record_definition(
    module: &PreparedModule,
    record: &RecordDef,
    statuses: &mut FxHashMap<Name, RecordValidationStatus>,
    stack: &mut Vec<Name>,
    errors: &mut Vec<RecordResolutionError>,
) -> RecordValidationStatus {
    if let Some(status) = statuses.get(&record.name) {
        return *status;
    }

    if let Some(index) = stack.iter().position(|name| name == &record.name) {
        let mut cycle = stack[index..].to_vec();
        cycle.push(record.name.clone());
        push_unique_record_error(
            errors,
            RecordResolutionError::InheritanceCycle {
                record: record.name.clone(),
                span: record.span,
                cycle: cycle.clone(),
            },
        );

        for name in cycle {
            statuses.insert(name, RecordValidationStatus::Invalid);
        }

        return RecordValidationStatus::Invalid;
    }

    stack.push(record.name.clone());

    let status = match resolve_base_record(module, record) {
        Ok(Some(base_record)) => {
            if validate_record_definition(module, &base_record, statuses, stack, errors)
                == RecordValidationStatus::Invalid
            {
                RecordValidationStatus::Invalid
            } else {
                validate_record_shape(module, record, errors)
            }
        }
        Ok(None) => validate_record_shape(module, record, errors),
        Err(error) => {
            push_unique_record_error(errors, error);
            RecordValidationStatus::Invalid
        }
    };

    stack.pop();
    statuses.insert(record.name.clone(), status);
    status
}

fn validate_record_shape(
    module: &PreparedModule,
    record: &RecordDef,
    errors: &mut Vec<RecordResolutionError>,
) -> RecordValidationStatus {
    match effective_record_shape(module, record) {
        Ok(_) => RecordValidationStatus::Valid,
        Err(error) => {
            push_unique_record_error(errors, error);
            RecordValidationStatus::Invalid
        }
    }
}

fn push_unique_record_error(errors: &mut Vec<RecordResolutionError>, error: RecordResolutionError) {
    if !errors.contains(&error) {
        errors.push(error);
    }
}

fn resolve_record_definition_inner(
    module: &PreparedModule,
    name: &Name,
    seen: &mut FxHashSet<Name>,
) -> Option<RecordDef> {
    if !seen.insert(name.clone()) {
        return None;
    }

    let result = match module
        .resolve_binding(PreparedNamespace::Type, name)
        .and_then(|binding| module.resolve_prepared_item(binding))
    {
        Some(ResolvedPreparedItem::Raw {
            item: Item::Record(record),
            ..
        }) => Some(record),
        Some(ResolvedPreparedItem::Raw {
            item: Item::TypeAlias(alias),
            ..
        }) => match &alias.ty {
            ast::TypeRef::Name(target) => resolve_record_definition_inner(module, target, seen),
            _ => None,
        },
        Some(ResolvedPreparedItem::Imported { item, .. }) => {
            if let Some(record) = interface_record(&item) {
                Some(record)
            } else if let Some(alias) = interface_type_alias(&item) {
                match &alias.ty {
                    ast::TypeRef::Name(target) => {
                        resolve_record_definition_inner(module, target, seen)
                    }
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    };

    seen.remove(name);
    result
}

fn resolve_record_shape_inner(
    module: &PreparedModule,
    record: &RecordDef,
    stack: &mut Vec<Name>,
) -> Result<ResolvedRecordShape, RecordResolutionError> {
    if let Some(index) = stack.iter().position(|name| name == &record.name) {
        let mut cycle = stack[index..].to_vec();
        cycle.push(record.name.clone());
        return Err(RecordResolutionError::InheritanceCycle {
            record: record.name.clone(),
            span: record.span,
            cycle,
        });
    }

    stack.push(record.name.clone());

    let result = if let Some(base_record) = resolve_base_record(module, record)? {
        let base_shape = resolve_record_shape_inner(module, &base_record, stack)?;
        let mut fields = base_shape.fields;

        for field in &record.properties {
            if field.is_content {
                if let Some(existing) = fields.iter().find(|existing| existing.field.is_content) {
                    stack.pop();
                    return Err(RecordResolutionError::DuplicateContentProperty {
                        record: record.name.clone(),
                        existing_field: existing.field.name.clone(),
                        existing_owner: existing.owner.clone(),
                        field: field.name.clone(),
                        span: field.span,
                    });
                }
            }

            if let Some(existing) = fields
                .iter()
                .find(|existing| existing.field.name == field.name)
            {
                stack.pop();
                return Err(RecordResolutionError::DuplicateInheritedField {
                    record: record.name.clone(),
                    field: field.name.clone(),
                    inherited_from: existing.owner.clone(),
                    span: field.span,
                });
            }

            fields.push(OwnedRecordField {
                field: field.clone(),
                owner: record.name.clone(),
            });
        }

        let mut ancestors = vec![base_record.name.clone()];
        ancestors.extend(base_shape.ancestors);

        ResolvedRecordShape {
            record: record.clone(),
            fields,
            ancestors,
        }
    } else {
        ResolvedRecordShape {
            record: record.clone(),
            fields: record
                .properties
                .iter()
                .cloned()
                .map(|field| OwnedRecordField {
                    field,
                    owner: record.name.clone(),
                })
                .collect(),
            ancestors: Vec::new(),
        }
    };

    stack.pop();
    Ok(result)
}

fn resolve_base_record(
    module: &PreparedModule,
    record: &RecordDef,
) -> Result<Option<RecordDef>, RecordResolutionError> {
    let Some(base_name) = record.base.as_ref() else {
        return Ok(None);
    };

    let mut seen = FxHashSet::default();
    resolve_base_record_inner(module, record, base_name, &mut seen).map(Some)
}

fn resolve_base_record_inner(
    module: &PreparedModule,
    record: &RecordDef,
    base_name: &Name,
    seen: &mut FxHashSet<Name>,
) -> Result<RecordDef, RecordResolutionError> {
    if !seen.insert(base_name.clone()) {
        return Err(RecordResolutionError::InvalidBase {
            record: record.name.clone(),
            base: record.base.clone().unwrap_or_else(|| base_name.clone()),
            span: record.span,
            reason: InvalidBaseReason::AliasCycle,
        });
    }

    let result = match module
        .resolve_binding(PreparedNamespace::Type, base_name)
        .and_then(|binding| module.resolve_prepared_item(binding))
    {
        Some(ResolvedPreparedItem::Raw {
            item: Item::Record(base_record),
            ..
        }) => validate_base_record(record, base_name, &base_record),
        Some(ResolvedPreparedItem::Raw {
            item: Item::TypeAlias(alias),
            ..
        }) => match &alias.ty {
            ast::TypeRef::Name(target) => resolve_base_record_inner(module, record, target, seen),
            _ => Err(invalid_base(
                record,
                base_name,
                InvalidBaseReason::NotRecord,
            )),
        },
        Some(ResolvedPreparedItem::Imported { item, .. }) => match &item.item {
            InterfaceItemKind::Record { .. } => {
                let base_record = interface_record(&item)
                    .expect("interface record should convert into record definition");
                validate_base_record(record, base_name, &base_record)
            }
            InterfaceItemKind::TypeAlias { ty, .. } => match ty {
                ast::TypeRef::Name(target) => {
                    resolve_base_record_inner(module, record, target, seen)
                }
                _ => Err(invalid_base(
                    record,
                    base_name,
                    InvalidBaseReason::NotRecord,
                )),
            },
            _ => Err(invalid_base(
                record,
                base_name,
                InvalidBaseReason::NotRecord,
            )),
        },
        Some(ResolvedPreparedItem::Raw { .. }) => Err(invalid_base(
            record,
            base_name,
            InvalidBaseReason::NotRecord,
        )),
        None => Err(invalid_base(record, base_name, InvalidBaseReason::NotFound)),
    };

    seen.remove(base_name);
    result
}

fn invalid_base(
    record: &RecordDef,
    base_name: &Name,
    reason: InvalidBaseReason,
) -> RecordResolutionError {
    RecordResolutionError::InvalidBase {
        record: record.name.clone(),
        base: record.base.clone().unwrap_or_else(|| base_name.clone()),
        span: record.span,
        reason,
    }
}

fn validate_base_record(
    record: &RecordDef,
    base_name: &Name,
    base_record: &RecordDef,
) -> Result<RecordDef, RecordResolutionError> {
    if base_record.kind == RecordKind::Action {
        Err(invalid_base(
            record,
            base_name,
            InvalidBaseReason::ActionRecord,
        ))
    } else if !base_record.is_abstract {
        Err(invalid_base(
            record,
            base_name,
            InvalidBaseReason::ConcreteRecord,
        ))
    } else {
        Ok(base_record.clone())
    }
}
