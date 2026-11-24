//! Formulas: overrides whose value reads the environment.
//!
//! <para>Every environment value is computed where it is read. An override whose value reads no
//! environment value gives the same value wherever it is computed, so it is evaluated once where it
//! is written and carried as a value. Any other override is a formula: it is carried as its node
//! and the locals it captured, and evaluated at each read that uses it. Whether a value reads the
//! environment is decided from its static read set: the `env.` reads it holds, and those of the
//! functions and values it names and of the defaults of the values it reads. A call whose callee is
//! known only at run time may read anything, so it makes the value a formula too.</para>

use crate::ir::{visit_declaration_expressions_mut, visit_expression, visit_expression_mut};
use crate::model::{
    CodegenDeclarationKind, CodegenExpression, CodegenExpressionKind, CodegenModule,
    CodegenReference, CodegenStatement,
};
use nx_program::{ResolvedItemKind, RuntimeModuleId};
use rustc_hash::{FxHashMap, FxHashSet};

type Key = (RuntimeModuleId, usize);

fn key(reference: &CodegenReference) -> Key {
    (reference.module_id, reference.definition_id.index())
}

/// What one expression reads directly: whether it reads the environment or calls something only
/// known at run time, and the declarations it names whose own reads count.
#[derive(Default)]
struct Reads {
    environment: bool,
    named: Vec<Key>,
}

fn direct_reads(expression: &CodegenExpression) -> Reads {
    let mut reads = Reads::default();
    visit_expression(expression, &mut |expression| match &expression.kind {
        CodegenExpressionKind::EnvRead { .. } => reads.environment = true,
        // The callee of a named call is a function-typed value whose declaration is known only
        // at run time.
        CodegenExpressionKind::NamedCall { .. } => reads.environment = true,
        CodegenExpressionKind::Call { callee, .. } => {
            if matches!(
                callee.kind,
                CodegenExpressionKind::Identifier {
                    reference: None,
                    ..
                }
            ) {
                reads.environment = true;
            }
        }
        CodegenExpressionKind::Identifier {
            reference: Some(reference),
            ..
        } if matches!(
            reference.kind,
            ResolvedItemKind::Function | ResolvedItemKind::Value
        ) =>
        {
            reads.named.push(key(reference))
        }
        _ => {}
    });
    reads
}

/// The declarations whose evaluation can read the environment.
fn reading_declarations(modules: &[CodegenModule]) -> FxHashSet<Key> {
    let mut direct: FxHashMap<Key, Reads> = FxHashMap::default();
    for module in modules {
        for declaration in &module.declarations {
            let expressions: Vec<&CodegenExpression> = match &declaration.kind {
                CodegenDeclarationKind::Function { params, body, .. } => params
                    .iter()
                    .filter_map(|param| param.default.as_ref())
                    .chain(std::iter::once(body))
                    .collect(),
                CodegenDeclarationKind::Value { value, .. } => vec![value],
                // A value's default is evaluated wherever it is read, so a read of it reads the
                // environment by definition; it is named by its `env.` read, never called.
                _ => continue,
            };
            let mut reads = Reads::default();
            for expression in expressions {
                let found = direct_reads(expression);
                reads.environment |= found.environment;
                reads.named.extend(found.named);
            }
            direct.insert(key(&declaration.reference), reads);
        }
    }
    let mut reading: FxHashSet<Key> = direct
        .iter()
        .filter(|(_, reads)| reads.environment)
        .map(|(key, _)| *key)
        .collect();
    loop {
        let before = reading.len();
        for (declaration, reads) in &direct {
            if !reading.contains(declaration)
                && reads.named.iter().any(|named| reading.contains(named))
            {
                reading.insert(*declaration);
            }
        }
        if reading.len() == before {
            return reading;
        }
    }
}

/// Whether `expression`'s static read set is not empty.
fn reads_environment(expression: &CodegenExpression, reading: &FxHashSet<Key>) -> bool {
    let reads = direct_reads(expression);
    reads.environment || reads.named.iter().any(|named| reading.contains(named))
}

/// Sets the formula flag on every override, in a `provide`, an element's `env.` attributes or an
/// `env.Update`, whose value reads the environment.
pub(crate) fn mark_formulas(modules: &mut [CodegenModule]) {
    let reading = reading_declarations(modules);
    let mut mark = |expression: &mut CodegenExpression| {
        let overrides = match &mut expression.kind {
            CodegenExpressionKind::Provide { overrides, .. }
            | CodegenExpressionKind::EnvUpdate { overrides, .. } => overrides,
            _ => return,
        };
        for provided in overrides.iter_mut() {
            provided.formula =
                provided.target.is_some() && reads_environment(&provided.value, &reading);
        }
    };
    for module in modules.iter_mut() {
        for declaration in module.declarations.iter_mut() {
            visit_declaration_expressions_mut(declaration, &mut mark);
        }
        // A main-module `provide env with { … }` holds its inline bundle here, not in a declaration.
        for provide in module.provides.iter_mut() {
            visit_expression_mut(&mut provide.value, &mut mark);
        }
    }
}

/// The locals a formula's value reads from around it, by name, in name order: the names it reads
/// that it does not bind itself.
pub(crate) fn free_locals(expression: &CodegenExpression) -> Vec<String> {
    let mut read = Vec::new();
    let mut bound = FxHashSet::default();
    visit_expression(expression, &mut |expression| match &expression.kind {
        CodegenExpressionKind::Identifier {
            name,
            reference: None,
        } => read.push(name.clone()),
        CodegenExpressionKind::Let { name, .. } => {
            bound.insert(name.clone());
        }
        CodegenExpressionKind::For { item, index, .. } => {
            bound.insert(item.clone());
            if let Some(index) = index {
                bound.insert(index.clone());
            }
        }
        CodegenExpressionKind::Block { statements, .. } => {
            for statement in statements {
                if let CodegenStatement::Let { name, .. } = statement {
                    bound.insert(name.clone());
                }
            }
        }
        CodegenExpressionKind::ActionHandler(_) => {
            bound.insert("action".to_string());
        }
        _ => {}
    });
    read.retain(|name| !bound.contains(name));
    read.sort();
    read.dedup();
    read
}
