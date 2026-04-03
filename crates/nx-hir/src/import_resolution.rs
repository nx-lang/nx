//! File-based local library import resolution for NX modules.
//!
//! This module augments a lowered entry module with visible declarations from:
//! - peer files in the same local library (public + internal)
//! - imported local libraries (public only)
//!
//! Remote Git and HTTP imports are accepted syntactically but reported as
//! semantic diagnostics because fetching is not implemented yet.

use crate::ast::{self, Expr, Stmt};
use crate::{
    Component, Element, ElementId, EnumDef, ExprId, Function, ImportKind, Item, LoweredModule,
    Name, Property, RecordDef, RecordField, SelectiveImport, TypeAlias, ValueDef, Visibility,
};
use nx_syntax::parse_file;
use rustc_hash::{FxHashMap, FxHashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ItemRef {
    file_index: usize,
    item_index: usize,
}

#[derive(Debug, Clone)]
struct LoadedFile {
    path: PathBuf,
    module: LoweredModule,
}

#[derive(Debug, Clone)]
struct LoadedLibrary {
    root: PathBuf,
    files: Vec<LoadedFile>,
    public_items: FxHashMap<String, Vec<ItemRef>>,
    visible_to_library_items: FxHashMap<String, Vec<ItemRef>>,
}

#[derive(Debug, Clone)]
struct Candidate {
    library_root: PathBuf,
    source_file: PathBuf,
    item: ItemRef,
}

#[derive(Debug, Default)]
struct LibraryLoader {
    cache: FxHashMap<PathBuf, LoadedLibrary>,
}

impl LibraryLoader {
    fn load_library(&mut self, root: &Path) -> io::Result<&LoadedLibrary> {
        let root = fs::canonicalize(root)?;
        if !self.cache.contains_key(&root) {
            let library = Self::build_library(&root)?;
            self.cache.insert(root.clone(), library);
        }

        Ok(self
            .cache
            .get(&root)
            .expect("library cache should contain requested root"))
    }

    fn build_library(root: &Path) -> io::Result<LoadedLibrary> {
        let mut files = Vec::new();
        let mut paths = Vec::new();
        collect_nx_files(root, &mut paths)?;
        paths.sort();

        for path in paths {
            let parse_result = parse_file(&path)?;
            let tree = match parse_result.tree {
                Some(tree) => tree,
                None => continue,
            };

            let module = crate::lower(
                tree.root(),
                crate::SourceId::new(parse_result.source_id.as_u32()),
            );
            files.push(LoadedFile { path, module });
        }

        let mut public_items: FxHashMap<String, Vec<ItemRef>> = FxHashMap::default();
        let mut visible_to_library_items: FxHashMap<String, Vec<ItemRef>> = FxHashMap::default();

        for (file_index, file) in files.iter().enumerate() {
            for (item_index, item) in file.module.items().iter().enumerate() {
                let item_ref = ItemRef {
                    file_index,
                    item_index,
                };

                if item.visibility() == Visibility::Public {
                    public_items
                        .entry(item.name().as_str().to_string())
                        .or_default()
                        .push(item_ref);
                }

                if item.visibility() != Visibility::Private {
                    visible_to_library_items
                        .entry(item.name().as_str().to_string())
                        .or_default()
                        .push(item_ref);
                }
            }
        }

        Ok(LoadedLibrary {
            root: root.to_path_buf(),
            files,
            public_items,
            visible_to_library_items,
        })
    }
}

fn collect_nx_files(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_nx_files(&path, out)?;
        } else if file_type.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("nx")
        {
            out.push(path);
        }
    }

    Ok(())
}

fn is_http_library_path(path: &str) -> bool {
    path.starts_with("http://") || path.starts_with("https://")
}

fn is_git_library_path(path: &str) -> bool {
    path.starts_with("git://")
}

fn normalize_local_library_path(base_file: &Path, library_path: &str) -> io::Result<PathBuf> {
    let candidate = if Path::new(library_path).is_absolute() {
        PathBuf::from(library_path)
    } else {
        base_file
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(library_path)
    };

    fs::canonicalize(candidate)
}

fn candidate_origin(candidate: &Candidate, visible_name: &str) -> String {
    format!(
        "{} ({})",
        visible_name,
        candidate
            .source_file
            .strip_prefix(&candidate.library_root)
            .unwrap_or(candidate.source_file.as_path())
            .display()
    )
}

fn add_ambiguous_binding_diagnostic(
    module: &mut LoweredModule,
    visible_name: &str,
    span: nx_diagnostics::TextSpan,
    candidates: &[Candidate],
) {
    let sources = candidates
        .iter()
        .take(2)
        .map(|candidate| candidate_origin(candidate, visible_name))
        .collect::<Vec<_>>()
        .join(" and ");

    module.add_diagnostic(crate::LoweringDiagnostic {
        message: format!(
            "Ambiguous imported name '{}' could refer to {}. Resolve this by switching to a selective import with `as Prefix.{}` or by using a namespace import.",
            visible_name, sources, visible_name
        ),
        span,
    });
}

fn collect_local_bindings_from_stmt(stmt: &Stmt, bindings: &mut FxHashSet<String>) {
    if let Stmt::Let { name, .. } = stmt {
        bindings.insert(name.as_str().to_string());
    }
}

fn flattened_member_name(module: &LoweredModule, expr_id: ExprId) -> Option<String> {
    match module.expr(expr_id) {
        Expr::Ident(name) => Some(name.as_str().to_string()),
        Expr::Member { base, member, .. } => {
            let mut base_name = flattened_member_name(module, *base)?;
            base_name.push('.');
            base_name.push_str(member.as_str());
            Some(base_name)
        }
        _ => None,
    }
}

struct AmbiguityVisitor<'a> {
    module: &'a mut LoweredModule,
    ambiguous: &'a FxHashMap<String, Vec<Candidate>>,
    locals: Vec<FxHashSet<String>>,
}

impl<'a> AmbiguityVisitor<'a> {
    fn new(
        module: &'a mut LoweredModule,
        ambiguous: &'a FxHashMap<String, Vec<Candidate>>,
    ) -> Self {
        Self {
            module,
            ambiguous,
            locals: vec![FxHashSet::default()],
        }
    }

    fn push_scope(&mut self) {
        self.locals.push(FxHashSet::default());
    }

    fn pop_scope(&mut self) {
        self.locals.pop();
    }

    fn bind(&mut self, name: &Name) {
        if let Some(scope) = self.locals.last_mut() {
            scope.insert(name.as_str().to_string());
        }
    }

    fn is_shadowed(&self, name: &str) -> bool {
        self.locals.iter().rev().any(|scope| scope.contains(name))
    }

    fn visit_item(&mut self, item: &Item) {
        match item {
            Item::Function(function) => {
                self.visit_type_refs(function.return_type.as_ref().into_iter(), function.span);
                self.push_scope();
                for param in &function.params {
                    self.bind(&param.name);
                }
                self.visit_expr(function.body, function.span);
                self.pop_scope();
            }
            Item::Value(value) => {
                self.visit_type_refs(value.ty.as_ref().into_iter(), value.span);
                self.visit_expr(value.value, value.span);
            }
            Item::Component(component) => {
                for prop in &component.props {
                    self.visit_type_ref(&prop.ty, prop.span);
                    if let Some(default) = prop.default {
                        self.visit_expr(default, prop.span);
                    }
                }
                for state in &component.state {
                    self.visit_type_ref(&state.ty, state.span);
                    if let Some(default) = state.default {
                        self.visit_expr(default, state.span);
                    }
                }
                self.visit_expr(component.body, component.span);
            }
            Item::TypeAlias(alias) => self.visit_type_ref(&alias.ty, alias.span),
            Item::Enum(_) => {}
            Item::Record(record) => {
                if let Some(base) = record.base.as_ref() {
                    self.check_name(base.as_str(), record.span);
                }
                for property in &record.properties {
                    self.visit_type_ref(&property.ty, property.span);
                    if let Some(default) = property.default {
                        self.visit_expr(default, property.span);
                    }
                }
            }
        }
    }

    fn visit_type_refs<'b>(
        &mut self,
        type_refs: impl Iterator<Item = &'b ast::TypeRef>,
        span: nx_diagnostics::TextSpan,
    ) {
        for type_ref in type_refs {
            self.visit_type_ref(type_ref, span);
        }
    }

    fn visit_type_ref(&mut self, type_ref: &ast::TypeRef, span: nx_diagnostics::TextSpan) {
        match type_ref {
            ast::TypeRef::Name(name) => self.check_name(name.as_str(), span),
            ast::TypeRef::Array(inner) | ast::TypeRef::Nullable(inner) => {
                self.visit_type_ref(inner, span);
            }
            ast::TypeRef::Function {
                params,
                return_type,
            } => {
                for param in params {
                    self.visit_type_ref(param, span);
                }
                self.visit_type_ref(return_type, span);
            }
        }
    }

    fn visit_expr(&mut self, expr_id: ExprId, fallback_span: nx_diagnostics::TextSpan) {
        let expr = self.module.expr(expr_id).clone();
        let expr_span = expr.span();
        let span = if expr_span.is_empty() {
            fallback_span
        } else {
            expr_span
        };

        match expr {
            Expr::Literal(_) | Expr::Error(_) => {}
            Expr::Ident(name) => {
                if !self.is_shadowed(name.as_str()) {
                    self.check_name(name.as_str(), span);
                }
            }
            Expr::BinaryOp { lhs, rhs, .. } => {
                self.visit_expr(lhs, span);
                self.visit_expr(rhs, span);
            }
            Expr::UnaryOp { expr, .. } => self.visit_expr(expr, span),
            Expr::Call { func, args, .. } => {
                if let Some(name) = flattened_member_name(self.module, func) {
                    self.check_name(&name, span);
                } else {
                    self.visit_expr(func, span);
                }
                for arg in args {
                    self.visit_expr(arg, span);
                }
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.visit_expr(condition, span);
                self.visit_expr(then_branch, span);
                if let Some(else_branch) = else_branch {
                    self.visit_expr(else_branch, span);
                }
            }
            Expr::Let {
                name, value, body, ..
            } => {
                self.visit_expr(value, span);
                self.push_scope();
                self.bind(&name);
                self.visit_expr(body, span);
                self.pop_scope();
            }
            Expr::Block { stmts, expr, .. } => {
                self.push_scope();
                for stmt in &stmts {
                    match stmt {
                        Stmt::Let { ty, init, span, .. } => {
                            if let Some(ty) = ty.as_ref() {
                                self.visit_type_ref(ty, *span);
                            }
                            self.visit_expr(*init, *span);
                            collect_local_bindings_from_stmt(
                                stmt,
                                self.locals.last_mut().expect("scope"),
                            );
                        }
                        Stmt::Expr(expr_id, stmt_span) => self.visit_expr(*expr_id, *stmt_span),
                    }
                }
                if let Some(expr) = expr {
                    self.visit_expr(expr, span);
                }
                self.pop_scope();
            }
            Expr::Array { elements, .. } => {
                for element in elements {
                    self.visit_expr(element, span);
                }
            }
            Expr::Index { base, index, .. } => {
                self.visit_expr(base, span);
                self.visit_expr(index, span);
            }
            Expr::Member { base, .. } => {
                if let Some(name) = flattened_member_name(self.module, expr_id) {
                    self.check_name(&name, span);
                } else {
                    self.visit_expr(base, span);
                }
            }
            Expr::RecordLiteral {
                record, properties, ..
            } => {
                self.check_name(record.as_str(), span);
                for (_, value) in properties {
                    self.visit_expr(value, span);
                }
            }
            Expr::Element { element, .. } => {
                let element = self.module.element(element).clone();
                self.check_name(element.tag.as_str(), element.span);
                for property in &element.properties {
                    self.visit_expr(property.value, property.span);
                }
                for child in &element.children {
                    self.visit_expr(*child, element.span);
                }
            }
            Expr::ActionHandler { body, .. } => {
                self.push_scope();
                self.bind(&Name::new("action"));
                self.visit_expr(body, span);
                self.pop_scope();
            }
            Expr::For {
                item,
                index,
                iterable,
                body,
                ..
            } => {
                self.visit_expr(iterable, span);
                self.push_scope();
                self.bind(&item);
                if let Some(index) = index.as_ref() {
                    self.bind(index);
                }
                self.visit_expr(body, span);
                self.pop_scope();
            }
        }
    }

    fn check_name(&mut self, name: &str, span: nx_diagnostics::TextSpan) {
        if let Some(candidates) = self.ambiguous.get(name) {
            add_ambiguous_binding_diagnostic(self.module, name, span, candidates);
        }
    }
}

struct ModuleCopier<'dst, 'src> {
    dst: &'dst mut LoweredModule,
    src: &'src LoweredModule,
    expr_map: FxHashMap<ExprId, ExprId>,
    element_map: FxHashMap<ElementId, ElementId>,
}

impl<'dst, 'src> ModuleCopier<'dst, 'src> {
    fn new(dst: &'dst mut LoweredModule, src: &'src LoweredModule) -> Self {
        Self {
            dst,
            src,
            expr_map: FxHashMap::default(),
            element_map: FxHashMap::default(),
        }
    }

    fn copy_item(&mut self, item: &Item, visible_name: &str) -> Item {
        match item {
            Item::Function(function) => Item::Function(Function {
                name: Name::new(visible_name),
                visibility: function.visibility,
                params: function.params.clone(),
                return_type: function.return_type.clone(),
                body: self.copy_expr(function.body),
                span: function.span,
            }),
            Item::Value(value) => Item::Value(ValueDef {
                name: Name::new(visible_name),
                visibility: value.visibility,
                ty: value.ty.clone(),
                value: self.copy_expr(value.value),
                span: value.span,
            }),
            Item::Component(component) => Item::Component(Component {
                name: Name::new(visible_name),
                visibility: component.visibility,
                props: component
                    .props
                    .iter()
                    .map(|field| self.copy_record_field(field))
                    .collect(),
                emits: component.emits.clone(),
                state: component
                    .state
                    .iter()
                    .map(|field| self.copy_record_field(field))
                    .collect(),
                body: self.copy_expr(component.body),
                span: component.span,
            }),
            Item::TypeAlias(alias) => Item::TypeAlias(TypeAlias {
                name: Name::new(visible_name),
                visibility: alias.visibility,
                ty: alias.ty.clone(),
                span: alias.span,
            }),
            Item::Enum(enum_def) => Item::Enum(EnumDef {
                name: Name::new(visible_name),
                visibility: enum_def.visibility,
                members: enum_def.members.clone(),
                span: enum_def.span,
            }),
            Item::Record(record) => Item::Record(RecordDef {
                name: Name::new(visible_name),
                visibility: record.visibility,
                kind: record.kind,
                is_abstract: record.is_abstract,
                base: record.base.clone(),
                properties: record
                    .properties
                    .iter()
                    .map(|field| self.copy_record_field(field))
                    .collect(),
                span: record.span,
            }),
        }
    }

    fn copy_record_field(&mut self, field: &RecordField) -> RecordField {
        RecordField {
            name: field.name.clone(),
            ty: field.ty.clone(),
            default: field.default.map(|expr| self.copy_expr(expr)),
            span: field.span,
        }
    }

    fn copy_stmt(&mut self, stmt: &Stmt) -> Stmt {
        match stmt {
            Stmt::Let {
                name,
                ty,
                init,
                span,
            } => Stmt::Let {
                name: name.clone(),
                ty: ty.clone(),
                init: self.copy_expr(*init),
                span: *span,
            },
            Stmt::Expr(expr, span) => Stmt::Expr(self.copy_expr(*expr), *span),
        }
    }

    fn copy_expr(&mut self, expr_id: ExprId) -> ExprId {
        if let Some(&mapped) = self.expr_map.get(&expr_id) {
            return mapped;
        }

        let expr = match self.src.expr(expr_id).clone() {
            Expr::Literal(literal) => Expr::Literal(literal),
            Expr::Ident(name) => Expr::Ident(name),
            Expr::BinaryOp { lhs, op, rhs, span } => Expr::BinaryOp {
                lhs: self.copy_expr(lhs),
                op,
                rhs: self.copy_expr(rhs),
                span,
            },
            Expr::UnaryOp { op, expr, span } => Expr::UnaryOp {
                op,
                expr: self.copy_expr(expr),
                span,
            },
            Expr::Call { func, args, span } => Expr::Call {
                func: self.copy_expr(func),
                args: args.into_iter().map(|arg| self.copy_expr(arg)).collect(),
                span,
            },
            Expr::If {
                condition,
                then_branch,
                else_branch,
                span,
            } => Expr::If {
                condition: self.copy_expr(condition),
                then_branch: self.copy_expr(then_branch),
                else_branch: else_branch.map(|branch| self.copy_expr(branch)),
                span,
            },
            Expr::Let {
                name,
                value,
                body,
                span,
            } => Expr::Let {
                name,
                value: self.copy_expr(value),
                body: self.copy_expr(body),
                span,
            },
            Expr::Block { stmts, expr, span } => Expr::Block {
                stmts: stmts.iter().map(|stmt| self.copy_stmt(stmt)).collect(),
                expr: expr.map(|expr| self.copy_expr(expr)),
                span,
            },
            Expr::Array { elements, span } => Expr::Array {
                elements: elements
                    .into_iter()
                    .map(|element| self.copy_expr(element))
                    .collect(),
                span,
            },
            Expr::Index { base, index, span } => Expr::Index {
                base: self.copy_expr(base),
                index: self.copy_expr(index),
                span,
            },
            Expr::Member { base, member, span } => Expr::Member {
                base: self.copy_expr(base),
                member,
                span,
            },
            Expr::RecordLiteral {
                record,
                properties,
                span,
            } => Expr::RecordLiteral {
                record,
                properties: properties
                    .into_iter()
                    .map(|(name, value)| (name, self.copy_expr(value)))
                    .collect(),
                span,
            },
            Expr::Element { element, span } => Expr::Element {
                element: self.copy_element(element),
                span,
            },
            Expr::ActionHandler {
                component,
                emit,
                action_name,
                body,
                span,
            } => Expr::ActionHandler {
                component,
                emit,
                action_name,
                body: self.copy_expr(body),
                span,
            },
            Expr::For {
                item,
                index,
                iterable,
                body,
                span,
            } => Expr::For {
                item,
                index,
                iterable: self.copy_expr(iterable),
                body: self.copy_expr(body),
                span,
            },
            Expr::Error(span) => Expr::Error(span),
        };

        let mapped = self.dst.alloc_expr(expr);
        self.expr_map.insert(expr_id, mapped);
        mapped
    }

    fn copy_element(&mut self, element_id: ElementId) -> ElementId {
        if let Some(&mapped) = self.element_map.get(&element_id) {
            return mapped;
        }

        let element = self.src.element(element_id).clone();
        let copied = Element {
            tag: element.tag,
            properties: element
                .properties
                .into_iter()
                .map(|property| Property {
                    key: property.key,
                    value: self.copy_expr(property.value),
                    span: property.span,
                })
                .collect(),
            children: element
                .children
                .into_iter()
                .map(|child| self.copy_expr(child))
                .collect(),
            close_name: element.close_name,
            span: element.span,
        };

        let mapped = self.dst.alloc_element(copied);
        self.element_map.insert(element_id, mapped);
        mapped
    }
}

fn visible_name_for_selective(entry: &SelectiveImport) -> String {
    match entry.qualifier.as_ref() {
        Some(prefix) => format!("{}.{}", prefix.as_str(), entry.name.as_str()),
        None => entry.name.as_str().to_string(),
    }
}

fn line_col_for_span(source: &str, span: nx_diagnostics::TextSpan) -> Option<(usize, usize)> {
    let start: usize = span.start().into();
    if start > source.len() {
        return None;
    }

    let mut line = 1usize;
    let mut column = 1usize;
    for (byte_index, ch) in source.char_indices() {
        if byte_index >= start {
            break;
        }

        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    Some((line, column))
}

/// Resolve peer-library and imported-library declarations into a lowered entry module.
pub fn resolve_local_library_imports(
    mut module: LoweredModule,
    root_path: &Path,
) -> io::Result<LoweredModule> {
    let root_path = fs::canonicalize(root_path)?;
    let root_source = fs::read_to_string(&root_path).ok();
    let current_library_root = root_path
        .parent()
        .map(fs::canonicalize)
        .transpose()?
        .unwrap_or_else(|| root_path.clone());

    let mut loader = LibraryLoader::default();
    let current_library = loader.load_library(&current_library_root)?.clone();

    let mut root_item_names = module
        .items()
        .iter()
        .map(|item| item.name().as_str().to_string())
        .collect::<FxHashSet<_>>();
    let mut candidates: FxHashMap<String, Vec<Candidate>> = FxHashMap::default();
    let mut ambiguous: FxHashMap<String, Vec<Candidate>> = FxHashMap::default();

    for (file_index, file) in current_library.files.iter().enumerate() {
        if file.path == root_path {
            continue;
        }

        if !file.module.imports.is_empty() {
            module.add_diagnostic(crate::LoweringDiagnostic {
                message: format!(
                    "Library file '{}' contains imports; nested library imports are not yet supported during import resolution",
                    file.path.display()
                ),
                span: nx_diagnostics::TextSpan::default(),
            });
        }

        for (name, item_refs) in &current_library.visible_to_library_items {
            for item_ref in item_refs {
                if item_ref.file_index != file_index {
                    continue;
                }
                candidates.entry(name.clone()).or_default().push(Candidate {
                    library_root: current_library.root.clone(),
                    source_file: file.path.clone(),
                    item: *item_ref,
                });
            }
        }
    }

    let mut seen_import_roots: FxHashMap<String, nx_diagnostics::TextSpan> = FxHashMap::default();

    for import in module.imports.clone() {
        if is_git_library_path(&import.library_path) {
            module.add_diagnostic(crate::LoweringDiagnostic {
                message: format!(
                    "Git library imports are not yet supported: '{}'",
                    import.library_path
                ),
                span: import.span,
            });
            continue;
        }

        if is_http_library_path(&import.library_path) {
            module.add_diagnostic(crate::LoweringDiagnostic {
                message: format!(
                    "HTTP zip library imports are not yet supported: '{}'",
                    import.library_path
                ),
                span: import.span,
            });
            continue;
        }

        let normalized_root = match normalize_local_library_path(&root_path, &import.library_path) {
            Ok(path) => path,
            Err(_) => {
                module.add_diagnostic(crate::LoweringDiagnostic {
                    message: format!(
                        "Local library import '{}' could not be resolved to a directory",
                        import.library_path
                    ),
                    span: import.span,
                });
                continue;
            }
        };

        if !normalized_root.is_dir() {
            module.add_diagnostic(crate::LoweringDiagnostic {
                message: format!(
                    "Local library import '{}' must resolve to a directory",
                    import.library_path
                ),
                span: import.span,
            });
            continue;
        }

        let normalized_key = normalized_root.to_string_lossy().to_string();
        if let Some(first_import_span) =
            seen_import_roots.insert(normalized_key.clone(), import.span)
        {
            let first_import_location = root_source
                .as_deref()
                .and_then(|source| line_col_for_span(source, first_import_span))
                .map(|(line, column)| {
                    format!("; first imported at line {}, column {}", line, column)
                })
                .unwrap_or_default();
            module.add_diagnostic(crate::LoweringDiagnostic {
                message: format!(
                    "Library '{}' is imported more than once in this file{}",
                    normalized_root.display(),
                    first_import_location
                ),
                span: import.span,
            });
            continue;
        }

        let library = loader.load_library(&normalized_root)?.clone();
        for file in &library.files {
            if !file.module.imports.is_empty() {
                module.add_diagnostic(crate::LoweringDiagnostic {
                    message: format!(
                        "Library file '{}' contains imports; nested library imports are not yet supported during import resolution",
                        file.path.display()
                    ),
                    span: import.span,
                });
            }
        }

        match &import.kind {
            ImportKind::Wildcard { alias } => {
                let items = &library.public_items;
                for (name, item_refs) in items {
                    let visible_name = match alias.as_ref() {
                        Some(prefix) => format!("{}.{}", prefix.as_str(), name),
                        None => name.clone(),
                    };

                    for item_ref in item_refs {
                        let source_file = library.files[item_ref.file_index].path.clone();
                        candidates
                            .entry(visible_name.clone())
                            .or_default()
                            .push(Candidate {
                                library_root: library.root.clone(),
                                source_file,
                                item: *item_ref,
                            });
                    }
                }
            }
            ImportKind::Selective { entries } => {
                for entry in entries {
                    let matches = library.public_items.get(entry.name.as_str());
                    if matches.is_none() {
                        module.add_diagnostic(crate::LoweringDiagnostic {
                            message: format!(
                                "Library '{}' does not export '{}'",
                                normalized_root.display(),
                                entry.name.as_str()
                            ),
                            span: entry.span,
                        });
                        continue;
                    }

                    let visible_name = visible_name_for_selective(entry);
                    for item_ref in matches.expect("checked above") {
                        let source_file = library.files[item_ref.file_index].path.clone();
                        candidates
                            .entry(visible_name.clone())
                            .or_default()
                            .push(Candidate {
                                library_root: library.root.clone(),
                                source_file,
                                item: *item_ref,
                            });
                    }
                }
            }
        }
    }

    for (visible_name, binding_candidates) in candidates {
        if root_item_names.contains(&visible_name) {
            continue;
        }

        if binding_candidates.len() > 1 {
            ambiguous.insert(visible_name, binding_candidates);
            continue;
        }

        let candidate = &binding_candidates[0];
        let library = loader.load_library(&candidate.library_root)?.clone();
        let loaded_file = &library.files[candidate.item.file_index];
        let item = &loaded_file.module.items()[candidate.item.item_index];

        let mut copier = ModuleCopier::new(&mut module, &loaded_file.module);
        let copied = copier.copy_item(item, &visible_name);
        root_item_names.insert(visible_name);
        module.add_item(copied);
    }

    let local_items: Vec<_> = module.items().to_vec();
    let mut visitor = AmbiguityVisitor::new(&mut module, &ambiguous);
    for item in &local_items {
        visitor.visit_item(item);
    }

    Ok(module)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SourceId;
    use nx_syntax::parse_file;
    use std::fs;
    use tempfile::TempDir;

    fn lower_and_resolve_imports(path: &Path) -> LoweredModule {
        let parse_result = parse_file(path).expect("source file should load");
        let tree = parse_result.tree.expect("source file should parse");
        let module = crate::lower(tree.root(), SourceId::new(parse_result.source_id.as_u32()));
        resolve_local_library_imports(module, path)
            .expect("library import resolution should succeed")
    }

    #[test]
    fn test_resolve_selective_import_with_qualified_prefix() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        let layout_dir = ui_dir.join("layout");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&layout_dir).expect("layout dir");

        fs::write(
            layout_dir.join("button.nx"),
            r#"let <Button /> = <button />"#,
        )
        .expect("library file");
        fs::write(
            app_dir.join("main.nx"),
            r#"import { Button as Layout.Button } from "../ui"
let root() = { <Layout.Button /> }"#,
        )
        .expect("root file");

        let module = lower_and_resolve_imports(&app_dir.join("main.nx"));

        assert!(
            module.find_item("Layout.Button").is_some(),
            "Expected qualified selective import to be resolved into the module"
        );
        assert!(
            module.diagnostics().is_empty(),
            "Unexpected diagnostics: {:?}",
            module.diagnostics()
        );
    }

    #[test]
    fn test_resolve_same_library_exposes_internal_but_not_private() {
        let temp = TempDir::new().expect("temp dir");
        let lib_dir = temp.path().join("lib");
        fs::create_dir_all(&lib_dir).expect("lib dir");

        fs::write(
            lib_dir.join("helpers.nx"),
            r#"internal let formatName: string = "NX"
private let secretName: string = "hidden""#,
        )
        .expect("helpers file");
        fs::write(lib_dir.join("main.nx"), r#"let root() = { formatName }"#).expect("main file");

        let module = lower_and_resolve_imports(&lib_dir.join("main.nx"));

        assert!(module.find_item("formatName").is_some());
        assert!(module.find_item("secretName").is_none());
        assert!(
            module.diagnostics().is_empty(),
            "Unexpected diagnostics: {:?}",
            module.diagnostics()
        );
    }

    #[test]
    fn test_resolve_same_library_exposes_internal_enum_but_not_private_enum() {
        let temp = TempDir::new().expect("temp dir");
        let lib_dir = temp.path().join("lib");
        fs::create_dir_all(&lib_dir).expect("lib dir");

        fs::write(
            lib_dir.join("shared.nx"),
            r#"internal enum SharedMode = Light | Dark
private enum HiddenMode = Secret | Hidden"#,
        )
        .expect("shared file");
        fs::write(lib_dir.join("main.nx"), r#"let root() = { <div /> }"#).expect("main file");

        let module = lower_and_resolve_imports(&lib_dir.join("main.nx"));

        assert!(module.find_item("SharedMode").is_some());
        assert!(module.find_item("HiddenMode").is_none());
        assert!(
            module.diagnostics().is_empty(),
            "Unexpected diagnostics: {:?}",
            module.diagnostics()
        );
    }

    #[test]
    fn test_resolve_duplicate_library_import_reports_diagnostic() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(ui_dir.join("button.nx"), r#"let <Button /> = <button />"#).expect("ui file");
        fs::write(
            app_dir.join("main.nx"),
            r#"import "../ui"
import "../app/../ui"
let root() = { <div /> }"#,
        )
        .expect("root file");

        let module = lower_and_resolve_imports(&app_dir.join("main.nx"));
        let duplicate_import = module
            .diagnostics()
            .iter()
            .find(|diagnostic| diagnostic.message.contains("imported more than once"))
            .expect("Expected duplicate import diagnostic");

        assert!(
            duplicate_import
                .message
                .contains("first imported at line 1, column 1"),
            "Expected duplicate import diagnostic to reference the first import, got {:?}",
            duplicate_import
        );
    }

    #[test]
    fn test_resolve_missing_selective_import_reports_diagnostic() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(ui_dir.join("button.nx"), r#"let <Button /> = <button />"#).expect("ui file");
        fs::write(
            app_dir.join("main.nx"),
            r#"import { NonExistent } from "../ui"
let root() = { <div /> }"#,
        )
        .expect("root file");

        let module = lower_and_resolve_imports(&app_dir.join("main.nx"));

        assert!(
            module
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.message.contains("does not export 'NonExistent'")),
            "Expected missing selective import diagnostic, got {:?}",
            module.diagnostics()
        );
    }

    #[test]
    fn test_resolve_ambiguous_import_is_reported_only_when_used() {
        let temp = TempDir::new().expect("temp dir");
        let unused_app_dir = temp.path().join("unused-app");
        let used_app_dir = temp.path().join("used-app");
        let ui_dir = temp.path().join("ui");
        let forms_dir = temp.path().join("forms");
        fs::create_dir_all(&unused_app_dir).expect("unused app dir");
        fs::create_dir_all(&used_app_dir).expect("used app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");
        fs::create_dir_all(&forms_dir).expect("forms dir");

        fs::write(ui_dir.join("button.nx"), r#"let <Button /> = <button />"#).expect("ui file");
        fs::write(forms_dir.join("button.nx"), r#"let <Button /> = <input />"#)
            .expect("forms file");

        fs::write(
            unused_app_dir.join("main.nx"),
            r#"import "../ui"
import "../forms"
let root() = { <div /> }"#,
        )
        .expect("unused root file");
        fs::write(
            used_app_dir.join("main.nx"),
            r#"import "../ui"
import "../forms"
let root() = { <Button /> }"#,
        )
        .expect("used root file");

        let unused_module = lower_and_resolve_imports(&unused_app_dir.join("main.nx"));
        assert!(
            unused_module.diagnostics().is_empty(),
            "Unused ambiguity should not emit diagnostics, got {:?}",
            unused_module.diagnostics()
        );

        let used_module = lower_and_resolve_imports(&used_app_dir.join("main.nx"));
        assert!(
            used_module.diagnostics().iter().any(|diagnostic| diagnostic
                .message
                .contains("Ambiguous imported name 'Button'")),
            "Expected deferred ambiguity diagnostic, got {:?}",
            used_module.diagnostics()
        );
    }

    #[test]
    fn test_resolve_remote_import_reports_not_supported() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::write(
            app_dir.join("main.nx"),
            r#"import "https://example.com/ui.zip"
let root() = { <div /> }"#,
        )
        .expect("root file");

        let module = lower_and_resolve_imports(&app_dir.join("main.nx"));
        assert!(
            module.diagnostics().iter().any(|diagnostic| diagnostic
                .message
                .contains("HTTP zip library imports are not yet supported")),
            "Expected remote import diagnostic, got {:?}",
            module.diagnostics()
        );
    }
}
