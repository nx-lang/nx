//! `viewer` declarations: what a viewer declaration's target is, and which `@nx/viewer` record its
//! value must be.

use crate::{
    ast, interface_component, interface_record, interface_type_alias, interface_union,
    props_record_name, DeclaringOrigin, InterfaceItem, InterfaceItemKind, Item, Name,
    PreparedModule, PreparedNamespace, RecordKind, ResolvedPreparedItem,
};

/// The root of the viewer standard library, which is also its import path.
pub const VIEWER_LIBRARY_ROOT: &str = "@nx/viewer";

/// The identity of the viewer standard library's one module, which declares every viewer record.
pub const VIEWER_LIBRARY_MODULE: &str = "@nx/viewer/viewer.nx";

/// The name of the type parameter every viewer record takes, which a viewer declaration binds.
pub const VIEWER_TARGET_PARAMETER: &str = "Target";

/// The `@nx/viewer` record a viewer declaration's value is, chosen by its target's kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewerRecord {
    /// `ElementViewer`, for a record, an action or a component.
    Element,
    /// `UnionViewer`, for a union whose cases are all constant cases.
    Union,
    /// `ValueViewer`, for an alias of a primitive type.
    Value,
}

impl ViewerRecord {
    /// The record's name in `@nx/viewer`.
    pub fn name(self) -> &'static str {
        match self {
            ViewerRecord::Element => "ElementViewer",
            ViewerRecord::Union => "UnionViewer",
            ViewerRecord::Value => "ValueViewer",
        }
    }
}

/// What a viewer declaration's target resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewerTarget {
    /// The declaration the target names.
    pub origin: DeclaringOrigin,
    /// The name the declaring module gave the declaration.
    pub declared_name: Name,
    /// What the declaration is, in the words a diagnostic uses: "record", "union", ...
    pub kind_label: &'static str,
    /// The viewer record the value must be.
    pub record: ViewerRecord,
    /// The type `Target` binds to, spelled as the viewer declaration's module sees it: the target
    /// itself, or its derived props record when it is a component.
    pub target_type: Name,
    /// Whether the target is a component, whose `Target` is its props record.
    pub is_component: bool,
}

/// Why a viewer declaration's target cannot have a viewer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewerTargetError {
    /// The target names nothing visible in the module.
    Unknown,
    /// The target names a declaration that cannot have a viewer, for the reason given.
    NotAllowed { reason: String },
}

/// Resolves a viewer declaration's target, written `target`, in `module`'s namespace and decides
/// which viewer record it takes.
///
/// <para>A record, an action (an inline emitted action included) or a component takes an
/// `ElementViewer`; a union whose cases are all constant cases a `UnionViewer`; an alias of a
/// primitive type a `ValueViewer`. A generic declaration, a derived one, a function, a value, a
/// union with a payload case and an alias of any other type cannot have a viewer in this
/// release.</para>
pub fn resolve_viewer_target(
    module: &PreparedModule,
    target: &Name,
) -> Result<ViewerTarget, ViewerTargetError> {
    for namespace in [
        PreparedNamespace::Type,
        PreparedNamespace::Element,
        PreparedNamespace::Value,
    ] {
        let Some(resolved) = module
            .resolve_binding(namespace, target)
            .and_then(|binding| module.resolve_prepared_item(binding))
        else {
            continue;
        };
        let origin = resolved.declaring_origin();
        let Some(item) = resolved_item(module, &resolved) else {
            continue;
        };
        return classify(target, origin, &item);
    }
    Err(ViewerTargetError::Unknown)
}

/// The item a resolved binding reaches, read from its declaring module where it can be and from
/// the library interface otherwise.
fn resolved_item(module: &PreparedModule, resolved: &ResolvedPreparedItem) -> Option<Item> {
    match resolved {
        ResolvedPreparedItem::Raw { item, .. } => Some(item.clone()),
        ResolvedPreparedItem::Imported { item, raw, .. } => raw
            .as_ref()
            .and_then(|raw| module.resolve_imported_raw_item(raw))
            .or_else(|| interface_item(item)),
    }
}

fn interface_item(item: &InterfaceItem) -> Option<Item> {
    match &item.item {
        InterfaceItemKind::Record { .. } => interface_record(item).map(Item::Record),
        InterfaceItemKind::Union { .. } => interface_union(item).map(Item::Union),
        InterfaceItemKind::TypeAlias { .. } => interface_type_alias(item).map(Item::TypeAlias),
        InterfaceItemKind::Component { .. } => interface_component(item).map(Item::Component),
        _ => None,
    }
}

fn not_allowed(reason: impl Into<String>) -> Result<ViewerTarget, ViewerTargetError> {
    Err(ViewerTargetError::NotAllowed {
        reason: reason.into(),
    })
}

fn classify(
    target: &Name,
    origin: DeclaringOrigin,
    item: &Item,
) -> Result<ViewerTarget, ViewerTargetError> {
    let element = |kind_label: &'static str, declared_name: &Name| ViewerTarget {
        origin: origin.clone(),
        declared_name: declared_name.clone(),
        kind_label,
        record: ViewerRecord::Element,
        target_type: target.clone(),
        is_component: false,
    };
    match item {
        Item::Record(record) => {
            let derived = match &record.kind {
                RecordKind::Update { .. } => Some("a derived update record"),
                RecordKind::State { .. } => Some("a derived state record"),
                RecordKind::Props { .. } => Some("a derived props record"),
                RecordKind::Plain | RecordKind::Action => None,
            };
            if let Some(derived) = derived {
                return not_allowed(format!("it is {derived}"));
            }
            if !record.type_params.is_empty() {
                return not_allowed("it is a generic record, and generic declarations cannot have viewers yet");
            }
            let kind_label = if record.is_action() { "action" } else { "record" };
            Ok(element(kind_label, &record.name))
        }
        Item::Component(component) => {
            if !component.type_params.is_empty() {
                return not_allowed("it is a generic component, and generic declarations cannot have viewers yet");
            }
            Ok(ViewerTarget {
                target_type: props_record_name(target.as_str()),
                is_component: true,
                ..element("component", &component.name)
            })
        }
        Item::Union(union_def) => {
            if union_def.property_target().is_some() {
                return not_allowed("it is a derived property union");
            }
            if !union_def.is_constant_union() {
                return not_allowed(
                    "it is a union with a payload case, and only a union of constant cases can have a viewer",
                );
            }
            Ok(ViewerTarget {
                origin,
                declared_name: union_def.name.clone(),
                kind_label: "union",
                record: ViewerRecord::Union,
                target_type: target.clone(),
                is_component: false,
            })
        }
        Item::TypeAlias(alias) => match &alias.ty {
            ast::TypeRef::Name(name) if is_primitive_type_name(name.as_str()) => Ok(ViewerTarget {
                origin,
                declared_name: alias.name.clone(),
                kind_label: "alias",
                record: ViewerRecord::Value,
                target_type: target.clone(),
                is_component: false,
            }),
            _ => not_allowed(
                "it is an alias of a type that is not a primitive type, and only an alias of a primitive type can have a viewer",
            ),
        },
        Item::Function(_) => not_allowed("it is a function"),
        Item::Value(_) => not_allowed("it is a value"),
    }
}

/// Whether `name` is a primitive type a value can have.
fn is_primitive_type_name(name: &str) -> bool {
    matches!(
        name,
        "string" | "int" | "int32" | "int64" | "float32" | "float64" | "boolean"
    )
}
