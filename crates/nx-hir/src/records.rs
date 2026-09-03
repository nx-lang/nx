use crate::{
    ast, interface_record, interface_type_alias, same_declaration, DeclarationKey, DeclaringOrigin,
    EffectiveField, InterfaceField, InterfaceItem, InterfaceItemKind, Item, LocalDefinitionId,
    Name, PreparedModule, PreparedNamespace, RecordDef, RecordKind, ResolvedPreparedItem,
};
use nx_diagnostics::TextSpan;
use rustc_hash::{FxHashMap, FxHashSet};

/// One record in another record's inheritance chain.
///
/// The name is how the extending record spelled its base; the origin is the declaration that
/// spelling reached in *that* record's module. A chain of names alone cannot answer whether a
/// record extends a given base, because the same spelling in the asking module may be a different
/// declaration entirely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordAncestor {
    pub name: Name,
    pub origin: Option<DeclaringOrigin>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveRecordShape {
    pub record: RecordDef,
    pub fields: Vec<EffectiveField>,
    pub ancestors: Vec<RecordAncestor>,
    /// The declaration this shape was resolved from, where the resolving context reached one.
    pub origin: Option<DeclaringOrigin>,
}

impl EffectiveRecordShape {
    pub fn content_property(&self) -> Option<&EffectiveField> {
        self.fields.iter().find(|field| field.is_content)
    }

    /// Returns true when this record is, or descends from, the declaration at `origin`.
    pub fn descends_from(&self, name: &Name, origin: Option<&DeclaringOrigin>) -> bool {
        if same_declaration(self.origin.as_ref(), &self.record.name, origin, name) {
            return true;
        }

        self.ancestors.iter().any(|ancestor| {
            same_declaration(ancestor.origin.as_ref(), &ancestor.name, origin, name)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidBaseReason {
    NotFound,
    NotRecord,
    AliasCycle,
    ConcreteRecord,
    KindMismatch {
        expected: RecordKind,
        found: RecordKind,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordResolutionError {
    InvalidBase {
        record: Name,
        record_kind: RecordKind,
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
                InvalidBaseReason::ConcreteRecord => "record-base-not-abstract",
                InvalidBaseReason::KindMismatch { .. } => "record-base-kind-mismatch",
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
                record_kind,
                reason,
                ..
            } => {
                let (kind_label, kind_singular, kind_plural) = match record_kind {
                    RecordKind::Plain => ("Record", "record", "records"),
                    RecordKind::Action => ("Action", "action", "actions"),
                };

                match reason {
                    InvalidBaseReason::NotFound => format!(
                        "{} '{}' extends '{}', but '{}' could not be resolved",
                        kind_label, record, base, base
                    ),
                    InvalidBaseReason::NotRecord => format!(
                        "{} '{}' extends '{}', but '{}' does not resolve to an abstract {} declaration",
                        kind_label, record, base, base, kind_singular
                    ),
                    InvalidBaseReason::AliasCycle => format!(
                        "{} '{}' extends '{}', but resolving '{}' encountered a type alias cycle",
                        kind_label, record, base, base
                    ),
                    InvalidBaseReason::ConcreteRecord => format!(
                        "{} '{}' extends '{}', but only abstract {} may be extended",
                        kind_label, record, base, kind_plural
                    ),
                    InvalidBaseReason::KindMismatch { expected, found } => {
                        let (_, _, expected_plural) = match expected {
                            RecordKind::Plain => ("Record", "record", "records"),
                            RecordKind::Action => ("Action", "action", "actions"),
                        };
                        let (_, _, found_plural) = match found {
                            RecordKind::Plain => ("Record", "record", "records"),
                            RecordKind::Action => ("Action", "action", "actions"),
                        };
                        format!(
                            "{} '{}' extends '{}', but {} cannot be used as base {}",
                            kind_label, record, base, found_plural, expected_plural
                        )
                    }
                }
            }
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
    field: EffectiveField,
    owner: Name,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedRecordShape {
    record: RecordDef,
    fields: Vec<OwnedRecordField>,
    ancestors: Vec<RecordAncestor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RecordFieldSource {
    Raw,
    Interface { properties: Vec<InterfaceField> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedRecordDefinition {
    record: RecordDef,
    /// The module whose namespace this record's own type references — its base above all — were
    /// written in.
    module_identity: String,
    definition_id: Option<crate::LocalDefinitionId>,
    field_source: RecordFieldSource,
}

impl ResolvedRecordDefinition {
    /// The identity an inheritance walk tracks this record by.
    fn key(&self) -> DeclarationKey {
        DeclarationKey::new(self.origin(), &self.record.name)
    }

    fn origin(&self) -> Option<DeclaringOrigin> {
        self.definition_id
            .map(|definition_id| DeclaringOrigin::new(&self.module_identity, definition_id))
    }

    fn declared_fields(&self) -> Vec<EffectiveField> {
        match &self.field_source {
            RecordFieldSource::Raw => self
                .record
                .properties
                .iter()
                .cloned()
                .map(|field| EffectiveField::from_record_field(field, self.module_identity.clone()))
                .collect(),
            RecordFieldSource::Interface { properties } => properties
                .iter()
                .map(|field| {
                    EffectiveField::from_interface_field(field, self.module_identity.clone())
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecordValidationStatus {
    Valid,
    Invalid,
}

pub fn resolve_record_definition(module: &PreparedModule, name: &Name) -> Option<RecordDef> {
    resolve_record_definition_with_identity(module, module.module_identity(), name)
        .map(|resolved| resolved.record)
}

/// Resolves a record definition together with the identity of the module that declares it.
///
/// The declaring module is the context in which the record's own field type references were
/// written, so a caller that needs to resolve those references needs it alongside the definition.
pub fn resolve_record_definition_with_module(
    module: &PreparedModule,
    name: &Name,
) -> Option<(String, RecordDef)> {
    resolve_record_definition_with_identity(module, module.module_identity(), name)
        .map(|resolved| (resolved.module_identity, resolved.record))
}

/// Returns the declaration a record name reaches, where one exists.
pub fn record_declaration_origin(module: &PreparedModule, name: &Name) -> Option<DeclaringOrigin> {
    resolve_record_definition_with_identity(module, module.module_identity(), name)
        .and_then(|resolved| resolved.origin())
}

pub fn effective_record_shape_for_name(
    module: &PreparedModule,
    name: &Name,
) -> Result<Option<EffectiveRecordShape>, RecordResolutionError> {
    effective_record_shape_for_name_in(module, module.module_identity(), name)
}

/// Returns the effective shape of the record declared at `origin`.
///
/// The declaration is read straight out of the module that declares it, so the record does not
/// have to be nameable in the asking module at all.
pub fn effective_record_shape_at(
    module: &PreparedModule,
    origin: &DeclaringOrigin,
) -> Result<Option<EffectiveRecordShape>, RecordResolutionError> {
    let Some(record) = record_definition_at(module, origin) else {
        return Ok(None);
    };

    effective_record_shape_resolved(module, &record).map(Some)
}

/// Resolves a record name in the namespace of `namespace_module` and returns its effective shape.
///
/// A property typed `p: Point` in another module means *that* module's `Point`. Resolving the name
/// here instead is what lets an unrelated local `Point` stand in for it.
pub fn effective_record_shape_for_name_in(
    module: &PreparedModule,
    namespace_module: &str,
    name: &Name,
) -> Result<Option<EffectiveRecordShape>, RecordResolutionError> {
    let Some(record) = resolve_record_definition_with_identity(module, namespace_module, name)
    else {
        return Ok(None);
    };

    effective_record_shape_resolved(module, &record).map(Some)
}

pub fn effective_record_shape(
    module: &PreparedModule,
    record: &RecordDef,
) -> Result<EffectiveRecordShape, RecordResolutionError> {
    let definition_id = module
        .raw_module()
        .find_item_with_definition(record.name.as_str())
        .map(|(definition_id, _)| definition_id);
    let record = ResolvedRecordDefinition {
        record: record.clone(),
        module_identity: module.module_identity().to_string(),
        definition_id,
        field_source: RecordFieldSource::Raw,
    };
    effective_record_shape_resolved(module, &record)
}

fn effective_record_shape_resolved(
    module: &PreparedModule,
    record: &ResolvedRecordDefinition,
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
        origin: record.origin(),
    })
}

fn resolve_record_definition_with_identity(
    module: &PreparedModule,
    namespace_module: &str,
    name: &Name,
) -> Option<ResolvedRecordDefinition> {
    resolve_record_definition_inner(module, namespace_module, name, &mut FxHashSet::default())
}

fn record_definition_from_interface_item(item: &InterfaceItem) -> Option<ResolvedRecordDefinition> {
    let record = interface_record(item)?;
    let properties = match &item.item {
        InterfaceItemKind::Record { properties, .. } => properties.clone(),
        _ => return None,
    };
    Some(ResolvedRecordDefinition {
        record,
        module_identity: item.module_identity.clone(),
        definition_id: Some(item.definition_id),
        field_source: RecordFieldSource::Interface { properties },
    })
}

fn record_definition_from_prepared_item(
    module: &PreparedModule,
    resolved: ResolvedPreparedItem,
) -> Option<ResolvedRecordDefinition> {
    match resolved {
        ResolvedPreparedItem::Raw {
            module_identity,
            definition_id,
            item: Item::Record(record),
            ..
        } => Some(ResolvedRecordDefinition {
            record,
            module_identity,
            definition_id: Some(definition_id),
            field_source: RecordFieldSource::Raw,
        }),
        ResolvedPreparedItem::Imported { item, raw, .. } => {
            if let Some(raw_ref) = raw.as_ref() {
                if let Some(Item::Record(record)) = module.resolve_imported_raw_item(raw_ref) {
                    return Some(ResolvedRecordDefinition {
                        record,
                        module_identity: raw_ref.module_identity.clone(),
                        definition_id: Some(raw_ref.definition_id),
                        field_source: RecordFieldSource::Raw,
                    });
                }
            }
            record_definition_from_interface_item(&item)
        }
        _ => None,
    }
}

/// Decides whether a record satisfies an expected record type.
///
/// Both sides carry the declaration they were resolved to, and both are compared by it. Comparing
/// the spellings instead is what let a consumer's own `Point` satisfy a property typed by a
/// different module's `Point`, passing a field the declaring module typed `int` a `string`.
pub fn is_record_subtype(
    module: &PreparedModule,
    actual: &Name,
    actual_origin: Option<&DeclaringOrigin>,
    expected: &Name,
    expected_origin: Option<&DeclaringOrigin>,
) -> Result<bool, RecordResolutionError> {
    let Some(actual_record) = resolve_record_reference(module, actual, actual_origin) else {
        return Ok(false);
    };
    let Some(expected_record) = resolve_record_reference(module, expected, expected_origin) else {
        return Ok(false);
    };

    let expected_origin = expected_record.origin();
    if same_declaration(
        actual_record.origin().as_ref(),
        &actual_record.record.name,
        expected_origin.as_ref(),
        &expected_record.record.name,
    ) {
        return Ok(true);
    }

    let actual_shape = effective_record_shape_resolved(module, &actual_record)?;
    Ok(actual_shape.ancestors.iter().any(|ancestor| {
        same_declaration(
            ancestor.origin.as_ref(),
            &ancestor.name,
            expected_origin.as_ref(),
            &expected_record.record.name,
        )
    }))
}

/// Resolves a record reference by the declaration it names, or by its spelling if it names none.
///
/// A reference that carries an origin is read straight from the module that declares it, so it
/// does not have to be nameable here at all — and if that read fails, the reference resolves to
/// nothing rather than falling back to the local name, which would be the substitution this whole
/// change exists to stop. Only a reference with no origin at all — one built where no resolved
/// program was available — resolves by name.
fn resolve_record_reference(
    module: &PreparedModule,
    name: &Name,
    origin: Option<&DeclaringOrigin>,
) -> Option<ResolvedRecordDefinition> {
    match origin {
        Some(origin) => record_definition_at(module, origin),
        None => resolve_record_definition_with_identity(module, module.module_identity(), name),
    }
}

/// Reads the record declared at `origin` directly out of the module that declares it.
fn record_definition_at(
    module: &PreparedModule,
    origin: &DeclaringOrigin,
) -> Option<ResolvedRecordDefinition> {
    let item = if origin.module_identity() == module.module_identity() {
        module
            .raw_module()
            .item_by_definition(origin.definition_id())
    } else {
        module
            .peer_module(origin.module_identity())
            .and_then(|peer| peer.item_by_definition(origin.definition_id()))
    }?;

    match item {
        Item::Record(record) => Some(ResolvedRecordDefinition {
            record: record.clone(),
            module_identity: origin.module_identity().to_string(),
            definition_id: Some(origin.definition_id()),
            field_source: RecordFieldSource::Raw,
        }),
        _ => None,
    }
}

pub fn validate_record_definitions(module: &PreparedModule) -> Vec<RecordResolutionError> {
    let mut errors = Vec::new();
    let mut statuses = FxHashMap::default();
    let mut stack = Vec::new();

    for (index, item) in module.raw_module().items().iter().enumerate() {
        let Item::Record(record) = item else {
            continue;
        };
        let record = ResolvedRecordDefinition {
            record: record.clone(),
            module_identity: module.module_identity().to_string(),
            definition_id: Some(LocalDefinitionId::new(index as u32)),
            field_source: RecordFieldSource::Raw,
        };
        validate_record_definition(module, &record, &mut statuses, &mut stack, &mut errors);
    }

    errors
}

/// Walks one record's inheritance chain, reporting the first thing wrong with it.
///
/// <para>The walk is keyed by declaration rather than by spelling, because it crosses module
/// boundaries: `type Shape extends ui.Shape` is a record extending a different record that happens
/// to share its name, and a name-keyed stack reads the base as a repeat visit and reports a cycle
/// that is not there. Each base is resolved in the namespace of the module that wrote the `extends`
/// clause, for the same reason it is everywhere else.</para>
fn validate_record_definition(
    module: &PreparedModule,
    record: &ResolvedRecordDefinition,
    statuses: &mut FxHashMap<DeclarationKey, RecordValidationStatus>,
    stack: &mut Vec<(DeclarationKey, Name)>,
    errors: &mut Vec<RecordResolutionError>,
) -> RecordValidationStatus {
    let key = record.key();
    if let Some(status) = statuses.get(&key) {
        return *status;
    }

    if let Some(index) = stack.iter().position(|(seen, _)| *seen == key) {
        let mut cycle: Vec<Name> = stack[index..]
            .iter()
            .map(|(_, name)| name.clone())
            .collect();
        cycle.push(record.record.name.clone());
        push_unique_record_error(
            errors,
            RecordResolutionError::InheritanceCycle {
                record: record.record.name.clone(),
                span: record.record.span,
                cycle,
            },
        );

        for (seen, _) in &stack[index..] {
            statuses.insert(seen.clone(), RecordValidationStatus::Invalid);
        }

        return RecordValidationStatus::Invalid;
    }

    stack.push((key.clone(), record.record.name.clone()));

    let status = match resolve_base_record(module, &record.module_identity, &record.record) {
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
    statuses.insert(key, status);
    status
}

fn validate_record_shape(
    module: &PreparedModule,
    record: &ResolvedRecordDefinition,
    errors: &mut Vec<RecordResolutionError>,
) -> RecordValidationStatus {
    match effective_record_shape_resolved(module, record) {
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

fn resolve_record_definition_inner(
    module: &PreparedModule,
    namespace_module: &str,
    name: &Name,
    seen: &mut FxHashSet<Name>,
) -> Option<ResolvedRecordDefinition> {
    if !seen.insert(name.clone()) {
        return None;
    }

    let result = match module.resolve_in_module(PreparedNamespace::Type, namespace_module, name) {
        Some(resolved) => {
            if let Some(record) = record_definition_from_prepared_item(module, resolved.clone()) {
                Some(record)
            } else if let Some(target) = type_alias_target_from_prepared_item(&resolved) {
                resolve_record_definition_inner(module, namespace_module, &target, seen)
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
    record: &ResolvedRecordDefinition,
    stack: &mut Vec<(DeclarationKey, Name)>,
) -> Result<ResolvedRecordShape, RecordResolutionError> {
    let key = record.key();
    if let Some(index) = stack.iter().position(|(seen, _)| *seen == key) {
        let mut cycle: Vec<Name> = stack[index..]
            .iter()
            .map(|(_, name)| name.clone())
            .collect();
        cycle.push(record.record.name.clone());
        return Err(RecordResolutionError::InheritanceCycle {
            record: record.record.name.clone(),
            span: record.record.span,
            cycle,
        });
    }

    stack.push((key, record.record.name.clone()));

    let result = if let Some(base_record) =
        resolve_base_record(module, &record.module_identity, &record.record)?
    {
        let base_shape = resolve_record_shape_inner(module, &base_record, stack)?;
        let mut fields = base_shape.fields;
        let declared_fields = record.declared_fields();

        for field in &declared_fields {
            if field.is_content {
                if let Some(existing) = fields.iter().find(|existing| existing.field.is_content) {
                    stack.pop();
                    return Err(RecordResolutionError::DuplicateContentProperty {
                        record: record.record.name.clone(),
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
                    record: record.record.name.clone(),
                    field: field.name.clone(),
                    inherited_from: existing.owner.clone(),
                    span: field.span,
                });
            }

            fields.push(OwnedRecordField {
                field: field.clone(),
                owner: record.record.name.clone(),
            });
        }

        let mut ancestors = vec![RecordAncestor {
            name: base_record.record.name.clone(),
            origin: base_record.origin(),
        }];
        ancestors.extend(base_shape.ancestors);

        ResolvedRecordShape {
            record: record.record.clone(),
            fields,
            ancestors,
        }
    } else {
        let declared_fields = record.declared_fields();
        ResolvedRecordShape {
            record: record.record.clone(),
            fields: declared_fields
                .iter()
                .cloned()
                .map(|field| OwnedRecordField {
                    field,
                    owner: record.record.name.clone(),
                })
                .collect(),
            ancestors: Vec::new(),
        }
    };

    stack.pop();
    Ok(result)
}

/// Resolves a record's base in the namespace of the module that wrote the `extends` clause.
///
/// `type Circle = Shape { .. }` in a library means that library's `Shape`. Resolving `Shape` in the
/// asking module instead is the same substitution one level up the inheritance chain.
fn resolve_base_record(
    module: &PreparedModule,
    namespace_module: &str,
    record: &RecordDef,
) -> Result<Option<ResolvedRecordDefinition>, RecordResolutionError> {
    let Some(base_name) = record.base.as_ref() else {
        return Ok(None);
    };

    let mut seen = FxHashSet::default();
    resolve_base_record_inner(module, namespace_module, record, base_name, &mut seen).map(Some)
}

fn resolve_base_record_inner(
    module: &PreparedModule,
    namespace_module: &str,
    record: &RecordDef,
    base_name: &Name,
    seen: &mut FxHashSet<Name>,
) -> Result<ResolvedRecordDefinition, RecordResolutionError> {
    if !seen.insert(base_name.clone()) {
        return Err(RecordResolutionError::InvalidBase {
            record: record.name.clone(),
            record_kind: record.kind,
            base: record.base.clone().unwrap_or_else(|| base_name.clone()),
            span: record.span,
            reason: InvalidBaseReason::AliasCycle,
        });
    }

    let result =
        match module.resolve_in_module(PreparedNamespace::Type, namespace_module, base_name) {
            Some(resolved) => {
                if let Some(base_record) =
                    record_definition_from_prepared_item(module, resolved.clone())
                {
                    validate_base_record(record, base_name, &base_record)
                } else if let Some(target) = type_alias_target_from_prepared_item(&resolved) {
                    resolve_base_record_inner(module, namespace_module, record, &target, seen)
                } else {
                    Err(invalid_base(
                        record,
                        base_name,
                        InvalidBaseReason::NotRecord,
                    ))
                }
            }
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
        record_kind: record.kind,
        base: record.base.clone().unwrap_or_else(|| base_name.clone()),
        span: record.span,
        reason,
    }
}

fn validate_base_record(
    record: &RecordDef,
    base_name: &Name,
    base_record: &ResolvedRecordDefinition,
) -> Result<ResolvedRecordDefinition, RecordResolutionError> {
    if base_record.record.kind != record.kind {
        return Err(invalid_base(
            record,
            base_name,
            InvalidBaseReason::KindMismatch {
                expected: record.kind,
                found: base_record.record.kind,
            },
        ));
    }

    if !base_record.record.is_abstract {
        return Err(invalid_base(
            record,
            base_name,
            InvalidBaseReason::ConcreteRecord,
        ));
    }

    Ok(base_record.clone())
}
