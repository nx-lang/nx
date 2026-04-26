use crate::{
    ast, effective_record_shape_for_name, interface_record, interface_type_alias, interface_union,
    InterfaceItem, Item, Name, PreparedModule, PreparedNamespace, RecordDef, RecordKind,
    ResolvedPreparedItem, UnionDef,
};
use nx_diagnostics::TextSpan;
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidUnionBaseReason {
    NotFound,
    NotRecord,
    AliasCycle,
    ConcreteRecord,
    ActionRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnionValidationError {
    DuplicateCase {
        union: Name,
        case_name: Name,
        span: TextSpan,
    },
    InvalidBase {
        union: Name,
        base: Name,
        span: TextSpan,
        reason: InvalidUnionBaseReason,
    },
    DuplicateInheritedField {
        union: Name,
        case_name: Name,
        field: Name,
        inherited_from: Name,
        span: TextSpan,
    },
    DuplicateContentProperty {
        union: Name,
        case_name: Name,
        existing_field: Name,
        field: Name,
        span: TextSpan,
    },
}

impl UnionValidationError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::DuplicateCase { .. } => "union-duplicate-case",
            Self::InvalidBase { reason, .. } => match reason {
                InvalidUnionBaseReason::NotFound => "union-base-not-found",
                InvalidUnionBaseReason::NotRecord => "union-base-not-record",
                InvalidUnionBaseReason::AliasCycle => "union-base-alias-cycle",
                InvalidUnionBaseReason::ConcreteRecord => "union-base-not-abstract",
                InvalidUnionBaseReason::ActionRecord => "union-base-not-record",
            },
            Self::DuplicateInheritedField { .. } => "union-duplicate-inherited-field",
            Self::DuplicateContentProperty { .. } => "union-duplicate-content-property",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::DuplicateCase {
                union, case_name, ..
            } => format!("Union '{}' declares case '{}' more than once", union, case_name),
            Self::InvalidBase {
                union,
                base,
                reason,
                ..
            } => match reason {
                InvalidUnionBaseReason::NotFound => format!(
                    "Union '{}' extends '{}', but '{}' could not be resolved",
                    union, base, base
                ),
                InvalidUnionBaseReason::NotRecord => format!(
                    "Union '{}' extends '{}', but '{}' does not resolve to an abstract record declaration",
                    union, base, base
                ),
                InvalidUnionBaseReason::AliasCycle => format!(
                    "Union '{}' extends '{}', but resolving '{}' encountered a type alias cycle",
                    union, base, base
                ),
                InvalidUnionBaseReason::ConcreteRecord => format!(
                    "Union '{}' extends '{}', but only abstract records may be extended",
                    union, base
                ),
                InvalidUnionBaseReason::ActionRecord => format!(
                    "Union '{}' extends '{}', but action records cannot be used as union bases",
                    union, base
                ),
            },
            Self::DuplicateInheritedField {
                union,
                case_name,
                field,
                inherited_from,
                ..
            } => format!(
                "Union '{}.{}' redeclares inherited field '{}' from '{}'",
                union, case_name, field, inherited_from
            ),
            Self::DuplicateContentProperty {
                union,
                case_name,
                existing_field,
                field,
                ..
            } => format!(
                "Union '{}.{}' declares content property '{}' but '{}' is already the content property",
                union, case_name, field, existing_field
            ),
        }
    }

    pub fn span(&self) -> TextSpan {
        match self {
            Self::DuplicateCase { span, .. }
            | Self::InvalidBase { span, .. }
            | Self::DuplicateInheritedField { span, .. }
            | Self::DuplicateContentProperty { span, .. } => *span,
        }
    }
}

pub fn validate_union_definitions(module: &PreparedModule) -> Vec<UnionValidationError> {
    let mut errors = Vec::new();

    for union in module
        .raw_module()
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::Union(union) => Some(union),
            _ => None,
        })
    {
        validate_union_definition(module, union, &mut errors);
    }

    errors
}

pub fn resolve_union_definition(module: &PreparedModule, name: &Name) -> Option<UnionDef> {
    module
        .resolve_binding(PreparedNamespace::Type, name)
        .and_then(|binding| module.resolve_prepared_item(binding))
        .and_then(|resolved| union_definition_from_prepared_item(module, resolved))
}

fn validate_union_definition(
    module: &PreparedModule,
    union: &UnionDef,
    errors: &mut Vec<UnionValidationError>,
) {
    validate_duplicate_cases(union, errors);
    validate_case_content_properties(union, errors);

    let Some(base_name) = union.base.as_ref() else {
        return;
    };

    match resolve_union_base_record(module, base_name, &mut FxHashSet::default()) {
        Ok(base_record) => {
            if base_record.kind == RecordKind::Action {
                errors.push(invalid_base(
                    union,
                    base_name,
                    InvalidUnionBaseReason::ActionRecord,
                ));
                return;
            }
            if !base_record.is_abstract {
                errors.push(invalid_base(
                    union,
                    base_name,
                    InvalidUnionBaseReason::ConcreteRecord,
                ));
                return;
            }
            validate_case_inherited_field_collisions(module, union, base_name, errors);
        }
        Err(reason) => errors.push(invalid_base(union, base_name, reason)),
    }
}

fn validate_duplicate_cases(union: &UnionDef, errors: &mut Vec<UnionValidationError>) {
    let mut seen = FxHashMap::<Name, TextSpan>::default();
    for case in &union.cases {
        if seen.insert(case.name.clone(), case.span).is_some() {
            errors.push(UnionValidationError::DuplicateCase {
                union: union.name.clone(),
                case_name: case.name.clone(),
                span: case.span,
            });
        }
    }
}

fn validate_case_content_properties(union: &UnionDef, errors: &mut Vec<UnionValidationError>) {
    for case in &union.cases {
        let mut content_field: Option<Name> = None;
        for field in &case.fields {
            if !field.is_content {
                continue;
            }

            if let Some(existing_field) = content_field.as_ref() {
                errors.push(UnionValidationError::DuplicateContentProperty {
                    union: union.name.clone(),
                    case_name: case.name.clone(),
                    existing_field: existing_field.clone(),
                    field: field.name.clone(),
                    span: field.span,
                });
            } else {
                content_field = Some(field.name.clone());
            }
        }
    }
}

fn validate_case_inherited_field_collisions(
    module: &PreparedModule,
    union: &UnionDef,
    base_name: &Name,
    errors: &mut Vec<UnionValidationError>,
) {
    let Ok(Some(base_shape)) = effective_record_shape_for_name(module, base_name) else {
        return;
    };

    for case in &union.cases {
        for field in &case.fields {
            if base_shape.fields.iter().any(|base| base.name == field.name) {
                errors.push(UnionValidationError::DuplicateInheritedField {
                    union: union.name.clone(),
                    case_name: case.name.clone(),
                    field: field.name.clone(),
                    inherited_from: base_name.clone(),
                    span: field.span,
                });
                continue;
            }

            if field.is_content {
                if let Some(existing) = base_shape.content_property() {
                    errors.push(UnionValidationError::DuplicateContentProperty {
                        union: union.name.clone(),
                        case_name: case.name.clone(),
                        existing_field: existing.name.clone(),
                        field: field.name.clone(),
                        span: field.span,
                    });
                }
            }
        }
    }
}

fn resolve_union_base_record(
    module: &PreparedModule,
    base_name: &Name,
    seen: &mut FxHashSet<Name>,
) -> Result<RecordDef, InvalidUnionBaseReason> {
    if !seen.insert(base_name.clone()) {
        return Err(InvalidUnionBaseReason::AliasCycle);
    }

    let result = match module
        .resolve_binding(PreparedNamespace::Type, base_name)
        .and_then(|binding| module.resolve_prepared_item(binding))
    {
        Some(resolved) => {
            if let Some(record) = record_definition_from_prepared_item(module, resolved.clone()) {
                Ok(record)
            } else if let Some(target) = type_alias_target_from_prepared_item(&resolved) {
                resolve_union_base_record(module, &target, seen)
            } else {
                Err(InvalidUnionBaseReason::NotRecord)
            }
        }
        None => Err(InvalidUnionBaseReason::NotFound),
    };

    seen.remove(base_name);
    result
}

fn invalid_base(
    union: &UnionDef,
    base_name: &Name,
    reason: InvalidUnionBaseReason,
) -> UnionValidationError {
    UnionValidationError::InvalidBase {
        union: union.name.clone(),
        base: union.base.clone().unwrap_or_else(|| base_name.clone()),
        span: union.span,
        reason,
    }
}

fn union_definition_from_prepared_item(
    module: &PreparedModule,
    resolved: ResolvedPreparedItem,
) -> Option<UnionDef> {
    match resolved {
        ResolvedPreparedItem::Raw {
            item: Item::Union(union),
            ..
        } => Some(union),
        ResolvedPreparedItem::Imported { item, raw, .. } => {
            if let Some(raw_ref) = raw.as_ref() {
                if let Some(Item::Union(union)) = module.resolve_imported_raw_item(raw_ref) {
                    return Some(union);
                }
            }
            union_definition_from_interface_item(&item)
        }
        _ => None,
    }
}

fn union_definition_from_interface_item(item: &InterfaceItem) -> Option<UnionDef> {
    interface_union(item)
}

fn record_definition_from_prepared_item(
    module: &PreparedModule,
    resolved: ResolvedPreparedItem,
) -> Option<RecordDef> {
    match resolved {
        ResolvedPreparedItem::Raw {
            item: Item::Record(record),
            ..
        } => Some(record),
        ResolvedPreparedItem::Imported { item, raw, .. } => {
            if let Some(raw_ref) = raw.as_ref() {
                if let Some(Item::Record(record)) = module.resolve_imported_raw_item(raw_ref) {
                    return Some(record);
                }
            }
            interface_record(&item)
        }
        _ => None,
    }
}

fn type_alias_target_from_prepared_item(resolved: &ResolvedPreparedItem) -> Option<Name> {
    match resolved {
        ResolvedPreparedItem::Raw {
            item: Item::TypeAlias(alias),
            ..
        } => match &alias.ty {
            ast::TypeRef::Name(target) => Some(target.clone()),
            _ => None,
        },
        ResolvedPreparedItem::Imported { item, .. } => {
            interface_type_alias(item).and_then(|alias| match &alias.ty {
                ast::TypeRef::Name(target) => Some(target.clone()),
                _ => None,
            })
        }
        _ => None,
    }
}
