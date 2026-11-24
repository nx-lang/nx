//! Editor support for `with`: completion of the fields a field list may set, and hover on the
//! keyword showing the type being patched.
//!
//! <para>A field list sets fields of the left side's type, or environment values when the left
//! side is an `env.Update` bundle, including the bundle after `provide env with`. The cursor's
//! context is read from the text before it, so completion answers while the entry is still being
//! typed; the left side's type comes from the analysis of the `with` the cursor is in.</para>

use crate::{
    hover, CompletionItem, CompletionItemKind, Declaration, DocumentScope, ResolvedPosition,
    WorkspaceSnapshot,
};
use nx_hir::{environment_value_name, Item};
use nx_syntax::{parse_str, SyntaxKind, SyntaxNode};
use nx_types::Type;
use text_size::{TextRange as ByteTextRange, TextSize};

/// The `$type` an environment bundle has.
const BUNDLE_TYPE: &str = "env.Update";

impl WorkspaceSnapshot {
    /// The hover on a `with` keyword and its range: the type being patched. Anywhere else, and on
    /// the `with` of `provide env with`, nothing.
    pub(crate) fn with_hover(
        &self,
        resolved: &ResolvedPosition<'_>,
    ) -> Option<(String, ByteTextRange)> {
        let source = resolved.document.source();
        let keyword = keyword_at(source, resolved.offset)?;
        let tree = parse_str(source, resolved.document.identity.as_str()).tree?;
        let node = enclosing(
            tree.node_at(usize::from(keyword.start()))?,
            SyntaxKind::WITH_EXPRESSION,
        )?;
        let ty = self.type_at(resolved, node.child_by_field("left")?.span())?;
        if crate::is_unresolved_type(&ty) {
            return None;
        }
        Some((hover::fenced(ty.to_string()), keyword))
    }

    /// The completions for an entry of a `with` field list the cursor is in: the fields of the
    /// left side's type, or of a field's record type after `name.`, or the environment values
    /// for a bundle. `None` where the cursor is not at an entry name of a field list.
    pub(crate) fn with_completions(
        &self,
        resolved: &ResolvedPosition<'_>,
    ) -> Option<Vec<CompletionItem>> {
        let source = resolved.document.source();
        let before = &source[..resolved.offset];
        let start = before
            .rfind(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.'))
            .map(|index| index + 1)
            .unwrap_or(0);
        let written = &before[start..];
        let open = field_list_open(before, start)?;
        let path = match written.rsplit_once('.') {
            Some((qualifier, _)) => qualifier.split('.').collect::<Vec<_>>(),
            None => Vec::new(),
        };
        let scope = &resolved.scope;

        // `provide env with { … }` and `provide env with brand with { … }` set environment values.
        let head = before[..open].trim_end();
        let head = head.strip_suffix("with")?.trim_end();
        let bundle_after_provide = is_provide_bundle_head(head);
        let left = if bundle_after_provide {
            None
        } else {
            self.with_left_type(resolved, open)
        };
        let is_bundle = bundle_after_provide
            || matches!(&left, Some(Type::Named(named)) if named.name.as_str() == BUNDLE_TYPE);
        if is_bundle {
            return environment_field_items(scope, &path);
        }
        let Type::Named(named) = left? else {
            return None;
        };
        let mut record = record_declaration(scope, &named)?;
        for field in path {
            let property = record
                .properties
                .iter()
                .find(|property| property.name == field)?;
            record = scope.type_declared_in(&property.declaring_module, &property.base_type)?;
        }
        Some(property_items(record))
    }

    /// The type of the left side of the `with` whose field list opens at `open`.
    fn with_left_type(&self, resolved: &ResolvedPosition<'_>, open: usize) -> Option<Type> {
        let source = resolved.document.source();
        let tree = parse_str(source, resolved.document.identity.as_str()).tree?;
        if let Some(left) = tree
            .node_at(open)
            .and_then(|node| enclosing(node, SyntaxKind::WITH_EXPRESSION))
            .and_then(|node| node.child_by_field("left"))
        {
            return self.type_at(resolved, left.span());
        }
        // Mid-edit the field list may not parse, `with { address. }`: the left side is then the
        // outermost expression that ends where the text before `with` does.
        let head = source[..open].trim_end().strip_suffix("with")?.trim_end();
        let end = TextSize::from(head.len() as u32);
        let analysis = self.module_analysis(&resolved.document.uri)?;
        let lowered = analysis.lowered_module();
        let ty = lowered
            .exprs()
            .filter(|(id, _)| lowered.expr_span(*id).end() == end)
            .filter_map(|(id, _)| {
                Some((
                    lowered.expr_span(id).len(),
                    analysis.type_env().get_expr_type(id)?,
                ))
            })
            .max_by_key(|(len, _)| *len)
            .map(|(_, ty)| ty.clone());
        ty
    }

    /// The type the analysis gave the expression written at `span`.
    fn type_at(&self, resolved: &ResolvedPosition<'_>, span: ByteTextRange) -> Option<Type> {
        let analysis = self.module_analysis(&resolved.document.uri)?;
        let lowered = analysis.lowered_module();
        let ty = lowered
            .exprs()
            .filter(|(id, _)| lowered.expr_span(*id) == span)
            .find_map(|(id, _)| analysis.type_env().get_expr_type(id).cloned());
        ty
    }
}

/// `node` or its nearest ancestor of `kind`.
fn enclosing(node: SyntaxNode<'_>, kind: SyntaxKind) -> Option<SyntaxNode<'_>> {
    let mut current = Some(node);
    while let Some(candidate) = current {
        if candidate.kind() == kind {
            return Some(candidate);
        }
        current = candidate.parent();
    }
    None
}

/// The range of the `with` keyword the offset is on, if it is on one.
fn keyword_at(source: &str, offset: usize) -> Option<ByteTextRange> {
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let start = source[..offset]
        .rfind(|c: char| !is_word(c))
        .map(|index| index + 1)
        .unwrap_or(0);
    let end = source[offset..]
        .find(|c: char| !is_word(c))
        .map(|index| offset + index)
        .unwrap_or(source.len());
    if &source[start..end] != "with" {
        return None;
    }
    // A field named `with`, `x.with`, or `with=` in a field list is a name, not the keyword.
    let previous = source[..start].chars().next_back();
    let next = source[end..].trim_start().chars().next();
    if previous == Some('.') || next == Some('=') || next == Some('.') {
        return None;
    }
    Some(ByteTextRange::new(
        TextSize::from(start as u32),
        TextSize::from(end as u32),
    ))
}

/// Where the field list the entry starting at `entry` is written in opens, `{`, when the text
/// before it is a field list's: the `{` is unmatched before the entry, follows `with`, and the
/// entry is a name rather than a value after `=`.
fn field_list_open(before: &str, entry: usize) -> Option<usize> {
    if before[..entry].trim_end().ends_with('=') {
        return None;
    }
    let mut depth = 0usize;
    let mut in_string = false;
    for (index, c) in before[..entry].char_indices().rev() {
        match c {
            '"' => in_string = !in_string,
            _ if in_string => {}
            '}' | ')' => depth += 1,
            '{' | '(' if depth > 0 => depth -= 1,
            '(' => return None,
            '{' => {
                let head = before[..index].trim_end();
                let is_with = head.strip_suffix("with").is_some_and(|rest| {
                    !rest.ends_with(|c: char| c.is_alphanumeric() || c == '_' || c == '.')
                });
                return is_with.then_some(index);
            }
            _ => {}
        }
    }
    None
}

/// Whether the text before a field list's `with` ends a `provide env with …` bundle: `provide env`
/// itself, or a chain of `with` steps after it.
fn is_provide_bundle_head(head: &str) -> bool {
    let words = head
        .split(|c: char| c.is_whitespace() || c == '{' || c == '}')
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let Some(provide) = words.iter().rposition(|word| *word == "provide") else {
        return false;
    };
    let after = &head[head.rfind("provide").unwrap_or(0)..];
    words.get(provide + 1) == Some(&"env")
        && words.get(provide + 2).is_none_or(|word| *word == "with")
        && !after.contains('=')
}

/// The record declaration a named type is: the record itself, or the record an update record of it
/// patches.
fn record_declaration<'a>(
    scope: &'a DocumentScope,
    named: &nx_types::ty::NamedType,
) -> Option<&'a Declaration> {
    let origin = named.origin()?;
    let module = origin.module_identity().to_string();
    if let Some(declaration) = scope
        .workspace
        .by_origin
        .get(&(module.clone(), origin.definition_id()))
    {
        if !declaration.properties.is_empty() {
            return Some(declaration);
        }
    }
    let name = named.name.as_str();
    let name = name.rsplit_once(':').map_or(name, |(_, name)| name);
    let target = name.strip_suffix(".Update").unwrap_or(name);
    scope.type_declared_in(&module, target)
}

fn property_items(record: &Declaration) -> Vec<CompletionItem> {
    record
        .properties
        .iter()
        .map(|property| CompletionItem {
            label: property.name.clone(),
            kind: CompletionItemKind::Property,
            detail: Some(property.display_type.clone()),
            documentation: property.doc.clone(),
        })
        .collect()
}

/// The environment values a bundle's field list can set, or the fields of one at `path`.
fn environment_field_items(scope: &DocumentScope, path: &[&str]) -> Option<Vec<CompletionItem>> {
    let values = scope
        .visible_declarations()
        .into_iter()
        .filter_map(|declaration| {
            let name = declaration.name.strip_prefix("env.")?;
            if name.contains('.') {
                return None;
            }
            let Some(Item::Value(value)) = scope.item(declaration) else {
                return None;
            };
            environment_value_name(value.name.as_str())?;
            Some((name.to_string(), declaration, value))
        });
    let Some((root, fields)) = path.split_first() else {
        let mut items = values
            .map(|(name, declaration, value)| CompletionItem {
                label: name,
                kind: CompletionItemKind::Property,
                detail: value.ty.as_ref().map(crate::type_ref_display),
                documentation: declaration.doc.clone(),
            })
            .collect::<Vec<_>>();
        items.sort_by(|lhs, rhs| lhs.label.cmp(&rhs.label));
        return Some(items);
    };
    let (_, declaration, value) = values.into_iter().find(|(name, _, _)| name == root)?;
    let ty = crate::type_ref_display(value.ty.as_ref()?);
    let mut record =
        scope.type_declared_in(&declaration.origin.0, ty.trim_end_matches(['?', '+', '*']))?;
    for field in fields {
        let property = record
            .properties
            .iter()
            .find(|property| property.name == *field)?;
        record = scope.type_declared_in(&property.declaring_module, &property.base_type)?;
    }
    Some(property_items(record))
}
