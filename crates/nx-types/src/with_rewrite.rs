//! Rewrites each checked `with` into what it means.
//!
//! <para>After checking, a record `with` is `apply(left, <T.Update …/>)`, an update-record `with`
//! is `merge(left, <T.Update …/>)`, and a bundle `with` is an `env.Update` whose bases come first.
//! Below here nothing sees a `with`: the IR builder, the emitters and every runtime see the
//! intrinsics and bundles they already know, so a program written with `with` is the program
//! written with `apply` and `merge`.</para>
//!
//! <para>A field list's entries apply in the order written. Consecutive entries without a path
//! become one update record; an entry `a.b=v` is `a=(left.a with { b=v })` on the result so far,
//! which is bound once with a `let` so it is evaluated once.</para>

use crate::infer::{WithKind, WithPathStep, WithSite};
use crate::{Type, TypeEnvironment};
use nx_diagnostics::TextSpan;
use nx_hir::{ast, ExprId, Name, PreparedModule, UpdateIntrinsic};
use rustc_hash::FxHashMap;

/// Rewrites every `with` the checker recorded a site for, typing every node it creates.
pub fn apply_with_rewrites(
    module: &mut PreparedModule,
    sites: &FxHashMap<ExprId, WithSite>,
    type_env: &mut TypeEnvironment,
) {
    let mut sites = sites.iter().collect::<Vec<_>>();
    sites.sort_by_key(|(expr_id, _)| u32::from(expr_id.into_raw()));
    let mut rewriter = Rewriter {
        module,
        type_env,
        next_binding: 0,
    };
    for (expr_id, site) in sites {
        rewriter.rewrite(*expr_id, site);
    }
}

struct Rewriter<'a> {
    module: &'a mut PreparedModule,
    type_env: &'a mut TypeEnvironment,
    next_binding: usize,
}

impl Rewriter<'_> {
    fn rewrite(&mut self, expr_id: ExprId, site: &WithSite) {
        let ast::Expr::With { left, right, span } = self.module.raw_module().expr(expr_id).clone()
        else {
            return;
        };
        let rewritten = match site.kind {
            WithKind::Bundle => {
                let (bases, overrides) = match right {
                    ast::WithRight::Value(value) => (vec![left, value], Vec::new()),
                    ast::WithRight::Fields(fields) => (
                        vec![left],
                        fields
                            .into_iter()
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
                            .collect(),
                    ),
                };
                ast::Expr::EnvUpdate {
                    bases,
                    overrides,
                    span,
                }
            }
            WithKind::Record | WithKind::Update => {
                let intrinsic = if site.kind == WithKind::Record {
                    UpdateIntrinsic::Apply
                } else {
                    UpdateIntrinsic::Merge
                };
                match right {
                    ast::WithRight::Value(value) => {
                        self.intrinsic_call(intrinsic, left, value, span)
                    }
                    ast::WithRight::Fields(fields) => {
                        let result = self.fields(intrinsic, left, site, &fields, span);
                        self.module.raw_module().expr(result).clone()
                    }
                }
            }
        };
        *self.module.raw_module_mut().expr_mut(expr_id) = rewritten;
    }

    /// The patches a field list makes to `left`, in order, as one expression typed as the target.
    fn fields(
        &mut self,
        intrinsic: UpdateIntrinsic,
        left: ExprId,
        site: &WithSite,
        fields: &[ast::WithField],
        span: TextSpan,
    ) -> ExprId {
        let mut acc = left;
        let mut plain: Vec<ast::RecordLiteralProperty> = Vec::new();
        let mut any = false;
        for (index, field) in fields.iter().enumerate() {
            let steps = site.paths.get(index).cloned().unwrap_or_default();
            if steps.is_empty() || field.path.len() < 2 {
                plain.push(ast::RecordLiteralProperty {
                    name: field.path[0].clone(),
                    value: field.value,
                    span: field.span,
                });
                continue;
            }
            if !plain.is_empty() {
                acc = self.patch(
                    intrinsic,
                    acc,
                    &site.target,
                    &site.update,
                    std::mem::take(&mut plain),
                    span,
                );
            }
            acc = self.path_patch(
                intrinsic,
                acc,
                &site.target,
                &site.update,
                &field.path,
                &steps,
                field.value,
                field.span,
            );
            any = true;
        }
        if !plain.is_empty() || !any {
            acc = self.patch(intrinsic, acc, &site.target, &site.update, plain, span);
        }
        acc
    }

    /// `intrinsic(base, <update …properties/>)`, typed as `target`.
    fn patch(
        &mut self,
        intrinsic: UpdateIntrinsic,
        base: ExprId,
        target: &Type,
        update: &Type,
        properties: Vec<ast::RecordLiteralProperty>,
        span: TextSpan,
    ) -> ExprId {
        let literal = self.update_literal(update, properties, span);
        let call = self.intrinsic_call(intrinsic, base, literal, span);
        self.alloc(call, target.clone(), span)
    }

    /// `path[0]=(base.path[0] with { path[1..]=value })` on `base`, with `base` bound once.
    #[allow(clippy::too_many_arguments)]
    fn path_patch(
        &mut self,
        intrinsic: UpdateIntrinsic,
        base: ExprId,
        target: &Type,
        update: &Type,
        path: &[Name],
        steps: &[WithPathStep],
        value: ExprId,
        span: TextSpan,
    ) -> ExprId {
        let (bound, wrap) = self.bind_once(base, target, span);
        let property_value = match steps.split_first() {
            Some((step, rest)) => {
                let member = self.alloc(
                    ast::Expr::Member {
                        base: bound,
                        member: path[0].clone(),
                        span,
                    },
                    step.record.clone(),
                    span,
                );
                if path.len() > 2 {
                    self.path_patch(
                        UpdateIntrinsic::Apply,
                        member,
                        &step.record,
                        &step.update,
                        &path[1..],
                        rest,
                        value,
                        span,
                    )
                } else {
                    self.patch(
                        UpdateIntrinsic::Apply,
                        member,
                        &step.record,
                        &step.update,
                        vec![ast::RecordLiteralProperty {
                            name: path[1].clone(),
                            value,
                            span,
                        }],
                        span,
                    )
                }
            }
            None => value,
        };
        let patched = self.patch(
            intrinsic,
            bound,
            target,
            update,
            vec![ast::RecordLiteralProperty {
                name: path[0].clone(),
                value: property_value,
                span,
            }],
            span,
        );
        match wrap {
            Some((name, init)) => self.alloc(
                ast::Expr::Let {
                    name,
                    value: init,
                    body: patched,
                    span,
                },
                target.clone(),
                span,
            ),
            None => patched,
        }
    }

    /// An expression reading `expr`'s value that may be used more than once: `expr` itself when it
    /// is a name, or a fresh `let` name bound to it, returned with the name and initializer the
    /// caller wraps its body in.
    fn bind_once(
        &mut self,
        expr: ExprId,
        ty: &Type,
        span: TextSpan,
    ) -> (ExprId, Option<(Name, ExprId)>) {
        if matches!(self.module.raw_module().expr(expr), ast::Expr::Ident(_)) {
            let ast::Expr::Ident(name) = self.module.raw_module().expr(expr).clone() else {
                unreachable!();
            };
            let read = self.alloc(ast::Expr::Ident(name), ty.clone(), span);
            return (read, None);
        }
        let name = Name::new(&format!("$with{}", self.next_binding));
        self.next_binding += 1;
        let read = self.alloc(ast::Expr::Ident(name.clone()), ty.clone(), span);
        (read, Some((name, expr)))
    }

    /// `<update …properties/>`, named in the module that declares it.
    fn update_literal(
        &mut self,
        update: &Type,
        properties: Vec<ast::RecordLiteralProperty>,
        span: TextSpan,
    ) -> ExprId {
        let (record, declared_in) = match update {
            Type::Named(named) => (named.name.clone(), named.origin().cloned()),
            _ => (Name::new("?"), None),
        };
        self.alloc(
            ast::Expr::RecordLiteral {
                record,
                properties,
                declared_in,
                span,
            },
            update.clone(),
            span,
        )
    }

    /// `intrinsic(first, second)`, with the callee a name of its own.
    fn intrinsic_call(
        &mut self,
        intrinsic: UpdateIntrinsic,
        first: ExprId,
        second: ExprId,
        span: TextSpan,
    ) -> ast::Expr {
        let callee = self
            .module
            .raw_module_mut()
            .alloc_expr(ast::Expr::Ident(Name::new(intrinsic.name())));
        self.module.raw_module_mut().set_expr_span(callee, span);
        ast::Expr::Call {
            func: callee,
            args: vec![first, second],
            span,
        }
    }

    fn alloc(&mut self, expr: ast::Expr, ty: Type, span: TextSpan) -> ExprId {
        let raw_module = self.module.raw_module_mut();
        let id = raw_module.alloc_expr(expr);
        raw_module.set_expr_span(id, span);
        self.type_env.set_expr_type(id, ty);
        id
    }
}
