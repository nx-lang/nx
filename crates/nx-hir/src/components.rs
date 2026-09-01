use crate::{
    ast, interface_component, same_declaration, Component, ComponentEmit, DeclarationKey,
    DeclaringOrigin, EffectiveEmit, EffectiveField, Element, ElementId, ExprId, InterfaceField,
    InterfaceItem, InterfaceItemKind, Item, LocalDefinitionId, Name, PreparedModule,
    PreparedNamespace, PropertyEntry, ResolvedPreparedItem,
};
use nx_diagnostics::TextSpan;
use rustc_hash::FxHashMap;

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
    DuplicateContentProperty {
        component: Name,
        existing_prop: Name,
        existing_owner: Name,
        prop: Name,
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
            ComponentResolutionError::DuplicateContentProperty { .. } => {
                "component-duplicate-content-prop"
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
            | ComponentResolutionError::DuplicateContentProperty { span, .. }
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
struct ResolvedComponentContract {
    component: Component,
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
    match item {
        Item::Function(function) => {
            collect_handler_rewrites_in_expr(module, function.body, rewrites)
        }
        Item::Value(value) => collect_handler_rewrites_in_expr(module, value.value, rewrites),
        Item::Component(component) => {
            for field in &component.props {
                if let Some(default) = field.default {
                    collect_handler_rewrites_in_expr(module, default, rewrites);
                }
            }
            for field in &component.state {
                if let Some(default) = field.default {
                    collect_handler_rewrites_in_expr(module, default, rewrites);
                }
            }
            if let Some(body) = component.body {
                collect_handler_rewrites_in_expr(module, body, rewrites);
            }
        }
        Item::Record(record) => {
            for field in &record.properties {
                if let Some(default) = field.default {
                    collect_handler_rewrites_in_expr(module, default, rewrites);
                }
            }
        }
        Item::Union(union_def) => {
            for case in &union_def.cases {
                for field in &case.fields {
                    if let Some(default) = field.default {
                        collect_handler_rewrites_in_expr(module, default, rewrites);
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
        ast::Expr::BinaryOp { lhs, rhs, .. } => {
            collect_handler_rewrites_in_expr(module, *lhs, rewrites);
            collect_handler_rewrites_in_expr(module, *rhs, rewrites);
        }
        ast::Expr::UnaryOp { expr, .. } => {
            collect_handler_rewrites_in_expr(module, *expr, rewrites);
        }
        ast::Expr::Call { func, args, .. } => {
            collect_handler_rewrites_in_expr(module, *func, rewrites);
            for arg in args {
                collect_handler_rewrites_in_expr(module, *arg, rewrites);
            }
        }
        ast::Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            collect_handler_rewrites_in_expr(module, *condition, rewrites);
            collect_handler_rewrites_in_expr(module, *then_branch, rewrites);
            if let Some(else_branch) = else_branch {
                collect_handler_rewrites_in_expr(module, *else_branch, rewrites);
            }
        }
        ast::Expr::Match {
            scrutinee,
            arms,
            else_branch,
            ..
        } => {
            collect_handler_rewrites_in_expr(module, *scrutinee, rewrites);
            for arm in arms {
                for pattern in &arm.patterns {
                    collect_handler_rewrites_in_expr(module, *pattern, rewrites);
                }
                collect_handler_rewrites_in_expr(module, arm.body, rewrites);
            }
            if let Some(else_branch) = else_branch {
                collect_handler_rewrites_in_expr(module, *else_branch, rewrites);
            }
        }
        ast::Expr::Let { value, body, .. } => {
            collect_handler_rewrites_in_expr(module, *value, rewrites);
            collect_handler_rewrites_in_expr(module, *body, rewrites);
        }
        ast::Expr::Block { stmts, expr, .. } => {
            for stmt in stmts {
                match stmt {
                    ast::Stmt::Let { init, .. } => {
                        collect_handler_rewrites_in_expr(module, *init, rewrites);
                    }
                    ast::Stmt::Expr(expr, _) => {
                        collect_handler_rewrites_in_expr(module, *expr, rewrites);
                    }
                }
            }
            if let Some(expr) = expr {
                collect_handler_rewrites_in_expr(module, *expr, rewrites);
            }
        }
        ast::Expr::Array { elements, .. } => {
            for element in elements {
                collect_handler_rewrites_in_expr(module, *element, rewrites);
            }
        }
        ast::Expr::Index { base, index, .. } => {
            collect_handler_rewrites_in_expr(module, *base, rewrites);
            collect_handler_rewrites_in_expr(module, *index, rewrites);
        }
        ast::Expr::Member { base, .. } => {
            collect_handler_rewrites_in_expr(module, *base, rewrites);
        }
        ast::Expr::RecordLiteral { properties, .. } => {
            for property in properties {
                collect_handler_rewrites_in_expr(module, property.value, rewrites);
            }
        }
        ast::Expr::Element { element, .. } => {
            collect_handler_rewrites_in_element(module, *element, rewrites);
        }
        ast::Expr::ActionHandler { body, .. } => {
            collect_handler_rewrites_in_expr(module, *body, rewrites);
        }
        ast::Expr::For { iterable, body, .. } => {
            collect_handler_rewrites_in_expr(module, *iterable, rewrites);
            collect_handler_rewrites_in_expr(module, *body, rewrites);
        }
    }
}

fn collect_handler_rewrites_in_element(
    module: &PreparedModule,
    element_id: ElementId,
    rewrites: &mut Vec<PendingHandlerRewrite>,
) {
    let element = module.raw_module().element(element_id);

    if let Ok(Some(contract)) = effective_component_contract_for_name(module, &element.tag) {
        collect_handler_rewrites_in_property_entries(
            module,
            element_id,
            element.property_entries(),
            &contract,
            rewrites,
        );
    }

    for content in &element.content {
        collect_handler_rewrites_in_expr(module, *content, rewrites);
    }
}

fn collect_handler_rewrites_in_property_entries(
    module: &PreparedModule,
    element_id: ElementId,
    entries: &[PropertyEntry],
    contract: &EffectiveComponentContract,
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
                                    span: property.span,
                                    body: property.value,
                                });
                            }
                        }
                    }
                }

                collect_handler_rewrites_in_expr(module, property.value, rewrites);
            }
            PropertyEntry::If {
                condition,
                then_entries,
                else_entries,
                ..
            } => {
                collect_handler_rewrites_in_expr(module, *condition, rewrites);
                collect_handler_rewrites_in_property_entries(
                    module,
                    element_id,
                    then_entries,
                    contract,
                    rewrites,
                );
                collect_handler_rewrites_in_property_entries(
                    module,
                    element_id,
                    else_entries,
                    contract,
                    rewrites,
                );
            }
            PropertyEntry::ConditionList {
                arms, else_entries, ..
            } => {
                for arm in arms {
                    collect_handler_rewrites_in_expr(module, arm.condition, rewrites);
                    collect_handler_rewrites_in_property_entries(
                        module,
                        element_id,
                        &arm.entries,
                        contract,
                        rewrites,
                    );
                }
                collect_handler_rewrites_in_property_entries(
                    module,
                    element_id,
                    else_entries,
                    contract,
                    rewrites,
                );
            }
            PropertyEntry::Match {
                scrutinee,
                arms,
                else_entries,
                ..
            } => {
                collect_handler_rewrites_in_expr(module, *scrutinee, rewrites);
                for arm in arms {
                    for pattern in &arm.patterns {
                        collect_handler_rewrites_in_expr(module, *pattern, rewrites);
                    }
                    collect_handler_rewrites_in_property_entries(
                        module,
                        element_id,
                        &arm.entries,
                        contract,
                        rewrites,
                    );
                }
                collect_handler_rewrites_in_property_entries(
                    module,
                    element_id,
                    else_entries,
                    contract,
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
        let mut props = base_contract.props;
        let mut emits = base_contract.emits;
        let declared_props = component.declared_props();

        for field in &declared_props {
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

        let mut ancestors = vec![ComponentAncestor {
            name: base_component.component.name.clone(),
            origin: base_component.origin(),
        }];
        ancestors.extend(base_contract.ancestors);

        ResolvedComponentContract {
            component: component.component.clone(),
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

        ResolvedComponentContract {
            component: component.component.clone(),
            props,
            emits,
            ancestors: Vec::new(),
        }
    };

    stack.pop();
    Ok(result)
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
}
