//! Renders hover content as markdown written in NX.
//!
//! <para>A hover is read, not parsed, but what it shows is code: a signature, a type, a field
//! list. Showing that as prose asks the reader to translate it back into the language they are
//! writing. Everything here emits a fenced `nx` block instead, spelled the way an author would
//! write the declaration, so a client that highlights NX highlights the hover too.</para>
//!
//! <para>Not every position has an NX spelling. A parameter, a record field, and a union case are
//! written inside a declaration and have no standalone form, so those carry a parenthesized kind
//! prefix — TypeScript's convention, and visibly not NX, so it cannot be mistaken for one. See
//! design D7.</para>

use nx_hir::{
    ast::{Expr, TypeRef},
    Component, Function, Item, RecordDef, RecordField, UnionDef,
};
use nx_types::Type;

use crate::{type_ref_display, DocumentSymbolKind};

/// The fragment reported for an expression: the union case it *is*, or else the type it has.
///
/// <para>An expression whose type is a union case is not always a union case. `admin` written at a
/// site expecting `Role` is one; a value declared `let chosen: Role = admin` and then read is not,
/// and reporting `(case) Role.admin` for a read of it would name the wrong thing. What tells them
/// apart is the expression node, not the type: a bare name resolved against its site and a
/// qualified `Role.admin` are cases, and an identifier bound to one is a use of it.</para>
pub(crate) fn expression_fragment(expr: &Expr, ty: &Type) -> String {
    union_case_expression(expr, ty).unwrap_or_else(|| ty.to_string())
}

/// `(case) Role.admin` where the expression is a union case written as one.
pub(crate) fn union_case_expression(expr: &Expr, ty: &Type) -> Option<String> {
    let Type::UnionCase(case_ty) = ty else {
        return None;
    };
    matches!(
        expr,
        Expr::ContextualName { .. } | Expr::ResolvedUnionCase { .. } | Expr::Member { .. }
    )
    .then(|| union_case(case_ty.union.as_str(), case_ty.case.as_str()))
}

/// Wraps one NX fragment in the fenced block hover content is made of.
pub(crate) fn fenced(nx: impl AsRef<str>) -> String {
    format!("```nx\n{}\n```", nx.as_ref().trim_end())
}

/// A parenthesized kind prefix on a fragment that has no NX spelling of its own — design D7.
fn prefixed(kind: &str, rest: impl AsRef<str>) -> String {
    format!("({}) {}", kind, rest.as_ref())
}

/// A declaration spelled the way its author wrote it, read from HIR.
///
/// <para>The item is the declaration itself, so this is the one place that knows an element-style
/// function from a plain one without guessing from source text.</para>
pub(crate) fn item_signature(
    item: &Item,
    kind: DocumentSymbolKind,
    inferred: Option<&str>,
) -> String {
    match item {
        Item::Function(function) => function_signature(function, kind),
        Item::Component(component) => component_signature(component),
        Item::Record(record) => record_signature(record),
        Item::Union(union_def) => union_signature(union_def),
        Item::TypeAlias(alias) => {
            format!(
                "type {} = {}",
                alias.name.as_str(),
                type_ref_display(&alias.ty)
            )
        }
        // A value's written annotation is what the author chose; only an unannotated one falls
        // back to what analysis inferred — design D5.
        Item::Value(value) => match value.ty.as_ref().map(type_ref_display).as_deref() {
            Some(ty) => format!("let {}: {}", value.name.as_str(), ty),
            None => match inferred {
                Some(ty) => format!("let {}: {}", value.name.as_str(), ty),
                None => format!("let {}", value.name.as_str()),
            },
        },
    }
}

/// `let add(count:int): int`, or `let <Panel title:string />` for an element-style function.
///
/// <para>Nothing on the HIR item says which of the two it is: both lower to a `Function`, and a
/// content parameter is optional in either. The kind is what the module projection already
/// decided, from the `let <` the author wrote, and it is passed in rather than guessed again
/// here.</para>
fn function_signature(function: &Function, kind: DocumentSymbolKind) -> String {
    let return_type = function
        .return_type
        .as_ref()
        .map(|ty| format!(": {}", type_ref_display(ty)))
        .unwrap_or_default();

    if kind == DocumentSymbolKind::Component {
        return format!(
            "let {}{}",
            tag_signature(
                function.name.as_str(),
                function
                    .params
                    .iter()
                    .map(|param| (param.name.as_str(), &param.ty)),
            ),
            return_type
        );
    }

    let params = function
        .params
        .iter()
        .map(|param| format!("{}:{}", param.name.as_str(), type_ref_display(&param.ty)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("let {}({}){}", function.name.as_str(), params, return_type)
}

/// `component <Panel title:string />`, with the modifiers the declaration carries.
fn component_signature(component: &Component) -> String {
    let mut prefix = String::new();
    if component.is_abstract {
        prefix.push_str("abstract ");
    }
    if component.is_external {
        prefix.push_str("external ");
    }
    format!(
        "{}component {}",
        prefix,
        tag_signature(
            component.name.as_str(),
            component
                .props
                .iter()
                .map(|property| (property.name.as_str(), &property.ty)),
        )
    )
}

/// `<Panel title:string />` — the tag with the properties it accepts.
fn tag_signature<'a>(
    name: &str,
    properties: impl Iterator<Item = (&'a str, &'a TypeRef)>,
) -> String {
    let properties = properties
        .map(|(property, ty)| format!("{}:{}", property, type_ref_display(ty)))
        .collect::<Vec<_>>()
        .join(" ");
    if properties.is_empty() {
        return format!("<{} />", name);
    }
    format!("<{} {} />", name, properties)
}

/// `type User extends UserBase = { … }`, one field per line.
fn record_signature(record: &RecordDef) -> String {
    let keyword = if record.is_action() { "action" } else { "type" };
    let mut head = String::new();
    if record.is_abstract() {
        head.push_str("abstract ");
    }
    head.push_str(keyword);
    head.push(' ');
    head.push_str(record.name.as_str());
    if let Some(base) = record.base.as_ref() {
        head.push_str(" extends ");
        head.push_str(base.as_str());
    }

    if record.properties.is_empty() {
        return format!("{} = {{ }}", head);
    }
    let fields = record
        .properties
        .iter()
        .map(|field| format!("  {}", field_signature(field)))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{} = {{\n{}\n}}", head, fields)
}

/// One record field's name and declared type: `name: string`.
///
/// <para>Not its default. `RecordField::default` is an `ExprId`, and turning one back into the NX
/// an author wrote needs an expression renderer this crate does not have — rendering only the
/// literal ones would make the hover's fidelity depend on what the default happens to be. The
/// omission is uniform instead.</para>
fn field_signature(field: &RecordField) -> String {
    format!("{}: {}", field.name.as_str(), type_ref_display(&field.ty))
}

/// `type Role = admin | guest`, or one bar-led case per line where that does not fit.
///
/// <para>The house style is the language's: a one-line union omits the leading bar, a multi-line
/// one carries a bar on every case, and a single-case union always keeps its bar, because
/// `type Wrapper = only` is a type alias rather than a union.</para>
fn union_signature(union_def: &UnionDef) -> String {
    let mut head = format!("type {}", union_def.name.as_str());
    if let Some(base) = union_def.base.as_ref() {
        head.push_str(" extends ");
        head.push_str(base.as_str());
    }

    let cases = union_def
        .cases
        .iter()
        .map(|case| {
            if case.fields.is_empty() {
                return case.name.as_str().to_string();
            }
            let fields = case
                .fields
                .iter()
                .map(|field| format!("{}:{}", field.name.as_str(), type_ref_display(&field.ty)))
                .collect::<Vec<_>>()
                .join(" ");
            format!("{} {{ {} }}", case.name.as_str(), fields)
        })
        .collect::<Vec<_>>();

    if cases.is_empty() {
        return head;
    }

    let one_line = format!("{} = {}", head, cases.join(" | "));
    if cases.len() > 1 && one_line.len() <= 72 && !one_line.contains('{') {
        return one_line;
    }
    let listed = cases
        .iter()
        .map(|case| format!("  | {}", case))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{} =\n{}", head, listed)
}

/// `(parameter) count: int`.
pub(crate) fn parameter(name: &str, ty: &str) -> String {
    prefixed("parameter", format!("{}: {}", name, ty))
}

/// `(property) User.name: string`.
pub(crate) fn property(qualifier: Option<&str>, name: &str, ty: &str) -> String {
    let name = match qualifier {
        Some(qualifier) => format!("{}.{}", qualifier, name),
        None => name.to_string(),
    };
    prefixed("property", format!("{}: {}", name, ty))
}

/// `(case) LoadState.failed`.
pub(crate) fn union_case(union: &str, case: &str) -> String {
    prefixed("case", format!("{}.{}", union, case))
}

/// `(primitive type) int` and `(built-in type) Element`.
pub(crate) fn builtin_type(kind: &str, name: &str) -> String {
    prefixed(kind, name)
}
