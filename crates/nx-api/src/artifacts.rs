use nx_diagnostics::{Diagnostic, Label, Severity, TextSize, TextSpan};
use nx_hir::{
    ast::{Expr, Stmt, TypeRef},
    lower, Component, ComponentEmit, Element, ElementId, EnumDef, EnumMember, Function, Import,
    ImportKind, Item, LoweredModule, LoweringDiagnostic, Name, Param, Property, RecordDef,
    RecordField, RecordKind, SelectiveImport, SourceId, TypeAlias, ValueDef, Visibility,
};
use nx_interpreter::{
    ModuleQualifiedItemRef, ResolvedItemKind, ResolvedModule, ResolvedProgram, RuntimeModuleId,
};
use nx_syntax::parse_str as syntax_parse_str;
use nx_types::{analyze_prepared_module, ModuleArtifact, Type, TypeEnvironment};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Export metadata for one symbol provided by a library artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryExport {
    pub module_file: String,
    pub item_name: String,
    pub kind: ResolvedItemKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryInterfaceParam {
    pub name: Name,
    pub ty: TypeRef,
    pub span: TextSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryInterfaceField {
    pub name: Name,
    pub ty: TypeRef,
    pub span: TextSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryInterfaceKind {
    Function {
        params: Vec<LibraryInterfaceParam>,
        return_type: TypeRef,
        span: TextSpan,
    },
    Value {
        ty: TypeRef,
        span: TextSpan,
    },
    Component {
        props: Vec<LibraryInterfaceField>,
        emits: Vec<ComponentEmit>,
        state: Vec<LibraryInterfaceField>,
        span: TextSpan,
    },
    TypeAlias {
        ty: TypeRef,
        span: TextSpan,
    },
    Enum {
        members: Vec<EnumMember>,
        span: TextSpan,
    },
    Record {
        kind: RecordKind,
        is_abstract: bool,
        base: Option<Name>,
        properties: Vec<LibraryInterfaceField>,
        span: TextSpan,
    },
}

impl LibraryInterfaceKind {
    fn kind(&self) -> ResolvedItemKind {
        match self {
            Self::Function { .. } => ResolvedItemKind::Function,
            Self::Value { .. } => ResolvedItemKind::Value,
            Self::Component { .. } => ResolvedItemKind::Component,
            Self::TypeAlias { .. } => ResolvedItemKind::TypeAlias,
            Self::Enum { .. } => ResolvedItemKind::Enum,
            Self::Record { .. } => ResolvedItemKind::Record,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryInterfaceItem {
    pub module_file: String,
    pub item_name: String,
    pub visibility: Visibility,
    pub item: LibraryInterfaceKind,
}

impl LibraryInterfaceItem {
    fn kind(&self) -> ResolvedItemKind {
        self.item.kind()
    }

    fn synthesize_item(&self, visible_name: &str, module: &mut LoweredModule) -> Item {
        match &self.item {
            LibraryInterfaceKind::Function {
                params,
                return_type,
                span,
            } => Item::Function(Function {
                name: Name::new(visible_name),
                visibility: self.visibility,
                params: params
                    .iter()
                    .map(|param| Param {
                        name: param.name.clone(),
                        ty: param.ty.clone(),
                        span: param.span,
                    })
                    .collect(),
                return_type: Some(return_type.clone()),
                body: module.alloc_expr(nx_hir::ast::Expr::Error(*span)),
                span: *span,
            }),
            LibraryInterfaceKind::Value { ty, span } => Item::Value(ValueDef {
                name: Name::new(visible_name),
                visibility: self.visibility,
                ty: Some(ty.clone()),
                value: module.alloc_expr(nx_hir::ast::Expr::Error(*span)),
                span: *span,
            }),
            LibraryInterfaceKind::Component {
                props,
                emits,
                state,
                span,
            } => Item::Component(Component {
                name: Name::new(visible_name),
                visibility: self.visibility,
                props: props.iter().map(interface_field_to_record_field).collect(),
                emits: emits.clone(),
                state: state.iter().map(interface_field_to_record_field).collect(),
                body: module.alloc_expr(nx_hir::ast::Expr::Error(*span)),
                span: *span,
            }),
            LibraryInterfaceKind::TypeAlias { ty, span } => Item::TypeAlias(TypeAlias {
                name: Name::new(visible_name),
                visibility: self.visibility,
                ty: ty.clone(),
                span: *span,
            }),
            LibraryInterfaceKind::Enum { members, span } => Item::Enum(EnumDef {
                name: Name::new(visible_name),
                visibility: self.visibility,
                members: members.clone(),
                span: *span,
            }),
            LibraryInterfaceKind::Record {
                kind,
                is_abstract,
                base,
                properties,
                span,
            } => Item::Record(RecordDef {
                name: Name::new(visible_name),
                visibility: self.visibility,
                kind: *kind,
                is_abstract: *is_abstract,
                base: base.clone(),
                properties: properties
                    .iter()
                    .map(interface_field_to_record_field)
                    .collect(),
                span: *span,
            }),
        }
    }
}

/// File-preserving artifact for one local NX library directory.
#[derive(Debug, Clone)]
pub struct LibraryArtifact {
    pub root_path: PathBuf,
    pub modules: Vec<ModuleArtifact>,
    pub exports: FxHashMap<String, LibraryExport>,
    pub interface_items: Vec<LibraryInterfaceItem>,
    pub public_items: FxHashMap<String, Vec<usize>>,
    pub visible_to_library_items: FxHashMap<String, Vec<usize>>,
    pub dependency_roots: Vec<PathBuf>,
    pub diagnostics: Vec<Diagnostic>,
    pub fingerprint: u64,
}

/// File-preserving artifact for one resolved NX program.
#[derive(Debug, Clone)]
pub struct ProgramArtifact {
    pub root_modules: Vec<ModuleArtifact>,
    pub libraries: Vec<Arc<LibraryArtifact>>,
    pub diagnostics: Vec<Diagnostic>,
    pub fingerprint: u64,
    pub resolved_program: ResolvedProgram,
}

#[derive(Debug, Default)]
struct LibraryRegistryState {
    libraries: FxHashMap<PathBuf, Arc<LibraryArtifact>>,
    dependency_graph: FxHashMap<PathBuf, Vec<PathBuf>>,
    loading: FxHashSet<PathBuf>,
}

/// Public owner of analyzed library snapshots.
#[derive(Debug, Clone, Default)]
pub struct LibraryRegistry {
    inner: Arc<RwLock<LibraryRegistryState>>,
}

impl LibraryRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_library_from_directory(
        &self,
        root_path: impl AsRef<Path>,
    ) -> Result<Arc<LibraryArtifact>, Vec<crate::NxDiagnostic>> {
        let artifact = self
            .load_library_from_directory_internal(root_path.as_ref())
            .map_err(|error| {
                let diagnostic = Diagnostic::error("library-load-error")
                    .with_message(format!(
                        "Failed to load library artifact from '{}': {}",
                        root_path.as_ref().display(),
                        error
                    ))
                    .build();
                crate::diagnostics::diagnostics_to_api(&[diagnostic], "")
            })?;

        let diagnostics = self.closure_diagnostics(&artifact.root_path);
        if has_error_diagnostics(&diagnostics) {
            return Err(crate::diagnostics::diagnostics_to_api(&diagnostics, ""));
        }

        Ok(artifact)
    }

    pub fn build_context(&self) -> ProgramBuildContext {
        ProgramBuildContext {
            registry: self.clone(),
            visible_roots: self.loaded_roots().into_iter().collect(),
        }
    }

    pub fn build_context_with_visible_roots<I, P>(
        &self,
        roots: I,
    ) -> io::Result<ProgramBuildContext>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut visible_roots = FxHashSet::default();
        for root in roots {
            visible_roots.insert(fs::canonicalize(root.as_ref())?);
        }

        Ok(ProgramBuildContext {
            registry: self.clone(),
            visible_roots,
        })
    }

    fn loaded_roots(&self) -> Vec<PathBuf> {
        let state = self.inner.read().expect("library registry lock poisoned");
        let mut roots = state.libraries.keys().cloned().collect::<Vec<_>>();
        roots.sort();
        roots
    }

    fn get_loaded_library(&self, root: &Path) -> Option<Arc<LibraryArtifact>> {
        let state = self.inner.read().expect("library registry lock poisoned");
        state.libraries.get(root).cloned()
    }

    fn load_library_from_directory_internal(
        &self,
        root_path: &Path,
    ) -> io::Result<Arc<LibraryArtifact>> {
        let mut loading_stack = Vec::new();
        self.load_library_from_directory_internal_with_stack(root_path, &mut loading_stack)
    }

    fn load_library_from_directory_internal_with_stack(
        &self,
        root_path: &Path,
        loading_stack: &mut Vec<PathBuf>,
    ) -> io::Result<Arc<LibraryArtifact>> {
        let root_path = fs::canonicalize(root_path)?;
        if let Some(existing) = self.get_loaded_library(&root_path) {
            return Ok(existing);
        }

        if let Some(index) = loading_stack.iter().position(|path| path == &root_path) {
            let mut cycle = loading_stack[index..].to_vec();
            cycle.push(root_path.clone());
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                circular_library_dependency_message(&cycle),
            ));
        }

        {
            let mut state = self.inner.write().expect("library registry lock poisoned");
            if !state.loading.insert(root_path.clone()) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Circular library dependency detected while loading '{}'",
                        root_path.display()
                    ),
                ));
            }
        }

        loading_stack.push(root_path.clone());
        let result = (|| {
            let dependency_roots = discover_library_dependency_roots(&root_path)?;
            for dependency_root in &dependency_roots {
                let _ = self.load_library_from_directory_internal_with_stack(
                    dependency_root,
                    loading_stack,
                )?;
            }

            let artifact = Arc::new(build_library_artifact_with_registry(&root_path, self)?);
            let mut state = self.inner.write().expect("library registry lock poisoned");
            let entry = state
                .libraries
                .entry(root_path.clone())
                .or_insert_with(|| artifact.clone())
                .clone();
            state
                .dependency_graph
                .insert(root_path.clone(), artifact.dependency_roots.clone());
            Ok(entry)
        })();

        let popped = loading_stack.pop();
        debug_assert_eq!(popped.as_deref(), Some(root_path.as_path()));

        let mut state = self.inner.write().expect("library registry lock poisoned");
        state.loading.remove(&root_path);
        result
    }

    fn dependency_roots(&self, root: &Path) -> Vec<PathBuf> {
        let state = self.inner.read().expect("library registry lock poisoned");
        state
            .dependency_graph
            .get(root)
            .cloned()
            .unwrap_or_default()
    }

    fn closure_diagnostics(&self, root: &Path) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut seen = FxHashSet::default();
        let mut queue = vec![root.to_path_buf()];

        while let Some(current) = queue.pop() {
            if !seen.insert(current.clone()) {
                continue;
            }

            if let Some(library) = self.get_loaded_library(&current) {
                diagnostics.extend(library.diagnostics.iter().cloned());
            }

            queue.extend(self.dependency_roots(&current));
        }

        diagnostics
    }
}

/// Registry-backed build scope used for one program build or tenant.
#[derive(Debug, Clone)]
pub struct ProgramBuildContext {
    registry: LibraryRegistry,
    visible_roots: FxHashSet<PathBuf>,
}

impl Default for ProgramBuildContext {
    fn default() -> Self {
        Self::empty()
    }
}

impl ProgramBuildContext {
    pub fn empty() -> Self {
        Self {
            registry: LibraryRegistry::new(),
            visible_roots: FxHashSet::default(),
        }
    }

    pub fn from_registry(registry: &LibraryRegistry) -> Self {
        registry.build_context()
    }

    fn visible_library(&self, root: &Path) -> Option<Arc<LibraryArtifact>> {
        if !self.visible_roots.contains(root) {
            return None;
        }

        self.registry.get_loaded_library(root)
    }
}

#[derive(Debug, Clone)]
struct LibrarySourceFile {
    file_name: String,
    path: PathBuf,
    source: String,
    source_id: SourceId,
    diagnostics: Vec<Diagnostic>,
    preserved_module: Option<LoweredModule>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct LocalLibraryItemRef {
    file_index: usize,
    item_index: usize,
}

#[derive(Debug, Clone)]
struct LocalLibraryCandidate {
    source_file: PathBuf,
    item: LocalLibraryItemRef,
}

#[derive(Debug, Clone)]
struct ResolvedBuildContextImport {
    normalized_root: PathBuf,
    library: Arc<LibraryArtifact>,
}

#[derive(Debug, Clone)]
struct PendingProgramLibrary {
    root: PathBuf,
    chain: Vec<PathBuf>,
    library: Option<Arc<LibraryArtifact>>,
}

#[derive(Debug, Default)]
struct ProgramLibrarySelection {
    libraries: Vec<Arc<LibraryArtifact>>,
    diagnostics: Vec<Diagnostic>,
}

struct PreparedRootModule {
    artifact: ModuleArtifact,
    libraries: Vec<Arc<LibraryArtifact>>,
    diagnostics: Vec<Diagnostic>,
}

struct ModuleCopier<'dst, 'src> {
    dst: &'dst mut LoweredModule,
    src: &'src LoweredModule,
    expr_map: FxHashMap<nx_hir::ExprId, nx_hir::ExprId>,
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

    fn copy_expr(&mut self, expr_id: nx_hir::ExprId) -> nx_hir::ExprId {
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

/// Builds a file-preserving library artifact from a local directory.
pub fn build_library_artifact_from_directory(
    root_path: impl AsRef<Path>,
) -> io::Result<LibraryArtifact> {
    let registry = LibraryRegistry::new();
    let artifact = registry.load_library_from_directory_internal(root_path.as_ref())?;
    Ok((*artifact).clone())
}

fn build_library_artifact_with_registry(
    root_path: &Path,
    registry: &LibraryRegistry,
) -> io::Result<LibraryArtifact> {
    let root_path = fs::canonicalize(root_path)?;
    let source_files = read_library_source_files(&root_path)?;
    let mut hasher = DefaultHasher::new();
    root_path.hash(&mut hasher);

    let mut dependency_roots = FxHashSet::default();
    for source_file in &source_files {
        source_file.path.hash(&mut hasher);
        source_file.source.hash(&mut hasher);

        if let Some(module) = source_file.preserved_module.as_ref() {
            collect_library_dependencies(&module.imports, &source_file.path, &mut dependency_roots);
        }
    }

    let mut dependency_roots = dependency_roots.into_iter().collect::<Vec<_>>();
    dependency_roots.sort();

    let dependency_context = registry.build_context();
    let prepared_modules = source_files
        .iter()
        .map(|source_file| {
            source_file.preserved_module.as_ref().map(|module| {
                let mut prepared_module = module.clone();
                apply_build_context_imports(
                    &mut prepared_module,
                    &source_file.path,
                    &dependency_context,
                    &source_file.source,
                );
                prepared_module
            })
        })
        .collect::<Vec<_>>();

    let mut modules = Vec::with_capacity(source_files.len());
    let mut exports = FxHashMap::default();
    let mut interface_items = Vec::new();
    let mut public_items = FxHashMap::default();
    let mut visible_to_library_items = FxHashMap::default();
    let mut diagnostics = Vec::new();

    for (index, source_file) in source_files.iter().enumerate() {
        let artifact =
            analyze_library_source_file(&root_path, &source_files, &prepared_modules, index);
        diagnostics.extend(artifact.diagnostics.iter().cloned());

        if let Some(module) = artifact.lowered_module.as_ref() {
            for item in module.items() {
                if let Some(interface_item) = build_interface_item(&artifact, item) {
                    let interface_index = interface_items.len();
                    if item.visibility() != Visibility::Private {
                        visible_to_library_items
                            .entry(item.name().as_str().to_string())
                            .or_insert_with(Vec::new)
                            .push(interface_index);
                    }
                    if item.visibility() == Visibility::Public {
                        public_items
                            .entry(item.name().as_str().to_string())
                            .or_insert_with(Vec::new)
                            .push(interface_index);
                        exports
                            .entry(item.name().as_str().to_string())
                            .or_insert_with(|| LibraryExport {
                                module_file: artifact.file_name.clone(),
                                item_name: item.name().as_str().to_string(),
                                kind: interface_item.kind(),
                            });
                    }
                    interface_items.push(interface_item);
                }
            }
        }

        let _ = source_file;
        modules.push(artifact);
    }

    Ok(LibraryArtifact {
        root_path,
        modules,
        exports,
        interface_items,
        public_items,
        visible_to_library_items,
        dependency_roots,
        diagnostics,
        fingerprint: hasher.finish(),
    })
}

fn discover_library_dependency_roots(root_path: &Path) -> io::Result<Vec<PathBuf>> {
    let source_files = read_library_source_files(root_path)?;
    let mut dependency_roots = FxHashSet::default();

    for source_file in &source_files {
        if let Some(module) = source_file.preserved_module.as_ref() {
            collect_library_dependencies(&module.imports, &source_file.path, &mut dependency_roots);
        }
    }

    let mut dependency_roots = dependency_roots.into_iter().collect::<Vec<_>>();
    dependency_roots.sort();
    Ok(dependency_roots)
}

fn circular_library_dependency_message(cycle: &[PathBuf]) -> String {
    let chain = cycle
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(" -> ");
    format!("Circular library dependency detected: {}", chain)
}

fn read_library_source_files(root_path: &Path) -> io::Result<Vec<LibrarySourceFile>> {
    let mut source_paths = Vec::new();
    collect_nx_files(root_path, &mut source_paths)?;
    source_paths.sort();

    let mut source_files = Vec::with_capacity(source_paths.len());
    for source_path in source_paths {
        let source = fs::read_to_string(&source_path)?;
        let file_name = source_path.display().to_string();
        let parse_result = syntax_parse_str(&source, &file_name);
        let source_id = SourceId::new(parse_result.source_id.as_u32());
        let diagnostics = normalize_diagnostics_file_name(parse_result.errors, &file_name);
        let preserved_module = parse_result.tree.map(|tree| lower(tree.root(), source_id));

        source_files.push(LibrarySourceFile {
            file_name,
            path: source_path,
            source,
            source_id,
            diagnostics,
            preserved_module,
        });
    }

    Ok(source_files)
}

fn analyze_library_source_file(
    library_root: &Path,
    source_files: &[LibrarySourceFile],
    prepared_modules: &[Option<LoweredModule>],
    current_file_index: usize,
) -> ModuleArtifact {
    let source_file = &source_files[current_file_index];
    let diagnostics = source_file.diagnostics.clone();
    let Some(preserved_module) = source_file.preserved_module.clone() else {
        return parse_failure_artifact(&source_file.file_name, source_file.source_id, diagnostics);
    };
    let mut analysis_module = prepared_modules[current_file_index]
        .clone()
        .expect("prepared module should exist when preserved module exists");
    apply_current_library_items(
        &mut analysis_module,
        library_root,
        source_files,
        prepared_modules,
        current_file_index,
    );
    finalize_module_artifact(
        &source_file.file_name,
        source_file.source_id,
        preserved_module,
        analysis_module,
        diagnostics,
    )
}

fn apply_current_library_items(
    module: &mut LoweredModule,
    library_root: &Path,
    source_files: &[LibrarySourceFile],
    prepared_modules: &[Option<LoweredModule>],
    current_file_index: usize,
) {
    let mut local_item_names = module
        .items()
        .iter()
        .map(|item| item.name().as_str().to_string())
        .collect::<FxHashSet<_>>();
    let mut external_item_names = local_item_names.clone();
    let mut peer_candidates = FxHashMap::<String, Vec<LocalLibraryCandidate>>::default();

    for (file_index, peer_module) in prepared_modules.iter().enumerate() {
        if file_index == current_file_index {
            continue;
        }

        let Some(peer_module) = peer_module.as_ref() else {
            continue;
        };
        let peer_path = &source_files[file_index].path;

        for (item_index, item) in peer_module.items().iter().enumerate() {
            let visible_name = item.name().as_str().to_string();
            if peer_module.is_external_item(item_index) {
                if !external_item_names.insert(visible_name.clone()) {
                    continue;
                }
                let mut copier = ModuleCopier::new(module, peer_module);
                let copied = copier.copy_item(item, &visible_name);
                module.add_external_item(copied);
                continue;
            }

            if item.visibility() == Visibility::Private {
                continue;
            }

            peer_candidates
                .entry(visible_name)
                .or_default()
                .push(LocalLibraryCandidate {
                    source_file: peer_path.clone(),
                    item: LocalLibraryItemRef {
                        file_index,
                        item_index,
                    },
                });
        }
    }

    let mut visible_names = peer_candidates.keys().cloned().collect::<Vec<_>>();
    visible_names.sort();

    for visible_name in visible_names {
        if local_item_names.contains(&visible_name) {
            continue;
        }

        let candidates = peer_candidates
            .remove(&visible_name)
            .expect("visible name was collected from peer candidates");
        if candidates.len() != 1 {
            let sources = candidates
                .iter()
                .map(|candidate| {
                    candidate
                        .source_file
                        .strip_prefix(library_root)
                        .unwrap_or(candidate.source_file.as_path())
                        .display()
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join(", ");
            module.add_diagnostic(LoweringDiagnostic {
                message: format!(
                    "Library item '{}' is defined in multiple files ({}). Use unique names within one library.",
                    visible_name, sources
                ),
                span: TextSpan::default(),
            });
            continue;
        }

        let candidate = &candidates[0];
        let peer_module = prepared_modules[candidate.item.file_index]
            .as_ref()
            .expect("peer prepared module should exist");
        let item = &peer_module.items()[candidate.item.item_index];
        let mut copier = ModuleCopier::new(module, peer_module);
        let copied = copier.copy_item(item, &visible_name);
        local_item_names.insert(visible_name);
        module.add_item(copied);
    }
}

/// Builds a file-preserving program artifact from source text against a caller-supplied build context.
pub fn build_program_artifact_from_source(
    source: &str,
    file_name: &str,
    build_context: &ProgramBuildContext,
) -> io::Result<ProgramArtifact> {
    let source_path = Path::new(file_name);
    let root_path = source_path.exists().then_some(source_path);
    let PreparedRootModule {
        artifact: root_artifact,
        mut libraries,
        diagnostics: mut selection_diagnostics,
    } = prepare_root_module_with_context(source, file_name, root_path, build_context);

    let mut hasher = DefaultHasher::new();
    file_name.hash(&mut hasher);
    source.hash(&mut hasher);

    if root_path.is_some() {
        libraries.sort_by(|lhs, rhs| lhs.root_path.cmp(&rhs.root_path));
        for library in &libraries {
            hasher.write_u64(library.fingerprint);
        }
    }

    let root_modules = vec![root_artifact];
    let mut diagnostics = root_modules[0].diagnostics.clone();
    diagnostics.append(&mut selection_diagnostics);
    for library in &libraries {
        diagnostics.extend(library.diagnostics.iter().cloned());
    }

    let fingerprint = hasher.finish();
    let resolved_program = build_resolved_program(&root_modules, &libraries, fingerprint);

    Ok(ProgramArtifact {
        root_modules,
        libraries,
        diagnostics,
        fingerprint,
        resolved_program,
    })
}

fn prepare_root_module_with_context(
    source: &str,
    file_name: &str,
    source_path: Option<&Path>,
    build_context: &ProgramBuildContext,
) -> PreparedRootModule {
    let parse_result = syntax_parse_str(source, file_name);
    let source_id = SourceId::new(parse_result.source_id.as_u32());
    let mut diagnostics = normalize_diagnostics_file_name(parse_result.errors, file_name);

    let Some(tree) = parse_result.tree else {
        return PreparedRootModule {
            artifact: parse_failure_artifact(file_name, source_id, diagnostics),
            libraries: Vec::new(),
            diagnostics: Vec::new(),
        };
    };

    let preserved_module = lower(tree.root(), source_id);
    let mut analysis_module = preserved_module.clone();
    let mut libraries = Vec::new();
    let mut selection_diagnostics = Vec::new();

    if let Some(path) = source_path {
        let resolved_imports =
            apply_build_context_imports(&mut analysis_module, path, build_context, source);
        let selection =
            selected_program_libraries(&resolved_imports, file_name, source, build_context);
        libraries = selection.libraries;
        selection_diagnostics = selection.diagnostics;
    } else if !preserved_module.imports.is_empty() {
        diagnostics.push(library_imports_require_path_diagnostic(source, file_name));
    }

    PreparedRootModule {
        artifact: finalize_module_artifact(
            file_name,
            source_id,
            preserved_module,
            analysis_module,
            diagnostics,
        ),
        libraries,
        diagnostics: selection_diagnostics,
    }
}

fn parse_failure_artifact(
    file_name: &str,
    source_id: SourceId,
    diagnostics: Vec<Diagnostic>,
) -> ModuleArtifact {
    ModuleArtifact {
        file_name: file_name.to_string(),
        source_id,
        parse_succeeded: false,
        lowered_module: None,
        type_env: TypeEnvironment::new(),
        diagnostics,
        imports: Vec::new(),
    }
}

fn finalize_module_artifact(
    file_name: &str,
    source_id: SourceId,
    preserved_module: LoweredModule,
    analysis_module: LoweredModule,
    diagnostics: Vec<Diagnostic>,
) -> ModuleArtifact {
    analyze_prepared_module(
        file_name,
        source_id,
        preserved_module,
        analysis_module,
        diagnostics,
    )
}

fn apply_build_context_imports(
    module: &mut LoweredModule,
    root_path: &Path,
    build_context: &ProgramBuildContext,
    source: &str,
) -> Vec<ResolvedBuildContextImport> {
    let root_path = match fs::canonicalize(root_path) {
        Ok(root_path) => root_path,
        Err(error) => {
            if module
                .imports
                .iter()
                .any(|import| {
                    !is_git_library_path(&import.library_path)
                        && !is_http_library_path(&import.library_path)
                })
            {
                module.add_diagnostic(LoweringDiagnostic {
                    message: format!(
                        "Local library import resolution was skipped because source file path '{}' could not be resolved: {}",
                        root_path.display(),
                        error
                    ),
                    span: full_source_span(source),
                });
            }
            return Vec::new();
        }
    };

    let mut local_item_names = module
        .items()
        .iter()
        .map(|item| item.name().as_str().to_string())
        .collect::<FxHashSet<_>>();
    let mut seen_import_roots = FxHashMap::default();
    let mut imported_visible_names = FxHashMap::default();
    let mut resolved_imports = Vec::new();

    for import in module.imports.clone() {
        if is_git_library_path(&import.library_path) {
            module.add_diagnostic(LoweringDiagnostic {
                message: format!(
                    "Git library imports are not yet supported: '{}'",
                    import.library_path
                ),
                span: import.span,
            });
            continue;
        }

        if is_http_library_path(&import.library_path) {
            module.add_diagnostic(LoweringDiagnostic {
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
                module.add_diagnostic(LoweringDiagnostic {
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
            module.add_diagnostic(LoweringDiagnostic {
                message: format!(
                    "Local library import '{}' must resolve to a directory",
                    import.library_path
                ),
                span: import.span,
            });
            continue;
        }

        let normalized_key = normalized_root.to_string_lossy().to_string();
        if let Some(first_import_span) = seen_import_roots.insert(normalized_key, import.span) {
            let first_import_location = line_col_for_span(source, first_import_span)
                .map(|(line, column)| {
                    format!("; first imported at line {}, column {}", line, column)
                })
                .unwrap_or_default();
            module.add_diagnostic(LoweringDiagnostic {
                message: format!(
                    "Library '{}' is imported more than once in this file{}",
                    normalized_root.display(),
                    first_import_location
                ),
                span: import.span,
            });
            continue;
        }

        let Some(library) = build_context.visible_library(&normalized_root) else {
            module.add_diagnostic(LoweringDiagnostic {
                message: format!(
                    "Missing loaded library '{}' in the supplied build context",
                    normalized_root.display()
                ),
                span: import.span,
            });
            continue;
        };

        resolved_imports.push(ResolvedBuildContextImport {
            normalized_root: normalized_root.clone(),
            library: library.clone(),
        });

        match &import.kind {
            ImportKind::Wildcard { alias } => {
                let mut export_names = library.public_items.keys().cloned().collect::<Vec<_>>();
                export_names.sort();

                for export_name in export_names {
                    let visible_name = alias
                        .as_ref()
                        .map(|prefix| format!("{}.{}", prefix.as_str(), export_name))
                        .unwrap_or_else(|| export_name.clone());
                    if local_item_names.contains(&visible_name) {
                        continue;
                    }

                    let Some(item_indices) = library.public_items.get(&export_name) else {
                        continue;
                    };
                    if item_indices.len() != 1 {
                        add_ambiguous_interface_diagnostic(
                            module,
                            &visible_name,
                            import.span,
                            &library,
                            item_indices,
                        );
                        continue;
                    }

                    if let Some(previous_origin) = imported_visible_names.get(&visible_name) {
                        module.add_diagnostic(LoweringDiagnostic {
                            message: format!(
                                "Imported name '{}' is provided by both {} and {}. Use aliases to disambiguate.",
                                visible_name,
                                previous_origin,
                                library.root_path.display()
                            ),
                            span: import.span,
                        });
                        continue;
                    }

                    let interface_item = &library.interface_items[item_indices[0]];
                    let synthesized = interface_item.synthesize_item(&visible_name, module);
                    module.add_external_item(synthesized);
                    imported_visible_names.insert(
                        visible_name.clone(),
                        library.root_path.display().to_string(),
                    );
                    local_item_names.insert(visible_name);
                }
            }
            ImportKind::Selective { entries } => {
                for entry in entries {
                    let Some(item_indices) = library.public_items.get(entry.name.as_str()) else {
                        module.add_diagnostic(LoweringDiagnostic {
                            message: format!(
                                "Library '{}' does not export '{}'",
                                normalized_root.display(),
                                entry.name.as_str()
                            ),
                            span: entry.span,
                        });
                        continue;
                    };

                    let visible_name = visible_name_for_selective(entry);
                    if local_item_names.contains(&visible_name) {
                        continue;
                    }

                    if item_indices.len() != 1 {
                        add_ambiguous_interface_diagnostic(
                            module,
                            &visible_name,
                            entry.span,
                            &library,
                            item_indices,
                        );
                        continue;
                    }

                    if let Some(previous_origin) = imported_visible_names.get(&visible_name) {
                        module.add_diagnostic(LoweringDiagnostic {
                            message: format!(
                                "Imported name '{}' is provided by both {} and {}. Use aliases to disambiguate.",
                                visible_name,
                                previous_origin,
                                library.root_path.display()
                            ),
                            span: entry.span,
                        });
                        continue;
                    }

                    let interface_item = &library.interface_items[item_indices[0]];
                    let synthesized = interface_item.synthesize_item(&visible_name, module);
                    module.add_external_item(synthesized);
                    imported_visible_names.insert(
                        visible_name.clone(),
                        library.root_path.display().to_string(),
                    );
                    local_item_names.insert(visible_name);
                }
            }
        }
    }

    resolved_imports
}

fn add_ambiguous_interface_diagnostic(
    module: &mut LoweredModule,
    visible_name: &str,
    span: TextSpan,
    library: &LibraryArtifact,
    item_indices: &[usize],
) {
    let sources = item_indices
        .iter()
        .take(2)
        .map(|index| interface_origin(&library.root_path, &library.interface_items[*index]))
        .collect::<Vec<_>>()
        .join(" and ");

    module.add_diagnostic(LoweringDiagnostic {
        message: format!(
            "Ambiguous imported name '{}' could refer to {}. Use a more specific import alias.",
            visible_name, sources
        ),
        span,
    });
}

fn selected_program_libraries(
    direct_imports: &[ResolvedBuildContextImport],
    file_name: &str,
    source: &str,
    build_context: &ProgramBuildContext,
) -> ProgramLibrarySelection {
    let mut selection = ProgramLibrarySelection::default();
    let mut seen_library_roots = FxHashSet::default();
    let mut reported_missing_roots = FxHashSet::default();
    let mut queue = direct_imports
        .iter()
        .map(|import| PendingProgramLibrary {
            root: import.normalized_root.clone(),
            chain: vec![import.normalized_root.clone()],
            library: Some(import.library.clone()),
        })
        .collect::<Vec<_>>();
    queue.sort_by(|lhs, rhs| rhs.root.cmp(&lhs.root));

    while let Some(pending) = queue.pop() {
        if !seen_library_roots.insert(pending.root.clone()) {
            continue;
        }

        let Some(library) = pending
            .library
            .or_else(|| build_context.registry.get_loaded_library(&pending.root))
        else {
            if reported_missing_roots.insert(pending.root.clone()) {
                selection
                    .diagnostics
                    .push(incomplete_library_dependency_closure_diagnostic(
                        file_name,
                        source,
                        &pending.chain,
                    ));
            }
            continue;
        };

        let mut dependency_roots = library.dependency_roots.clone();
        dependency_roots.sort();
        for dependency_root in dependency_roots {
            if !seen_library_roots.contains(&dependency_root) {
                let mut chain = pending.chain.clone();
                chain.push(dependency_root.clone());
                queue.push(PendingProgramLibrary {
                    root: dependency_root,
                    chain,
                    library: None,
                });
            }
        }
        queue.sort_by(|lhs, rhs| rhs.root.cmp(&lhs.root));
        selection.libraries.push(library);
    }

    selection
}

fn build_resolved_program(
    root_modules: &[ModuleArtifact],
    libraries: &[Arc<LibraryArtifact>],
    fingerprint: u64,
) -> ResolvedProgram {
    let mut modules = Vec::new();
    let mut module_ids = FxHashMap::default();
    let mut root_module_ids = Vec::new();

    for artifact in root_modules {
        let Some(module) = artifact.lowered_module.clone() else {
            continue;
        };

        let module_id = RuntimeModuleId::new(modules.len() as u32);
        module_ids.insert(artifact.file_name.clone(), module_id);
        root_module_ids.push(module_id);
        modules.push(ResolvedModule {
            id: module_id,
            identity: artifact.file_name.clone(),
            lowered_module: module,
        });
    }

    for library in libraries {
        for artifact in &library.modules {
            let Some(module) = artifact.lowered_module.clone() else {
                continue;
            };

            let module_id = RuntimeModuleId::new(modules.len() as u32);
            module_ids.insert(artifact.file_name.clone(), module_id);
            modules.push(ResolvedModule {
                id: module_id,
                identity: artifact.file_name.clone(),
                lowered_module: module,
            });
        }
    }

    let mut entry_functions = FxHashMap::default();
    let mut entry_components = FxHashMap::default();
    let mut entry_records = FxHashMap::default();
    let mut entry_enums = FxHashMap::default();
    let mut imports = FxHashMap::default();

    for artifact in root_modules {
        let Some(module) = artifact.lowered_module.as_ref() else {
            continue;
        };
        let Some(&module_id) = module_ids.get(&artifact.file_name) else {
            continue;
        };

        for item in module.items() {
            insert_entry_symbol(
                &mut entry_functions,
                &mut entry_components,
                &mut entry_records,
                &mut entry_enums,
                item.name().as_str(),
                ModuleQualifiedItemRef {
                    module_id,
                    item_name: item.name().as_str().to_string(),
                    kind: resolved_item_kind(item),
                },
            );
        }
    }

    for library in libraries {
        for (export_name, export) in &library.exports {
            let Some(&module_id) = module_ids.get(&export.module_file) else {
                continue;
            };

            insert_entry_symbol(
                &mut entry_functions,
                &mut entry_components,
                &mut entry_records,
                &mut entry_enums,
                export_name,
                ModuleQualifiedItemRef {
                    module_id,
                    item_name: export.item_name.clone(),
                    kind: export.kind,
                },
            );
        }
    }

    let library_by_root = libraries
        .iter()
        .map(|library| (library.root_path.clone(), library.clone()))
        .collect::<FxHashMap<_, _>>();

    for artifact in root_modules
        .iter()
        .chain(libraries.iter().flat_map(|library| library.modules.iter()))
    {
        let Some(&module_id) = module_ids.get(&artifact.file_name) else {
            continue;
        };
        let module_file = Path::new(&artifact.file_name);
        let mut visible_imports = FxHashMap::default();

        for import in &artifact.imports {
            let Some(normalized_root) =
                normalize_supported_library_path(module_file, &import.library_path)
            else {
                continue;
            };
            let Some(library) = library_by_root.get(&normalized_root) else {
                continue;
            };

            match &import.kind {
                ImportKind::Wildcard { alias } => {
                    for (export_name, export) in &library.exports {
                        let visible_name = alias
                            .as_ref()
                            .map(|prefix| format!("{}.{}", prefix.as_str(), export_name))
                            .unwrap_or_else(|| export_name.clone());
                        let Some(&target_module_id) = module_ids.get(&export.module_file) else {
                            continue;
                        };

                        visible_imports.entry(visible_name).or_insert_with(|| {
                            ModuleQualifiedItemRef {
                                module_id: target_module_id,
                                item_name: export.item_name.clone(),
                                kind: export.kind,
                            }
                        });
                    }
                }
                ImportKind::Selective { entries } => {
                    for entry in entries {
                        let Some(export) = library.exports.get(entry.name.as_str()) else {
                            continue;
                        };
                        let Some(&target_module_id) = module_ids.get(&export.module_file) else {
                            continue;
                        };

                        visible_imports
                            .entry(visible_name_for_selective(entry))
                            .or_insert_with(|| ModuleQualifiedItemRef {
                                module_id: target_module_id,
                                item_name: export.item_name.clone(),
                                kind: export.kind,
                            });
                    }
                }
            }
        }

        if !visible_imports.is_empty() {
            imports.insert(module_id, visible_imports);
        }
    }

    ResolvedProgram::new(
        fingerprint,
        root_module_ids,
        modules,
        entry_functions,
        entry_components,
        entry_records,
        entry_enums,
        imports,
    )
}

fn interface_field_to_record_field(field: &LibraryInterfaceField) -> RecordField {
    RecordField {
        name: field.name.clone(),
        ty: field.ty.clone(),
        default: None,
        span: field.span,
    }
}

fn build_interface_item(artifact: &ModuleArtifact, item: &Item) -> Option<LibraryInterfaceItem> {
    let item_name = item.name().as_str().to_string();
    let visibility = item.visibility();
    let item = match item {
        Item::Function(function) => {
            let return_type = function.return_type.clone().or_else(|| {
                artifact
                    .type_env
                    .lookup(&function.name)
                    .and_then(type_from_function_binding)
            })?;

            LibraryInterfaceKind::Function {
                params: function
                    .params
                    .iter()
                    .map(|param| LibraryInterfaceParam {
                        name: param.name.clone(),
                        ty: param.ty.clone(),
                        span: param.span,
                    })
                    .collect(),
                return_type,
                span: function.span,
            }
        }
        Item::Value(value) => {
            let ty = value.ty.clone().or_else(|| {
                artifact
                    .type_env
                    .lookup(&value.name)
                    .and_then(type_to_type_ref)
            })?;

            LibraryInterfaceKind::Value {
                ty,
                span: value.span,
            }
        }
        Item::Component(component) => LibraryInterfaceKind::Component {
            props: component
                .props
                .iter()
                .map(record_field_to_interface_field)
                .collect(),
            emits: component.emits.clone(),
            state: component
                .state
                .iter()
                .map(record_field_to_interface_field)
                .collect(),
            span: component.span,
        },
        Item::TypeAlias(type_alias) => LibraryInterfaceKind::TypeAlias {
            ty: type_alias.ty.clone(),
            span: type_alias.span,
        },
        Item::Enum(enum_def) => LibraryInterfaceKind::Enum {
            members: enum_def.members.clone(),
            span: enum_def.span,
        },
        Item::Record(record_def) => LibraryInterfaceKind::Record {
            kind: record_def.kind,
            is_abstract: record_def.is_abstract,
            base: record_def.base.clone(),
            properties: record_def
                .properties
                .iter()
                .map(record_field_to_interface_field)
                .collect(),
            span: record_def.span,
        },
    };

    Some(LibraryInterfaceItem {
        module_file: artifact.file_name.clone(),
        item_name,
        visibility,
        item,
    })
}

fn record_field_to_interface_field(field: &RecordField) -> LibraryInterfaceField {
    LibraryInterfaceField {
        name: field.name.clone(),
        ty: field.ty.clone(),
        span: field.span,
    }
}

fn type_from_function_binding(ty: &Type) -> Option<TypeRef> {
    match ty {
        Type::Function { ret, .. } => type_to_type_ref(ret),
        _ => None,
    }
}

fn type_to_type_ref(ty: &Type) -> Option<TypeRef> {
    match ty {
        Type::Primitive(primitive) => Some(TypeRef::name(primitive.as_str())),
        Type::Array(inner) => Some(TypeRef::array(type_to_type_ref(inner)?)),
        Type::Nullable(inner) => Some(TypeRef::nullable(type_to_type_ref(inner)?)),
        Type::Function { params, ret } => Some(TypeRef::function(
            params
                .iter()
                .map(type_to_type_ref)
                .collect::<Option<Vec<_>>>()?,
            type_to_type_ref(ret)?,
        )),
        Type::Named(name) => Some(TypeRef::name(name.clone())),
        Type::Enum(enum_type) => Some(TypeRef::name(enum_type.name.clone())),
        Type::Variable(_) | Type::Unknown | Type::Error => None,
    }
}

fn insert_entry_symbol(
    entry_functions: &mut FxHashMap<String, ModuleQualifiedItemRef>,
    entry_components: &mut FxHashMap<String, ModuleQualifiedItemRef>,
    entry_records: &mut FxHashMap<String, ModuleQualifiedItemRef>,
    entry_enums: &mut FxHashMap<String, ModuleQualifiedItemRef>,
    visible_name: &str,
    item_ref: ModuleQualifiedItemRef,
) {
    match item_ref.kind {
        ResolvedItemKind::Function => {
            entry_functions
                .entry(visible_name.to_string())
                .or_insert(item_ref);
        }
        ResolvedItemKind::Component => {
            entry_components
                .entry(visible_name.to_string())
                .or_insert(item_ref);
        }
        ResolvedItemKind::Record | ResolvedItemKind::TypeAlias | ResolvedItemKind::Value => {
            entry_records
                .entry(visible_name.to_string())
                .or_insert(item_ref);
        }
        ResolvedItemKind::Enum => {
            entry_enums
                .entry(visible_name.to_string())
                .or_insert(item_ref);
        }
    }
}

fn resolved_item_kind(item: &Item) -> ResolvedItemKind {
    match item {
        Item::Function(_) => ResolvedItemKind::Function,
        Item::Value(_) => ResolvedItemKind::Value,
        Item::Component(_) => ResolvedItemKind::Component,
        Item::TypeAlias(_) => ResolvedItemKind::TypeAlias,
        Item::Enum(_) => ResolvedItemKind::Enum,
        Item::Record(_) => ResolvedItemKind::Record,
    }
}

fn interface_origin(root_path: &Path, item: &LibraryInterfaceItem) -> String {
    let item_path = Path::new(&item.module_file);
    let relative = item_path.strip_prefix(root_path).unwrap_or(item_path);
    format!("{} ({})", item.item_name, relative.display())
}

fn collect_library_dependencies(
    imports: &[Import],
    source_file: &Path,
    dependency_roots: &mut FxHashSet<PathBuf>,
) {
    for import in imports {
        let Some(root) = normalize_supported_library_path(source_file, &import.library_path) else {
            continue;
        };
        dependency_roots.insert(root);
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

fn normalize_supported_library_path(base_file: &Path, library_path: &str) -> Option<PathBuf> {
    if is_http_library_path(library_path) || is_git_library_path(library_path) {
        return None;
    }

    normalize_local_library_path(base_file, library_path).ok()
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

fn visible_name_for_selective(entry: &SelectiveImport) -> String {
    match entry.qualifier.as_ref() {
        Some(prefix) => format!("{}.{}", prefix.as_str(), entry.name.as_str()),
        None => entry.name.as_str().to_string(),
    }
}

fn line_col_for_span(source: &str, span: TextSpan) -> Option<(usize, usize)> {
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

fn full_source_span(source: &str) -> TextSpan {
    let source_len = u32::try_from(source.len())
        .expect("NX source size should be validated before creating source diagnostics");
    TextSpan::new(TextSize::from(0), TextSize::from(source_len))
}

fn library_imports_require_path_diagnostic(source: &str, file_name: &str) -> Diagnostic {
    Diagnostic::error("library-imports-require-path")
        .with_message("Library imports require an on-disk source path")
        .with_label(Label::primary(file_name, full_source_span(source)))
        .with_help("Pass a real file path as file_name or use a file-based entry point.")
        .build()
}

fn incomplete_library_dependency_closure_diagnostic(
    file_name: &str,
    source: &str,
    chain: &[PathBuf],
) -> Diagnostic {
    let chain_text = chain
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(" -> ");
    let missing_root = chain
        .last()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<unknown>".to_string());

    Diagnostic::error("library-dependency-closure-incomplete")
        .with_message(format!(
            "Loaded library dependency closure is incomplete: {}",
            chain_text
        ))
        .with_label(Label::primary(file_name, full_source_span(source)))
        .with_help(format!(
            "Reload '{}' and its dependency closure into the supplied library registry before building the program.",
            missing_root
        ))
        .build()
}

fn normalize_diagnostics_file_name(
    diagnostics: Vec<Diagnostic>,
    file_name: &str,
) -> Vec<Diagnostic> {
    diagnostics
        .into_iter()
        .map(|diagnostic| {
            let labels = diagnostic
                .labels()
                .iter()
                .cloned()
                .map(|mut label| {
                    if label.file.is_empty() {
                        label.file = file_name.to_string();
                    }
                    label
                })
                .collect::<Vec<_>>();

            let code = diagnostic.code().unwrap_or("diagnostic");
            let mut builder = match diagnostic.severity() {
                Severity::Error => Diagnostic::error(code),
                Severity::Warning => Diagnostic::warning(code),
                Severity::Info => Diagnostic::info(code),
                Severity::Hint => Diagnostic::hint(code),
            }
            .with_message(diagnostic.message())
            .with_labels(labels);

            if let Some(help) = diagnostic.help() {
                builder = builder.with_help(help);
            }

            if let Some(note) = diagnostic.note() {
                builder = builder.with_note(note);
            }

            builder.build()
        })
        .collect()
}

pub(crate) fn has_error_diagnostics(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity() == Severity::Error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::eval_program_artifact;
    use crate::EvalResult;
    use tempfile::TempDir;

    #[test]
    fn library_artifact_preserves_modules_and_exports_separately() {
        let temp = TempDir::new().expect("temp dir");
        let ui_dir = temp.path().join("ui");
        let forms_dir = ui_dir.join("forms");
        fs::create_dir_all(&forms_dir).expect("forms dir");

        fs::write(ui_dir.join("button.nx"), r#"let <Button /> = <button />"#).expect("button");
        fs::write(forms_dir.join("input.nx"), r#"let <Input /> = <input />"#).expect("input");

        let artifact =
            build_library_artifact_from_directory(&ui_dir).expect("Expected library artifact");

        assert_eq!(artifact.modules.len(), 2);
        assert_eq!(artifact.public_items.get("Button").map(Vec::len), Some(1));
        assert_eq!(artifact.public_items.get("Input").map(Vec::len), Some(1));
        assert!(
            artifact
                .modules
                .iter()
                .all(|module| module.lowered_module.is_some()),
            "Expected one preserved lowered module per source file"
        );
    }

    #[test]
    fn library_registry_loads_library_closure_without_program_build() {
        let temp = TempDir::new().expect("temp dir");
        let widgets_dir = temp.path().join("widgets");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&widgets_dir).expect("widgets dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(ui_dir.join("button.nx"), r#"let <Button /> = <button />"#).expect("ui file");
        fs::write(
            widgets_dir.join("search-box.nx"),
            r#"import "../ui"
let root() = { <Button /> }"#,
        )
        .expect("widgets file");

        let registry = LibraryRegistry::new();
        let artifact = registry
            .load_library_from_directory(&widgets_dir)
            .expect("Expected registry load to succeed");

        assert_eq!(artifact.root_path, fs::canonicalize(&widgets_dir).unwrap());
        assert!(registry
            .get_loaded_library(&fs::canonicalize(&ui_dir).unwrap())
            .is_some());
    }

    #[test]
    fn library_registry_rejects_circular_library_dependencies() {
        let temp = TempDir::new().expect("temp dir");
        let a_dir = temp.path().join("a");
        let b_dir = temp.path().join("b");
        fs::create_dir_all(&a_dir).expect("a dir");
        fs::create_dir_all(&b_dir).expect("b dir");

        fs::write(
            a_dir.join("entry.nx"),
            r#"import "../b"
let from_a() = { from_b() }"#,
        )
        .expect("a file");
        fs::write(
            b_dir.join("entry.nx"),
            r#"import "../a"
let from_b() = { from_a() }"#,
        )
        .expect("b file");

        let registry = LibraryRegistry::new();
        let error = registry
            .load_library_from_directory(&a_dir)
            .expect_err("Expected circular library dependency to fail");
        let a_root = fs::canonicalize(&a_dir).expect("canonical a");
        let b_root = fs::canonicalize(&b_dir).expect("canonical b");
        let expected_chain = format!(
            "{} -> {} -> {}",
            a_root.display(),
            b_root.display(),
            a_root.display()
        );

        assert_eq!(
            error.len(),
            1,
            "Expected one diagnostic for the circular dependency"
        );
        assert!(
            error[0]
                .message
                .contains("Circular library dependency detected"),
            "Expected circular dependency diagnostic, got {:?}",
            error
        );
        assert!(
            error[0].message.contains(&expected_chain),
            "Expected circular dependency chain '{}', got {:?}",
            expected_chain,
            error
        );
        assert!(registry
            .get_loaded_library(&fs::canonicalize(&a_dir).unwrap())
            .is_none());
        assert!(registry
            .get_loaded_library(&fs::canonicalize(&b_dir).unwrap())
            .is_none());
    }

    #[test]
    fn program_artifact_uses_registry_backed_build_context() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(ui_dir.join("answer.nx"), r#"let answer() = { 42 }"#).expect("ui file");
        let registry = LibraryRegistry::new();
        let ui_snapshot = registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected registry load");
        let build_context = registry.build_context();

        let main_path = app_dir.join("main.nx");
        let source = r#"import "../ui"
let root() = { answer() }"#;
        fs::write(&main_path, source).expect("main file");

        let artifact = build_program_artifact_from_source(
            source,
            &main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected program artifact");

        assert_eq!(artifact.libraries.len(), 1);
        assert!(Arc::ptr_eq(&artifact.libraries[0], &ui_snapshot));
        assert!(
            artifact.root_modules[0]
                .lowered_module
                .as_ref()
                .expect("Expected preserved root module")
                .find_item("answer")
                .is_none(),
            "Expected preserved root module to remain file-scoped"
        );

        let EvalResult::Ok(value) = eval_program_artifact(&artifact) else {
            panic!("Expected registry-backed program artifact evaluation to succeed");
        };
        assert_eq!(value, nx_value::NxValue::Int(42));
    }

    #[test]
    fn program_build_reports_missing_library_from_context() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");
        fs::write(ui_dir.join("button.nx"), r#"let <Button /> = <button />"#).expect("ui file");

        let main_path = app_dir.join("main.nx");
        let source = r#"import "../ui"
let root() = { <Button /> }"#;
        fs::write(&main_path, source).expect("main file");

        let artifact = build_program_artifact_from_source(
            source,
            &main_path.display().to_string(),
            &ProgramBuildContext::empty(),
        )
        .expect("Expected program artifact with diagnostics");

        assert!(artifact
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message().contains("Missing loaded library")));
    }

    #[test]
    fn apply_build_context_imports_reports_unresolved_source_path_for_local_imports() {
        let source = r#"import "../ui"
let root() = { answer() }"#;
        let parse_result = syntax_parse_str(source, "virtual.nx");
        let source_id = SourceId::new(parse_result.source_id.as_u32());
        let tree = parse_result.tree.expect("Expected syntax tree");
        let mut module = lower(tree.root(), source_id);

        let resolved_imports = apply_build_context_imports(
            &mut module,
            Path::new("/path/that/does/not/exist/main.nx"),
            &ProgramBuildContext::empty(),
            source,
        );

        assert!(resolved_imports.is_empty());
        assert!(module.diagnostics().iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("Local library import resolution was skipped because source file path")
                && diagnostic.message.contains("could not be resolved")
        }));
    }

    #[test]
    fn program_artifact_survives_build_context_drop() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(ui_dir.join("answer.nx"), r#"let answer() = { 42 }"#).expect("ui file");
        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected registry load");
        let build_context = registry.build_context();

        let main_path = app_dir.join("main.nx");
        let source = r#"import "../ui"
let root() = { answer() }"#;
        fs::write(&main_path, source).expect("main file");

        let artifact = build_program_artifact_from_source(
            source,
            &main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected program artifact");
        drop(build_context);

        let EvalResult::Ok(value) = eval_program_artifact(&artifact) else {
            panic!("Expected program artifact to remain executable after context release");
        };
        assert_eq!(value, nx_value::NxValue::Int(42));
    }

    #[test]
    fn build_context_with_visible_roots_includes_transitive_dependencies_of_visible_library() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let widgets_dir = temp.path().join("widgets");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&widgets_dir).expect("widgets dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(ui_dir.join("answer.nx"), r#"let ui_answer() = { 42 }"#).expect("ui file");
        fs::write(
            widgets_dir.join("answer.nx"),
            r#"import "../ui"
let answer() = { ui_answer() }"#,
        )
        .expect("widgets file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&widgets_dir)
            .expect("Expected widgets registry load");
        let build_context = registry
            .build_context_with_visible_roots([&widgets_dir])
            .expect("Expected filtered build context");

        let main_path = app_dir.join("main.nx");
        let source = r#"import "../widgets"
let root() = { answer() }"#;
        fs::write(&main_path, source).expect("main file");

        let artifact = build_program_artifact_from_source(
            source,
            &main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected program artifact");

        assert!(
            !has_error_diagnostics(&artifact.diagnostics),
            "Expected transitive dependency closure to be selected for the visible library"
        );

        let widget_root = fs::canonicalize(&widgets_dir).expect("canonical widgets");
        let ui_root = fs::canonicalize(&ui_dir).expect("canonical ui");
        let library_roots = artifact
            .libraries
            .iter()
            .map(|library| library.root_path.clone())
            .collect::<Vec<_>>();
        assert!(
            library_roots.contains(&widget_root),
            "Expected selected libraries to include the visible library"
        );
        assert!(
            library_roots.contains(&ui_root),
            "Expected selected libraries to include the visible library's transitive dependency"
        );

        let EvalResult::Ok(value) = eval_program_artifact(&artifact) else {
            panic!("Expected transitive dependency program artifact evaluation to succeed");
        };
        assert_eq!(value, nx_value::NxValue::Int(42));
    }

    #[test]
    fn build_context_with_visible_roots_limits_visible_libraries() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        let admin_dir = temp.path().join("admin");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");
        fs::create_dir_all(&admin_dir).expect("admin dir");

        fs::write(ui_dir.join("answer.nx"), r#"let answer() = { 42 }"#).expect("ui file");
        fs::write(admin_dir.join("secret.nx"), r#"let secret() = { 7 }"#).expect("admin file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected ui registry load");
        registry
            .load_library_from_directory(&admin_dir)
            .expect("Expected admin registry load");
        let build_context = registry
            .build_context_with_visible_roots([&ui_dir])
            .expect("Expected filtered build context");

        let visible_main_path = app_dir.join("visible-main.nx");
        let visible_source = r#"import "../ui"
let root() = { answer() }"#;
        fs::write(&visible_main_path, visible_source).expect("visible main file");

        let visible_artifact = build_program_artifact_from_source(
            visible_source,
            &visible_main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected visible program artifact");

        assert!(
            !has_error_diagnostics(&visible_artifact.diagnostics),
            "Expected visible library import to succeed"
        );

        let EvalResult::Ok(value) = eval_program_artifact(&visible_artifact) else {
            panic!("Expected visible library program artifact evaluation to succeed");
        };
        assert_eq!(value, nx_value::NxValue::Int(42));

        let hidden_main_path = app_dir.join("hidden-main.nx");
        let hidden_source = r#"import "../admin"
let root() = { secret() }"#;
        fs::write(&hidden_main_path, hidden_source).expect("hidden main file");

        let hidden_artifact = build_program_artifact_from_source(
            hidden_source,
            &hidden_main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected hidden program artifact with diagnostics");

        assert!(
            hidden_artifact.libraries.is_empty(),
            "Expected hidden libraries to stay out of the selected program library set"
        );
        assert!(hidden_artifact
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message().contains("Missing loaded library")));
    }

    #[test]
    fn program_build_reports_incomplete_loaded_library_dependency_closure() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let widgets_dir = temp.path().join("widgets");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&widgets_dir).expect("widgets dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(ui_dir.join("answer.nx"), r#"let ui_answer() = { 42 }"#).expect("ui file");
        fs::write(
            widgets_dir.join("answer.nx"),
            r#"import "../ui"
let answer() = { ui_answer() }"#,
        )
        .expect("widgets file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&widgets_dir)
            .expect("Expected widgets registry load");
        let ui_root = fs::canonicalize(&ui_dir).expect("canonical ui");
        {
            let mut state = registry
                .inner
                .write()
                .expect("library registry lock poisoned");
            let removed = state.libraries.remove(&ui_root);
            assert!(
                removed.is_some(),
                "Expected inconsistent registry setup to remove the transitive dependency snapshot"
            );
        }

        let build_context = registry.build_context();
        let main_path = app_dir.join("main.nx");
        let source = r#"import "../widgets"
let root() = { answer() }"#;
        fs::write(&main_path, source).expect("main file");

        let artifact = build_program_artifact_from_source(
            source,
            &main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected program artifact with diagnostics");

        let widgets_root = fs::canonicalize(&widgets_dir).expect("canonical widgets");
        let expected_chain = format!("{} -> {}", widgets_root.display(), ui_root.display());
        assert!(artifact.diagnostics.iter().any(|diagnostic| {
            diagnostic.code() == Some("library-dependency-closure-incomplete")
                && diagnostic.message().contains(&expected_chain)
        }));
    }
}
