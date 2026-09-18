use crate::{
    ast, interface_component, same_declaration, Component, ComponentEmit, ComponentEmitKind,
    DeclarationKey, DeclaringOrigin, EffectiveEmit, EffectiveField, EffectiveTypeParameter,
    Element, ElementId, ExprId, InterfaceField, InterfaceItem, InterfaceItemKind, Item,
    LocalDefinitionId, Name, PreparedModule, PreparedNamespace, PropertyEntry,
    ResolvedPreparedItem,
};
use nx_diagnostics::TextSpan;
use rustc_hash::{FxHashMap, FxHashSet};

/// One component in another component's inheritance chain.
///
/// The name is how the extending component spelled its base; the origin is the declaration that
/// spelling reached in *that* component's module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentAncestor {
    pub name: Name,
    pub origin: Option<DeclaringOrigin>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveComponentContract {
    pub component: Component,
    /// Type parameters along the abstract base chain, inherited first, then the component's own.
    pub type_params: Vec<EffectiveTypeParameter>,
    pub props: Vec<EffectiveField>,
    pub emits: Vec<EffectiveEmit>,
    pub ancestors: Vec<ComponentAncestor>,
    /// The declaration this contract was resolved from, where the resolving context reached one.
    pub origin: Option<DeclaringOrigin>,
}

impl EffectiveComponentContract {
    pub fn content_prop(&self) -> Option<&EffectiveField> {
        self.props.iter().find(|field| field.is_content)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingHandlerRewrite {
    element: ElementId,
    property_span: TextSpan,
    component: Name,
    emit: Name,
    action_name: Name,
    action_module_identity: String,
    owner: Option<Name>,
    span: TextSpan,
    body: ExprId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidComponentBaseReason {
    NotFound,
    NotComponent,
    ConcreteComponent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentResolutionError {
    InvalidBase {
        component: Name,
        base: Name,
        span: TextSpan,
        reason: InvalidComponentBaseReason,
    },
    InheritanceCycle {
        component: Name,
        span: TextSpan,
        cycle: Vec<Name>,
    },
    DuplicateInheritedProp {
        component: Name,
        prop: Name,
        inherited_from: Name,
        span: TextSpan,
    },
    /// A derived type parameter or prop takes the name of an inherited type parameter.
    DuplicateInheritedTypeParameter {
        component: Name,
        name: Name,
        inherited_from: Name,
        span: TextSpan,
    },
    DuplicateContentProperty {
        component: Name,
        existing_prop: Name,
        existing_owner: Name,
        prop: Name,
        span: TextSpan,
    },
    /// An inline emitted action's payload field is typed by one of the component's effective type
    /// parameters, declared or inherited.
    TypeParameterInEmitPayload {
        component: Name,
        emit: Name,
        field: Name,
        parameter: Name,
        span: TextSpan,
    },
    DuplicateInheritedEmit {
        component: Name,
        emit: Name,
        inherited_from: Name,
        span: TextSpan,
    },
    HandlerNameCollision {
        component: Name,
        prop: Name,
        emit: Name,
        span: TextSpan,
    },
}

impl ComponentResolutionError {
    pub fn code(&self) -> &'static str {
        match self {
            ComponentResolutionError::InvalidBase { reason, .. } => match reason {
                InvalidComponentBaseReason::NotFound => "component-base-not-found",
                InvalidComponentBaseReason::NotComponent => "component-base-not-component",
                InvalidComponentBaseReason::ConcreteComponent => "component-base-not-abstract",
            },
            ComponentResolutionError::InheritanceCycle { .. } => "component-inheritance-cycle",
            ComponentResolutionError::DuplicateInheritedProp { .. } => {
                "component-duplicate-inherited-prop"
            }
            ComponentResolutionError::DuplicateInheritedTypeParameter { .. } => {
                "component-duplicate-inherited-type-parameter"
            }
            ComponentResolutionError::DuplicateContentProperty { .. } => {
                "component-duplicate-content-prop"
            }
            ComponentResolutionError::TypeParameterInEmitPayload { .. } => {
                "component-type-parameter-in-emit-payload"
            }
            ComponentResolutionError::DuplicateInheritedEmit { .. } => {
                "component-duplicate-inherited-emit"
            }
            ComponentResolutionError::HandlerNameCollision { .. } => {
                "component-handler-name-collision"
            }
        }
    }

    pub fn message(&self) -> String {
        match self {
            ComponentResolutionError::InvalidBase {
                component,
                base,
                reason,
                ..
            } => match reason {
                InvalidComponentBaseReason::NotFound => format!(
                    "Component '{}' extends '{}', but '{}' could not be resolved",
                    component, base, base
                ),
                InvalidComponentBaseReason::NotComponent => format!(
                    "Component '{}' extends '{}', but '{}' does not resolve to an abstract component declaration",
                    component, base, base
                ),
                InvalidComponentBaseReason::ConcreteComponent => format!(
                    "Component '{}' extends '{}', but only abstract components may be extended",
                    component, base
                ),
            },
            ComponentResolutionError::InheritanceCycle { cycle, .. } => {
                let chain = cycle
                    .iter()
                    .map(|name| name.as_str())
                    .collect::<Vec<_>>()
                    .join(" -> ");
                format!("Component inheritance cycle detected: {}", chain)
            }
            ComponentResolutionError::DuplicateInheritedProp {
                component,
                prop,
                inherited_from,
                ..
            } => format!(
                "Component '{}' redeclares inherited prop '{}' from '{}'",
                component, prop, inherited_from
            ),
            ComponentResolutionError::DuplicateInheritedTypeParameter {
                component,
                name,
                inherited_from,
                ..
            } => format!(
                "Component '{}' redeclares inherited type parameter '{}' from '{}'",
                component, name, inherited_from
            ),
            ComponentResolutionError::TypeParameterInEmitPayload {
                component,
                emit,
                field,
                parameter,
                ..
            } => format!(
                "Emitted action '{}' on component '{}' cannot type its payload field '{}' by type parameter '{}'; a type parameter is a type only in the component's props, state, and body",
                emit, component, field, parameter
            ),
            ComponentResolutionError::DuplicateContentProperty {
                component,
                existing_prop,
                existing_owner,
                prop,
                ..
            } => {
                if existing_owner == component {
                    format!(
                        "Component '{}' declares more than one content prop: '{}' and '{}'",
                        component, existing_prop, prop
                    )
                } else {
                    format!(
                        "Component '{}' declares content prop '{}' but already inherits content prop '{}' from '{}'",
                        component, prop, existing_prop, existing_owner
                    )
                }
            }
            ComponentResolutionError::DuplicateInheritedEmit {
                component,
                emit,
                inherited_from,
                ..
            } => format!(
                "Component '{}' redeclares inherited emitted action '{}' from '{}'",
                component, emit, inherited_from
            ),
            ComponentResolutionError::HandlerNameCollision {
                component,
                prop,
                emit,
                ..
            } => format!(
                "Component '{}' declares prop '{}' which collides with emitted action handler '{}'",
                component,
                prop,
                handler_prop_name(emit.as_str())
            ),
        }
    }

    pub fn span(&self) -> TextSpan {
        match self {
            ComponentResolutionError::InvalidBase { span, .. }
            | ComponentResolutionError::InheritanceCycle { span, .. }
            | ComponentResolutionError::DuplicateInheritedProp { span, .. }
            | ComponentResolutionError::DuplicateInheritedTypeParameter { span, .. }
            | ComponentResolutionError::DuplicateContentProperty { span, .. }
            | ComponentResolutionError::TypeParameterInEmitPayload { span, .. }
            | ComponentResolutionError::DuplicateInheritedEmit { span, .. }
            | ComponentResolutionError::HandlerNameCollision { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OwnedRecordField {
    field: EffectiveField,
    owner: Name,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OwnedComponentEmit {
    emit: ComponentEmit,
    owner: Name,
    module_identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OwnedTypeParameter {
    param: EffectiveTypeParameter,
    owner: Name,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedComponentContract {
    component: Component,
    type_params: Vec<OwnedTypeParameter>,
    props: Vec<OwnedRecordField>,
    emits: Vec<OwnedComponentEmit>,
    ancestors: Vec<ComponentAncestor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ComponentFieldSource {
    Raw,
    Interface {
        props: Vec<InterfaceField>,
        state: Vec<InterfaceField>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedComponentDefinition {
    component: Component,
    /// The module whose namespace this component's own type references — its base above all —
    /// were written in.
    module_identity: String,
    definition_id: Option<LocalDefinitionId>,
    field_source: ComponentFieldSource,
}

impl ResolvedComponentDefinition {
    /// The identity an inheritance walk tracks this component by.
    fn key(&self) -> DeclarationKey {
        DeclarationKey::new(self.origin(), &self.component.name)
    }

    fn origin(&self) -> Option<DeclaringOrigin> {
        self.definition_id
            .map(|definition_id| DeclaringOrigin::new(&self.module_identity, definition_id))
    }

    fn declared_type_params(&self) -> Vec<OwnedTypeParameter> {
        self.component
            .type_params
            .iter()
            .cloned()
            .map(|param| OwnedTypeParameter {
                param: EffectiveTypeParameter::from_type_parameter(
                    param,
                    self.module_identity.clone(),
                ),
                owner: self.component.name.clone(),
            })
            .collect()
    }

    fn declared_props(&self) -> Vec<EffectiveField> {
        match &self.field_source {
            ComponentFieldSource::Raw => self
                .component
                .props
                .iter()
                .cloned()
                .map(|field| EffectiveField::from_record_field(field, self.module_identity.clone()))
                .collect(),
            ComponentFieldSource::Interface { props, .. } => props
                .iter()
                .map(|field| {
                    EffectiveField::from_interface_field(field, self.module_identity.clone())
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ComponentValidationStatus {
    Valid,
    Invalid,
}

pub fn resolve_component_definition(module: &PreparedModule, name: &Name) -> Option<Component> {
    resolve_component_definition_with_identity(module, module.module_identity(), name)
        .map(|resolved| resolved.component)
}

/// Returns the declaration a component name reaches, where one exists.
pub fn component_declaration_origin(
    module: &PreparedModule,
    name: &Name,
) -> Option<DeclaringOrigin> {
    resolve_component_definition_with_identity(module, module.module_identity(), name)
        .and_then(|resolved| resolved.origin())
}

pub fn effective_component_contract_for_name(
    module: &PreparedModule,
    name: &Name,
) -> Result<Option<EffectiveComponentContract>, ComponentResolutionError> {
    let Some(component) =
        resolve_component_definition_with_identity(module, module.module_identity(), name)
    else {
        return Ok(None);
    };

    effective_component_contract_resolved(module, &component).map(Some)
}

/// Returns the effective contract of the component declared at `origin`.
///
/// The declaration is read straight out of the module that declares it, so the component does not
/// have to be nameable in the asking module at all.
pub fn effective_component_contract_at(
    module: &PreparedModule,
    origin: &DeclaringOrigin,
) -> Result<Option<EffectiveComponentContract>, ComponentResolutionError> {
    let Some(component) = component_definition_at(module, origin) else {
        return Ok(None);
    };

    effective_component_contract_resolved(module, &component).map(Some)
}

/// Reads the component declared at `origin` directly out of the module that declares it.
fn component_definition_at(
    module: &PreparedModule,
    origin: &DeclaringOrigin,
) -> Option<ResolvedComponentDefinition> {
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
        Item::Component(component) => Some(ResolvedComponentDefinition {
            component: component.clone(),
            module_identity: origin.module_identity().to_string(),
            definition_id: Some(origin.definition_id()),
            field_source: ComponentFieldSource::Raw,
        }),
        _ => None,
    }
}

/// Decides whether a component satisfies an expected component type.
///
/// Both sides carry the declaration they were resolved to, and both are compared by it. Comparing
/// the spellings instead let a consumer's own `Card` satisfy a property typed by a different
/// module's `Card`.
pub fn is_component_subtype(
    module: &PreparedModule,
    actual: &Name,
    actual_origin: Option<&DeclaringOrigin>,
    expected: &Name,
    expected_origin: Option<&DeclaringOrigin>,
) -> Result<bool, ComponentResolutionError> {
    let Some(actual_component) = component_reference(module, actual, actual_origin) else {
        return Ok(false);
    };
    let Some(expected_component) = component_reference(module, expected, expected_origin) else {
        return Ok(false);
    };

    let expected_origin = expected_component.origin();
    if same_declaration(
        actual_component.origin().as_ref(),
        &actual_component.component.name,
        expected_origin.as_ref(),
        &expected_component.component.name,
    ) {
        return Ok(true);
    }

    let actual_contract = effective_component_contract_resolved(module, &actual_component)?;
    Ok(actual_contract.ancestors.iter().any(|ancestor| {
        same_declaration(
            ancestor.origin.as_ref(),
            &ancestor.name,
            expected_origin.as_ref(),
            &expected_component.component.name,
        )
    }))
}

/// Resolves a component reference by the declaration it names, or by its spelling if it names none.
///
/// As with records, a reference that carries an origin never falls back to the local name: a read
/// that fails resolves to nothing rather than to whatever the asking module happens to call that.
fn component_reference(
    module: &PreparedModule,
    name: &Name,
    origin: Option<&DeclaringOrigin>,
) -> Option<ResolvedComponentDefinition> {
    match origin {
        Some(origin) => component_definition_at(module, origin),
        None => resolve_component_definition_with_identity(module, module.module_identity(), name),
    }
}

pub fn effective_component_contract(
    module: &PreparedModule,
    component: &Component,
) -> Result<EffectiveComponentContract, ComponentResolutionError> {
    let definition_id = module
        .raw_module()
        .find_item_with_definition(component.name.as_str())
        .map(|(definition_id, _)| definition_id);
    let component = ResolvedComponentDefinition {
        component: component.clone(),
        module_identity: module.module_identity().to_string(),
        definition_id,
        field_source: ComponentFieldSource::Raw,
    };
    effective_component_contract_resolved(module, &component)
}

fn effective_component_contract_resolved(
    module: &PreparedModule,
    component: &ResolvedComponentDefinition,
) -> Result<EffectiveComponentContract, ComponentResolutionError> {
    let resolved = resolve_component_contract_inner(module, component, &mut Vec::new())?;
    Ok(EffectiveComponentContract {
        component: resolved.component,
        type_params: resolved
            .type_params
            .into_iter()
            .map(|owned| owned.param)
            .collect(),
        props: resolved
            .props
            .into_iter()
            .map(|field| field.field)
            .collect(),
        emits: resolved
            .emits
            .into_iter()
            .map(|emit| EffectiveEmit {
                emit: emit.emit,
                module_identity: emit.module_identity,
            })
            .collect(),
        ancestors: resolved.ancestors,
        origin: component.origin(),
    })
}

fn resolve_component_definition_with_identity(
    module: &PreparedModule,
    namespace_module: &str,
    name: &Name,
) -> Option<ResolvedComponentDefinition> {
    let resolved = module.resolve_in_module(PreparedNamespace::Element, namespace_module, name)?;
    component_definition_from_prepared_item(module, resolved)
}

fn component_definition_from_interface_item(
    item: &InterfaceItem,
) -> Option<ResolvedComponentDefinition> {
    let component = interface_component(item)?;
    let (props, state) = match &item.item {
        InterfaceItemKind::Component { props, state, .. } => (props.clone(), state.clone()),
        _ => return None,
    };
    Some(ResolvedComponentDefinition {
        component,
        module_identity: item.module_identity.clone(),
        definition_id: Some(item.definition_id),
        field_source: ComponentFieldSource::Interface { props, state },
    })
}

fn component_definition_from_prepared_item(
    module: &PreparedModule,
    resolved: ResolvedPreparedItem,
) -> Option<ResolvedComponentDefinition> {
    match resolved {
        ResolvedPreparedItem::Raw {
            module_identity,
            definition_id,
            item: Item::Component(component),
            ..
        } => Some(ResolvedComponentDefinition {
            component,
            module_identity,
            definition_id: Some(definition_id),
            field_source: ComponentFieldSource::Raw,
        }),
        ResolvedPreparedItem::Imported { item, raw, .. } => {
            if let Some(raw_ref) = raw.as_ref() {
                if let Some(Item::Component(component)) = module.resolve_imported_raw_item(raw_ref)
                {
                    return Some(ResolvedComponentDefinition {
                        component,
                        module_identity: raw_ref.module_identity.clone(),
                        definition_id: Some(raw_ref.definition_id),
                        field_source: ComponentFieldSource::Raw,
                    });
                }
            }
            component_definition_from_interface_item(&item)
        }
        _ => None,
    }
}

pub fn validate_component_definitions(module: &PreparedModule) -> Vec<ComponentResolutionError> {
    let mut errors = Vec::new();
    let mut statuses = FxHashMap::default();
    let mut stack = Vec::new();

    for (index, item) in module.raw_module().items().iter().enumerate() {
        let Item::Component(component) = item else {
            continue;
        };
        let component = ResolvedComponentDefinition {
            component: component.clone(),
            module_identity: module.module_identity().to_string(),
            definition_id: Some(LocalDefinitionId::new(index as u32)),
            field_source: ComponentFieldSource::Raw,
        };
        validate_component_definition(module, &component, &mut statuses, &mut stack, &mut errors);
    }

    errors
}

/// Rewrites resolved contextual names into the qualified member access they resolved to.
///
/// Type analysis resolves a bare name against the declared type of its binding site and reports
/// which union case it named. Applying those resolutions here means nothing after
/// type checking — the interpreter, codegen, or the IR — can tell a contextual literal from the
/// qualified form, which is what lets every downstream consumer stay unchanged.
/// One resolved contextual name, reduced to what the rewrite needs.
///
/// The origin is the `(module identity, definition id)` pair addressing the union the bare name
/// resolved against. A resolution that reached no origin is left as it was written: rewriting it to
/// a reference nothing can resolve would only move the failure downstream.
pub struct ContextualRewrite {
    pub union: Name,
    pub case: Name,
    pub module_identity: String,
    pub definition_id: LocalDefinitionId,
}

/// Rewrites every resolved [`ast::Expr::ContextualName`] to the case it resolved to.
///
/// The rewrite target carries the union's declaring origin rather than a name, so nothing below
/// type checking has to find the declaration again by a spelling that need not be visible here.
pub fn apply_contextual_name_resolutions<T>(
    module: &mut PreparedModule,
    resolutions: &FxHashMap<ExprId, T>,
    rewrite: impl Fn(&T) -> Option<ContextualRewrite>,
) {
    if resolutions.is_empty() {
        return;
    }

    let raw_module = module.raw_module_mut();
    for (expr_id, resolution) in resolutions {
        let span = match raw_module.expr(*expr_id) {
            ast::Expr::ContextualName { span, .. } => *span,
            // Already rewritten, or never a contextual name: leave it alone.
            _ => continue,
        };
        let Some(rewrite) = rewrite(resolution) else {
            continue;
        };
        *raw_module.expr_mut(*expr_id) = ast::Expr::ResolvedUnionCase {
            union: rewrite.union,
            case: rewrite.case,
            module_identity: rewrite.module_identity,
            definition_id: rewrite.definition_id,
            span,
        };
    }
}

/// Replaces each constant expression that took a type from its site with the literal it folded to.
///
/// <para>`folded` maps a constant expression, arithmetic over numeric literals only, to its value
/// at the type its literals have on their own. The expression keeps its id, so the type analysis
/// recorded for it still applies, and [`apply_literal_conversions`] then gives the literal the
/// site's width, as it would a literal the author wrote. Run it first.</para>
pub fn apply_constant_folds(module: &mut PreparedModule, folded: &FxHashMap<ExprId, ast::Literal>) {
    let raw_module = module.raw_module_mut();
    for (expr_id, literal) in folded {
        *raw_module.expr_mut(*expr_id) = ast::Expr::Literal(literal.clone());
    }
}

/// Rewrites each numeric literal that took a type from its site into a literal of that width.
///
/// <para>Runs on the same terms as [`apply_contextual_name_resolutions`] and for the same reason:
/// once this has run, source that wrote `24` at a float-typed property is indistinguishable from
/// source that wrote `24.0`, and `1` at an `int32` site is an `int32` literal, so no consumer below
/// type checking needs to know the rule exists — or is able to observe that it applied.</para>
///
/// <para>`converted` maps each literal to the primitive it was typed as. An integer literal at a
/// floating-point site becomes a real literal of that width; at `int32` it becomes an `int32`
/// literal; at `int64` it stays as it is, since `int64` shares the `int` literal form. A real
/// literal at a `float32` site is rounded to the nearest `float32`, as a `float32` literal is in
/// any language. The span is kept, so a diagnostic or a source map still points at what the author
/// wrote.</para>
pub fn apply_literal_conversions(
    module: &mut PreparedModule,
    converted: &FxHashMap<ExprId, ast::PrimitiveType>,
) {
    if converted.is_empty() {
        return;
    }

    let raw_module = module.raw_module_mut();
    for (expr_id, target) in converted {
        let rewritten = match (raw_module.expr(*expr_id), target) {
            (ast::Expr::Literal(ast::Literal::Int(value)), ast::PrimitiveType::Float64) => {
                ast::Literal::Float(ast::OrderedFloat(*value as f64))
            }
            (ast::Expr::Literal(ast::Literal::Int(value)), ast::PrimitiveType::Float32) => {
                ast::Literal::Float32(ast::OrderedFloat(f64::from(*value as f32)))
            }
            (ast::Expr::Literal(ast::Literal::Int(value)), ast::PrimitiveType::Int32) => {
                // The checker recorded `int32` only after checking the range.
                ast::Literal::Int32(*value as i32)
            }
            (ast::Expr::Literal(ast::Literal::Float(value)), ast::PrimitiveType::Float32) => {
                ast::Literal::Float32(ast::OrderedFloat(f64::from(value.0 as f32)))
            }
            // `int` and `int64` keep the literal form they have; anything else is already
            // rewritten, or was never a numeric literal.
            _ => continue,
        };
        *raw_module.expr_mut(*expr_id) = ast::Expr::Literal(rewritten);
    }
}

/// Wraps each branch of a join that type analysis found to widen in an [`ast::Expr::Widen`].
///
/// <para>Runs on the same terms as [`apply_string_conversions`]. `widened` maps a branch of an
/// `if` or `match`, or an element of a list literal, to the numeric type the join has. The
/// wrapper takes the branch's place in its parent, so the branch keeps its id and the types
/// recorded for it stay valid. The nodes this creates are returned with the branch each wraps, so
/// the caller can type them.</para>
pub fn apply_join_widenings(
    module: &mut PreparedModule,
    widened: &FxHashMap<ExprId, ast::PrimitiveType>,
) -> Vec<(ExprId, ExprId)> {
    let mut created = Vec::new();
    if widened.is_empty() {
        return created;
    }

    let raw_module = module.raw_module_mut();
    let parents = raw_module
        .exprs()
        .filter(|(_, expr)| {
            matches!(
                expr,
                ast::Expr::If { .. } | ast::Expr::Match { .. } | ast::Expr::Array { .. }
            )
        })
        .map(|(id, _)| id)
        .collect::<Vec<_>>();

    for parent in parents {
        let mut parent_expr = raw_module.expr(parent).clone();
        let mut children = match &mut parent_expr {
            ast::Expr::If {
                then_branch,
                else_branch,
                ..
            } => std::iter::once(then_branch)
                .chain(else_branch.as_mut())
                .collect::<Vec<_>>(),
            ast::Expr::Match {
                arms, else_branch, ..
            } => arms
                .iter_mut()
                .map(|arm| &mut arm.body)
                .chain(else_branch.as_mut())
                .collect(),
            ast::Expr::Array { elements, .. } => elements.iter_mut().collect(),
            _ => continue,
        };

        let mut changed = false;
        for child in children.iter_mut() {
            let Some(ty) = widened.get(child) else {
                continue;
            };
            let span = raw_module.expr_span(**child);
            let wrapped = raw_module.alloc_expr(ast::Expr::Widen {
                expr: **child,
                ty: *ty,
                span,
            });
            raw_module.set_expr_span(wrapped, span);
            created.push((wrapped, **child));
            **child = wrapped;
            changed = true;
        }
        drop(children);
        if changed {
            *raw_module.expr_mut(parent) = parent_expr;
        }
    }

    created
}

/// The string conversions type analysis decided on, to be applied to the prepared module.
///
/// <para>Every `+` in `concatenations` had a string operand; every expression in `text_conversions`
/// is an operand (of one of those, or of a joined body) whose primitive value is rendered as text;
/// every element in `joined_bodies` has a body of text runs and braced values that binds to a
/// `string` content property and is joined into one string.</para>
#[derive(Debug, Default, Clone)]
pub struct StringConversions {
    pub concatenations: FxHashSet<ExprId>,
    pub text_conversions: FxHashMap<ExprId, ast::PrimitiveType>,
    pub joined_bodies: FxHashSet<ElementId>,
}

/// Rewrites the additions, operands and bodies type analysis recorded as string conversions.
///
/// <para>Runs on the same terms as [`apply_contextual_name_resolutions`]: the type checker is the
/// one place every operand's type is known, so it decides which `+` concatenates, and after this
/// has run neither the interpreter nor a code generator has to decide again. A recorded `+`
/// becomes an [`ast::Expr::Concat`]; a recorded operand is wrapped in an [`ast::Expr::ToText`]
/// naming its type; a recorded body is replaced by one `Concat` chain over its pieces, with the
/// layout taken out of the text (see [`collapse_line_breaks`]): the whitespace between two pieces
/// and inside each text run is kept, except that a stretch containing a line break becomes one
/// space, and a text run at the start or end of the body is stripped of its leading or trailing
/// whitespace. A braced value, and raw text, is kept as written.</para>
///
/// <para>Existing expressions keep their ids, so the types recorded for them stay valid. The nodes
/// this creates are returned so the caller can record their type, which is `string` for every
/// one.</para>
pub fn apply_string_conversions(
    module: &mut PreparedModule,
    conversions: &StringConversions,
) -> Vec<ExprId> {
    let mut created = Vec::new();
    if conversions.concatenations.is_empty() && conversions.joined_bodies.is_empty() {
        return created;
    }

    let raw_module = module.raw_module_mut();

    // Wraps a recorded operand in its text conversion; any other expression is returned as is.
    fn as_text(
        raw_module: &mut crate::LoweredModule,
        conversions: &StringConversions,
        created: &mut Vec<ExprId>,
        operand: ExprId,
    ) -> ExprId {
        let Some(ty) = conversions.text_conversions.get(&operand) else {
            return operand;
        };
        let span = raw_module.expr_span(operand);
        let wrapped = raw_module.alloc_expr(ast::Expr::ToText {
            expr: operand,
            ty: *ty,
            span,
        });
        raw_module.set_expr_span(wrapped, span);
        created.push(wrapped);
        wrapped
    }

    for expr_id in &conversions.concatenations {
        let (lhs, rhs, span) = match raw_module.expr(*expr_id) {
            ast::Expr::BinaryOp {
                lhs,
                op: ast::BinOp::Add,
                rhs,
                span,
            } => (*lhs, *rhs, *span),
            // Already rewritten, or never an addition: leave it alone.
            _ => continue,
        };
        let lhs = as_text(raw_module, conversions, &mut created, lhs);
        let rhs = as_text(raw_module, conversions, &mut created, rhs);
        *raw_module.expr_mut(*expr_id) = ast::Expr::Concat { lhs, rhs, span };
    }

    for element_id in &conversions.joined_bodies {
        let element = raw_module.element(*element_id);
        let pieces = element.content.clone();
        let whitespace_runs = element.whitespace_runs.clone();
        let text_runs = element.text_runs.clone();
        let typed_body = element.text_type.is_some();
        let starts_with_text = text_runs.first() == Some(&0);
        let ends_with_text = text_runs.last() == Some(&(pieces.len().wrapping_sub(1)));
        let span = element.span;
        if pieces.is_empty() {
            continue;
        }

        // The pieces in source order, with the whitespace between two of them put back. In a plain
        // body the line breaks in it, and in each text run, read as a space; in a typed body the
        // text is kept as written and only its indentation comes off below.
        let mut sequence = Vec::with_capacity(pieces.len() * 2);
        let mut layout = Vec::new();
        for (index, piece) in pieces.iter().enumerate() {
            if index > 0 {
                for run in whitespace_runs.iter().filter(|run| run.before == index) {
                    let text = if typed_body {
                        run.text.clone()
                    } else {
                        collapse_line_breaks(&run.text)
                    };
                    let literal =
                        raw_module.alloc_expr(ast::Expr::Literal(ast::Literal::String(text)));
                    raw_module.set_expr_span(literal, span);
                    created.push(literal);
                    layout.push(literal);
                    sequence.push(literal);
                }
            }
            if text_runs.contains(&index) {
                if !typed_body {
                    if let ast::Expr::Literal(ast::Literal::String(text)) = raw_module.expr(*piece)
                    {
                        let collapsed = ast::Literal::String(collapse_line_breaks(text));
                        *raw_module.expr_mut(*piece) = ast::Expr::Literal(collapsed);
                    }
                }
                layout.push(*piece);
            }
            sequence.push(*piece);
        }

        if typed_body {
            dedent_typed_body(raw_module, &sequence, &layout);
        } else {
            // Layout around the body is not text: a run that opens the body loses its leading
            // whitespace and one that closes it its trailing, the way the body reads. A braced
            // string is a value, so it keeps its spaces wherever it stands.
            let first = sequence[0];
            if starts_with_text {
                if let ast::Expr::Literal(ast::Literal::String(text)) = raw_module.expr(first) {
                    let trimmed = ast::Literal::String(smol_str::SmolStr::new(text.trim_start()));
                    *raw_module.expr_mut(first) = ast::Expr::Literal(trimmed);
                }
            }
            let last = sequence[sequence.len() - 1];
            if ends_with_text {
                if let ast::Expr::Literal(ast::Literal::String(text)) = raw_module.expr(last) {
                    let trimmed = ast::Literal::String(smol_str::SmolStr::new(text.trim_end()));
                    *raw_module.expr_mut(last) = ast::Expr::Literal(trimmed);
                }
            }
        }

        let mut chain = as_text(raw_module, conversions, &mut created, sequence[0]);
        for piece in &sequence[1..] {
            let rhs = as_text(raw_module, conversions, &mut created, *piece);
            let joined = raw_module.alloc_expr(ast::Expr::Concat {
                lhs: chain,
                rhs,
                span,
            });
            raw_module.set_expr_span(joined, span);
            created.push(joined);
            chain = joined;
        }
        raw_module.element_mut(*element_id).content = vec![chain];
    }

    created
}

/// Takes the source's indentation out of a typed body, leaving its line breaks as written.
///
/// <para>A typed body (`<Note:markdown>`) is text for a processor the host supplies, so its blank
/// lines and list markers are its own and a plain body's rule — a line break reads as one space —
/// would destroy them. What is layout in a typed body is only the indentation it is written at:
/// the common indentation of its lines comes off every line, the line break that opens the body
/// goes, and so does the whitespace-only line that closes it. A braced value keeps its own text,
/// and a line break inside one is not indentation.</para>
fn dedent_typed_body(
    raw_module: &mut crate::LoweredModule,
    sequence: &[ExprId],
    layout: &[ExprId],
) {
    // The body as written, with each braced value standing in as one non-space character, so a
    // line that holds only a value still counts as a line with text on it.
    let mut written = String::new();
    for piece in sequence {
        match (layout.contains(piece), raw_module.expr(*piece)) {
            (true, ast::Expr::Literal(ast::Literal::String(text))) => written.push_str(text),
            _ => written.push('\u{0}'),
        }
    }

    let indent = common_indent(&written);
    if !indent.is_empty() {
        for piece in layout {
            let ast::Expr::Literal(ast::Literal::String(text)) = raw_module.expr(*piece) else {
                continue;
            };
            let stripped = strip_indent(text, &indent);
            *raw_module.expr_mut(*piece) = ast::Expr::Literal(ast::Literal::String(stripped));
        }
    }

    // The tag's own line and the closing tag's are the source's, not the text's.
    let first = sequence[0];
    if layout.contains(&first) {
        if let ast::Expr::Literal(ast::Literal::String(text)) = raw_module.expr(first) {
            if let Some(rest) = text.split_once('\n').and_then(|(head, rest)| {
                head.chars()
                    .all(|ch| ch == ' ' || ch == '\t')
                    .then_some(rest)
            }) {
                let opened = ast::Literal::String(smol_str::SmolStr::new(rest));
                *raw_module.expr_mut(first) = ast::Expr::Literal(opened);
            }
        }
    }
    let last = sequence[sequence.len() - 1];
    if layout.contains(&last) {
        if let ast::Expr::Literal(ast::Literal::String(text)) = raw_module.expr(last) {
            if let Some(head) = text.rsplit_once('\n').and_then(|(head, tail)| {
                tail.chars()
                    .all(|ch| ch == ' ' || ch == '\t')
                    .then_some(head)
            }) {
                let closed = ast::Literal::String(smol_str::SmolStr::new(head));
                *raw_module.expr_mut(last) = ast::Expr::Literal(closed);
            }
        }
    }
}

/// The whitespace every line of `written` that has text on it begins with.
///
/// <para>The first line is the tag's own line, which carries no indentation of its own, and a
/// blank line indents nothing, so neither takes part.</para>
fn common_indent(written: &str) -> String {
    let mut common: Option<&str> = None;
    for line in written.split('\n').skip(1) {
        if line.chars().all(char::is_whitespace) {
            continue;
        }
        let indent = &line[..line.len() - line.trim_start().len()];
        common = Some(match common {
            None => indent,
            Some(current) => {
                let shared = current
                    .char_indices()
                    .zip(indent.chars())
                    .take_while(|((_, left), right)| left == right)
                    .map(|((offset, left), _)| offset + left.len_utf8())
                    .last()
                    .unwrap_or(0);
                &current[..shared]
            }
        });
    }
    common.unwrap_or("").to_string()
}

/// `text` with `indent` removed from the start of every line it begins.
fn strip_indent(text: &str, indent: &str) -> smol_str::SmolStr {
    let mut stripped = String::with_capacity(text.len());
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            stripped.push('\n');
            stripped.push_str(line.strip_prefix(indent).unwrap_or(line));
        } else {
            stripped.push_str(line);
        }
    }
    smol_str::SmolStr::new(stripped)
}

/// Replaces each stretch of whitespace in `text` that contains a line break with one space.
///
/// <para>A line break in a body is layout: where a line ends and how far the next is indented
/// depend on how the source is formatted, not on the text, so a body reads the same whether it is
/// written on one line or several. Whitespace within a line is kept as written.</para>
pub fn collapse_line_breaks(text: &str) -> smol_str::SmolStr {
    let mut collapsed = String::with_capacity(text.len());
    let mut stretch = String::new();
    for ch in text.chars() {
        if ch.is_whitespace() {
            stretch.push(ch);
            continue;
        }
        push_whitespace_stretch(&mut collapsed, &stretch);
        stretch.clear();
        collapsed.push(ch);
    }
    push_whitespace_stretch(&mut collapsed, &stretch);
    smol_str::SmolStr::new(collapsed)
}

fn push_whitespace_stretch(out: &mut String, stretch: &str) {
    if stretch.contains(['\n', '\r']) {
        out.push(' ');
    } else {
        out.push_str(stretch);
    }
}

/// Replaces every reference to one of `params` in `ty` with the top type `object`, keeping the
/// `[]` and `?` layers around it.
///
/// <para>This is the erasure a generated surface applies when it receives a component value
/// dynamically and so has nothing to bind a type parameter to: the IR prop schema, the C# contract
/// record, and the serializable TypeScript element type. Function types are left alone: a type
/// parameter cannot appear in one, since the only place a parameter is a type is a component's
/// own prop and state annotations.</para>
pub fn erase_type_parameters(ty: &ast::TypeRef, params: &[Name]) -> ast::TypeRef {
    match ty {
        ast::TypeRef::Name(name) if params.iter().any(|param| param == name) => {
            ast::TypeRef::name("object")
        }
        ast::TypeRef::Name(_) | ast::TypeRef::Function { .. } => ty.clone(),
        ast::TypeRef::Array(inner) => ast::TypeRef::array(erase_type_parameters(inner, params)),
        ast::TypeRef::Nullable(inner) => {
            ast::TypeRef::nullable(erase_type_parameters(inner, params))
        }
    }
}

/// Removes every plain `Value` property entry whose value expression is in `consumed` from every
/// element in the module.
///
/// <para>Runs on the same terms as [`apply_contextual_name_resolutions`]: a type-argument binding
/// such as `TItem=Contact` is a spelling only the type checker understands, and once the checker
/// has consumed it nothing below — the interpreter, the IR builder, generated code — should be
/// able to observe that it was written. Only plain entries qualify; a type argument inside a
/// conditional fragment is rejected by the checker and never recorded here.</para>
pub fn remove_property_entries(module: &mut PreparedModule, consumed: &FxHashSet<ExprId>) {
    if consumed.is_empty() {
        return;
    }

    let raw_module = module.raw_module_mut();
    for (_, element) in raw_module.elements_mut() {
        element
            .properties
            .retain(|property| !consumed.contains(&property.value));
        element.property_entries.retain(|entry| {
            !matches!(entry, PropertyEntry::Value(property) if consumed.contains(&property.value))
        });
    }
}

pub fn promote_component_handler_bindings(module: &mut PreparedModule) {
    let rewrites = collect_component_handler_rewrites(module);
    if rewrites.is_empty() {
        return;
    }

    let raw_module = module.raw_module_mut();
    for rewrite in &rewrites {
        let handler = raw_module.alloc_expr(ast::Expr::ActionHandler {
            component: rewrite.component.clone(),
            emit: rewrite.emit.clone(),
            action_name: rewrite.action_name.clone(),
            action_module_identity: Some(rewrite.action_module_identity.clone()),
            owner: rewrite.owner.clone(),
            body: rewrite.body,
            span: rewrite.span,
        });
        rewrite_property_handler(
            raw_module.element_mut(rewrite.element),
            rewrite.property_span,
            handler,
        );
    }
    raw_module.diagnostics_mut().retain(|diagnostic| {
        !rewrites.iter().any(|rewrite| {
            diagnostic.span == rewrite.span
                && diagnostic.message
                    == missing_emit_handler_message(&rewrite.component, &rewrite.emit)
        })
    });
}

fn collect_component_handler_rewrites(module: &PreparedModule) -> Vec<PendingHandlerRewrite> {
    let mut rewrites = Vec::new();
    for item in module.raw_module().items() {
        collect_handler_rewrites_in_item(module, item, &mut rewrites);
    }
    rewrites
}

fn collect_handler_rewrites_in_item(
    module: &PreparedModule,
    item: &Item,
    rewrites: &mut Vec<PendingHandlerRewrite>,
) {
    // A handler written inside a component declaration is owned by that component; anywhere else
    // it is bound at the root.
    let owner = match item {
        Item::Component(component) => Some(&component.name),
        _ => None,
    };
    match item {
        Item::Function(function) => {
            collect_handler_rewrites_in_expr(module, function.body, owner, rewrites)
        }
        Item::Value(value) => {
            collect_handler_rewrites_in_expr(module, value.value, owner, rewrites)
        }
        Item::Component(component) => {
            for field in &component.props {
                if let Some(default) = field.default {
                    collect_handler_rewrites_in_expr(module, default, owner, rewrites);
                }
            }
            for field in &component.state {
                if let Some(default) = field.default {
                    collect_handler_rewrites_in_expr(module, default, owner, rewrites);
                }
            }
            if let Some(body) = component.body {
                collect_handler_rewrites_in_expr(module, body, owner, rewrites);
            }
        }
        Item::Record(record) => {
            for field in &record.properties {
                if let Some(default) = field.default {
                    collect_handler_rewrites_in_expr(module, default, owner, rewrites);
                }
            }
        }
        Item::Union(union_def) => {
            for case in &union_def.cases {
                for field in &case.fields {
                    if let Some(default) = field.default {
                        collect_handler_rewrites_in_expr(module, default, owner, rewrites);
                    }
                }
            }
        }
        Item::TypeAlias(_) => {}
    }
}

fn collect_handler_rewrites_in_expr(
    module: &PreparedModule,
    expr_id: ExprId,
    owner: Option<&Name>,
    rewrites: &mut Vec<PendingHandlerRewrite>,
) {
    match module.raw_module().expr(expr_id) {
        // `ContextualName` and the resolved case it becomes are leaves: neither carries a
        // sub-expression to rewrite.
        ast::Expr::Literal(_)
        | ast::Expr::Ident(_)
        | ast::Expr::ContextualName { .. }
        | ast::Expr::ResolvedUnionCase { .. }
        | ast::Expr::Error(_) => {}
        ast::Expr::BinaryOp { lhs, rhs, .. } | ast::Expr::Concat { lhs, rhs, .. } => {
            collect_handler_rewrites_in_expr(module, *lhs, owner, rewrites);
            collect_handler_rewrites_in_expr(module, *rhs, owner, rewrites);
        }
        ast::Expr::UnaryOp { expr, .. }
        | ast::Expr::ToText { expr, .. }
        | ast::Expr::Widen { expr, .. } => {
            collect_handler_rewrites_in_expr(module, *expr, owner, rewrites);
        }
        ast::Expr::Call { func, args, .. } => {
            collect_handler_rewrites_in_expr(module, *func, owner, rewrites);
            for arg in args {
                collect_handler_rewrites_in_expr(module, *arg, owner, rewrites);
            }
        }
        ast::Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            collect_handler_rewrites_in_expr(module, *condition, owner, rewrites);
            collect_handler_rewrites_in_expr(module, *then_branch, owner, rewrites);
            if let Some(else_branch) = else_branch {
                collect_handler_rewrites_in_expr(module, *else_branch, owner, rewrites);
            }
        }
        ast::Expr::Match {
            scrutinee,
            arms,
            else_branch,
            ..
        } => {
            collect_handler_rewrites_in_expr(module, *scrutinee, owner, rewrites);
            for arm in arms {
                for pattern in &arm.patterns {
                    collect_handler_rewrites_in_expr(module, *pattern, owner, rewrites);
                }
                collect_handler_rewrites_in_expr(module, arm.body, owner, rewrites);
            }
            if let Some(else_branch) = else_branch {
                collect_handler_rewrites_in_expr(module, *else_branch, owner, rewrites);
            }
        }
        ast::Expr::Let { value, body, .. } => {
            collect_handler_rewrites_in_expr(module, *value, owner, rewrites);
            collect_handler_rewrites_in_expr(module, *body, owner, rewrites);
        }
        ast::Expr::Block { stmts, expr, .. } => {
            for stmt in stmts {
                match stmt {
                    ast::Stmt::Let { init, .. } => {
                        collect_handler_rewrites_in_expr(module, *init, owner, rewrites);
                    }
                    ast::Stmt::Expr(expr, _) => {
                        collect_handler_rewrites_in_expr(module, *expr, owner, rewrites);
                    }
                }
            }
            if let Some(expr) = expr {
                collect_handler_rewrites_in_expr(module, *expr, owner, rewrites);
            }
        }
        ast::Expr::Array { elements, .. } => {
            for element in elements {
                collect_handler_rewrites_in_expr(module, *element, owner, rewrites);
            }
        }
        ast::Expr::Index { base, index, .. } => {
            collect_handler_rewrites_in_expr(module, *base, owner, rewrites);
            collect_handler_rewrites_in_expr(module, *index, owner, rewrites);
        }
        ast::Expr::Member { base, .. } => {
            collect_handler_rewrites_in_expr(module, *base, owner, rewrites);
        }
        ast::Expr::RecordLiteral { properties, .. } => {
            for property in properties {
                collect_handler_rewrites_in_expr(module, property.value, owner, rewrites);
            }
        }
        ast::Expr::Element { element, .. } => {
            collect_handler_rewrites_in_element(module, *element, owner, rewrites);
        }
        ast::Expr::ActionHandler { body, .. } => {
            collect_handler_rewrites_in_expr(module, *body, owner, rewrites);
        }
        ast::Expr::For { iterable, body, .. } => {
            collect_handler_rewrites_in_expr(module, *iterable, owner, rewrites);
            collect_handler_rewrites_in_expr(module, *body, owner, rewrites);
        }
    }
}

fn collect_handler_rewrites_in_element(
    module: &PreparedModule,
    element_id: ElementId,
    owner: Option<&Name>,
    rewrites: &mut Vec<PendingHandlerRewrite>,
) {
    let element = module.raw_module().element(element_id);

    if let Ok(Some(contract)) = effective_component_contract_for_name(module, &element.tag) {
        collect_handler_rewrites_in_property_entries(
            module,
            element_id,
            element.property_entries(),
            &contract,
            owner,
            rewrites,
        );
    }

    for content in &element.content {
        collect_handler_rewrites_in_expr(module, *content, owner, rewrites);
    }
}

fn collect_handler_rewrites_in_property_entries(
    module: &PreparedModule,
    element_id: ElementId,
    entries: &[PropertyEntry],
    contract: &EffectiveComponentContract,
    owner: Option<&Name>,
    rewrites: &mut Vec<PendingHandlerRewrite>,
) {
    for entry in entries {
        match entry {
            PropertyEntry::Value(property) => {
                if !contract
                    .props
                    .iter()
                    .any(|field| field.name == property.key)
                {
                    let prop_name = property.key.as_str();
                    if is_handler_binding_candidate(prop_name) {
                        if let Some(emit) = contract
                            .emits
                            .iter()
                            .find(|emit| handler_prop_name(emit.emit.name.as_str()) == prop_name)
                        {
                            if !matches!(
                                module.raw_module().expr(property.value),
                                ast::Expr::ActionHandler { .. }
                            ) {
                                rewrites.push(PendingHandlerRewrite {
                                    element: element_id,
                                    property_span: property.span,
                                    component: contract.component.name.clone(),
                                    emit: emit.emit.name.clone(),
                                    action_name: emit.emit.action_name.clone(),
                                    action_module_identity: emit.module_identity.clone(),
                                    owner: owner.cloned(),
                                    span: property.span,
                                    body: property.value,
                                });
                            }
                        }
                    }
                }

                collect_handler_rewrites_in_expr(module, property.value, owner, rewrites);
            }
            PropertyEntry::If {
                condition,
                then_entries,
                else_entries,
                ..
            } => {
                collect_handler_rewrites_in_expr(module, *condition, owner, rewrites);
                collect_handler_rewrites_in_property_entries(
                    module,
                    element_id,
                    then_entries,
                    contract,
                    owner,
                    rewrites,
                );
                collect_handler_rewrites_in_property_entries(
                    module,
                    element_id,
                    else_entries,
                    contract,
                    owner,
                    rewrites,
                );
            }
            PropertyEntry::ConditionList {
                arms, else_entries, ..
            } => {
                for arm in arms {
                    collect_handler_rewrites_in_expr(module, arm.condition, owner, rewrites);
                    collect_handler_rewrites_in_property_entries(
                        module,
                        element_id,
                        &arm.entries,
                        contract,
                        owner,
                        rewrites,
                    );
                }
                collect_handler_rewrites_in_property_entries(
                    module,
                    element_id,
                    else_entries,
                    contract,
                    owner,
                    rewrites,
                );
            }
            PropertyEntry::Match {
                scrutinee,
                arms,
                else_entries,
                ..
            } => {
                collect_handler_rewrites_in_expr(module, *scrutinee, owner, rewrites);
                for arm in arms {
                    for pattern in &arm.patterns {
                        collect_handler_rewrites_in_expr(module, *pattern, owner, rewrites);
                    }
                    collect_handler_rewrites_in_property_entries(
                        module,
                        element_id,
                        &arm.entries,
                        contract,
                        owner,
                        rewrites,
                    );
                }
                collect_handler_rewrites_in_property_entries(
                    module,
                    element_id,
                    else_entries,
                    contract,
                    owner,
                    rewrites,
                );
            }
        }
    }
}

fn rewrite_property_handler(element: &mut Element, property_span: TextSpan, handler: ExprId) {
    for property in &mut element.properties {
        if property.span == property_span {
            property.value = handler;
        }
    }
    rewrite_property_entry_handler(&mut element.property_entries, property_span, handler);
}

fn rewrite_property_entry_handler(
    entries: &mut [PropertyEntry],
    property_span: TextSpan,
    handler: ExprId,
) -> bool {
    for entry in entries {
        match entry {
            PropertyEntry::Value(property) if property.span == property_span => {
                property.value = handler;
                return true;
            }
            PropertyEntry::Value(_) => {}
            PropertyEntry::If {
                then_entries,
                else_entries,
                ..
            } => {
                if rewrite_property_entry_handler(then_entries, property_span, handler)
                    || rewrite_property_entry_handler(else_entries, property_span, handler)
                {
                    return true;
                }
            }
            PropertyEntry::ConditionList {
                arms, else_entries, ..
            } => {
                for arm in arms {
                    if rewrite_property_entry_handler(&mut arm.entries, property_span, handler) {
                        return true;
                    }
                }
                if rewrite_property_entry_handler(else_entries, property_span, handler) {
                    return true;
                }
            }
            PropertyEntry::Match {
                arms, else_entries, ..
            } => {
                for arm in arms {
                    if rewrite_property_entry_handler(&mut arm.entries, property_span, handler) {
                        return true;
                    }
                }
                if rewrite_property_entry_handler(else_entries, property_span, handler) {
                    return true;
                }
            }
        }
    }

    false
}

/// Walks one component's inheritance chain, reporting the first thing wrong with it.
///
/// <para>Keyed by declaration rather than by spelling, and each base resolved where its `extends`
/// clause was written, for the same reason as the record walk: a component extending a same-named
/// component in another module is not a cycle.</para>
fn validate_component_definition(
    module: &PreparedModule,
    component: &ResolvedComponentDefinition,
    statuses: &mut FxHashMap<DeclarationKey, ComponentValidationStatus>,
    stack: &mut Vec<(DeclarationKey, Name)>,
    errors: &mut Vec<ComponentResolutionError>,
) -> ComponentValidationStatus {
    let key = component.key();
    if let Some(status) = statuses.get(&key) {
        return *status;
    }

    if let Some(index) = stack.iter().position(|(seen, _)| *seen == key) {
        let mut cycle: Vec<Name> = stack[index..]
            .iter()
            .map(|(_, name)| name.clone())
            .collect();
        cycle.push(component.component.name.clone());
        push_unique_component_error(
            errors,
            ComponentResolutionError::InheritanceCycle {
                component: component.component.name.clone(),
                span: component.component.span,
                cycle,
            },
        );

        for (seen, _) in &stack[index..] {
            statuses.insert(seen.clone(), ComponentValidationStatus::Invalid);
        }

        return ComponentValidationStatus::Invalid;
    }

    stack.push((key.clone(), component.component.name.clone()));

    let status =
        match resolve_base_component(module, &component.module_identity, &component.component) {
            Ok(Some(base_component)) => {
                if validate_component_definition(module, &base_component, statuses, stack, errors)
                    == ComponentValidationStatus::Invalid
                {
                    ComponentValidationStatus::Invalid
                } else {
                    validate_component_contract(module, component, errors)
                }
            }
            Ok(None) => validate_component_contract(module, component, errors),
            Err(error) => {
                push_unique_component_error(errors, error);
                ComponentValidationStatus::Invalid
            }
        };

    stack.pop();
    statuses.insert(key, status);
    status
}

fn validate_component_contract(
    module: &PreparedModule,
    component: &ResolvedComponentDefinition,
    errors: &mut Vec<ComponentResolutionError>,
) -> ComponentValidationStatus {
    match effective_component_contract_resolved(module, component) {
        Ok(_) => ComponentValidationStatus::Valid,
        Err(error) => {
            push_unique_component_error(errors, error);
            ComponentValidationStatus::Invalid
        }
    }
}

fn push_unique_component_error(
    errors: &mut Vec<ComponentResolutionError>,
    error: ComponentResolutionError,
) {
    if !errors.contains(&error) {
        errors.push(error);
    }
}

fn resolve_component_contract_inner(
    module: &PreparedModule,
    component: &ResolvedComponentDefinition,
    stack: &mut Vec<(DeclarationKey, Name)>,
) -> Result<ResolvedComponentContract, ComponentResolutionError> {
    let key = component.key();
    if let Some(index) = stack.iter().position(|(seen, _)| *seen == key) {
        let mut cycle: Vec<Name> = stack[index..]
            .iter()
            .map(|(_, name)| name.clone())
            .collect();
        cycle.push(component.component.name.clone());
        return Err(ComponentResolutionError::InheritanceCycle {
            component: component.component.name.clone(),
            span: component.component.span,
            cycle,
        });
    }

    stack.push((key, component.component.name.clone()));

    let result = if let Some(base_component) =
        resolve_base_component(module, &component.module_identity, &component.component)?
    {
        let base_contract = resolve_component_contract_inner(module, &base_component, stack)?;
        let mut type_params = base_contract.type_params;
        let mut props = base_contract.props;
        let mut emits = base_contract.emits;
        let declared_props = component.declared_props();

        for owned in component.declared_type_params() {
            if let Some(existing) = type_params
                .iter()
                .find(|existing| existing.param.name == owned.param.name)
            {
                stack.pop();
                return Err(ComponentResolutionError::DuplicateInheritedTypeParameter {
                    component: component.component.name.clone(),
                    name: owned.param.name.clone(),
                    inherited_from: existing.owner.clone(),
                    span: owned.param.span,
                });
            }
            if let Some(existing) = props
                .iter()
                .find(|existing| existing.field.name == owned.param.name)
            {
                stack.pop();
                return Err(ComponentResolutionError::DuplicateInheritedProp {
                    component: component.component.name.clone(),
                    prop: owned.param.name.clone(),
                    inherited_from: existing.owner.clone(),
                    span: owned.param.span,
                });
            }
            type_params.push(owned);
        }

        for field in &declared_props {
            if let Some(existing) = type_params
                .iter()
                .find(|existing| existing.param.name == field.name)
            {
                stack.pop();
                return Err(ComponentResolutionError::DuplicateInheritedTypeParameter {
                    component: component.component.name.clone(),
                    name: field.name.clone(),
                    inherited_from: existing.owner.clone(),
                    span: field.span,
                });
            }

            if field.is_content {
                if let Some(existing) = props.iter().find(|existing| existing.field.is_content) {
                    stack.pop();
                    return Err(ComponentResolutionError::DuplicateContentProperty {
                        component: component.component.name.clone(),
                        existing_prop: existing.field.name.clone(),
                        existing_owner: existing.owner.clone(),
                        prop: field.name.clone(),
                        span: field.span,
                    });
                }
            }

            if let Some(existing) = props
                .iter()
                .find(|existing| existing.field.name == field.name)
            {
                stack.pop();
                return Err(ComponentResolutionError::DuplicateInheritedProp {
                    component: component.component.name.clone(),
                    prop: field.name.clone(),
                    inherited_from: existing.owner.clone(),
                    span: field.span,
                });
            }

            if let Some(existing) = emits.iter().find(|existing| {
                handler_prop_name(existing.emit.name.as_str()) == field.name.as_str()
            }) {
                stack.pop();
                return Err(ComponentResolutionError::HandlerNameCollision {
                    component: component.component.name.clone(),
                    prop: field.name.clone(),
                    emit: existing.emit.name.clone(),
                    span: field.span,
                });
            }

            props.push(OwnedRecordField {
                field: field.clone(),
                owner: component.component.name.clone(),
            });
        }

        for emit in &component.component.emits {
            if let Some(existing) = emits
                .iter()
                .find(|existing| existing.emit.name == emit.name)
            {
                stack.pop();
                return Err(ComponentResolutionError::DuplicateInheritedEmit {
                    component: component.component.name.clone(),
                    emit: emit.name.clone(),
                    inherited_from: existing.owner.clone(),
                    span: emit.span,
                });
            }

            let handler_name = handler_prop_name(emit.name.as_str());
            if props
                .iter()
                .any(|existing| existing.field.name.as_str() == handler_name)
            {
                stack.pop();
                return Err(ComponentResolutionError::HandlerNameCollision {
                    component: component.component.name.clone(),
                    prop: Name::new(&handler_name),
                    emit: emit.name.clone(),
                    span: emit.span,
                });
            }

            emits.push(OwnedComponentEmit {
                emit: emit.clone(),
                owner: component.component.name.clone(),
                module_identity: component.module_identity.clone(),
            });
        }

        if let Err(error) = check_emit_payloads_for_type_parameters(module, component, &type_params)
        {
            stack.pop();
            return Err(error);
        }

        let mut ancestors = vec![ComponentAncestor {
            name: base_component.component.name.clone(),
            origin: base_component.origin(),
        }];
        ancestors.extend(base_contract.ancestors);

        ResolvedComponentContract {
            component: component.component.clone(),
            type_params,
            props,
            emits,
            ancestors,
        }
    } else {
        let declared_props = component.declared_props();
        let props = declared_props
            .iter()
            .cloned()
            .map(|field| OwnedRecordField {
                field,
                owner: component.component.name.clone(),
            })
            .collect::<Vec<_>>();
        let emits = component
            .component
            .emits
            .iter()
            .cloned()
            .map(|emit| OwnedComponentEmit {
                emit,
                owner: component.component.name.clone(),
                module_identity: component.module_identity.clone(),
            })
            .collect::<Vec<_>>();

        // Validate local prop/emitted-action handler-name collisions even without inheritance.
        for field in &declared_props {
            if let Some(existing) = emits
                .iter()
                .find(|emit| handler_prop_name(emit.emit.name.as_str()) == field.name.as_str())
            {
                stack.pop();
                return Err(ComponentResolutionError::HandlerNameCollision {
                    component: component.component.name.clone(),
                    prop: field.name.clone(),
                    emit: existing.emit.name.clone(),
                    span: field.span,
                });
            }
        }

        let type_params = component.declared_type_params();
        if let Err(error) = check_emit_payloads_for_type_parameters(module, component, &type_params)
        {
            stack.pop();
            return Err(error);
        }

        ResolvedComponentContract {
            component: component.component.clone(),
            type_params,
            props,
            emits,
            ancestors: Vec::new(),
        }
    };

    stack.pop();
    Ok(result)
}

/// Rejects an inline emitted action whose payload field is typed by one of the component's
/// effective type parameters, inherited ones included.
///
/// <para>The payload lowers to an action record of its own, checked and generated outside the
/// component, where the parameter is not a type. The check runs where the component's own module
/// is the resolving one, which is where its inline action records live; a component resolved from
/// another module was checked when its own module was.</para>
fn check_emit_payloads_for_type_parameters(
    module: &PreparedModule,
    component: &ResolvedComponentDefinition,
    type_params: &[OwnedTypeParameter],
) -> Result<(), ComponentResolutionError> {
    if type_params.is_empty() || component.module_identity != module.module_identity() {
        return Ok(());
    }
    for emit in &component.component.emits {
        if emit.kind != ComponentEmitKind::Inline {
            continue;
        }
        let Some(Item::Record(record)) = module.raw_module().find_item(emit.action_name.as_str())
        else {
            continue;
        };
        for field in &record.properties {
            let parameter = crate::type_ref_names(&field.ty)
                .into_iter()
                .find(|named| type_params.iter().any(|param| param.param.name == **named));
            if let Some(parameter) = parameter {
                return Err(ComponentResolutionError::TypeParameterInEmitPayload {
                    component: component.component.name.clone(),
                    emit: emit.name.clone(),
                    field: field.name.clone(),
                    parameter: parameter.clone(),
                    span: field.span,
                });
            }
        }
    }
    Ok(())
}

/// Resolves a component's base in the namespace of the module that wrote the `extends` clause.
fn resolve_base_component(
    module: &PreparedModule,
    namespace_module: &str,
    component: &Component,
) -> Result<Option<ResolvedComponentDefinition>, ComponentResolutionError> {
    let Some(base_name) = component.base.as_ref() else {
        return Ok(None);
    };

    let resolved =
        module.resolve_in_module(PreparedNamespace::Element, namespace_module, base_name);

    match resolved {
        Some(resolved) => {
            let Some(base_component) = component_definition_from_prepared_item(module, resolved)
            else {
                return Err(invalid_base(
                    component,
                    base_name,
                    InvalidComponentBaseReason::NotComponent,
                ));
            };
            validate_base_component(component, base_name, &base_component).map(Some)
        }
        None => Err(invalid_base(
            component,
            base_name,
            InvalidComponentBaseReason::NotFound,
        )),
    }
}

fn invalid_base(
    component: &Component,
    base_name: &Name,
    reason: InvalidComponentBaseReason,
) -> ComponentResolutionError {
    ComponentResolutionError::InvalidBase {
        component: component.name.clone(),
        base: component.base.clone().unwrap_or_else(|| base_name.clone()),
        span: component.span,
        reason,
    }
}

fn is_handler_binding_candidate(prop_name: &str) -> bool {
    if !prop_name.starts_with("on") || prop_name.len() <= 2 {
        return false;
    }

    prop_name
        .as_bytes()
        .get(2)
        .map(|ch| ch.is_ascii_uppercase())
        .unwrap_or(false)
}

fn missing_emit_handler_message(component: &Name, emit: &Name) -> String {
    let handler = handler_prop_name(emit.as_str());
    format!(
        "Component '{}' does not emit '{}' required by handler '{}'",
        component, emit, handler
    )
}

fn validate_base_component(
    component: &Component,
    base_name: &Name,
    base_component: &ResolvedComponentDefinition,
) -> Result<ResolvedComponentDefinition, ComponentResolutionError> {
    if !base_component.component.is_abstract {
        return Err(invalid_base(
            component,
            base_name,
            InvalidComponentBaseReason::ConcreteComponent,
        ));
    }

    Ok(base_component.clone())
}

fn handler_prop_name(emit_name: &str) -> String {
    format!("on{}", emit_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lower, SourceId};
    use nx_syntax::parse_str;

    #[test]
    fn effective_external_component_contract_excludes_declared_state_fields() {
        let source = r#"
            abstract external component <SearchBase placeholder:string />
            external component <SearchBox extends SearchBase showSearchIcon:boolean = true /> = {
              state { query:string }
            }
        "#;

        let parse_result = parse_str(source, "component-contract.nx");
        let tree = parse_result
            .tree
            .expect("Expected component contract source to parse");
        let lowered = lower(tree.root(), SourceId::new(0));
        let prepared = PreparedModule::standalone("component-contract.nx", lowered);

        let contract = effective_component_contract_for_name(&prepared, &Name::new("SearchBox"))
            .expect("Expected component contract resolution to succeed")
            .expect("Expected resolved SearchBox contract");

        let prop_names = contract
            .props
            .iter()
            .map(|field| field.name.as_str().to_string())
            .collect::<Vec<_>>();
        assert_eq!(prop_names, vec!["placeholder", "showSearchIcon"]);
        assert!(
            contract
                .props
                .iter()
                .all(|field| field.name.as_str() != "query"),
            "Declared external state must not become part of the effective prop contract"
        );
        assert_eq!(contract.component.state.len(), 1);
        assert_eq!(contract.component.state[0].name.as_str(), "query");
        assert!(contract.component.body.is_none());
    }

    fn prepared(source: &str) -> PreparedModule {
        let parse_result = parse_str(source, "component-contract.nx");
        let tree = parse_result.tree.expect("Expected source to parse");
        let lowered = lower(tree.root(), SourceId::new(0));
        PreparedModule::standalone("component-contract.nx", lowered)
    }

    #[test]
    fn derived_component_inherits_type_parameters_ahead_of_its_own() {
        let prepared = prepared(
            r#"
            abstract component <ItemsBase TItem:type items:TItem[] />
            component <Keyed extends ItemsBase TKey:type keys:TKey[] /> = { <Label /> }
        "#,
        );

        let contract = effective_component_contract_for_name(&prepared, &Name::new("Keyed"))
            .expect("Expected component contract resolution to succeed")
            .expect("Expected resolved Keyed contract");

        assert_eq!(
            contract
                .type_params
                .iter()
                .map(|param| param.name.as_str())
                .collect::<Vec<_>>(),
            vec!["TItem", "TKey"]
        );
        assert!(contract
            .type_params
            .iter()
            .all(|param| param.module_identity == prepared.module_identity()));
        assert_eq!(
            contract
                .props
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            vec!["items", "keys"]
        );
    }

    /// The DrawnUI catalog the fiddle generates leans on this: an event declared on a base class is
    /// stated once, on the component for that class, and never restated on the controls below it.
    #[test]
    fn redeclaring_an_inherited_emit_is_rejected() {
        let prepared = prepared(
            r#"
            abstract component <ToggleBase emits { Toggled { value:boolean } } />
            component <Bad extends ToggleBase emits { Toggled { value:boolean } } /> = { <Label /> }
        "#,
        );

        let messages = validate_component_definitions(&prepared)
            .into_iter()
            .map(|error| error.message())
            .collect::<Vec<_>>();
        assert_eq!(
            messages,
            vec![
                "Component 'Bad' redeclares inherited emitted action 'Toggled' from 'ToggleBase'"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn redeclaring_an_inherited_type_parameter_is_rejected() {
        let prepared = prepared(
            r#"
            abstract component <ItemsBase TItem:type />
            component <Bad extends ItemsBase TItem:type /> = { <Label /> }
            component <Worse extends ItemsBase TItem:string /> = { <Label /> }
        "#,
        );

        let messages = validate_component_definitions(&prepared)
            .into_iter()
            .map(|error| error.message())
            .collect::<Vec<_>>();
        assert_eq!(
            messages,
            vec![
                "Component 'Bad' redeclares inherited type parameter 'TItem' from 'ItemsBase'"
                    .to_string(),
                "Component 'Worse' redeclares inherited type parameter 'TItem' from 'ItemsBase'"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn erase_type_parameters_replaces_parameter_names_under_suffixes() {
        let params = [Name::new("TItem")];
        let erased = erase_type_parameters(
            &ast::TypeRef::nullable(ast::TypeRef::array(ast::TypeRef::name("TItem"))),
            &params,
        );
        assert_eq!(
            erased,
            ast::TypeRef::nullable(ast::TypeRef::array(ast::TypeRef::name("object")))
        );
        assert_eq!(
            erase_type_parameters(&ast::TypeRef::name("Contact"), &params),
            ast::TypeRef::name("Contact")
        );
    }

    #[test]
    fn remove_property_entries_drops_plain_entries_by_value_expression() {
        let mut prepared = prepared(
            r#"
            external component <List TItem:type items:string[]? />
            let v = <List TItem=string items={} />
        "#,
        );

        let (element_id, argument) = prepared
            .raw_module()
            .exprs()
            .find_map(|(_, expr)| match expr {
                ast::Expr::Element {
                    element: element_id,
                    ..
                } => {
                    let element = prepared.raw_module().element(*element_id);
                    element
                        .properties
                        .iter()
                        .find(|property| property.key.as_str() == "TItem")
                        .map(|property| (*element_id, property.value))
                }
                _ => None,
            })
            .expect("Expected the List element with a TItem binding");

        let consumed = FxHashSet::from_iter([argument]);
        remove_property_entries(&mut prepared, &consumed);

        let element = prepared.raw_module().element(element_id);
        assert_eq!(
            element
                .properties
                .iter()
                .map(|property| property.key.as_str())
                .collect::<Vec<_>>(),
            vec!["items"]
        );
        assert_eq!(element.property_entries.len(), 1);
        assert!(matches!(
            &element.property_entries[0],
            PropertyEntry::Value(property) if property.key.as_str() == "items"
        ));
    }
}
