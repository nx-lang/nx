use crate::{
    ast, interface_component, Component, ComponentEmit, EffectiveField, ElementId, ExprId,
    InterfaceField, InterfaceItem, InterfaceItemKind, Item, Name, PreparedModule,
    PreparedNamespace, ResolvedPreparedItem,
};
use nx_diagnostics::TextSpan;
use rustc_hash::FxHashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveComponentContract {
    pub component: Component,
    pub props: Vec<EffectiveField>,
    pub emits: Vec<ComponentEmit>,
    pub ancestors: Vec<Name>,
}

impl EffectiveComponentContract {
    pub fn content_prop(&self) -> Option<&EffectiveField> {
        self.props.iter().find(|field| field.is_content)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingHandlerRewrite {
    element: ElementId,
    property_index: usize,
    component: Name,
    emit: Name,
    action_name: Name,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedComponentContract {
    component: Component,
    props: Vec<OwnedRecordField>,
    emits: Vec<OwnedComponentEmit>,
    ancestors: Vec<Name>,
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
    module_identity: String,
    field_source: ComponentFieldSource,
}

impl ResolvedComponentDefinition {
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
    resolve_component_definition_with_identity(module, name).map(|resolved| resolved.component)
}

pub fn effective_component_contract_for_name(
    module: &PreparedModule,
    name: &Name,
) -> Result<Option<EffectiveComponentContract>, ComponentResolutionError> {
    let Some(component) = resolve_component_definition_with_identity(module, name) else {
        return Ok(None);
    };

    effective_component_contract_resolved(module, &component).map(Some)
}

pub fn effective_component_contract(
    module: &PreparedModule,
    component: &Component,
) -> Result<EffectiveComponentContract, ComponentResolutionError> {
    let component = ResolvedComponentDefinition {
        component: component.clone(),
        module_identity: module.module_identity().to_string(),
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
        emits: resolved.emits.into_iter().map(|emit| emit.emit).collect(),
        ancestors: resolved.ancestors,
    })
}

fn resolve_component_definition_with_identity(
    module: &PreparedModule,
    name: &Name,
) -> Option<ResolvedComponentDefinition> {
    let resolved = module
        .resolve_binding(PreparedNamespace::Element, name)
        .and_then(|binding| module.resolve_prepared_item(binding))?;
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
            item: Item::Component(component),
            ..
        } => Some(ResolvedComponentDefinition {
            component,
            module_identity,
            field_source: ComponentFieldSource::Raw,
        }),
        ResolvedPreparedItem::Imported { item, raw, .. } => {
            if let Some(raw_ref) = raw.as_ref() {
                if let Some(Item::Component(component)) = module.resolve_imported_raw_item(raw_ref)
                {
                    return Some(ResolvedComponentDefinition {
                        component,
                        module_identity: raw_ref.module_identity.clone(),
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

    for component in module
        .raw_module()
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::Component(component) => Some(component),
            _ => None,
        })
    {
        validate_component_definition(module, component, &mut statuses, &mut stack, &mut errors);
    }

    errors
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
            body: rewrite.body,
            span: rewrite.span,
        });
        raw_module.element_mut(rewrite.element).properties[rewrite.property_index].value = handler;
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
        Item::TypeAlias(_) | Item::Enum(_) => {}
    }
}

fn collect_handler_rewrites_in_expr(
    module: &PreparedModule,
    expr_id: ExprId,
    rewrites: &mut Vec<PendingHandlerRewrite>,
) {
    match module.raw_module().expr(expr_id) {
        ast::Expr::Literal(_) | ast::Expr::Ident(_) | ast::Expr::Error(_) => {}
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
            for (_, value) in properties {
                collect_handler_rewrites_in_expr(module, *value, rewrites);
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
        for (property_index, property) in element.properties.iter().enumerate() {
            if contract
                .props
                .iter()
                .any(|field| field.name == property.key)
            {
                continue;
            }

            let prop_name = property.key.as_str();
            if !is_handler_binding_candidate(prop_name) {
                continue;
            }

            let Some(emit) = contract
                .emits
                .iter()
                .find(|emit| handler_prop_name(emit.name.as_str()) == prop_name)
            else {
                continue;
            };

            if matches!(
                module.raw_module().expr(property.value),
                ast::Expr::ActionHandler { .. }
            ) {
                continue;
            }

            rewrites.push(PendingHandlerRewrite {
                element: element_id,
                property_index,
                component: contract.component.name.clone(),
                emit: emit.name.clone(),
                action_name: emit.action_name.clone(),
                span: property.span,
                body: property.value,
            });
        }
    }

    for property in &element.properties {
        collect_handler_rewrites_in_expr(module, property.value, rewrites);
    }
    for content in &element.content {
        collect_handler_rewrites_in_expr(module, *content, rewrites);
    }
}

fn validate_component_definition(
    module: &PreparedModule,
    component: &Component,
    statuses: &mut FxHashMap<Name, ComponentValidationStatus>,
    stack: &mut Vec<Name>,
    errors: &mut Vec<ComponentResolutionError>,
) -> ComponentValidationStatus {
    if let Some(status) = statuses.get(&component.name) {
        return *status;
    }

    if let Some(index) = stack.iter().position(|name| name == &component.name) {
        let mut cycle = stack[index..].to_vec();
        cycle.push(component.name.clone());
        push_unique_component_error(
            errors,
            ComponentResolutionError::InheritanceCycle {
                component: component.name.clone(),
                span: component.span,
                cycle: cycle.clone(),
            },
        );

        for name in cycle {
            statuses.insert(name, ComponentValidationStatus::Invalid);
        }

        return ComponentValidationStatus::Invalid;
    }

    stack.push(component.name.clone());

    let status = match resolve_base_component(module, component) {
        Ok(Some(base_component)) => {
            if validate_component_definition(
                module,
                &base_component.component,
                statuses,
                stack,
                errors,
            ) == ComponentValidationStatus::Invalid
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
    statuses.insert(component.name.clone(), status);
    status
}

fn validate_component_contract(
    module: &PreparedModule,
    component: &Component,
    errors: &mut Vec<ComponentResolutionError>,
) -> ComponentValidationStatus {
    match effective_component_contract(module, component) {
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
    stack: &mut Vec<Name>,
) -> Result<ResolvedComponentContract, ComponentResolutionError> {
    if let Some(index) = stack
        .iter()
        .position(|name| name == &component.component.name)
    {
        let mut cycle = stack[index..].to_vec();
        cycle.push(component.component.name.clone());
        return Err(ComponentResolutionError::InheritanceCycle {
            component: component.component.name.clone(),
            span: component.component.span,
            cycle,
        });
    }

    stack.push(component.component.name.clone());

    let result = if let Some(base_component) = resolve_base_component(module, &component.component)?
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
            });
        }

        let mut ancestors = vec![base_component.component.name.clone()];
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

fn resolve_base_component(
    module: &PreparedModule,
    component: &Component,
) -> Result<Option<ResolvedComponentDefinition>, ComponentResolutionError> {
    let Some(base_name) = component.base.as_ref() else {
        return Ok(None);
    };

    let resolved = module
        .resolve_binding(PreparedNamespace::Element, base_name)
        .and_then(|binding| module.resolve_prepared_item(binding));

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
            external component <SearchBox extends SearchBase showSearchIcon:bool = true /> = {
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
