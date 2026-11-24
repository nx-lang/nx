//! `with` expressions: `left with { field=value }` and `left with update`.
//!
//! <para>A `with` is typed by its left side. An update record merges (`merge`), a record applies
//! its update record (`apply`), and an `env.Update` bundle extends with more entries. What the
//! checker decides is recorded as a [`WithSite`], from which the rewrite after checking builds the
//! `apply`, `merge` or bundle expression the `with` means.</para>

use super::InferenceContext;
use crate::ty::{read_type, NamedType, Occurrence, Type};
use nx_diagnostics::TextSpan;
use nx_hir::{ast, ExprId, Name};

/// What a `with` does, decided by its left side's type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WithKind {
    /// A record patched with its update record: `apply`.
    Record,
    /// An update record merged with another: `merge`.
    Update,
    /// An `env.Update` bundle extended with more entries.
    Bundle,
}

/// One record a path entry steps into, after the record on the left.
#[derive(Debug, Clone, PartialEq)]
pub struct WithPathStep {
    /// The field's record type.
    pub record: Type,
    /// That record's update record type.
    pub update: Type,
}

/// What the checker decided about one `with`.
#[derive(Debug, Clone, PartialEq)]
pub struct WithSite {
    /// What the `with` does.
    pub kind: WithKind,
    /// The left side's type, which is also the result's.
    pub target: Type,
    /// The update record a field list builds: the target's own for [`WithKind::Update`], its
    /// update record for [`WithKind::Record`], and `env.Update` for a bundle.
    pub update: Type,
    /// For each field-list entry, the records its path steps into after the target: empty for
    /// an entry without a path.
    pub paths: Vec<Vec<WithPathStep>>,
}

impl InferenceContext<'_> {
    /// Types `left with right`, recording what it does for the rewrite after checking.
    pub(super) fn infer_with(
        &mut self,
        expr_id: ExprId,
        left: ExprId,
        right: &ast::WithRight,
        span: TextSpan,
    ) -> Type {
        let left_ty = self.infer_expr(left);
        if left_ty.is_error() {
            for value in right.exprs() {
                self.infer_expr(value);
            }
            return Type::Error;
        }

        if Self::is_environment_update_type(&left_ty) {
            self.infer_bundle_with(right, span);
            self.with_sites.insert(
                expr_id,
                WithSite {
                    kind: WithKind::Bundle,
                    target: left_ty.clone(),
                    update: left_ty.clone(),
                    paths: Vec::new(),
                },
            );
            return left_ty;
        }

        let Some((kind, record, update)) = self.with_target(&left_ty, span) else {
            for value in right.exprs() {
                self.infer_expr(value);
            }
            return Type::Error;
        };

        let paths = match right {
            ast::WithRight::Fields(fields) => {
                self.check_with_fields(kind, &record, &update, fields)
            }
            ast::WithRight::Value(value) => {
                let actual = self.infer_expr(*value);
                self.check_with_update_value(kind, &record, &update, &actual, span);
                Vec::new()
            }
        };
        self.with_sites.insert(
            expr_id,
            WithSite {
                kind,
                target: left_ty.clone(),
                update: Type::Named(update),
                paths,
            },
        );
        left_ty
    }

    /// The kind of `with` the left side's type takes, with the record and the update record the
    /// right side is typed by, or `None` after reporting `with-target`.
    fn with_target(
        &mut self,
        left_ty: &Type,
        span: TextSpan,
    ) -> Option<(WithKind, NamedType, NamedType)> {
        let occurrence = left_ty.occurrence();
        if occurrence != Occurrence::ONE {
            let (item, _) = left_ty.split();
            let patchable =
                matches!(item, Type::Named(named) if self.record_definition_for(named).is_some());
            let hint = if patchable {
                "; a `with` patches one value, so unwrap it first: `(x ?? fallback) with { … }`, \
                 or test it with `if x is { some }`"
                    .to_string()
            } else {
                String::new()
            };
            self.error(
                "with-target",
                format!("`with` patches a record, an update record or an `env.Update`, but this is a {left_ty}{hint}"),
                span,
            );
            return None;
        }
        if let Type::Named(named) = left_ty {
            if let Some(record) = self.record_definition_for(named) {
                if record.update_target().is_some() {
                    return Some((WithKind::Update, named.clone(), named.clone()));
                }
                if !Self::is_derived_record_name(named) {
                    let update = self.derived_type_of(named, nx_hir::update_record_name);
                    if let Type::Named(update) = Self::carry_type_arguments(named, update) {
                        return Some((WithKind::Record, named.clone(), update));
                    }
                }
            }
        }
        self.error(
            "with-target",
            format!(
                "`with` patches a record, an update record or an `env.Update`, but this is a {left_ty}"
            ),
            span,
        );
        None
    }

    /// Checks a bundle's right side: a field list read as the fields of an `<env.Update … />`, or
    /// another `env.Update`.
    fn infer_bundle_with(&mut self, right: &ast::WithRight, span: TextSpan) {
        match right {
            ast::WithRight::Fields(fields) => {
                let overrides = fields
                    .iter()
                    .filter_map(|field| {
                        let (name, path) = field.path.split_first()?;
                        Some(ast::EnvOverride {
                            name: name.clone(),
                            path: path.to_vec(),
                            value: field.value,
                            binding: None,
                            span: field.span,
                        })
                    })
                    .collect::<Vec<_>>();
                self.check_environment_duplicates(&overrides);
                for provided in &overrides {
                    self.check_environment_entry(provided, true);
                }
            }
            ast::WithRight::Value(value) => {
                let actual = self.infer_expr(*value);
                if !actual.is_error() && !Self::is_environment_update_type(&actual) {
                    self.error(
                        "value-type-mismatch",
                        format!(
                            "A `with` on an `env.Update` takes a field list or another `env.Update`, \
                             but this is a {actual}"
                        ),
                        span,
                    );
                }
            }
        }
    }

    /// Checks a field list against `update`, and each path against the records it steps into.
    /// Returns the records each entry's path steps into.
    fn check_with_fields(
        &mut self,
        kind: WithKind,
        record: &NamedType,
        update: &NamedType,
        fields: &[ast::WithField],
    ) -> Vec<Vec<WithPathStep>> {
        let mut seen: Vec<&[Name]> = Vec::new();
        let mut paths = Vec::with_capacity(fields.len());
        for field in fields {
            let spelled = field
                .path
                .iter()
                .map(Name::as_str)
                .collect::<Vec<_>>()
                .join(".");
            if seen.contains(&field.path.as_slice()) {
                self.error(
                    "duplicate-property",
                    format!("'{spelled}' is set more than once in this field list"),
                    field.span,
                );
            }
            seen.push(&field.path);

            let actual = self.infer_expr(field.value);
            let Some((last, steps)) = field.path.split_last() else {
                paths.push(Vec::new());
                continue;
            };
            let mut path_steps = Vec::new();
            let mut patched = update.clone();
            if !steps.is_empty() {
                if kind == WithKind::Update {
                    self.error(
                        "with-path",
                        format!(
                            "'{spelled}' steps into a field of an update record, whose fields may be \
                             absent; set the whole field instead: `{}=…`",
                            steps[0]
                        ),
                        field.span,
                    );
                    paths.push(Vec::new());
                    continue;
                }
                let Some(walked) = self.with_path_steps(record, steps, &spelled, field.span) else {
                    paths.push(Vec::new());
                    continue;
                };
                if let Some(step) = walked.last() {
                    if let Type::Named(named) = &step.update {
                        patched = named.clone();
                    }
                }
                path_steps = walked;
            }
            self.check_with_field_value(&patched, last, field.value, &actual, &spelled, field.span);
            paths.push(path_steps);
        }
        paths
    }

    /// The records a path's leading steps reach from `record`, or `None` after reporting
    /// `with-path` at a step that is not a required field of a record type.
    fn with_path_steps(
        &mut self,
        record: &NamedType,
        steps: &[Name],
        spelled: &str,
        span: TextSpan,
    ) -> Option<Vec<WithPathStep>> {
        let mut current = record.clone();
        let mut walked = Vec::with_capacity(steps.len());
        for step in steps {
            let field = match self.record_shape_of(&current) {
                Ok(Some(shape)) => shape
                    .fields
                    .iter()
                    .find(|field| field.name == *step)
                    .cloned(),
                _ => None,
            };
            let Some(field) = field else {
                self.error(
                    "unknown-record-field",
                    format!("Record '{}' has no field '{step}'", current.name),
                    span,
                );
                return None;
            };
            let field_ty =
                self.type_from_type_ref_in_quietly(Some(&field.module_identity), &field.ty);
            let field_ty = read_type(&field_ty, field.optional);
            let next = match &field_ty {
                Type::Named(named)
                    if field_ty.occurrence() == Occurrence::ONE
                        && self
                            .record_definition_for(named)
                            .is_some_and(|def| def.update_target().is_none())
                        && !Self::is_derived_record_name(named) =>
                {
                    Some(named.clone())
                }
                _ => None,
            };
            let Some(next) = next else {
                self.error(
                    "with-path",
                    format!(
                        "'{spelled}' steps through '{step}', which is a {field_ty}; a path steps only \
                         through required record fields, so set the whole field instead: `{step}=…`"
                    ),
                    span,
                );
                return None;
            };
            let update = self.derived_type_of(&next, nx_hir::update_record_name);
            let update = Self::carry_type_arguments(&next, update);
            walked.push(WithPathStep {
                record: Type::Named(next.clone()),
                update,
            });
            current = next;
        }
        Some(walked)
    }

    /// Checks one entry's value against the field it sets in `update`.
    fn check_with_field_value(
        &mut self,
        update: &NamedType,
        name: &Name,
        value: ExprId,
        actual: &Type,
        spelled: &str,
        span: TextSpan,
    ) {
        let field = match self.record_shape_of(update) {
            Ok(Some(shape)) => shape
                .fields
                .iter()
                .find(|field| field.name == *name)
                .cloned(),
            _ => None,
        };
        let Some(field) = field else {
            self.error(
                "unknown-record-field",
                format!("Record '{}' has no field '{name}'", update.name),
                span,
            );
            return;
        };
        let expected = self.type_from_type_ref_in_quietly(Some(&field.module_identity), &field.ty);
        let expected = read_type(&expected, field.optional);
        self.check_typed_binding_for(
            Some(value),
            actual,
            &expected,
            span,
            "record-field-type-mismatch",
            format!("Field '{spelled}' of '{}'", update.name),
            spelled,
        );
    }

    /// Checks an operand on the right of a record or update-record `with`: it must be the
    /// left side's update record.
    fn check_with_update_value(
        &mut self,
        kind: WithKind,
        record: &NamedType,
        update: &NamedType,
        actual: &Type,
        span: TextSpan,
    ) {
        if actual.is_error() {
            return;
        }
        let fits = match actual {
            Type::Named(named) if actual.occurrence() == Occurrence::ONE => {
                match self
                    .record_definition_for(named)
                    .and_then(|def| def.update_target().cloned())
                {
                    Some(target) => match kind {
                        WithKind::Update => named == update,
                        _ => {
                            self.update_patches_record(named, &target, record)
                                && named.args() == record.args()
                        }
                    },
                    None => false,
                }
            }
            _ => false,
        };
        if !fits {
            self.error(
                "value-type-mismatch",
                format!(
                    "A `with` on a {} takes a field list or a {}, but this is a {actual}",
                    Type::Named(record.clone()),
                    Type::Named(update.clone())
                ),
                span,
            );
        }
    }

    /// What the checker decided about each `with`, by expression.
    pub fn with_sites(&self) -> &rustc_hash::FxHashMap<ExprId, WithSite> {
        &self.with_sites
    }
}
