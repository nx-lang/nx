//! `visualizer` declarations: what a visualizer declaration's target is, and which `@nx/visualizer`
//! record its value must be.

use crate::{
    ast, interface_component, interface_record, interface_type_alias, interface_union,
    props_record_name, DeclaringOrigin, InterfaceItem, InterfaceItemKind, Item, Name,
    PreparedModule, PreparedNamespace, RecordKind, ResolvedPreparedItem,
};

/// The root of the visualizer standard library, which is also its import path.
pub const VISUALIZER_LIBRARY_ROOT: &str = "@nx/visualizer";

/// The identity of the visualizer standard library's one module, which declares every visualizer
/// record.
pub const VISUALIZER_LIBRARY_MODULE: &str = "@nx/visualizer/visualizer.nx";

/// The name of the type parameter every visualizer record takes, which a visualizer declaration
/// binds.
pub const VISUALIZER_TARGET_PARAMETER: &str = "Target";

/// The `@nx/visualizer` record a visualizer declaration's value is, chosen by its target's kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VisualizerRecord {
    /// `ElementVisualizer`, for a record, an action or a component.
    Element,
    /// `ValueVisualizer`, for an alias of a primitive type.
    Value,
    /// `FunctionVisualizer`, for an element-style function invocation.
    Function,
}

impl VisualizerRecord {
    /// The record's name in `@nx/visualizer`.
    pub fn name(self) -> &'static str {
        match self {
            VisualizerRecord::Element => "ElementVisualizer",
            VisualizerRecord::Value => "ValueVisualizer",
            VisualizerRecord::Function => "FunctionVisualizer",
        }
    }
}

/// What a visualizer declaration's target resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualizerTarget {
    /// The declaration the target names.
    pub origin: DeclaringOrigin,
    /// The name the declaring module gave the declaration.
    pub declared_name: Name,
    /// What the declaration is, in the words a diagnostic uses: "record", "union", ...
    pub kind_label: &'static str,
    /// The visualizer record the value must be.
    pub record: VisualizerRecord,
    /// The type `Target` binds to, spelled as the visualizer declaration's module sees it: the
    /// target itself, or its derived props record when it is a component.
    /// None for FunctionVisualizer, which has no Target parameter.
    pub target_type: Option<Name>,
    /// Whether the target is a component, whose `Target` is its props record.
    pub is_component: bool,
}

/// Why a visualizer declaration's target cannot have a visualizer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisualizerTargetError {
    /// The target names nothing visible in the module.
    Unknown,
    /// The target names a declaration that cannot have a visualizer, for the reason given.
    NotAllowed { reason: String },
}

/// Resolves a visualizer declaration's target, written `target`, in `module`'s namespace and
/// decides which visualizer record it takes.
///
/// <para>A record, an action (an inline emitted action included) or a component takes an
/// `ElementVisualizer`, and an alias of a primitive type a `ValueVisualizer`. A generic declaration,
/// a derived one, a union, a paren-style function, a value and an alias of any other type cannot have a
/// visualizer in this release. An element-style function takes a presentation-only
/// `FunctionVisualizer`.</para>
pub fn resolve_visualizer_target(
    module: &PreparedModule,
    target: &Name,
) -> Result<VisualizerTarget, VisualizerTargetError> {
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
    Err(VisualizerTargetError::Unknown)
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

fn not_allowed(reason: impl Into<String>) -> Result<VisualizerTarget, VisualizerTargetError> {
    Err(VisualizerTargetError::NotAllowed {
        reason: reason.into(),
    })
}

fn classify(
    target: &Name,
    origin: DeclaringOrigin,
    item: &Item,
) -> Result<VisualizerTarget, VisualizerTargetError> {
    let element = |kind_label: &'static str, declared_name: &Name| VisualizerTarget {
        origin: origin.clone(),
        declared_name: declared_name.clone(),
        kind_label,
        record: VisualizerRecord::Element,
        target_type: Some(target.clone()),
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
                return not_allowed("it is a generic record, and generic declarations cannot have visualizers yet");
            }
            let kind_label = if record.is_action() { "action" } else { "record" };
            Ok(element(kind_label, &record.name))
        }
        Item::Component(component) => {
            if !component.type_params.is_empty() {
                return not_allowed("it is a generic component, and generic declarations cannot have visualizers yet");
            }
            Ok(VisualizerTarget {
                target_type: Some(props_record_name(target.as_str())),
                is_component: true,
                ..element("component", &component.name)
            })
        }
        Item::Union(union_def) if union_def.property_target().is_some() => {
            not_allowed("it is a derived property union")
        }
        Item::Union(_) => not_allowed("it is a union, and unions cannot have visualizers yet"),
        // A constrained alias of a primitive, `type Ratio = float32 where in 0.0..=1.0`, is still
        // an alias of that primitive.
        Item::TypeAlias(alias) => match match &alias.ty {
            ast::TypeRef::Constrained { base, .. } => base.as_ref(),
            ty => ty,
        } {
            ast::TypeRef::Name(name) if is_primitive_type_name(name.as_str()) => Ok(VisualizerTarget {
                origin,
                declared_name: alias.name.clone(),
                kind_label: "alias",
                record: VisualizerRecord::Value,
                target_type: Some(target.clone()),
                is_component: false,
            }),
            _ => not_allowed(
                "it is an alias of a type that is not a primitive type, and only an alias of a primitive type can have a visualizer",
            ),
        },
        Item::Function(function) if function.form == crate::FunctionForm::Element => Ok(VisualizerTarget {
            origin,
            declared_name: function.name.clone(),
            kind_label: "element function",
            record: VisualizerRecord::Function,
            target_type: None,
            is_component: false,
        }),
        Item::Function(_) => not_allowed("it is a paren-style function; only an element-style function (`let <Name …/>`) can have a FunctionVisualizer"),
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
