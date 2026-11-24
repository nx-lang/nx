//! The `environment-cycle` check: an environment value whose default can read the value itself.
//!
//! <para>A read evaluates a default each time it reaches one, so a default that reads its own
//! value, through other defaults or the functions and values it calls, never finishes. The check
//! follows `env.` reads to the defaults of the values they read, and identifiers to the functions
//! and values they name; it does not follow a component's body, which a descriptor does not run.
//! An entry-module provide is not followed: while one is evaluated for a value, a read of that
//! value sees only the sources after it, so a provide cannot loop.</para>

use crate::ir::{module_identity, visit_declaration_expressions, visit_expression};
use crate::model::{
    CodegenDeclaration, CodegenDeclarationKind, CodegenExpression, CodegenExpressionKind,
    CodegenModule, CodegenReference,
};
use nx_diagnostics::{Diagnostic, Label};
use nx_program::{ResolvedItemKind, RuntimeModuleId};
use rustc_hash::{FxHashMap, FxHashSet};

type Key = (RuntimeModuleId, usize);

fn key(reference: &CodegenReference) -> Key {
    (reference.module_id, reference.definition_id.index())
}

/// One `environment-cycle` error at each environment declaration whose default can read it.
pub(crate) fn environment_cycle_diagnostics(modules: &[CodegenModule]) -> Vec<Diagnostic> {
    let declarations: FxHashMap<Key, (&CodegenModule, &CodegenDeclaration)> = modules
        .iter()
        .flat_map(|module| {
            module
                .declarations
                .iter()
                .map(move |declaration| (key(&declaration.reference), (module, declaration)))
        })
        .collect();
    let mut diagnostics = Vec::new();
    for module in modules {
        for declaration in &module.declarations {
            let CodegenDeclarationKind::Environment { default, .. } = &declaration.kind else {
                continue;
            };
            let target = key(&declaration.reference);
            let mut walk = Walk {
                declarations: &declarations,
                target,
                visited: FxHashSet::default(),
                path: Vec::new(),
            };
            if let Some(path) = walk.expression(default) {
                let name = display_name(&declaration.reference.name);
                let message = if path.is_empty() {
                    format!("The default of environment value '{name}' reads '{name}' itself.")
                } else {
                    format!(
                        "The default of environment value '{name}' reads '{name}' again, through {}.",
                        path.iter()
                            .map(|step| format!("'{step}'"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                diagnostics.push(
                    Diagnostic::error("environment-cycle")
                        .with_message(message)
                        .with_label(Label::primary(module_identity(module), declaration.span))
                        .with_help(
                            "A read evaluates the default each time, so this default never finishes; give it a value that does not depend on itself.",
                        )
                        .build(),
                );
            }
        }
    }
    for module in modules {
        let mut overrides: Vec<(CodegenReference, CodegenExpression)> = module
            .provides
            .iter()
            .filter_map(|provide| Some((provide.target.clone()?, provide.value.clone())))
            .collect();
        let mut collect = |expression: &CodegenExpression| {
            if let CodegenExpressionKind::Provide {
                overrides: list, ..
            }
            | CodegenExpressionKind::EnvUpdate {
                overrides: list, ..
            } = &expression.kind
            {
                for provided in list {
                    if let Some(target) = &provided.target {
                        overrides.push((target.clone(), provided.value.clone()));
                    }
                }
            }
        };
        for declaration in &module.declarations {
            visit_declaration_expressions(declaration, &mut collect);
        }
        // A main-module bundle provide, `provide env with { … }`, holds its entries here.
        for provide in module
            .provides
            .iter()
            .filter(|provide| provide.target.is_none())
        {
            visit_expression(&provide.value, &mut collect);
        }
        for (target, value) in overrides {
            if let Some(diagnostic) = override_cycle(module, &declarations, &target, &value) {
                diagnostics.push(diagnostic);
            }
        }
    }
    diagnostics
}

/// The `environment-cycle` error for an override of `target` whose value reads a value `W` whose
/// default reads `target`: under the override, a read of `W` evaluates the default, then the
/// override, then `W` again with no source left.
fn override_cycle(
    module: &CodegenModule,
    declarations: &FxHashMap<Key, (&CodegenModule, &CodegenDeclaration)>,
    target: &CodegenReference,
    value: &CodegenExpression,
) -> Option<Diagnostic> {
    let target_key = key(target);
    let mut reached = Vec::new();
    let mut visited = FxHashSet::default();
    reached_environment(value, declarations, target_key, &mut visited, &mut reached);
    for read in reached {
        let Some((_, declaration)) = declarations.get(&read).copied() else {
            continue;
        };
        let CodegenDeclarationKind::Environment { default, .. } = &declaration.kind else {
            continue;
        };
        let mut walk = Walk {
            declarations,
            target: target_key,
            visited: FxHashSet::default(),
            path: Vec::new(),
        };
        let Some(path) = walk.expression(default) else {
            continue;
        };
        let name = display_name(&target.name);
        let through = display_name(&declaration.reference.name);
        let mut steps = vec![format!("'{through}'")];
        steps.extend(path.iter().map(|step| format!("'{step}'")));
        return Some(
            Diagnostic::error("environment-cycle")
                .with_message(format!(
                    "This override of 'env.{name}' reads 'env.{through}', whose default reads '{name}' again, through {}.",
                    steps.join(", ")
                ))
                .with_label(Label::primary(module_identity(module), value.span))
                .with_help(format!(
                    "Under this override, a read of 'env.{through}' evaluates its default, then the override, then 'env.{through}' again with nothing left to give it; read '{through}' outside the override, or give it a value here too."
                ))
                .build(),
        );
    }
    None
}

/// The environment values `expression` can read, following the functions and values it names and
/// the defaults of the values it reads, except `except`'s, whose read under its own override sees
/// the sources outside it.
fn reached_environment(
    expression: &CodegenExpression,
    declarations: &FxHashMap<Key, (&CodegenModule, &CodegenDeclaration)>,
    except: Key,
    visited: &mut FxHashSet<Key>,
    reached: &mut Vec<Key>,
) {
    let mut found = Vec::new();
    visit_expression(expression, &mut |expression| match &expression.kind {
        CodegenExpressionKind::EnvRead { reference } => found.push(reference.clone()),
        CodegenExpressionKind::Identifier {
            reference: Some(reference),
            ..
        }
        | CodegenExpressionKind::Member {
            reference: Some(reference),
            ..
        } if matches!(
            reference.kind,
            ResolvedItemKind::Function | ResolvedItemKind::Value
        ) =>
        {
            found.push(reference.clone())
        }
        _ => {}
    });
    for reference in found {
        let found = key(&reference);
        if found == except || !visited.insert(found) {
            continue;
        }
        let Some((_, declaration)) = declarations.get(&found).copied() else {
            continue;
        };
        match &declaration.kind {
            CodegenDeclarationKind::Environment { default, .. } => {
                reached.push(found);
                reached_environment(default, declarations, except, visited, reached);
            }
            CodegenDeclarationKind::Value { value, .. } => {
                reached_environment(value, declarations, except, visited, reached)
            }
            CodegenDeclarationKind::Function { params, body, .. } => {
                for default in params.iter().filter_map(|param| param.default.as_ref()) {
                    reached_environment(default, declarations, except, visited, reached);
                }
                reached_environment(body, declarations, except, visited, reached);
            }
            _ => {}
        }
    }
}

/// The name a diagnostic gives a declaration: an environment value without its `env.` prefix.
fn display_name(name: &str) -> &str {
    name.strip_prefix("env.").unwrap_or(name)
}

struct Walk<'a> {
    declarations: &'a FxHashMap<Key, (&'a CodegenModule, &'a CodegenDeclaration)>,
    target: Key,
    visited: FxHashSet<Key>,
    path: Vec<String>,
}

impl Walk<'_> {
    /// The declarations between the walk's start and a read of its target in `expression`, when
    /// it can reach one.
    fn expression(&mut self, expression: &CodegenExpression) -> Option<Vec<String>> {
        let mut reached = Vec::new();
        visit_expression(expression, &mut |expression| match &expression.kind {
            CodegenExpressionKind::EnvRead { reference } => reached.push(reference.clone()),
            CodegenExpressionKind::Identifier {
                reference: Some(reference),
                ..
            }
            | CodegenExpressionKind::Member {
                reference: Some(reference),
                ..
            } if matches!(
                reference.kind,
                ResolvedItemKind::Function | ResolvedItemKind::Value
            ) =>
            {
                reached.push(reference.clone())
            }
            _ => {}
        });
        for reference in reached {
            let found = key(&reference);
            if found == self.target {
                return Some(self.path.clone());
            }
            if !self.visited.insert(found) {
                continue;
            }
            let Some((_, declaration)) = self.declarations.get(&found).copied() else {
                continue;
            };
            self.path
                .push(display_name(&declaration.reference.name).to_string());
            let result = match &declaration.kind {
                CodegenDeclarationKind::Environment { default, .. } => self.expression(default),
                CodegenDeclarationKind::Value { value, .. } => self.expression(value),
                CodegenDeclarationKind::Function { params, body, .. } => params
                    .iter()
                    .filter_map(|param| param.default.as_ref())
                    .find_map(|default| self.expression(default))
                    .or_else(|| self.expression(body)),
                _ => None,
            };
            if result.is_some() {
                return result;
            }
            self.path.pop();
        }
        None
    }
}
