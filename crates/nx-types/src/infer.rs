//! Type inference for expressions.

use crate::{
    numeric_literal_target,
    semantics::{common_item_supertype, PRIMITIVE_TYPE_NAMES},
    ty::{
        check_function_satisfies, read_type, DeclaringOrigin, FunctionMismatch, FunctionParam,
        NamedType, Occurrence, Primitive, TypeParameterRef, UnionCaseType, UnionType,
    },
    type_satisfies_expected as generic_type_satisfies_expected, Type, TypeEnvironment,
};
use nx_diagnostics::{Diagnostic, Label, TextSpan};
use nx_hir::{
    ast, effective_component_contract_for_name, effective_record_shape_for_name,
    interface_component, interface_function_signature, interface_type_alias, interface_union,
    is_record_subtype, ElementId, ExprId, InterfaceItemKind, Item, Name, PreparedBindingOrigin,
    PreparedItemKind, PreparedModule, PreparedNamespace, PropertyEntry, ResolvedPreparedItem,
    StringConversions, UnionCaseDef, UnionDef, UpdateIntrinsic,
};
use rustc_hash::{FxHashMap, FxHashSet};

/// What the name `Range` means in the module being checked.
enum PreludeRangeName {
    /// The prelude's declaration, which is what the range operators construct.
    Prelude(DeclaringOrigin),
    /// Another declaration or import took the name; `by` is the module that declares it.
    Hidden { by: String },
    /// Nothing is visible under the name, which is a build with no prelude at all.
    Absent,
}

/// One component's lineage, nearest first: the component itself, then each ancestor.
fn component_lineage(
    contract: &nx_hir::EffectiveComponentContract,
) -> Vec<nx_hir::ComponentAncestor> {
    let mut lineage = vec![nx_hir::ComponentAncestor {
        name: contract.component.name.clone(),
        origin: contract.origin.clone(),
    }];
    lineage.extend(contract.ancestors.iter().cloned());
    lineage
}

/// One record's lineage, nearest first: the record itself, then each ancestor.
fn record_lineage(shape: &nx_hir::EffectiveRecordShape) -> Vec<nx_hir::RecordAncestor> {
    let mut lineage = vec![nx_hir::RecordAncestor {
        name: shape.record.name.clone(),
        origin: shape.origin.clone(),
    }];
    lineage.extend(shape.ancestors.iter().cloned());
    lineage
}

/// One type alias's target, under whatever name this module reaches the alias by.
///
/// <para>There is deliberately no span here. An alias in this map may have been declared by
/// another module, and that declaration's span cannot underline anything in this file -- it can
/// run past the end of it. A diagnostic raised while resolving an alias target takes its span
/// from `local_type_alias_spans` when this module wrote the alias, and from the reference that
/// reached it otherwise.</para>
struct TypeAliasInfo {
    target: ast::TypeRef,
}

/// One discriminated union definition together with the declaration it came from.
///
/// A union is registered under whatever name this module reaches it by, which is not necessarily
/// the name it was declared with. The origin is what says which declaration that name reached.
#[derive(Clone)]
struct UnionEntry {
    def: UnionDef,
    origin: Option<DeclaringOrigin>,
}

impl UnionEntry {
    /// The `UnionType` shape of this definition, carrying its declaring origin.
    fn shape(&self) -> UnionType {
        UnionType::new(
            self.def.name.clone(),
            self.def
                .cases
                .iter()
                .map(|case| case.name.clone())
                .collect(),
            self.def.base.clone(),
            self.origin.clone(),
        )
    }

    /// The case type for one of this union's cases, carrying the union's declaring origin.
    fn case_type(&self, case: Name) -> Type {
        Type::union_case_type(self.def.name.clone(), case, self.origin.clone())
    }
}

/// One value an action handler's body can return, as the routing check sees it.
enum HandlerResultItem {
    /// A value of this type, written at this span.
    Value(Type, TextSpan),
    /// A value that may be empty, which a handler may never return.
    MayBeEmpty(Type, TextSpan),
}

/// A call's callee when it names a declared function: how it was declared, and each parameter as
/// its name and whether a caller may omit it.
struct DeclaredCallee {
    name: Name,
    form: nx_hir::FunctionForm,
    params: Vec<(Name, bool)>,
}

struct ElementBindingSpec {
    content_property: Option<Name>,
    properties: FxHashMap<Name, ElementPropertySpec>,
    handler_properties: FxHashSet<Name>,
    /// The target's type parameters. A binding under one of these names is a type argument, which
    /// is consumed (or reported) before the value bindings are checked, never a property.
    type_parameters: FxHashSet<Name>,
}

struct ElementPropertySpec {
    ty: Type,
    is_required: bool,
    /// The type parameter this property's declared type mentioned and the use site left
    /// unspecified, if any. `ty` then has the bottom type in that parameter's place, and a binding
    /// that fails against it is reported by naming the parameter rather than the bottom type.
    unspecified_parameter: Option<Name>,
}

impl ElementPropertySpec {
    fn new(ty: Type, is_required: bool) -> Self {
        Self {
            ty,
            is_required,
            unspecified_parameter: None,
        }
    }
}

/// The type arguments one use site bound, resolved and ready to instantiate the target's contract.
struct ResolvedTypeArguments {
    /// Every effective type parameter of the target: the argument it was bound to, or its rigid
    /// type when the use site bound none.
    scope: FxHashMap<Name, Type>,
    /// The parameters the use site bound nothing to.
    unspecified: FxHashSet<Name>,
    /// The bindings that resolved, in declaration order of the parameters.
    resolved: Vec<(Name, Type)>,
    /// The value expressions of every plain type-argument binding, for removal after analysis.
    consumed: Vec<ExprId>,
}

/// One resolved contextual name, and everything needed to rewrite it to a reference.
#[derive(Clone, Debug)]
pub struct ContextualResolution {
    /// The union that the bare name resolved against, as the declaring module displays it.
    pub type_name: Name,
    /// The case it named.
    pub member: Name,
    /// Where that union is declared.
    ///
    /// This is what the rewrite carries in place of a name, so the case reaches code generation and
    /// evaluation without the union being nameable at the use site.
    pub origin: Option<DeclaringOrigin>,
}

#[derive(Clone)]
struct PropertyPath {
    properties: Vec<PropertyPathBinding>,
}

#[derive(Clone)]
struct PropertyPathBinding {
    key: Name,
    ty: Type,
    span: TextSpan,
    /// The expression the type was inferred from, so a contextual name can be recorded once it
    /// resolves against this binding's expected type.
    value: ExprId,
}

/// What to write instead when an operator that takes exactly one value meets a value that may
/// be empty (supply one with `??`) or may be many (take its items with `for`).
fn occurrence_operand_hint(ty: &Type) -> &'static str {
    if ty.admits_many() {
        "take its items with `for`"
    } else {
        "supply a value for the empty case with `??`"
    }
}

/// The source spelling of a binary operator, for diagnostics.
fn binop_spelling(op: ast::BinOp) -> &'static str {
    use ast::BinOp::*;
    match op {
        Add => "+",
        Sub => "-",
        Mul => "*",
        Div => "/",
        Mod => "%",
        Eq => "==",
        Ne => "!=",
        Lt => "<",
        Le => "<=",
        Gt => ">",
        Ge => ">=",
        And => "&&",
        Or => "||",
    }
}

/// Why a constant expression has no value.
enum ConstantProblem {
    DivisionByZero,
    Overflow,
}

/// Applies an arithmetic operator to two folded constants, as evaluation would: two integers as an
/// `int`, and anything else as a `float64`.
fn fold_binary(
    op: ast::BinOp,
    lhs: ast::Literal,
    rhs: ast::Literal,
) -> Result<ast::Literal, ConstantProblem> {
    use ast::{BinOp, Literal, OrderedFloat};

    if let (Literal::Int(a), Literal::Int(b)) = (&lhs, &rhs) {
        let (a, b) = (*a, *b);
        if matches!(op, BinOp::Div | BinOp::Mod) && b == 0 {
            return Err(ConstantProblem::DivisionByZero);
        }
        let value = match op {
            BinOp::Add => a.checked_add(b),
            BinOp::Sub => a.checked_sub(b),
            BinOp::Mul => a.checked_mul(b),
            BinOp::Div => a.checked_div(b),
            _ => a.checked_rem(b),
        };
        return value.map(Literal::Int).ok_or(ConstantProblem::Overflow);
    }

    let real = |literal: &Literal| match literal {
        Literal::Int(value) => *value as f64,
        Literal::Float(value) => value.0,
        _ => unreachable!("only numeric literals are folded"),
    };
    let (a, b) = (real(&lhs), real(&rhs));
    if matches!(op, BinOp::Div | BinOp::Mod) && b == 0.0 {
        return Err(ConstantProblem::DivisionByZero);
    }
    let value = match op {
        BinOp::Add => a + b,
        BinOp::Sub => a - b,
        BinOp::Mul => a * b,
        BinOp::Div => a / b,
        _ => a % b,
    };
    Ok(Literal::Float(OrderedFloat(value)))
}

fn handler_prop_name(emit_name: &str) -> String {
    format!("on{}", emit_name)
}

/// Type inference context.
///
/// Manages type inference state and provides methods for inferring types
/// of expressions within a module.
pub struct InferenceContext<'a> {
    /// The module being type-checked
    module: &'a PreparedModule,
    /// Original caller-provided file name for diagnostics.
    file_name: String,
    /// Type environment (name → type, expr → type)
    env: TypeEnvironment,
    /// Type errors collected during inference
    diagnostics: Vec<Diagnostic>,
    /// Next type variable ID for inference
    next_var_id: u32,
    /// Placeholder return types for functions without explicit annotations
    function_return_placeholders: FxHashMap<Name, Type>,
    /// Registered type aliases
    type_aliases: FxHashMap<Name, TypeAliasInfo>,
    /// Registered discriminated union definitions.
    union_defs: FxHashMap<Name, UnionEntry>,
    /// The declaration each visible record name reaches.
    ///
    /// A record type is a `Type::Named`, so without this the type carries only a spelling and an
    /// unrelated same-named record in another module satisfies it.
    record_origins: FxHashMap<Name, DeclaringOrigin>,
    /// The declaration each visible component name reaches, for the same reason.
    ///
    /// Components live in the element namespace rather than the type namespace, but a property may
    /// still be typed by one, so they are reached the same way.
    component_origins: FxHashMap<Name, DeclaringOrigin>,
    /// Union definitions reached through a foreign declaration's own signature.
    ///
    /// <para>Keyed by the declaration each entry is, not by the name it was reached under. Two
    /// foreign unions a module receives under one spelling are two declarations, and a map keyed
    /// by that spelling would keep only whichever arrived last — dropping the other exactly where
    /// a post-resolution lookup needs it. Consulted only for a type that came from another module,
    /// so it can neither shadow a local union nor make a foreign name spellable in source.</para>
    foreign_union_defs: FxHashMap<DeclaringOrigin, UnionEntry>,
    /// Foreign type aliases currently being followed, so a cycle among them terminates.
    ///
    /// <para>A foreign alias is resolved by resolving what it names, which may be another alias in
    /// the same module. The declaring module reports its own cycle; this only stops the consumer
    /// from following one forever.</para>
    foreign_alias_stack: FxHashSet<DeclaringOrigin>,
    /// Contextual names resolved at binding sites, as `expr → (declaring type, member)`.
    ///
    /// Consumed after analysis to rewrite each `Expr::ContextualName` into the qualified member
    /// access it resolved to, so nothing downstream of type checking can observe the bare spelling.
    resolved_contextual_names: FxHashMap<ExprId, ContextualResolution>,
    /// Numeric literals that took a numeric type from their site, and the type each took.
    ///
    /// Consumed after analysis to rewrite each one into a literal of that width, on the same terms
    /// and for the same reason as `resolved_contextual_names`: nothing downstream of type checking
    /// should have to know that the author wrote `24` where `24.0` was expected, or `1` where an
    /// `int32` was, or be able to tell.
    converted_literals: FxHashMap<ExprId, Primitive>,
    /// Constant expressions that took a numeric type from their site, and the value each folded
    /// to, at the type its literals have on their own.
    ///
    /// Consumed after analysis to replace each with that literal, before `converted_literals`
    /// gives the literal the site's width. The expression is also in `converted_literals`.
    folded_constants: FxHashMap<ExprId, ast::Literal>,
    /// The additions that concatenate, the operands rendered as text, and the text bodies joined
    /// into one string.
    ///
    /// Consumed after analysis to rewrite each into `Concat` and `ToText` nodes, on the same
    /// terms as `converted_literals`. Whether a `+` adds or concatenates is decided here, where
    /// every operand's type is known, and nowhere else.
    string_conversions: StringConversions,
    /// The branches of a join that widen, and the numeric type each widens to.
    ///
    /// Joining an `int` branch with a `float64` one types the join as `float64`, but the `int`
    /// branch still produces an integer. Consumed after analysis to wrap each such branch in an
    /// `Expr::Widen`, on the same terms as `converted_literals`, so a runtime that tells the two
    /// apart produces the value the join's type says.
    widened_joins: FxHashMap<ExprId, Primitive>,
    /// The branches of an `if`, match or `??` that the join lifted from an item to a sequence.
    ///
    /// <para>The join types a branch producing `T` beside a sequence as `T[]`, and the lifting is
    /// only a claim about the type until the branch's value follows it. Each branch recorded here
    /// is wrapped as a one-item sequence after analysis, beside the widenings, so every engine
    /// evaluates a taken `if c { 1 }` to `[1]` — the value its `int[]` type promises — rather
    /// than to a bare `1` that iterating would reject or, in generated JavaScript, misread.</para>
    lifted_joins: FxHashSet<ExprId>,
    /// What each local type alias's target resolved to, so each target is walked once.
    ///
    /// <para>An alias's target used to be walked again at every use, which reported whatever was
    /// wrong with it once per use as well as once at the alias -- `type Rows = Names[]` used
    /// twice was three diagnostics, two of them pointing at a line that names neither `Names` nor
    /// a `[]`. Resolving once is also what keeps the eager alias pass from doubling every report
    /// it was added to make. `Type::Error` is cached like any other answer, because it is the
    /// answer.</para>
    resolved_type_aliases: FxHashMap<Name, Type>,
    /// Where each alias *this* module declares was written.
    ///
    /// <para>`type_aliases` holds imported aliases too, under the name this module reaches them
    /// by and with the span the declaring module wrote them at. A span from another file cannot
    /// underline anything in this one, so only the aliases here are ones a diagnostic raised
    /// while resolving a target may be moved onto.</para>
    local_type_alias_spans: FxHashMap<Name, TextSpan>,
    /// The type parameters a type annotation can currently name, and what each one denotes.
    ///
    /// <para>While a component's signature, defaults, and body are checked, each of its effective
    /// type parameters denotes its own rigid type. While a use site's contract is instantiated,
    /// each of the target's parameters denotes the argument the site bound — or the rigid type,
    /// so the parameter can be found and replaced by the bottom type afterwards. Empty everywhere
    /// else, which is what keeps a parameter from being a type outside its declaration.</para>
    ///
    /// <para>Consulted only for the names a type reference spells directly. A name an alias
    /// expands to was written where the alias was declared, outside any component, and is resolved
    /// there.</para>
    type_parameter_scope: FxHashMap<Name, Type>,
    /// The value expression of every plain type-argument binding the checker consumed.
    ///
    /// Consumed after analysis to remove each binding from its element, on the same terms as
    /// `resolved_contextual_names`: a type argument is a spelling only the checker understands.
    consumed_type_arguments: FxHashSet<ExprId>,
    /// Elements whose tag named a function-typed value rather than a declared element, so the
    /// element is a call of that value with arguments bound by name; each maps to the name of the
    /// type's content parameter, which body content binds to, when the type has one.
    function_value_calls: FxHashMap<ElementId, Option<Name>>,
    /// The type each use site bound to each of its target's type parameters, by element.
    ///
    /// Nothing in analysis reads this back; it is kept in the analysis result so that carrying
    /// use-site arguments into generated output later is an additive change below the checker.
    resolved_type_arguments: FxHashMap<ElementId, Vec<(Name, Type)>>,
    /// The `for` expressions whose iterable is a range rather than a list.
    ///
    /// <para>The two iterate differently below the checker — NX IR encodes a range loop as its own
    /// node — and only the checker knows which is which, because the interpreter and the builder
    /// see the iterable after the range expression has become an ordinary record
    /// construction.</para>
    range_for_expressions: FxHashSet<ExprId>,
    /// The `for` loops with an index whose body is being checked, innermost last.
    ///
    /// Read when a type error is reported, to say which name is the item and which the index
    /// when the error reads like the two were swapped.
    indexed_loops: Vec<IndexedLoop>,
    /// The paths a presence test has narrowed, innermost last: `b.author` read as `Person` inside
    /// `if b.author? { … }`.
    ///
    /// <para>An entry is pushed when a branch is entered under a narrowing condition and popped
    /// when the branch is left, so nothing escapes its branch. Values are immutable, so nothing in
    /// the branch invalidates an entry. A path is an identifier followed by member names.</para>
    narrowings: Vec<Narrowing>,
    /// Where a diagnostic about a type *reference* is reported.
    ///
    /// <para>A `TypeRef` carries no span of its own, so a problem with one — an applied type that
    /// omits an argument, a generic record's name written bare — is reported at the nearest
    /// construct that does have one, which is the annotation or field that wrote it. Callers that
    /// know the construct set it around the conversion.</para>
    type_ref_span: TextSpan,
}

/// A path a presence test or a match arm has narrowed, and the binding it narrowed.
struct Narrowing {
    path: Vec<Name>,
    ty: Type,
    /// The depth of the scope the path's root identifier resolved in when the narrowing was
    /// pushed. A binding of the same name in an inner scope shadows the narrowed one, so the
    /// narrowing applies only while the root still resolves at this depth.
    root_depth: Option<usize>,
}

/// The names an indexed `for` binds, and where its body uses them.
struct IndexedLoop {
    item: Name,
    index: Name,
    item_ty: Type,
    /// The depth of the scope the loop binds its names in.
    scope_depth: usize,
    item_uses: Vec<TextSpan>,
    index_uses: Vec<TextSpan>,
}

impl<'a> InferenceContext<'a> {
    /// Creates a new inference context for a module.
    pub fn new(module: &'a PreparedModule) -> Self {
        Self::with_file_name(module, "")
    }

    /// Creates a new inference context for a module with a diagnostic file name.
    pub fn with_file_name(module: &'a PreparedModule, file_name: impl Into<String>) -> Self {
        let mut ctx = Self {
            module,
            file_name: file_name.into(),
            env: TypeEnvironment::new(),
            diagnostics: Vec::new(),
            next_var_id: 0,
            function_return_placeholders: FxHashMap::default(),
            type_aliases: FxHashMap::default(),
            foreign_union_defs: FxHashMap::default(),
            foreign_alias_stack: FxHashSet::default(),
            union_defs: FxHashMap::default(),
            record_origins: FxHashMap::default(),
            component_origins: FxHashMap::default(),
            resolved_contextual_names: FxHashMap::default(),
            converted_literals: FxHashMap::default(),
            folded_constants: FxHashMap::default(),
            string_conversions: StringConversions::default(),
            widened_joins: FxHashMap::default(),
            lifted_joins: FxHashSet::default(),
            resolved_type_aliases: FxHashMap::default(),
            local_type_alias_spans: FxHashMap::default(),
            type_parameter_scope: FxHashMap::default(),
            type_ref_span: TextSpan::new(0.into(), 0.into()),
            consumed_type_arguments: FxHashSet::default(),
            function_value_calls: FxHashMap::default(),
            resolved_type_arguments: FxHashMap::default(),
            range_for_expressions: FxHashSet::default(),
            indexed_loops: Vec::new(),
            narrowings: Vec::new(),
        };
        ctx.register_type_definitions();
        // Aliases before anything that can resolve one, so a target that nests a sequence is
        // reported at the alias itself rather than at whichever signature, binding or field
        // happens to reach it first.
        ctx.validate_local_type_aliases();
        ctx.register_function_signatures();
        ctx.register_value_bindings();
        ctx.validate_local_record_defaults();
        ctx.validate_local_union_defaults();
        ctx
    }

    /// Generates a fresh type variable for inference.
    fn fresh_var(&mut self) -> Type {
        let id = self.next_var_id;
        self.next_var_id += 1;
        Type::var(id)
    }

    fn flattened_expr_name(&self, expr_id: ExprId) -> Option<Name> {
        match self.module.raw_module().expr(expr_id) {
            ast::Expr::Ident(name) => Some(name.clone()),
            ast::Expr::Member { base, member, .. } => {
                let mut name = self.flattened_expr_name(*base)?.as_str().to_string();
                name.push('.');
                name.push_str(member.as_str());
                Some(Name::new(&name))
            }
            _ => None,
        }
    }

    /// The path an expression spells — an identifier followed by member names, written with `.`
    /// or `?.` — or `None` for any other expression.
    fn expr_path(&self, expr_id: ExprId) -> Option<Vec<Name>> {
        match self.module.raw_module().expr(expr_id) {
            ast::Expr::Ident(name) => Some(vec![name.clone()]),
            ast::Expr::Member { base, member, .. }
            | ast::Expr::OptionalMember { base, member, .. } => {
                let mut path = self.expr_path(*base)?;
                path.push(member.clone());
                Some(path)
            }
            _ => None,
        }
    }

    /// The type a presence test has narrowed `path` to, innermost narrowing first.
    fn narrowed_type(&self, path: &[Name]) -> Option<Type> {
        let root_depth = self.env.binding_depth(path.first()?);
        self.narrowings
            .iter()
            .rev()
            .find(|narrowing| narrowing.path == path && narrowing.root_depth == root_depth)
            .map(|narrowing| narrowing.ty.clone())
    }

    /// Puts `narrowings` in force, returning the depth to pass to `pop_narrowings` when the
    /// branch they apply to is left.
    fn push_narrowings(&mut self, narrowings: Vec<(Vec<Name>, Type)>) -> usize {
        let depth = self.narrowings.len();
        for (path, ty) in narrowings {
            let root_depth = path.first().and_then(|root| self.env.binding_depth(root));
            self.narrowings.push(Narrowing {
                path,
                ty,
                root_depth,
            });
        }
        depth
    }

    /// Removes the narrowings pushed since `push_narrowings` returned `depth`.
    fn pop_narrowings(&mut self, depth: usize) {
        self.narrowings.truncate(depth);
    }

    /// `ty`, unless the expression is a path a presence test has narrowed, in which case the
    /// narrowed type wins.
    fn narrowed_or(&self, expr_id: ExprId, ty: Type) -> Type {
        match self
            .expr_path(expr_id)
            .and_then(|path| self.narrowed_type(&path))
        {
            Some(narrowed) if !ty.is_error() => narrowed,
            _ => ty,
        }
    }

    /// Infers `expr` with `narrowings` in force, and removes them afterwards.
    fn infer_under_narrowings(&mut self, narrowings: Vec<(Vec<Name>, Type)>, expr: ExprId) -> Type {
        let depth = self.push_narrowings(narrowings);
        let ty = self.infer_expr(expr);
        self.pop_narrowings(depth);
        ty
    }

    /// True when two values whose occurrences differ can still be compared for equality: every
    /// value compares as a sequence, so `int?` and `int+` compare item by item. The item types
    /// must be comparable as exactly-one values are, and the empty type compares with anything.
    fn items_comparable(&mut self, lhs_ty: &Type, rhs_ty: &Type) -> bool {
        let (lhs_item, rhs_item) = (lhs_ty.item().clone(), rhs_ty.item().clone());
        if lhs_item == Type::never() || rhs_item == Type::never() {
            return true;
        }
        match (&lhs_item, &rhs_item) {
            (Type::Primitive(a), Type::Primitive(b)) if a.is_numeric() && b.is_numeric() => {
                Primitive::numeric_promotion(*a, *b).is_some()
            }
            (Type::UnionCase(lhs_case), Type::UnionCase(rhs_case)) => {
                lhs_case.shares_union_with(rhs_case)
            }
            _ => {
                self.type_satisfies_expected(&lhs_item, &rhs_item)
                    || self.type_satisfies_expected(&rhs_item, &lhs_item)
            }
        }
    }

    /// The narrowings a condition establishes in the branch it proves (`when` is `true`) or
    /// refutes (`when` is `false`).
    ///
    /// <para>Four forms narrow and nothing else does: `p?` narrows `p` where it is true; `!c`
    /// swaps the branches of `c`; `a && b` narrows by both operands where it is true; and a
    /// tested chain `a?.b?` narrows every receiver along it, since the chain can only hold a value
    /// when each step did. A disjunction narrows nothing in either branch, and a call, a `for`
    /// body or a `let` binding of a test result narrows nothing at all. The types are read from
    /// the condition's own inference, which ran before this.</para>
    fn condition_narrowings(&self, condition: ExprId, when: bool) -> Vec<(Vec<Name>, Type)> {
        let mut narrowings = Vec::new();
        match self.module.raw_module().expr(condition) {
            ast::Expr::Exists { operand, .. } if when => {
                self.presence_narrowings(*operand, &mut narrowings);
            }
            ast::Expr::UnaryOp {
                op: ast::UnOp::Not,
                expr,
                ..
            } => return self.condition_narrowings(*expr, !when),
            ast::Expr::BinaryOp {
                lhs,
                op: ast::BinOp::And,
                rhs,
                ..
            } if when => {
                narrowings.extend(self.condition_narrowings(*lhs, true));
                narrowings.extend(self.condition_narrowings(*rhs, true));
            }
            _ => {}
        }
        narrowings
    }

    /// The narrowings a presence test of `operand` establishes: the operand's own path, and the
    /// receiver of every `?.` step along it, each read with zero removed.
    fn presence_narrowings(&self, operand: ExprId, narrowings: &mut Vec<(Vec<Name>, Type)>) {
        if let ast::Expr::OptionalMember { base, .. } = self.module.raw_module().expr(operand) {
            self.presence_narrowings(*base, narrowings);
        }
        let Some(path) = self.expr_path(operand) else {
            return;
        };
        let Some(ty) = self.env.get_expr_type(operand) else {
            return;
        };
        if ty.is_error() || !ty.admits_zero() {
            return;
        }
        let narrowed = ty.with_occurrence(ty.occurrence().without_zero());
        narrowings.push((path, narrowed));
    }

    /// Infers the type of an expression.
    pub fn infer_expr(&mut self, expr_id: ExprId) -> Type {
        let expr = self.module.raw_module().expr(expr_id);

        let ty = match expr {
            // Literals have known types
            ast::Expr::Literal(lit) => self.infer_literal(lit),

            // A bare name has no context-free type. It carries its own name forward as a pending
            // marker and is resolved at the binding site, which is the only place that knows the
            // expected type.
            ast::Expr::ContextualName { name, .. } => Type::ContextualName(name.clone()),

            // Already resolved, and typed by the union it names rather than by anything visible
            // here. Re-inference reaches this only when a caller types an expression twice.
            ast::Expr::ResolvedUnionCase {
                union,
                case,
                module_identity,
                definition_id,
                ..
            } => Type::union_case_type(
                union.clone(),
                case.clone(),
                Some(DeclaringOrigin::new(module_identity, *definition_id)),
            ),

            // Identifiers look up in environment, then in the narrowings in force.
            ast::Expr::Ident(name) => {
                self.record_loop_name_use(expr_id, name);
                let ty = match self.env.lookup(name) {
                    Some(ty) => ty.clone(),
                    None => Type::Error,
                };
                self.narrowed_or(expr_id, ty)
            }

            // Binary operations
            ast::Expr::BinaryOp { lhs, op, rhs, span } => {
                self.infer_binop(expr_id, *op, *lhs, *rhs, *span)
            }

            // A range expression means the prelude's `Range`, and nothing else.
            ast::Expr::Range {
                start,
                end,
                inclusive,
                span,
            } => self.infer_range(*start, *end, *inclusive, *span),

            // Only reached by re-inference: both are produced by the rewrite that runs after
            // analysis, from decisions this checker made.
            ast::Expr::Concat { .. } | ast::Expr::ToText { .. } => Type::string(),
            ast::Expr::Widen { expr, ty, .. } => {
                let inner = self.infer_expr(*expr);
                widened_type(&inner, *ty)
            }

            // Unary operations
            ast::Expr::UnaryOp { op, expr, span } => {
                let expr_ty = self.infer_expr(*expr);
                self.infer_unop(*op, &expr_ty, *span)
            }

            // Function calls
            ast::Expr::Call { func, args, span } => {
                // An intrinsic resolves before anything in scope, so the callee is never looked
                // up: a parameter or local named `merge` is invisible at a call.
                if let Some(intrinsic) = self.intrinsic_callee(*func) {
                    self.infer_intrinsic_call(intrinsic, args, *span)
                } else {
                    let func_ty = self.infer_expr(*func);

                    // A function-typed value is called as an element, never by position: the
                    // value's own declaration may order — or omit — parameters differently from
                    // the type, so positions would mean nothing at run time.
                    if let Some((name, params)) = self.function_value_callee(*func) {
                        let form = std::iter::once(format!("<{name}"))
                            .chain(params.iter().map(|param| format!("{}=...", param.name)))
                            .collect::<Vec<_>>()
                            .join(" ")
                            + " />";
                        self.error(
                            "positional-call-of-function-value",
                            format!(
                                "'{name}' is a function-typed value, so its arguments bind by name; \
                                 call it as an element, {form}"
                            ),
                            *span,
                        );
                        for arg in args {
                            self.infer_expr(*arg);
                        }
                        return Type::Error;
                    }

                    // An element-style function binds its arguments by name only: its signature
                    // lists them as attributes, in no order a caller is meant to rely on.
                    let declared = self.declared_function_callee(*func);
                    if let Some(callee) = declared
                        .as_ref()
                        .filter(|callee| callee.form == nx_hir::FunctionForm::Element)
                    {
                        let form = std::iter::once(format!("<{}", callee.name))
                            .chain(callee.params.iter().map(|(name, _)| format!("{name}=...")))
                            .collect::<Vec<_>>()
                            .join(" ")
                            + " />";
                        self.error(
                            "positional-call-of-element-function",
                            format!(
                                "'{}' is declared in element style, so its arguments bind by \
                                 name; call it as an element, {form}",
                                callee.name
                            ),
                            *span,
                        );
                        for arg in args {
                            self.infer_expr(*arg);
                        }
                        return Type::Error;
                    }

                    // A call may leave off the trailing parameters a caller may omit; the function
                    // fills each with its default, or with empty.
                    let required = declared.as_ref().map(|callee| {
                        callee
                            .params
                            .iter()
                            .rposition(|(_, omissible)| !omissible)
                            .map_or(0, |last| last + 1)
                    });

                    // Infer argument types
                    let arg_tys: Vec<_> = args.iter().map(|arg| self.infer_expr(*arg)).collect();

                    self.infer_call(&func_ty, args, &arg_tys, required, *span)
                }
            }

            // If expressions
            ast::Expr::If {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                let cond_ty = self.infer_expr(*condition);

                // Condition must be boolean
                if !cond_ty.is_compatible_with(&Type::boolean()) && !cond_ty.is_error() {
                    self.error(
                        "type-mismatch",
                        format!("If condition must be boolean, found {}", cond_ty),
                        *span,
                    );
                }

                // A presence test in the condition narrows the tested path in the branch it
                // proves: `p?` in the `then`, `!p?` in the `else`.
                let then_narrowings = self.condition_narrowings(*condition, true);
                let then_ty = self.infer_under_narrowings(then_narrowings, *then_branch);

                // A missing `else` is an `else { }`. Joining with the empty value is the whole of
                // the rule: the join takes the least upper bound of the occurrences, so a branch
                // producing `T` gives `T?` and one producing `T+` gives `T*` -- including a branch
                // that is itself a conditional with no `else`, which is why nesting needs nothing
                // of its own. Only branches that exist can be widened.
                match else_branch {
                    Some(else_id) => {
                        let else_narrowings = self.condition_narrowings(*condition, false);
                        let else_ty = self.infer_under_narrowings(else_narrowings, *else_id);
                        let joined = self.common_supertype(&then_ty, &else_ty);
                        let members = [(*then_branch, then_ty), (*else_id, else_ty)];
                        self.record_join_lifts(&members, &joined);
                        self.record_join_widenings(&members, &joined);
                        joined
                    }
                    None => {
                        let joined = self.common_supertype(&then_ty, &Type::empty());
                        let members = [(*then_branch, then_ty)];
                        self.record_join_lifts(&members, &joined);
                        self.record_join_widenings(&members, &joined);
                        joined
                    }
                }
            }

            ast::Expr::Match {
                scrutinee,
                arms,
                else_branch,
                span,
            } => self.infer_match_expr(*scrutinee, arms, *else_branch, *span),

            // Braced values
            ast::Expr::Array { elements, span } => {
                if elements.is_empty() {
                    // `{}` is the empty value, and its type is the empty type: the bottom item
                    // type under `?`. That is its type outright, not a placeholder for one the
                    // site has yet to supply: `never` is below every item type and `?` below `*`,
                    // so the one value is usable at every `?` and `*` site without anything
                    // having to be resolved later.
                    Type::empty()
                } else if elements.len() == 1 {
                    // A single item is itself: `{x}` is `x`.
                    self.infer_expr(elements[0])
                } else {
                    // Items splice: each contributes its item type and its occurrence, and the
                    // occurrences add -- two items that may each be present may be two, and the
                    // collection is at least one when any item is. The join is over item types,
                    // and so is the widening that follows it: what `{ns 1.5}` has to widen to
                    // `float64` is the `int` inside `ns`, not `ns` itself. The widening is still
                    // recorded against the item, because the item is what a widening wraps -- and
                    // wrapping a sequence widens it element by element.
                    let contributions: Vec<_> = elements
                        .iter()
                        .map(|e| {
                            self.infer_expr(*e);
                            self.item_contribution_of(*e)
                        })
                        .collect();
                    let item_types: Vec<_> =
                        contributions.iter().map(|(ty, _)| ty.clone()).collect();
                    let item_ty = self.common_sequence_item_type(&item_types, *span);
                    let occ = Self::summed_occurrence(contributions.iter().map(|(_, occ)| *occ));
                    let members = elements.iter().copied().zip(item_types).collect::<Vec<_>>();
                    self.record_join_widenings(&members, &item_ty);
                    Type::seq(item_ty, occ)
                }
            }

            // Index operation
            ast::Expr::Index { base, index, span } => {
                let base_ty = self.infer_expr(*base);
                let index_ty = self.infer_expr(*index);

                // Index must be an integer of any width
                if !index_ty.is_compatible_with(&Type::int()) && !index_ty.is_error() {
                    self.error(
                        "type-mismatch",
                        format!("Index must be an integer, found {}", index_ty),
                        *span,
                    );
                }

                // Base must be a sequence
                match base_ty {
                    Type::Seq { item, .. } => *item,
                    Type::Error => Type::Error,
                    _ => {
                        self.error(
                            "type-mismatch",
                            format!("Cannot index into {}, which is not a sequence", base_ty),
                            *span,
                        );
                        Type::Error
                    }
                }
            }

            // Member access
            ast::Expr::Member { base, member, span } => {
                let ty = if let Some(name) = self.flattened_expr_name(expr_id) {
                    if let Some(ty) = self.env.lookup(&name) {
                        ty.clone()
                    } else if self.report_unresolved_property_reference(&name, *span) {
                        Type::Error
                    } else if let Some((entry, case)) = self.union_case_from_qualified_name(&name) {
                        let union_name = entry.def.name.clone();
                        let case_name = case.name.clone();
                        let is_fieldless = case.fields.is_empty();
                        let case_ty = entry.case_type(case_name.clone());
                        if is_fieldless {
                            case_ty
                        } else {
                            self.error(
                                "payload-union-case-requires-constructor",
                                format!(
                                    "Union case '{}.{}' requires element-style payload construction",
                                    union_name, case_name
                                ),
                                *span,
                            );
                            Type::Error
                        }
                    } else if let Some(union_info) = self.union_info_for_expr(*base) {
                        self.union_case_by_member(&union_info, member, *span)
                    } else {
                        let base_ty = self.infer_expr(*base);
                        self.infer_member_access(*base, &base_ty, member, *span)
                    }
                } else if let Some(union_info) = self.union_info_for_expr(*base) {
                    self.union_case_by_member(&union_info, member, *span)
                } else {
                    let base_ty = self.infer_expr(*base);
                    self.infer_member_access(*base, &base_ty, member, *span)
                };
                self.narrowed_or(expr_id, ty)
            }

            // `x?.m`: a step through a receiver that may be empty.
            ast::Expr::OptionalMember { base, member, span } => {
                let base_ty = self.infer_expr(*base);
                let ty = self.infer_optional_member_access(&base_ty, member, *span);
                self.narrowed_or(expr_id, ty)
            }

            // `x?`: true when `x` holds an item. The operand must be able to be empty, or the
            // test is always true and is reported as such.
            ast::Expr::Exists { operand, span } => {
                let operand_ty = self.infer_expr(*operand);
                if !operand_ty.is_error() && !operand_ty.admits_zero() {
                    self.warn(
                        "presence-test-always-true",
                        format!(
                            "A value of type {} is always present, so the `?` test is always true",
                            operand_ty
                        ),
                        *span,
                    );
                }
                Type::boolean()
            }

            // `x ?? y`: `x` when it holds an item, `y` otherwise. The result admits what `x` admits
            // with zero removed, joined with what `y` admits.
            ast::Expr::Coalesce { left, right, span } => {
                let left_ty = self.infer_expr(*left);
                let right_ty = self.infer_expr(*right);
                if left_ty.is_error() || right_ty.is_error() {
                    Type::Error
                } else {
                    if !left_ty.admits_zero() {
                        self.warn(
                            "fallback-never-taken",
                            format!(
                                "A value of type {} is always present, so the `??` fallback is never taken",
                                left_ty
                            ),
                            *span,
                        );
                    }
                    let present = left_ty.with_occurrence(left_ty.occurrence().without_zero());
                    let joined = self.common_supertype(&present, &right_ty);
                    let members = [(*left, present), (*right, right_ty)];
                    self.record_join_lifts(&members, &joined);
                    self.record_join_widenings(&members, &joined);
                    joined
                }
            }

            ast::Expr::Element { element, span } => {
                let element_ref = self.module.raw_module().element(*element).clone();
                self.infer_element_expression(*element, &element_ref, *span)
            }

            ast::Expr::RecordLiteral {
                record,
                properties,
                span,
            } => self.infer_record_literal(record, properties, *span),
            ast::Expr::ActionHandler {
                action_name,
                action_module_identity,
                owner,
                body,
                ..
            } => self.infer_action_handler(
                action_name,
                action_module_identity.as_deref(),
                owner.as_ref(),
                *body,
            ),

            // Block expressions
            ast::Expr::Block { stmts: _, expr, .. } => {
                // TODO: Process statements
                if let Some(expr_id) = expr {
                    self.infer_expr(*expr_id)
                } else {
                    // Unreachable from source: lowering builds no `Block`, so a block with no
                    // trailing expression exists only in hand-written HIR. The bottom type is the
                    // honest answer for an expression that produces no value, and it keeps every
                    // inferred type one an author could be shown.
                    Type::never()
                }
            }

            // For loop expressions
            ast::Expr::For {
                item,
                index,
                iterable,
                body,
                ..
            } => {
                // Infer iterable type: a sequence or an optional, or a range of an integer type.
                let iterable_ty = self.infer_expr(*iterable);
                let (item_ty, iterated_occ) = match iterable_ty.clone() {
                    Type::Seq { item, occ } => (*item, occ),
                    Type::Error => (Type::Error, Occurrence::ZERO_OR_MORE),
                    // An optional range is not accepted, exactly as an optional sequence is
                    // iterated as what it holds: `prelude_range_argument` matches the type as it
                    // stands. A range may be empty, so it iterates zero or more times.
                    other => match self.prelude_range_argument(&other) {
                        // A range of an integer type counts; the item is an integer of that type
                        // and the index, as for a sequence, an `int`.
                        Some(Type::Primitive(primitive)) if primitive.is_integer() => {
                            self.range_for_expressions.insert(expr_id);
                            (Type::Primitive(primitive), Occurrence::ZERO_OR_MORE)
                        }
                        // Any other range is rejected rather than given a guessed step.
                        Some(_) => {
                            self.error(
                                "range-not-iterable",
                                format!(
                                    "A for iterable must be a sequence or a range of an integer \
                                     type, found {other}: only a range of an integer type iterates"
                                ),
                                expr.span(),
                            );
                            (Type::Error, Occurrence::ZERO_OR_MORE)
                        }
                        None => {
                            self.error(
                                "type-mismatch",
                                format!(
                                    "For iterable must be a sequence or an optional value, found {}",
                                    other
                                ),
                                expr.span(),
                            );
                            (Type::Error, Occurrence::ZERO_OR_MORE)
                        }
                    },
                };

                self.env.push_scope();
                self.env.bind(item.clone(), item_ty.clone());
                if let Some(index_name) = index {
                    self.env.bind(index_name.clone(), Type::int());
                    self.indexed_loops.push(IndexedLoop {
                        item: item.clone(),
                        index: index_name.clone(),
                        item_ty,
                        scope_depth: self.env.scope_depth(),
                        item_uses: Vec::new(),
                        index_uses: Vec::new(),
                    });
                }
                // A `for` concatenates what its body yields, so the result's item type is what
                // one iteration contributes, and its occurrence is the product of the iterated
                // occurrence and the body's: at least one only when both are, bounded by one only
                // when both are. Over `int+` a body of `int` yields `int+`, a body that may
                // yield nothing yields `int*`, and over an optional a body of `string` yields
                // `string?`.
                let (body_item, body_occ) = self.item_contribution(*body);
                if index.is_some() {
                    self.indexed_loops.pop();
                }
                self.env.pop_scope();

                Type::seq(body_item, iterated_occ.product(body_occ))
            }

            // Let expressions (used for match lowering)
            ast::Expr::Let {
                name, value, body, ..
            } => {
                // Infer the type of the value
                let value_ty = self.infer_expr(*value);

                // Create a new scope for the let binding
                self.env.push_scope();

                // Bind the name to the value type in this scope
                self.env.bind(name.clone(), value_ty);

                // Infer the body with the binding in scope
                let body_ty = self.infer_expr(*body);

                // Pop the scope to remove the binding
                self.env.pop_scope();

                body_ty
            }

            // Error expressions
            ast::Expr::Error(_) => Type::Error,
        };

        // Record the inferred type
        self.env.set_expr_type(expr_id, ty.clone());
        ty
    }

    /// Infers all types within a function, binding parameters while visiting the body.
    pub fn infer_function(&mut self, func: &nx_hir::Function) {
        // Parameters get a scope of their own, so one that shares a name with a top-level binding
        // — a function-typed parameter `Row` beside a declared function `Row` — shadows it for
        // the body and leaves it in place afterwards.
        self.env.push_scope();
        // Each annotation is resolved once, here, where the parameter's span says where a
        // problem with it was written; the signature below reuses these types rather than
        // resolving them a second time.
        let mut param_types = Vec::with_capacity(func.params.len());
        for param in &func.params {
            let param_ty = self.property_slot_type(param.span, &param.name, &param.ty);
            // A default is checked before its own parameter is bound, so it sees the parameters
            // declared before it and nothing later.
            if let Some(default) = param.default {
                let actual = self.infer_expr(default);
                self.check_typed_binding_for(
                    Some(default),
                    &actual,
                    &param_ty,
                    param.span,
                    "parameter-default-type-mismatch",
                    format!("Default value for parameter '{}'", param.name),
                );
            }
            // The body reads an optional parameter at a type that admits zero.
            self.env
                .bind(param.name.clone(), read_type(&param_ty, param.optional));
            // A parameter rejected for a type that admits zero stays an error in the body, but the
            // signature reads it as the fix-it writes it, `b?:T` for `b:T?`, so a call still checks
            // the argument written for it, as an element's property does.
            let (signature_ty, optional) = match param_ty {
                Type::Error => {
                    let declared = self.type_from_type_ref_in_quietly(None, &param.ty);
                    if declared.admits_zero() {
                        let marked = if declared.admits_many() {
                            Occurrence::ONE_OR_MORE
                        } else {
                            Occurrence::ONE
                        };
                        (declared.with_occurrence(marked), true)
                    } else {
                        (Type::Error, param.optional)
                    }
                }
                ty => (ty, param.optional),
            };
            param_types.push(FunctionParam {
                name: param.name.clone(),
                ty: signature_ty,
                is_content: param.is_content,
                optional,
            });
        }

        let body_ty = self.infer_expr(func.body);
        self.env.pop_scope();

        let return_ty = if let Some(ty) = func.return_type.as_ref() {
            let expected = self.type_from_type_ref_at(func.span, ty);
            self.check_typed_binding_for(
                Some(func.body),
                &body_ty,
                &expected,
                func.span,
                "return-type-mismatch",
                format!("Return value for function '{}'", func.name),
            );
            expected
        } else {
            // The same rule as an unannotated value binding, at the other place a binding's type
            // can be fixed by an empty value. `{}` is a type this function could simply have; it
            // is reported because a signature saying only "nothing in particular" tells a caller
            // nothing, and the annotation is where that gets said.
            if Self::mentions_never(&body_ty) {
                self.error(
                    "empty-value-type-unknown",
                    format!(
                        "Cannot determine the item type of the empty value returned by '{}'; \
                         annotate the return type with the type you mean, as in \
                         'let {}(): string* = {{}}'",
                        func.name, func.name
                    ),
                    func.span,
                );
            }
            body_ty.clone()
        };

        self.env.bind(
            func.name.clone(),
            Type::function(param_types, return_ty.clone()),
        );
        if func.return_type.is_none() {
            self.function_return_placeholders.remove(&func.name);
        }
    }

    /// Infers a component's prop and state defaults and its body.
    ///
    /// <para>A component body is markup written against the same binding sites an element in a
    /// function body has, so it is inferred the same way. Props and state are bound by name, because
    /// the body reads them, and they are bound in a pushed scope so a prop cannot outlive the
    /// component that declared it.</para>
    ///
    /// <para>The props bound are the component's *effective* ones, so a prop reached through the
    /// base chain reads at its declared type the way a directly declared one does. Scope building
    /// already resolves an inherited name, so leaving it unbound would not fail the read — it would
    /// make it infer vacuously, which is the one outcome worse than either. Each inherited type is
    /// resolved in the module that declared the field, since that is the module whose names it was
    /// written against.</para>
    ///
    /// <para>Fields are bound in the order both runtimes materialize them — the effective props,
    /// then the state — and each default is checked *before* its own field is bound. That is what
    /// makes a default's environment here the same one it will have when it runs: it sees the
    /// fields materialized before it and nothing else. A name from later in the declaration is
    /// reported as undefined by scope checking, so nothing is silently accepted; it simply is not
    /// this pass's diagnostic to give.</para>
    ///
    /// <para>A defaulted prop is checked against its own declared type, which is also what lets a
    /// contextual literal be written as a default: the default's binding site is the declaration
    /// itself.</para>
    pub fn infer_component(&mut self, component: &nx_hir::Component) {
        self.env.push_scope();

        let contract = self
            .effective_component_contract(&component.name)
            .ok()
            .flatten();
        let type_param_names: Vec<Name> = match contract.as_ref() {
            Some(contract) => contract
                .type_params
                .iter()
                .map(|param| param.name.clone())
                .collect(),
            None => component
                .type_params
                .iter()
                .map(|param| param.name.clone())
                .collect(),
        };
        // Every effective type parameter is a rigid type from here to the end of the body, ahead
        // of whatever else the name might reach. The scope is replaced rather than extended
        // because components do not nest: nothing enclosing has parameters of its own.
        let owner = nx_hir::component_declaration_origin(self.module, &component.name);
        let previous_scope = std::mem::replace(
            &mut self.type_parameter_scope,
            Self::rigid_type_parameter_scope(&type_param_names, owner),
        );
        let effective_props = contract.map(|contract| contract.props);

        match effective_props {
            Some(props) => {
                for field in &props {
                    // Only a prop *this* component declared is reported here, at the span it was
                    // written at. An inherited one belongs to the base, whose own pass reports it
                    // — in this module if the base is local, and in its own module's check if not,
                    // where its span means something. Reporting it again here would be a second
                    // copy of the same message, and for a base in another module it would land at
                    // offset 0 of this file.
                    let declared_here = component
                        .props
                        .iter()
                        .any(|declared| declared.name == field.name);
                    let field_ty = if declared_here {
                        self.property_slot_type_in(
                            field.span,
                            Some(&field.module_identity),
                            &field.name,
                            &field.ty,
                        )
                    } else {
                        self.type_from_type_ref_in_quietly(Some(&field.module_identity), &field.ty)
                    };
                    self.check_component_field_default(component, &field.name, &field_ty);
                    // The body reads an optional prop at a type that admits zero.
                    self.env
                        .bind(field.name.clone(), read_type(&field_ty, field.optional));
                }
            }
            // Without a contract there is no base chain to read, and the component's own props are
            // the whole of what its body and its defaults can see.
            None => {
                for field in &component.props {
                    let field_ty = self.property_slot_type(field.span, &field.name, &field.ty);
                    self.check_component_field_default(component, &field.name, &field_ty);
                    self.env
                        .bind(field.name.clone(), read_type(&field_ty, field.optional));
                }
            }
        }

        for field in &component.state {
            let field_ty = self.property_slot_type(field.span, &field.name, &field.ty);
            self.check_component_field_default(component, &field.name, &field_ty);
            // Optional state starts empty and reads at a type that admits zero.
            self.env
                .bind(field.name.clone(), read_type(&field_ty, field.optional));
        }

        // A body is absent exactly when the component is abstract or external, and there is then
        // nothing below the declaration to infer.
        if let Some(body) = component.body {
            self.infer_expr(body);
        }

        self.type_parameter_scope = previous_scope;
        self.env.pop_scope();
    }

    /// The scope in which each of `names` denotes its own rigid type, owned by `owner`.
    fn rigid_type_parameter_scope(
        names: &[Name],
        owner: Option<DeclaringOrigin>,
    ) -> FxHashMap<Name, Type> {
        names
            .iter()
            .enumerate()
            .map(|(ordinal, name)| {
                (
                    name.clone(),
                    Type::parameter(name.clone(), owner.clone(), ordinal),
                )
            })
            .collect()
    }

    /// Infers a handler body where it is bound, then checks where each of its results can go.
    ///
    /// <para>The body sees what its binding site sees — the owner's props and state, enclosing
    /// `let` bindings and loop variables, all already in the environment — plus `action`, typed by
    /// the action the component emits. That action is resolved in the module that declared the
    /// emit, since that is where its name was written.</para>
    ///
    /// <para>The handler value itself has no first-class type, so the expression stays
    /// `Type::Error`, which no binding site reports. A handler property has no declared type to
    /// check against; the body is what gets checked, here.</para>
    fn infer_action_handler(
        &mut self,
        action_name: &Name,
        action_module_identity: Option<&str>,
        owner: Option<&Name>,
        body: ExprId,
    ) -> Type {
        let action_ty = self.type_from_type_ref_in(
            action_module_identity,
            &ast::TypeRef::name(action_name.as_str()),
        );
        self.env.push_scope();
        self.env.bind(Name::new("action"), action_ty);
        self.infer_expr(body);
        self.env.pop_scope();

        let mut items = Vec::new();
        self.collect_handler_result_items(body, &mut items);
        for item in items {
            match item {
                HandlerResultItem::MayBeEmpty(ty, span) => self.error(
                    "handler-result-empty",
                    format!(
                        "Action handler must return at least one action or update record; found \
                         a value of type {} that may be empty",
                        ty
                    ),
                    span,
                ),
                HandlerResultItem::Value(ty, span) => {
                    self.check_handler_result_item(owner, &ty, span)
                }
            }
        }

        Type::Error
    }

    /// Splits a handler body's result into the values it returns, each with where it was written.
    ///
    /// <para>A list literal is split by element, and each branch of an `if` and each arm of a
    /// match separately, so each value is judged on its own type rather than on a join of
    /// unrelated records. Anything else is judged by its type, a list type by its element
    /// type.</para>
    fn collect_handler_result_items(&self, expr_id: ExprId, items: &mut Vec<HandlerResultItem>) {
        let raw_module = self.module.raw_module();
        let span = raw_module.expr_span(expr_id);
        match raw_module.expr(expr_id) {
            ast::Expr::Array { elements, .. } if !elements.is_empty() => {
                for element in elements {
                    let ty = self
                        .env
                        .get_expr_type(*element)
                        .cloned()
                        .unwrap_or(Type::Error);
                    items.push(HandlerResultItem::Value(ty, raw_module.expr_span(*element)));
                }
            }
            ast::Expr::If {
                then_branch,
                else_branch: Some(else_branch),
                ..
            } => {
                self.collect_handler_result_items(*then_branch, items);
                self.collect_handler_result_items(*else_branch, items);
            }
            ast::Expr::Match {
                arms, else_branch, ..
            } => {
                for arm in arms {
                    self.collect_handler_result_items(arm.body, items);
                }
                if let Some(else_branch) = else_branch {
                    self.collect_handler_result_items(*else_branch, items);
                }
            }
            _ => match self
                .env
                .get_expr_type(expr_id)
                .cloned()
                .unwrap_or(Type::Error)
            {
                // A handler's result must not admit zero: it is one or more actions or updates.
                ty if ty.admits_zero() => items.push(HandlerResultItem::MayBeEmpty(ty, span)),
                Type::Seq { item, .. } => items.push(HandlerResultItem::Value(*item, span)),
                ty => items.push(HandlerResultItem::Value(ty, span)),
            },
        }
    }

    /// Checks one handler result against where a handler bound at this site may send it.
    ///
    /// <para>Inside a component a result either patches that component's state — its own update
    /// record — or goes to its parent, which only an action the component emits may do. At the root
    /// there is no component to patch or parent to reach, so any action or update record is an
    /// effect for the host.</para>
    fn check_handler_result_item(&mut self, owner: Option<&Name>, ty: &Type, span: TextSpan) {
        let named = match ty {
            Type::Error => return,
            Type::Named(named) => named,
            other => {
                self.report_handler_result_not_action(&other.to_string(), span);
                return;
            }
        };
        let Some(record) = self.record_definition_for(named) else {
            // An unresolved `X.Update` was already reported where it was written, so a second
            // diagnostic here would only repeat that the record does not exist.
            if !nx_hir::is_update_record_name(named.name.as_str()) {
                self.report_handler_result_not_action(named.name.as_str(), span);
            }
            return;
        };
        let Some(owner) = owner else {
            if record.kind == nx_hir::RecordKind::Plain {
                self.report_handler_result_not_action(named.name.as_str(), span);
            }
            return;
        };

        match &record.kind {
            nx_hir::RecordKind::Plain => {
                self.report_handler_result_not_action(named.name.as_str(), span)
            }
            nx_hir::RecordKind::Update { .. } => {
                let own_update = nx_hir::update_record_name(owner.as_str());
                let expected = match self.nominal_named_type(&own_update) {
                    Type::Named(expected) if expected.origin().is_some() => Some(expected),
                    _ => None,
                };
                match expected {
                    Some(expected) if named.is_same_declaration_as(&expected) => {}
                    Some(_) => self.error(
                        "handler-update-wrong-target",
                        format!(
                            "Action handler inside component '{}' returns '{}', but only '{}' can change this component's state",
                            owner, named.name, own_update
                        ),
                        span,
                    ),
                    // A component without state has no update record, so there is nothing to
                    // change and no own record to name.
                    None => self.error(
                        "handler-update-wrong-target",
                        format!(
                            "Action handler inside component '{}' returns '{}', but '{}' declares no state to change",
                            owner, named.name, owner
                        ),
                        span,
                    ),
                }
            }
            nx_hir::RecordKind::Action => {
                if !self.component_emits_action(owner, named) {
                    self.error(
                        "handler-action-not-emitted",
                        format!(
                            "Action handler inside component '{}' returns action '{}', which '{}' does not emit; add '{}' to the component's emits to send it to the parent",
                            owner, named.name, owner, named.name
                        ),
                        span,
                    );
                }
            }
        }
    }

    fn report_handler_result_not_action(&mut self, found: &str, span: TextSpan) {
        self.error(
            "handler-result-not-action",
            format!(
                "Action handler must return an action, an update record, or a non-empty list of those; found '{}'",
                found
            ),
            span,
        );
    }

    /// The record a named type denotes, read from its declaration where the type reached one.
    fn record_definition_for(&self, named: &NamedType) -> Option<nx_hir::RecordDef> {
        match named.origin() {
            Some(origin) => nx_hir::resolve_record_definition_at(self.module, origin),
            None => self.resolve_record_definition(&named.name),
        }
    }

    /// Returns true when `component` declares or inherits an emit of exactly this action.
    ///
    /// <para>Each emit's action is resolved in the module that wrote the `emits` clause, and
    /// compared by declaration, so a same-named action elsewhere does not count.</para>
    fn component_emits_action(&mut self, component: &Name, action: &NamedType) -> bool {
        let Some(contract) = self.effective_component_contract(component).ok().flatten() else {
            return false;
        };
        contract.emits.iter().any(|emit| {
            let emit_ty = self.type_from_type_ref_in(
                Some(emit.module_identity.as_str()),
                &ast::TypeRef::name(emit.emit.action_name.as_str()),
            );
            matches!(&emit_ty, Type::Named(emitted) if emitted.is_same_declaration_as(action))
        })
    }

    /// Checks the default this component declares for one of its fields, where it declares one.
    ///
    /// An inherited field's default belongs to the module that declared it and is checked there;
    /// what is checked here is only what this declaration wrote.
    fn check_component_field_default(
        &mut self,
        component: &nx_hir::Component,
        name: &Name,
        field_ty: &Type,
    ) {
        let declared = component
            .props
            .iter()
            .chain(component.state.iter())
            .find(|field| &field.name == name);
        let Some(field) = declared else {
            return;
        };
        let Some(default) = field.default else {
            return;
        };

        let actual = self.infer_expr(default);
        self.check_typed_binding_for(
            Some(default),
            &actual,
            field_ty,
            field.span,
            "component-default-type-mismatch",
            format!("Default value for '{}.{}'", component.name, field.name),
        );
    }

    /// Infers the type of a literal.
    fn infer_literal(&mut self, lit: &ast::Literal) -> Type {
        match lit {
            ast::Literal::String(_) => Type::string(),
            ast::Literal::Int(_) => Type::int(),
            ast::Literal::Int32(_) => Type::int32(),
            ast::Literal::Float(_) => Type::float64(),
            ast::Literal::Float32(_) => Type::float32(),
            ast::Literal::Boolean(_) => Type::boolean(),
        }
    }

    fn infer_match_expr(
        &mut self,
        scrutinee: ExprId,
        arms: &[ast::MatchArm],
        else_branch: Option<ExprId>,
        span: TextSpan,
    ) -> Type {
        let scrutinee_ty = self.infer_expr(scrutinee);
        let scrutinee_path = self.expr_path(scrutinee);
        // A union under `?` still matches by case; the `{}` arm is what covers its absence.
        let union_ty = match scrutinee_ty.item() {
            Type::Union(union_ty) => Some(union_ty.clone()),
            _ => None,
        };

        let mut covered_cases = FxHashSet::default();
        let mut covered_empty = false;
        let mut result_tys = Vec::new();
        let mut result_members = Vec::new();

        for arm in arms {
            // The arms after a `{}` arm can only be reached by a value that holds an item, so a
            // path scrutinee reads as present in them, exactly as a presence test narrows it.
            let present_narrowings = self.present_scrutinee_narrowings(
                scrutinee_path.as_deref(),
                &scrutinee_ty,
                covered_empty,
            );
            let pattern_tys = arm
                .patterns
                .iter()
                .map(|pattern| {
                    let pattern_ty = self.infer_match_pattern(*pattern, &scrutinee_ty);
                    self.check_match_pattern(
                        &scrutinee_ty,
                        union_ty.as_ref(),
                        &pattern_ty,
                        &mut covered_cases,
                        &mut covered_empty,
                        self.module.raw_module().expr(*pattern).span(),
                    );
                    pattern_ty
                })
                .collect::<Vec<_>>();

            let narrowed_case = self.match_arm_narrowed_case(union_ty.as_ref(), &pattern_tys);
            let mut narrowings = present_narrowings;
            if let (Some(path), Some(case_ty)) = (scrutinee_path.as_ref(), narrowed_case) {
                narrowings.push((path.clone(), Type::UnionCase(case_ty)));
            }
            let body_ty = self.infer_under_narrowings(narrowings, arm.body);

            result_members.push((arm.body, body_ty.clone()));
            result_tys.push(body_ty);
        }

        let is_exhaustive = union_ty.as_ref().is_some_and(|union_ty| {
            union_ty
                .cases
                .iter()
                .all(|case| covered_cases.contains(case))
                && (covered_empty || !scrutinee_ty.admits_zero())
        });

        // A path no arm covers is the match's own absent `else`: the arms that are there decide
        // the item type, and the uncovered path contributes the empty value.
        let mut uncovered = false;
        if let Some(else_id) = else_branch {
            let narrowings = self.present_scrutinee_narrowings(
                scrutinee_path.as_deref(),
                &scrutinee_ty,
                covered_empty,
            );
            let else_ty = self.infer_under_narrowings(narrowings, else_id);
            result_members.push((else_id, else_ty.clone()));
            result_tys.push(else_ty);
        } else if let Some(union_ty) = union_ty.as_ref() {
            if !is_exhaustive {
                let mut missing = union_ty
                    .cases
                    .iter()
                    .filter(|case| !covered_cases.contains(*case))
                    .map(|case| case.as_str().to_string())
                    .collect::<Vec<_>>();
                if scrutinee_ty.admits_zero() && !covered_empty {
                    missing.push("{}".to_string());
                }
                self.error(
                    "non-exhaustive-union-match",
                    format!(
                        "Union match on '{}' is missing cases: {}",
                        union_ty.name,
                        missing.join(", ")
                    ),
                    span,
                );
                // The missing cases are the mistake, and they are reported. Reading them as an
                // `else { }` too would add a mismatch against the result type on top of it.
            }
        } else {
            uncovered = true;
        }

        // A path no arm covers is this match's missing `else`, and is read as an `else { }` --
        // the same rule an `if` without one follows, joined over arms rather than over a branch.
        if uncovered && !result_tys.is_empty() {
            result_tys.push(Type::empty());
        }
        let joined = self.common_result_type(&result_tys);
        self.record_join_lifts(&result_members, &joined);
        self.record_join_widenings(&result_members, &joined);
        joined
    }

    /// The narrowing the arms after a `{}` arm, and the `else` arm of a match with one, read a
    /// path scrutinee under: present, with zero removed from its occurrence.
    fn present_scrutinee_narrowings(
        &self,
        scrutinee_path: Option<&[Name]>,
        scrutinee_ty: &Type,
        covered_empty: bool,
    ) -> Vec<(Vec<Name>, Type)> {
        let Some(path) = scrutinee_path else {
            return Vec::new();
        };
        if !covered_empty || scrutinee_ty.is_error() || !scrutinee_ty.admits_zero() {
            return Vec::new();
        }
        let narrowed = scrutinee_ty.with_occurrence(scrutinee_ty.occurrence().without_zero());
        vec![(path.to_vec(), narrowed)]
    }

    fn infer_match_pattern(&mut self, pattern: ExprId, scrutinee_ty: &Type) -> Type {
        // A bare pattern resolves against the scrutinee's type in preference to any lexically
        // visible binding of the same name. The preference is reported so a pattern that used to
        // compare against a variable never changes meaning silently.
        if let ast::Expr::ContextualName { name, span, .. } = self.module.raw_module().expr(pattern)
        {
            let name = name.clone();
            let span = *span;
            if self.env.lookup(&name).is_some() {
                self.error(
                    "contextual-name-displaces-binding",
                    format!(
                        "Pattern '{}' resolves as a case of '{}', not as the binding named '{}' \
                         that is in scope here",
                        name, scrutinee_ty, name
                    ),
                    span,
                );
            }
            let context = format!("Pattern '{}'", name);
            let resolved =
                self.resolve_contextual_name_in(pattern, &name, scrutinee_ty, span, &context, true);
            let ty = resolved.unwrap_or(Type::Error);
            self.env.set_expr_type(pattern, ty.clone());
            return ty;
        }

        if let Some(name) = self.flattened_expr_name(pattern) {
            if let Some((entry, case)) = self.union_case_from_qualified_name(&name) {
                let ty = entry.case_type(case.name.clone());
                self.env.set_expr_type(pattern, ty.clone());
                return ty;
            }
        }

        self.infer_expr(pattern)
    }

    fn check_match_pattern(
        &mut self,
        scrutinee_ty: &Type,
        union_ty: Option<&UnionType>,
        pattern_ty: &Type,
        covered_cases: &mut FxHashSet<Name>,
        covered_empty: &mut bool,
        span: TextSpan,
    ) {
        if pattern_ty.is_error() || scrutinee_ty.is_error() {
            return;
        }

        // The `{}` pattern matches the empty value, so it is only worth writing where the
        // scrutinee can be empty.
        if pattern_ty.is_empty_type() {
            if scrutinee_ty.admits_zero() {
                *covered_empty = true;
            } else {
                self.warn(
                    "empty-pattern-never-matches",
                    format!(
                        "A value of type {} is always present, so the `{{}}` pattern never matches",
                        scrutinee_ty
                    ),
                    span,
                );
            }
            return;
        }

        if let Some(union_ty) = union_ty {
            match pattern_ty {
                Type::UnionCase(case_ty) if case_ty.is_same_union_as(union_ty) => {
                    covered_cases.insert(case_ty.case.clone());
                }
                Type::UnionCase(case_ty) => {
                    self.error(
                        "wrong-union-pattern",
                        format!(
                            "Pattern '{}.{}' is not a case of union '{}'",
                            case_ty.union, case_ty.case, union_ty.name
                        ),
                        span,
                    );
                }
                _ => {
                    self.error(
                        "invalid-union-case-pattern",
                        format!(
                            "Union match on '{}' requires union case patterns",
                            union_ty.name
                        ),
                        span,
                    );
                }
            }
            return;
        }

        if !self.type_satisfies_expected(scrutinee_ty, pattern_ty)
            && !self.type_satisfies_expected(pattern_ty, scrutinee_ty)
        {
            self.error(
                "type-mismatch",
                format!("Cannot compare types {} and {}", scrutinee_ty, pattern_ty),
                span,
            );
        }
    }

    fn match_arm_narrowed_case(
        &self,
        union_ty: Option<&UnionType>,
        pattern_tys: &[Type],
    ) -> Option<UnionCaseType> {
        if pattern_tys.len() != 1 {
            return None;
        }

        let union_ty = union_ty?;
        match &pattern_tys[0] {
            Type::UnionCase(case_ty) if case_ty.is_same_union_as(union_ty) => Some(case_ty.clone()),
            _ => None,
        }
    }

    /// Records each branch of a join that the join lifted from an item to a sequence.
    ///
    /// <para>Each one is wrapped as a one-item sequence after analysis, so the branch's value is
    /// what the join's type says: a taken `if c { 1 } else { xs }` evaluates to `[1]`. A join to
    /// `?` lifts nothing, since an optional value that holds an item is the item itself.</para>
    fn record_join_lifts(&mut self, members: &[(ExprId, Type)], joined: &Type) {
        // A branch is lifted when the join admits many and the branch does not: the join made a
        // sequence of it. A branch that is already a sequence, the empty value, and one of type
        // `never` or an error, contributes nothing of its own to lift. Forgotten otherwise, since
        // inference can visit a join more than once.
        let joined_is_sequence = joined.admits_many();
        for (expr, ty) in members {
            let lifted = joined_is_sequence
                && !ty.admits_many()
                && !ty.is_empty_type()
                && !ty.is_error()
                && !matches!(ty, Type::Primitive(Primitive::Never));
            if lifted {
                self.lifted_joins.insert(*expr);
            } else {
                self.lifted_joins.remove(expr);
            }
        }
    }

    /// Records each branch of a join whose numeric type is narrower than the join's.
    ///
    /// <para>A branch is recorded against the join's numeric type, which is also what it widens
    /// to under an occurrence. A branch that already has the join's type, or that
    /// the join did not widen (an `object` join, say), is forgotten, since inference can visit a
    /// join more than once.</para>
    fn record_join_widenings(&mut self, members: &[(ExprId, Type)], joined: &Type) {
        for (expr, ty) in members {
            match join_widening(ty, joined) {
                Some(target) => {
                    self.widened_joins.insert(*expr, target);
                }
                None => {
                    self.widened_joins.remove(expr);
                }
            }
        }
    }

    fn common_result_type(&self, result_tys: &[Type]) -> Type {
        // Nothing to join is the bottom type, the identity of the join.
        let mut current = result_tys.first().cloned().unwrap_or_else(Type::never);

        for ty in result_tys.iter().skip(1) {
            current = self.common_supertype(&current, ty);
        }

        current
    }

    /// Infers the result type of a binary operation, typing its operands first.
    ///
    /// <para>Two numeric operands are typed at the narrowest type both widen to, for arithmetic
    /// and comparison alike; a pair with no such type is rejected naming both. A literal operand
    /// first takes the other operand's numeric type, on the same terms as at a binding site, so a
    /// width the author chose for one operand is not lost to the literal's default: `w * 1.5` is
    /// a `float32` when `w` is.</para>
    ///
    /// <para>`+` with a string operand is concatenation. That is decided here, the one place both
    /// operand types are known, and recorded for the rewrite that makes it an `Expr::Concat` with
    /// each non-string operand wrapped in its text conversion.</para>
    /// Types `a..b` and `a..=b`: the prelude's `Range` over the operands' common numeric type.
    ///
    /// <para>The operators are sugar for a construction of the prelude's `Range`, so they are only
    /// meaningful where the name `Range` still means that declaration. `T` is the operands' common
    /// numeric type here; a site that expects a `<Range T=X/>` binds the operands to `X` instead,
    /// which the literal conversion of a binding site does.</para>
    fn infer_range(&mut self, start: ExprId, end: ExprId, inclusive: bool, span: TextSpan) -> Type {
        let origin = match self.prelude_range_name() {
            PreludeRangeName::Prelude(origin) => origin,
            PreludeRangeName::Hidden { by } => {
                self.report_range_hidden(&by, span);
                // The operands are still typed, so a fault inside one is reported too.
                self.infer_expr(start);
                self.infer_expr(end);
                return Type::Error;
            }
            PreludeRangeName::Absent => {
                self.error(
                    "range-prelude-unavailable",
                    format!(
                        "The range operator builds the built-in '{}' record, and the prelude is \
                         not available in this build",
                        nx_hir::PRELUDE_RANGE_NAME
                    ),
                    span,
                );
                self.infer_expr(start);
                self.infer_expr(end);
                return Type::Error;
            }
        };

        let start_ty = self.infer_expr(start);
        let end_ty = self.infer_expr(end);
        if start_ty.is_error() || end_ty.is_error() {
            return Type::Error;
        }

        let argument = match (&start_ty, &end_ty) {
            (Type::Primitive(lhs), Type::Primitive(rhs))
                if lhs.is_numeric() && rhs.is_numeric() =>
            {
                match Primitive::numeric_promotion(*lhs, *rhs) {
                    Some(promoted) => Type::Primitive(promoted),
                    None => {
                        self.error(
                            "type-mismatch",
                            format!(
                                "Cannot build a range from {} and {}: neither converts to the \
                                 other without loss, so the conversion is not implicit",
                                start_ty, end_ty
                            ),
                            span,
                        );
                        return Type::Error;
                    }
                }
            }
            _ => {
                let offender = if matches!(&start_ty, Type::Primitive(primitive) if primitive.is_numeric())
                {
                    &end_ty
                } else {
                    &start_ty
                };
                // The element form is the way to build a range of a non-numeric type, so it is the
                // help to offer — except when the offender is itself a range, where naming it as
                // `T` would advise a range of ranges rather than the mistake the author made.
                let message = if self.prelude_range_argument(offender).is_some() {
                    format!("A range operand must be numeric, found {offender}: a range is not a numeric bound")
                } else {
                    format!(
                        "A range operand must be numeric, found {offender}; write the element form \
                         <{} T={offender} start={{…}} end={{…}} endInclusive={{{inclusive}}} /> for \
                         a range of another type",
                        nx_hir::PRELUDE_RANGE_NAME
                    )
                };
                self.error("range-operand-not-numeric", message, span);
                return Type::Error;
            }
        };

        Type::Named(NamedType::applied(
            Name::new(nx_hir::PRELUDE_RANGE_NAME),
            Some(origin),
            vec![(Name::new("T"), argument)],
        ))
    }

    /// What the name `Range` means in this module.
    ///
    /// <para>The module's bindings decide it, and every namespace counts: a declaration of the
    /// module's own hides the prelude's whatever namespace it occupies, because the construction
    /// this operator rewrites to resolves `Range` by name alone. Asking the record origins first
    /// would miss a `let Range = 5`, which leaves the type namespace free.</para>
    fn prelude_range_name(&self) -> PreludeRangeName {
        let name = Name::new(nx_hir::PRELUDE_RANGE_NAME);

        for namespace in [
            PreparedNamespace::Type,
            PreparedNamespace::Element,
            PreparedNamespace::Value,
        ] {
            let Some(binding) = self.module.resolve_binding(namespace, &name) else {
                continue;
            };
            let by = match &binding.origin {
                PreparedBindingOrigin::Imported { module_identity }
                | PreparedBindingOrigin::Peer { module_identity } => module_identity.clone(),
                PreparedBindingOrigin::Local => self.module.module_identity().to_string(),
            };
            if by != nx_hir::PRELUDE_MODULE_IDENTITY {
                return PreludeRangeName::Hidden { by };
            }
        }

        // Every namespace holding the name holds the prelude's declaration. Its origin is the one
        // the record bindings recorded; nothing under the name at all is a build with no prelude.
        match self.record_origins.get(&name) {
            Some(origin) if origin.module_identity() == nx_hir::PRELUDE_MODULE_IDENTITY => {
                PreludeRangeName::Prelude(origin.clone())
            }
            Some(origin) => PreludeRangeName::Hidden {
                by: origin.module_identity().to_string(),
            },
            None => PreludeRangeName::Absent,
        }
    }

    /// Reports that the name `Range` in this module is not the prelude's, naming what took it.
    fn report_range_hidden(&mut self, hiding_module: &str, span: TextSpan) {
        let here = hiding_module == self.module.module_identity();
        let declaration_span = here
            .then(|| {
                self.module
                    .raw_module()
                    .items()
                    .iter()
                    // Whatever kind of declaration took the name, pointing at it is the help the
                    // author needs: a `let Range = 5` hides the prelude's record as surely as a
                    // record of the module's own does.
                    .find(|item| item.name().as_str() == nx_hir::PRELUDE_RANGE_NAME)
                    .map(Item::span)
            })
            .flatten();

        let message = if here {
            format!(
                "The range operator builds the built-in '{}' record, and this module declares its \
                 own '{}'",
                nx_hir::PRELUDE_RANGE_NAME,
                nx_hir::PRELUDE_RANGE_NAME
            )
        } else {
            format!(
                "The range operator builds the built-in '{}' record, and '{}' here is the one \
                 imported from '{hiding_module}'",
                nx_hir::PRELUDE_RANGE_NAME,
                nx_hir::PRELUDE_RANGE_NAME
            )
        };

        let mut builder = Diagnostic::error("range-hidden")
            .with_message(message)
            .with_label(Label::primary(self.file_name.clone(), span))
            .with_help(format!(
                "Construct the module's own '{}' with the element form, or rename it to use the \
                 range operator here",
                nx_hir::PRELUDE_RANGE_NAME
            ));
        if let Some(declaration_span) = declaration_span {
            builder = builder.with_label(
                Label::secondary(self.file_name.clone(), declaration_span)
                    .with_message(format!("'{}' is declared here", nx_hir::PRELUDE_RANGE_NAME)),
            );
        }
        self.diagnostics.push(builder.build());
    }

    /// The type argument of a `<Range T=X/>` that is the prelude's, if `ty` is one.
    ///
    /// <para>`ty` is matched as it stands, so an optional `<Range T=X/>?` is not one: a caller that
    /// accepts an optional range — a binding site, where `range={0..1}` at an optional field is
    /// ordinary — strips the nullability itself.</para>
    fn prelude_range_argument(&self, ty: &Type) -> Option<Type> {
        let Type::Named(named) = ty else {
            return None;
        };
        if named.name.as_str() != nx_hir::PRELUDE_RANGE_NAME {
            return None;
        }
        if named.origin().map(DeclaringOrigin::module_identity)
            != Some(nx_hir::PRELUDE_MODULE_IDENTITY)
        {
            return None;
        }
        named
            .args()
            .iter()
            .find(|(parameter, _)| parameter.as_str() == "T")
            .map(|(_, argument)| argument.clone())
    }

    fn infer_binop(
        &mut self,
        expr_id: ExprId,
        op: ast::BinOp,
        lhs: ExprId,
        rhs: ExprId,
        span: nx_diagnostics::TextSpan,
    ) -> Type {
        use ast::BinOp::*;

        let mut lhs_ty = self.infer_expr(lhs);
        let mut rhs_ty = self.infer_expr(rhs);

        // Skip error checking if either operand is error
        if lhs_ty.is_error() || rhs_ty.is_error() {
            return Type::Error;
        }

        if !matches!(op, And | Or) {
            match self.convert_literal_operand(op, lhs, &rhs_ty, span) {
                Some(Ok(ty)) => lhs_ty = ty,
                Some(Err(())) => return Type::Error,
                None => {}
            }
            match self.convert_literal_operand(op, rhs, &lhs_ty, span) {
                Some(Ok(ty)) => rhs_ty = ty,
                Some(Err(())) => return Type::Error,
                None => {}
            }
        }
        let (lhs_ty, rhs_ty) = (lhs_ty, rhs_ty);

        let numeric_pair = match (&lhs_ty, &rhs_ty) {
            (Type::Primitive(a), Type::Primitive(b)) if a.is_numeric() && b.is_numeric() => {
                Some((*a, *b))
            }
            _ => None,
        };

        match op {
            // Arithmetic: the narrowest type both operands widen to
            Add | Sub | Mul | Div | Mod => {
                if let Some((a, b)) = numeric_pair {
                    return match Primitive::numeric_promotion(a, b) {
                        Some(promoted) => Type::Primitive(promoted),
                        None => {
                            self.error(
                                "type-mismatch",
                                format!(
                                    "Cannot apply {} to {} and {}: neither converts to the other \
                                     without loss, so the conversion is not implicit",
                                    binop_spelling(op),
                                    lhs_ty,
                                    rhs_ty
                                ),
                                span,
                            );
                            Type::Error
                        }
                    };
                }
                if op == Add {
                    if let Some(ty) =
                        self.infer_concatenation(expr_id, lhs, &lhs_ty, rhs, &rhs_ty, span)
                    {
                        return ty;
                    }
                }
                let hint = [&lhs_ty, &rhs_ty]
                    .into_iter()
                    .find(|ty| !ty.occurrence().is_one())
                    .map(|ty| {
                        format!(
                            "; {} is not exactly one value: {}",
                            ty,
                            occurrence_operand_hint(ty)
                        )
                    })
                    .unwrap_or_default();
                self.error(
                    "type-mismatch",
                    format!(
                        "Binary operator {:?} cannot be applied to types {} and {}{}",
                        op, lhs_ty, rhs_ty, hint
                    ),
                    span,
                );
                Type::Error
            }

            // Comparison: T × T → bool (where T supports comparison). Two numeric operands compare
            // at the narrowest type both widen to. Two cases of one union are comparable for
            // equality with each other, since a value of that union is either of them; union
            // cases have no order, so the relational operators stay rejected.
            Eq | Ne | Lt | Le | Gt | Ge => {
                // Ordering is defined on exactly one value per side; a value that may be empty or
                // may be many has no order, and the runtime would fail on it.
                if matches!(op, Lt | Le | Gt | Ge)
                    && (!lhs_ty.occurrence().is_one() || !rhs_ty.occurrence().is_one())
                {
                    let hint = if lhs_ty.admits_many() || rhs_ty.admits_many() {
                        "compare the items inside a `for`"
                    } else {
                        "supply a value with `??` first"
                    };
                    self.error(
                        "type-mismatch",
                        format!(
                            "Cannot apply {} to {} and {}: it orders exactly one value on each \
                             side; {}",
                            binop_spelling(op),
                            lhs_ty,
                            rhs_ty,
                            hint
                        ),
                        span,
                    );
                    return Type::Error;
                }
                if let Some((a, b)) = numeric_pair {
                    if Primitive::numeric_promotion(a, b).is_some() {
                        return Type::boolean();
                    }
                    self.error(
                        "type-mismatch",
                        format!(
                            "Cannot compare {} and {}: neither converts to the other without \
                             loss, so the conversion is not implicit",
                            lhs_ty, rhs_ty
                        ),
                        span,
                    );
                    return Type::Error;
                }
                let sibling_cases = matches!(op, Eq | Ne)
                    && matches!(
                        (&lhs_ty, &rhs_ty),
                        (Type::UnionCase(lhs_case), Type::UnionCase(rhs_case))
                            if lhs_case.shares_union_with(rhs_case)
                    );
                if sibling_cases
                    || self.type_satisfies_expected(&lhs_ty, &rhs_ty)
                    || self.type_satisfies_expected(&rhs_ty, &lhs_ty)
                    || (matches!(op, Eq | Ne) && self.items_comparable(&lhs_ty, &rhs_ty))
                {
                    Type::boolean()
                } else {
                    self.error(
                        "type-mismatch",
                        format!("Cannot compare types {} and {}", lhs_ty, rhs_ty),
                        span,
                    );
                    Type::Error
                }
            }

            // Logical: boolean × boolean → boolean
            And | Or => {
                if lhs_ty == Type::boolean() && rhs_ty == Type::boolean() {
                    Type::boolean()
                } else {
                    self.error(
                        "type-mismatch",
                        format!(
                            "Logical operator {:?} requires boolean operands, found {} and {}",
                            op, lhs_ty, rhs_ty
                        ),
                        span,
                    );
                    Type::Error
                }
            }
        }
    }

    /// Types a literal operand by the other operand's numeric type, if it is one.
    ///
    /// <para>`None` when `operand` is not a numeric literal or `other` is not numeric, so the
    /// operand keeps the type it has. `Some(Ok(ty))` is the type the literal now has, and
    /// `Some(Err(()))` means it could not take the type — out of range, or not exact — and the
    /// diagnostic is already reported.</para>
    fn convert_literal_operand(
        &mut self,
        op: ast::BinOp,
        operand: ExprId,
        other: &Type,
        span: TextSpan,
    ) -> Option<Result<Type, ()>> {
        // Equality compares every value as a sequence, so a literal beside `float32?` or
        // `int32+` is typed by that side's item type; every other operator takes exactly one.
        let other = if matches!(op, ast::BinOp::Eq | ast::BinOp::Ne) {
            other.item()
        } else {
            other
        };
        if !matches!(other, Type::Primitive(primitive) if primitive.is_numeric()) {
            return None;
        }
        let context = format!("An operand of {}", binop_spelling(op));
        match self.convert_literals(operand, other, span, &context)? {
            true => Some(Ok(self
                .env
                .get_expr_type(operand)
                .cloned()
                .unwrap_or(Type::Error))),
            false => Some(Err(())),
        }
    }

    /// Types a `+` as string concatenation when either operand is a string.
    ///
    /// <para>`None` when neither operand is a string, so the caller reports the addition as it
    /// would any other mistyped one. With a string operand, the other must be a string or a
    /// stringifiable primitive — a number or a boolean — and is recorded for its text conversion;
    /// anything else is rejected naming its type. A value that admits zero or many is among the
    /// rejected: there is no one text a missing value, or several, obviously has.</para>
    fn infer_concatenation(
        &mut self,
        expr_id: ExprId,
        lhs: ExprId,
        lhs_ty: &Type,
        rhs: ExprId,
        rhs_ty: &Type,
        span: TextSpan,
    ) -> Option<Type> {
        let string = Type::string();
        if lhs_ty != &string && rhs_ty != &string {
            return None;
        }

        let mut accepted = true;
        for (operand, ty) in [(lhs, lhs_ty), (rhs, rhs_ty)] {
            if ty == &string {
                continue;
            }
            match Self::stringifiable_primitive(ty) {
                Some(primitive) => {
                    self.string_conversions
                        .text_conversions
                        .insert(operand, primitive);
                }
                None => {
                    let message = if !ty.occurrence().is_one() {
                        format!(
                            "Cannot join {} to a string with +: + takes exactly one value on each \
                             side; {}",
                            ty,
                            occurrence_operand_hint(ty)
                        )
                    } else {
                        format!(
                            "Cannot join {} to a string with +: only a string, a number or a \
                             boolean has a text form",
                            ty
                        )
                    };
                    self.error("type-mismatch", message, span);
                    accepted = false;
                }
            }
        }
        if !accepted {
            return Some(Type::Error);
        }
        self.string_conversions.concatenations.insert(expr_id);
        Some(string)
    }

    /// The HIR name of `ty` when it is a primitive with a canonical text form.
    fn stringifiable_primitive(ty: &Type) -> Option<ast::PrimitiveType> {
        match ty {
            Type::Primitive(primitive) if primitive.is_stringifiable() => primitive.hir_type(),
            _ => None,
        }
    }

    /// Infers the result type of a unary operation.
    fn infer_unop(
        &mut self,
        op: ast::UnOp,
        operand: &Type,
        span: nx_diagnostics::TextSpan,
    ) -> Type {
        if operand.is_error() {
            return Type::Error;
        }
        let hint = if operand.occurrence().is_one() {
            String::new()
        } else {
            format!(
                "; {} is not exactly one value: {}",
                operand,
                occurrence_operand_hint(operand)
            )
        };

        match op {
            ast::UnOp::Neg => {
                if let Type::Primitive(p) = operand {
                    if p.is_numeric() {
                        return operand.clone();
                    }
                }
                self.error(
                    "type-mismatch",
                    format!(
                        "Negation requires a numeric type, found {}{}",
                        operand, hint
                    ),
                    span,
                );
                Type::Error
            }
            ast::UnOp::Not => {
                if operand == &Type::boolean() {
                    Type::boolean()
                } else {
                    self.error(
                        "type-mismatch",
                        format!("Logical NOT requires boolean, found {}{}", operand, hint),
                        span,
                    );
                    Type::Error
                }
            }
        }
    }

    /// Infers the result type of a function call. `required` is how many leading arguments the
    /// call must supply when the callee is a declaration that lets trailing ones be omitted;
    /// otherwise every parameter is required.
    fn infer_call(
        &mut self,
        func_ty: &Type,
        args: &[ExprId],
        arg_tys: &[Type],
        required: Option<usize>,
        span: nx_diagnostics::TextSpan,
    ) -> Type {
        // A call that cannot be checked has already reported what is wrong with it. Adding a
        // second diagnostic about its arguments would point the author at something downstream of
        // the thing they actually have to fix.
        if func_ty.is_error() {
            return Type::Error;
        }

        match func_ty {
            Type::Function { params, ret } => {
                // Check argument count
                let required = required.unwrap_or(params.len()).min(params.len());
                if arg_tys.len() < required || arg_tys.len() > params.len() {
                    let expected = if required == params.len() {
                        params.len().to_string()
                    } else {
                        format!("{} to {}", required, params.len())
                    };
                    self.error(
                        "arg-count-mismatch",
                        format!(
                            "Function expects {} arguments, got {}",
                            expected,
                            arg_tys.len()
                        ),
                        span,
                    );
                    return Type::Error;
                }

                // Check argument types against each parameter's read type, so an optional
                // parameter accepts `{}` and a `?` value. The argument expression is passed so a
                // literal written there can take the parameter's type.
                for (i, (param, arg_ty)) in params.iter().zip(arg_tys.iter()).enumerate() {
                    self.check_typed_binding_for(
                        args.get(i).copied(),
                        arg_ty,
                        &param.read_type(),
                        span,
                        "type-mismatch",
                        format!("Argument {}", i),
                    );
                }

                (**ret).clone()
            }
            _ => {
                self.error(
                    "not-a-function",
                    format!("Cannot call non-function type {}", func_ty),
                    span,
                );
                Type::Error
            }
        }
    }

    fn infer_member_access(
        &mut self,
        base: ExprId,
        base_ty: &Type,
        member: &Name,
        span: TextSpan,
    ) -> Type {
        // A space splits `?.`, so `x? .m` reads a member of the presence test, a `boolean`.
        if let ast::Expr::Exists { operand, .. } = self.module.raw_module().expr(base) {
            let fix = match self.flattened_expr_name(*operand) {
                Some(receiver) => format!("{}?.{}", receiver, member),
                None => format!("?.{}", member),
            };
            self.error(
                "member-access-on-presence-test",
                format!(
                    "A presence test is a boolean, which has no member '{}'; to read a member \
                     of a value that may be empty, write `{}` with no space",
                    member, fix
                ),
                span,
            );
            return Type::Error;
        }
        // A receiver that may be empty is read with `?.`, and a sequence has no members at all:
        // the occurrence is decided here, and the field is read from the item type below.
        match base_ty.occurrence() {
            Occurrence::ONE => {}
            Occurrence::OPTIONAL => {
                // Show the whole access with `?.` when the receiver is a path the author can
                // copy, and the step alone otherwise.
                let fix = match self.flattened_expr_name(base) {
                    Some(receiver) => format!("{}?.{}", receiver, member),
                    None => format!("?.{}", member),
                };
                self.error_involving(
                    "member-access-on-optional",
                    format!(
                        "A value of type {} may be empty, so `.{}` cannot read it; write `{}`, \
                         which is empty when the receiver is empty",
                        base_ty, member, fix
                    ),
                    span,
                    Some(base_ty),
                );
                return Type::Error;
            }
            _ => return self.member_access_on_sequence(base_ty, member, span),
        }
        self.infer_item_member_access(base_ty, member, span)
    }

    /// Types `x?.m`: the member's read type, admitting zero because the receiver may.
    fn infer_optional_member_access(
        &mut self,
        base_ty: &Type,
        member: &Name,
        span: TextSpan,
    ) -> Type {
        match base_ty.occurrence() {
            Occurrence::OPTIONAL => {}
            Occurrence::ONE => {
                if !base_ty.is_error() {
                    self.warn(
                        "unnecessary-optional-step",
                        format!(
                            "A value of type {} is always present, so `?.{}` is unnecessary; \
                             write `.{}`",
                            base_ty, member, member
                        ),
                        span,
                    );
                }
                return self.infer_item_member_access(base_ty, member, span);
            }
            _ => return self.member_access_on_sequence(base_ty, member, span),
        }
        let member_ty = self.infer_item_member_access(base_ty.item(), member, span);
        if member_ty.is_error() {
            return member_ty;
        }
        member_ty.with_occurrence(member_ty.occurrence().join(Occurrence::OPTIONAL))
    }

    /// Rejects `.m` and `?.m` on a `+` or `*` receiver, which has no members.
    fn member_access_on_sequence(&mut self, base_ty: &Type, member: &Name, span: TextSpan) -> Type {
        if base_ty.is_error() {
            return Type::Error;
        }
        self.error_involving(
            "member-access-on-sequence",
            format!(
                "A value of type {} is a sequence, which has no members; read `.{}` from each \
                 item with `for`",
                base_ty, member
            ),
            span,
            Some(base_ty),
        );
        Type::Error
    }

    /// The read type of `member` on an exactly-one receiver: a record's field at its read type
    /// (`T?` for a field declared `p?:T`), a union's shared field, or a case's field.
    fn infer_item_member_access(&mut self, base_ty: &Type, member: &Name, span: TextSpan) -> Type {
        match base_ty {
            Type::Union(union_ty) => {
                if let Some(ty) =
                    self.union_shared_field_type(&union_ty.name, union_ty.origin(), member)
                {
                    return ty;
                }

                if self.union_has_case_field(&union_ty.name, union_ty.origin(), member) {
                    self.error(
                        "union-case-field-requires-narrowing",
                        format!(
                            "Field '{}' is case-specific on union '{}' and requires narrowing",
                            member, union_ty.name
                        ),
                        span,
                    );
                    Type::Error
                } else {
                    self.error(
                        "unknown-union-field",
                        format!("Union '{}' has no shared field '{}'", union_ty.name, member),
                        span,
                    );
                    Type::Error
                }
            }
            Type::UnionCase(case_ty) => self
                .union_case_field_type(&case_ty.union, case_ty.origin(), &case_ty.case, member)
                .unwrap_or_else(|| {
                    self.error(
                        "unknown-union-case-field",
                        format!(
                            "Union case '{}.{}' has no field '{}'",
                            case_ty.union, case_ty.case, member
                        ),
                        span,
                    );
                    Type::Error
                }),
            // A record's field, reached through the effective shape so an inherited field is
            // found as readily as a declared one. Each field's type is resolved in the module that
            // declared *that field*, which is not always the module the record itself came from.
            Type::Named(named) => {
                let Ok(Some(shape)) = self.record_shape_of(named) else {
                    return self.unknown_member(member, span, base_ty);
                };
                let Some(field) = shape.fields.iter().find(|field| field.name == *member) else {
                    let known = shape
                        .fields
                        .iter()
                        .map(|field| field.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    self.error(
                        "unknown-record-field",
                        format!(
                            "Record '{}' has no field '{}'; it has: {}",
                            named.name, member, known
                        ),
                        span,
                    );
                    return Type::Error;
                };
                let declaring_module = field.module_identity.clone();
                let field_ty = field.ty.clone();
                // Every field of an update record may be absent, and an absent field reads as
                // the empty value, so each reads as an optional field does whatever the target
                // declared. Absent and cleared read alike; `changed` tells them apart.
                let is_update = self
                    .record_definition_for(named)
                    .is_some_and(|record| matches!(record.kind, nx_hir::RecordKind::Update { .. }));
                let optional = field.optional || is_update;
                let declared = if named.args().is_empty() {
                    self.type_from_type_ref_in_quietly(Some(&declaring_module), &field_ty)
                } else {
                    // Reading a field of an instantiation gives the field's declared type with
                    // each parameter replaced by its argument, which is what binding the
                    // parameters into the scope the annotation resolves in does in one step.
                    let scope: FxHashMap<Name, Type> = named.args().iter().cloned().collect();
                    let previous_scope = std::mem::replace(&mut self.type_parameter_scope, scope);
                    let ty = self.type_from_type_ref_in_quietly(Some(&declaring_module), &field_ty);
                    self.type_parameter_scope = previous_scope;
                    ty
                };
                // A field declared `p?:T` reads as `T?`, and `p?:T+` as `T*`; so does any field
                // of an update record.
                read_type(&declared, optional)
            }
            Type::Error => Type::Error,
            _ => self.unknown_member(member, span, base_ty),
        }
    }

    /// Reports a member access on a value whose type has no such member.
    fn unknown_member(&mut self, member: &Name, span: TextSpan, base_ty: &Type) -> Type {
        self.error_involving(
            "unknown-member",
            format!("A value of type {} has no member '{}'", base_ty, member),
            span,
            Some(base_ty),
        );
        Type::Error
    }

    /// Returns the union declaration a resolved union type denotes.
    ///
    /// <para>A resolved `Type::Union` or `Type::UnionCase` already names one declaration, so the
    /// entry is selected by that declaration. Looking it up by spelling alone would let a
    /// same-named union visible here answer for a foreign one — the capture that carrying an
    /// origin exists to prevent. A foreign entry is addressed by its origin outright. The local
    /// map is keyed by the name this module reaches a union under, so there the name is tried
    /// first, because it is the common case and reaches the same entry, and a scan covers a
    /// collision.</para>
    fn union_entry_for(
        &self,
        union_name: &Name,
        union_origin: Option<&DeclaringOrigin>,
    ) -> Option<&UnionEntry> {
        let denotes = |entry: &&UnionEntry| {
            nx_hir::same_declaration(
                entry.origin.as_ref(),
                &entry.def.name,
                union_origin,
                union_name,
            )
        };

        self.union_defs
            .get(union_name)
            .filter(denotes)
            .or_else(|| {
                union_origin
                    .and_then(|origin| self.foreign_union_defs.get(origin))
                    .filter(denotes)
            })
            .or_else(|| self.union_defs.values().find(denotes))
    }

    /// The type of a field every case of a union shares, through the union's abstract base.
    ///
    /// <para>The base is a name the union's own module wrote, so it is resolved there, and each
    /// inherited field's type is resolved in the module that declared that field. Resolving either
    /// here would let an unrelated local record supply the fields of a foreign union.</para>
    fn union_shared_field_type(
        &mut self,
        union_name: &Name,
        union_origin: Option<&DeclaringOrigin>,
        member: &Name,
    ) -> Option<Type> {
        let entry = self.union_entry_for(union_name, union_origin)?;
        let base_name = entry.def.base.clone()?;
        let base_origin = entry
            .origin
            .as_ref()
            .and_then(|origin| self.record_origin_in(origin.module_identity(), &base_name));

        let shape = match base_origin.as_ref() {
            Some(origin) => nx_hir::effective_record_shape_at(self.module, origin),
            None => effective_record_shape_for_name(self.module, &base_name),
        }
        .ok()
        .flatten()?;

        let field = shape.fields.iter().find(|field| field.name == *member)?;
        let field_module = field.module_identity.clone();
        let field_ty = field.ty.clone();
        let optional = field.optional;
        let declared = self.type_from_type_ref_in_quietly(Some(&field_module), &field_ty);
        Some(read_type(&declared, optional))
    }

    fn union_has_case_field(
        &self,
        union_name: &Name,
        union_origin: Option<&DeclaringOrigin>,
        member: &Name,
    ) -> bool {
        self.union_entry_for(union_name, union_origin)
            .map(|entry| {
                entry
                    .def
                    .cases
                    .iter()
                    .any(|case| case.fields.iter().any(|field| field.name == *member))
            })
            .unwrap_or(false)
    }

    /// The type of a field on one case of a union, shared fields included.
    ///
    /// <para>A case's own field types are written in the union's module, so they are resolved
    /// there rather than here.</para>
    fn union_case_field_type(
        &mut self,
        union_name: &Name,
        union_origin: Option<&DeclaringOrigin>,
        case_name: &Name,
        member: &Name,
    ) -> Option<Type> {
        if let Some(ty) = self.union_shared_field_type(union_name, union_origin, member) {
            return Some(ty);
        }

        let entry = self.union_entry_for(union_name, union_origin)?;
        let declaring_module = entry
            .origin
            .as_ref()
            .map(|origin| origin.module_identity().to_string());
        let union_def = entry.def.clone();
        let case = union_def
            .cases
            .iter()
            .find(|case| case.name == *case_name)?;
        let field = case.fields.iter().find(|field| field.name == *member)?;
        let field_ty = field.ty.clone();
        let optional = field.optional;
        let declared = self.type_from_type_ref_in_quietly(declaring_module.as_deref(), &field_ty);
        Some(read_type(&declared, optional))
    }

    fn infer_record_literal(
        &mut self,
        record: &Name,
        properties: &[ast::RecordLiteralProperty],
        span: TextSpan,
    ) -> Type {
        if let Some(record_def) = self.resolve_record_definition(record) {
            if record_def.is_abstract {
                self.error(
                    "abstract-record-instantiation",
                    format!("Cannot instantiate abstract record '{}'", record_def.name),
                    span,
                );
            }

            let effective_shape = self.effective_record_shape(record).ok().flatten();
            let type_params: Vec<Name> = record_def
                .type_params
                .iter()
                .map(|param| param.name.clone())
                .collect();
            let owner = self.record_origins.get(record).cloned();
            let arguments = (!type_params.is_empty()).then(|| {
                let bindings = properties
                    .iter()
                    .filter(|property| type_params.contains(&property.name))
                    .map(|property| (property.name.clone(), property.value, property.span))
                    .collect();
                self.resolve_record_type_arguments(
                    bindings,
                    record,
                    &type_params,
                    owner.clone(),
                    span,
                )
            });
            let unspecified = arguments
                .as_ref()
                .map(|arguments| arguments.unspecified.clone())
                .unwrap_or_default();
            let is_unspecified = |param: &TypeParameterRef| {
                param.owner == owner && unspecified.contains(&param.name)
            };

            for property in properties {
                // A type-argument binding is not a field; it was resolved above and is removed
                // from the construction once analysis is done.
                if type_params.contains(&property.name) {
                    continue;
                }
                match self.record_field_type_ref(
                    &record_def,
                    effective_shape.as_ref(),
                    &property.name,
                ) {
                    Some((field_ty, optional)) => {
                        // A written value is checked against the field's read type, so an
                        // optional field accepts `{}` and a required one does not.
                        let expected = self.under_type_arguments(arguments.as_ref(), |this| {
                            read_type(
                                &this.type_from_type_ref_in_quietly(None, &field_ty),
                                optional,
                            )
                        });
                        let actual = self.infer_expr(property.value);
                        // A field typed by a parameter the construction left unbound is skipped:
                        // the missing argument is the one real problem and is already reported.
                        if expected.find_parameter(&is_unspecified).is_some() {
                            continue;
                        }
                        self.check_typed_binding_for(
                            Some(property.value),
                            &actual,
                            &expected,
                            property.span,
                            "record-field-type-mismatch",
                            format!("Record field '{}' on '{}'", property.name, record),
                        );
                    }
                    None => {
                        self.error(
                            "unknown-record-field",
                            format!("Record '{}' has no field '{}'", record, property.name),
                            property.span,
                        );
                    }
                }
            }

            // A field that is not written binds its default, or the empty value when it is
            // optional; otherwise it is missing. A field whose declared type admits zero was
            // rejected at the declaration and reads as marked, as an element's property does.
            let required: Vec<Name> = match effective_shape.as_ref() {
                Some(shape) => shape
                    .fields
                    .iter()
                    .filter(|field| {
                        field.is_required
                            && !self
                                .type_from_type_ref_in_quietly(
                                    Some(&field.module_identity),
                                    &field.ty,
                                )
                                .admits_zero()
                    })
                    .map(|field| field.name.clone())
                    .collect(),
                None => record_def
                    .properties
                    .iter()
                    .filter(|field| {
                        field.default.is_none()
                            && !field.optional
                            && !self
                                .type_from_type_ref_in_quietly(None, &field.ty)
                                .admits_zero()
                    })
                    .map(|field| field.name.clone())
                    .collect(),
            };
            for name in required {
                if !properties.iter().any(|property| property.name == name) {
                    self.error(
                        "missing-property",
                        format!("Element '{}' requires property '{}'", record, name),
                        span,
                    );
                }
            }

            let resolved = arguments.map(|arguments| {
                self.consumed_type_arguments.extend(arguments.consumed);
                arguments.resolved
            });
            match (self.nominal_named_type(record), resolved) {
                (Type::Named(named), Some(resolved)) => Type::Named(named.with_args(resolved)),
                (ty, _) => ty,
            }
        } else {
            self.nominal_named_type(record)
        }
    }

    /// A record field's declared type reference and whether it carries the `?` mark.
    fn record_field_type_ref(
        &self,
        record_def: &nx_hir::RecordDef,
        effective_shape: Option<&nx_hir::EffectiveRecordShape>,
        name: &Name,
    ) -> Option<(ast::TypeRef, bool)> {
        if let Some(shape) = effective_shape {
            return shape
                .fields
                .iter()
                .find(|field| field.name == *name)
                .map(|field| (field.ty.clone(), field.optional));
        }

        record_def
            .properties
            .iter()
            .find(|field| field.name == *name)
            .map(|field| (field.ty.clone(), field.optional))
    }

    fn infer_element_expression(
        &mut self,
        element_id: ElementId,
        element: &nx_hir::Element,
        span: TextSpan,
    ) -> Type {
        // A tag that names a function-typed value — a prop, a parameter, a `let` — is a call of
        // that value: every parameter of the type is required, since the value's own declaration
        // may need any of them, and an argument the type lacks has nowhere to go.
        if let Some((params, ret)) = self.function_typed_value(&element.tag) {
            let content_property = params
                .iter()
                .find(|param| param.is_content)
                .map(|param| param.name.clone());
            let spec = ElementBindingSpec {
                content_property: content_property.clone(),
                properties: params
                    .iter()
                    .map(|param| {
                        (
                            param.name.clone(),
                            ElementPropertySpec::new(param.read_type(), !param.optional),
                        )
                    })
                    .collect(),
                handler_properties: FxHashSet::default(),
                type_parameters: FxHashSet::default(),
            };
            self.check_element_bindings(element_id, element, span, &spec);
            self.function_value_calls
                .insert(element_id, content_property);
            return ret;
        }

        if let Some(function) = self.resolve_function_definition(&element.tag) {
            let declaring_module = function.module_identity().to_string();
            match function {
                ResolvedPreparedItem::Raw {
                    item: Item::Function(function),
                    ..
                } => {
                    self.check_element_bindings_against_function(
                        element_id,
                        element,
                        &function,
                        span,
                        Some(declaring_module.as_str()),
                    );
                    if let Some(Type::Function { ret, .. }) = self.env.lookup(&element.tag) {
                        return (**ret).clone();
                    }
                    return function
                        .return_type
                        .as_ref()
                        .map(|ty| self.type_from_type_ref_in_quietly(None, ty))
                        .unwrap_or_else(|| self.nominal_named_type(&element.tag));
                }
                ResolvedPreparedItem::Imported { item, .. } => {
                    if let Some((_name, _visibility, _form, params, return_type, _span)) =
                        interface_function_signature(&item)
                    {
                        let declaring_module = item.module_identity.clone();
                        let spec = self.build_element_binding_spec_in(
                            Some(declaring_module.as_str()),
                            params.iter().map(|param| {
                                (
                                    &param.name,
                                    &param.ty,
                                    param.is_content,
                                    !param.is_omissible(),
                                    param.optional,
                                )
                            }),
                        );
                        self.check_element_bindings(element_id, element, span, &spec);
                        return self.type_from_type_ref_in_quietly(None, &return_type);
                    }
                }
                _ => {}
            }
        }

        if let Some(component) = self.resolve_component_definition(&element.tag) {
            let declaring_module = component.module_identity().to_string();
            match component {
                ResolvedPreparedItem::Raw {
                    item: Item::Component(component),
                    ..
                } => {
                    if component.is_abstract {
                        self.error(
                            "abstract-component-instantiation",
                            format!("Cannot instantiate abstract component '{}'", component.name),
                            span,
                        );
                    }
                    self.check_element_bindings_against_component(
                        element_id,
                        element,
                        &component,
                        span,
                        Some(declaring_module.as_str()),
                    );
                    return self.nominal_named_type(&element.tag);
                }
                ResolvedPreparedItem::Imported { item, .. } => {
                    if let Some(component) = interface_component(&item) {
                        if component.is_abstract {
                            self.error(
                                "abstract-component-instantiation",
                                format!(
                                    "Cannot instantiate abstract component '{}'",
                                    component.name
                                ),
                                span,
                            );
                        }
                        self.check_element_bindings_against_component(
                            element_id,
                            element,
                            &component,
                            span,
                            Some(declaring_module.as_str()),
                        );
                        return self.nominal_named_type(&element.tag);
                    }
                }
                _ => {}
            }
        }

        if let Some((declaring_module, record_def)) =
            self.resolve_record_definition_with_origin(&element.tag)
        {
            if record_def.is_abstract {
                self.error(
                    "abstract-record-instantiation",
                    format!("Cannot instantiate abstract record '{}'", record_def.name),
                    span,
                );
            }
            return self.check_element_bindings_against_record(
                element_id,
                element,
                &record_def,
                span,
                Some(declaring_module.as_str()),
            );
        }

        if let Some((entry, case)) = self.union_case_from_qualified_name(&element.tag) {
            let entry = entry.clone();
            let case = case.clone();
            let declaring_module = entry
                .origin
                .as_ref()
                .map(|origin| origin.module_identity().to_string());
            self.check_element_bindings_against_union_case(
                element_id,
                element,
                &entry.def,
                &case,
                span,
                declaring_module.as_deref(),
            );
            return entry.case_type(case.name);
        }

        self.report_unresolved_update_tag(&element.tag, span);

        // A tag that resolves to nothing has no binding contract, so there is nothing to check the
        // element's properties and content against. Every expression written inside it is still an
        // expression, though, and the four resolved paths above infer theirs as a side effect of
        // checking bindings. Infer these on their own terms so that the absent tag is the only
        // thing left unchecked, and so that the recorded types reach callers reading the type
        // environment.
        let property_paths = self.property_paths_for_entries(element.property_entries());
        // Supplying one property twice is a defect in the element, not in its contract, so it is
        // reported here as it is on every resolved path: the absent tag is the only thing left
        // unchecked.
        self.report_duplicate_property_paths(&property_paths, &element.tag);
        for content in &element.content {
            self.infer_expr(*content);
        }

        self.nominal_named_type(&element.tag)
    }

    /// Reports an unresolved tag spelled like an update record, which is never a host element.
    ///
    /// <para>Inside a component, lowering already rewrote a bare `Update` to the component's own
    /// record, so a bare one that reaches here was written outside any component. `X.Update` that
    /// did not resolve names a declaration with no update record: a component without state, an
    /// update record itself, since update records have none of their own, or nothing at all. A
    /// bare `Update` inside a component without state arrives here as that component's qualified
    /// name, so this is the one diagnostic it gets.</para>
    fn report_unresolved_update_tag(&mut self, tag: &Name, span: TextSpan) {
        let suffix = nx_hir::UPDATE_RECORD_SUFFIX;
        if tag.as_str() == suffix {
            self.error(
                "bare-update-outside-component",
                "A bare 'Update' names the enclosing component's update record, and there is no enclosing component here; write the qualified form for the record to patch, as in '<Type.Update ... />'"
                    .to_string(),
                span,
            );
            return;
        }
        let Some(target) = tag
            .as_str()
            .strip_suffix(suffix)
            .and_then(|prefix| prefix.strip_suffix('.'))
        else {
            return;
        };
        let target = Name::new(target);
        let message = if self
            .resolve_record_definition(&target)
            .is_some_and(|record| record.update_target().is_some())
        {
            format!(
                "Unknown type '{}': '{}' is an update record, and update records have no update record of their own",
                tag, target
            )
        } else if self.is_property_union(&target) {
            format!(
                "Unknown type '{}': '{}' is a property union, and property unions have no update record",
                tag, target
            )
        } else if self.resolve_component_definition(&target).is_some() {
            format!(
                "Unknown type '{}': component '{}' declares no state, so it has no update record",
                tag, target
            )
        } else {
            format!(
                "Unknown type '{}': '{}' is not a record, action, or component with state",
                tag, target
            )
        };
        self.error("unknown-update-record", message, span);
    }

    /// Returns true when `name` reaches a derived property union here.
    fn is_property_union(&self, name: &Name) -> bool {
        self.union_defs
            .get(name)
            .is_some_and(|entry| entry.def.property_target().is_some())
    }

    /// Reports a member access whose base is spelled like a property union that resolves to
    /// nothing, and returns whether it did.
    ///
    /// <para>Inside a component, lowering already rewrote a bare `Property` to the component's own
    /// union, so a bare one that reaches here was written outside any component — unless something
    /// declared here is actually named `Property`, in which case the access means that. `X.Property`
    /// that did not resolve names a declaration with no property union: a component without state,
    /// a derived declaration, since derived declarations have none of their own, or nothing at
    /// all. Reporting here, at the first member access on the missing union, is what keeps the
    /// access from cascading into a diagnostic about every segment after it.</para>
    fn report_unresolved_property_reference(&mut self, name: &Name, span: TextSpan) -> bool {
        let Some((base, _member)) = name.as_str().rsplit_once('.') else {
            return false;
        };
        let suffix = nx_hir::PROPERTY_UNION_SUFFIX;
        if base == suffix {
            let base_name = Name::new(base);
            if self.env.lookup(&base_name).is_some()
                || self.union_defs.contains_key(&base_name)
                || self.type_aliases.contains_key(&base_name)
                || self.resolve_record_definition(&base_name).is_some()
                || self.resolve_component_definition(&base_name).is_some()
            {
                return false;
            }
            self.error(
                "bare-property-outside-component",
                "A bare 'Property' names the enclosing component's property union, and there is no enclosing component here; write the qualified form for the declaration whose fields to name, as in 'Type.Property.field'"
                    .to_string(),
                span,
            );
            return true;
        }
        if !nx_hir::is_property_union_name(base) || self.union_defs.contains_key(&Name::new(base)) {
            return false;
        }
        let target = Name::new(
            base.strip_suffix(suffix)
                .and_then(|prefix| prefix.strip_suffix('.'))
                .unwrap_or(base),
        );
        let message = if self
            .resolve_record_definition(&target)
            .is_some_and(|record| record.update_target().is_some())
        {
            format!(
                "Unknown type '{}': '{}' is an update record, and update records have no property union",
                base, target
            )
        } else if self.is_property_union(&target) {
            format!(
                "Unknown type '{}': '{}' is a property union, and property unions have no property union of their own",
                base, target
            )
        } else if self.resolve_component_definition(&target).is_some() {
            format!(
                "Unknown type '{}': component '{}' declares no state, so it has no property union",
                base, target
            )
        } else {
            format!(
                "Unknown type '{}': '{}' is not a record, action, or component with state",
                base, target
            )
        };
        self.error("unknown-property-union", message, span);
        true
    }

    /// The intrinsic a call's callee names, when the callee is a bare identifier naming one.
    fn intrinsic_callee(&self, func: ExprId) -> Option<UpdateIntrinsic> {
        match self.module.raw_module().expr(func) {
            ast::Expr::Ident(name) => UpdateIntrinsic::from_name(name.as_str()),
            _ => None,
        }
    }

    /// Types a call to one of the update intrinsics by rule.
    ///
    /// <para>These are the typing rules the four prelude signatures would state if the language
    /// had generics — `apply<T>(record:T, update:T.Update): T` and its siblings — and nothing
    /// else, so that the day they become declared signatures this is a deletion. "Same `T`" is
    /// decided by declaring origin: `Named.Update` is not `User.Update` even though `User` extends
    /// `Named`, and a same-named record in another module is not this one.</para>
    fn infer_intrinsic_call(
        &mut self,
        intrinsic: UpdateIntrinsic,
        args: &[ExprId],
        span: TextSpan,
    ) -> Type {
        let arg_tys: Vec<_> = args.iter().map(|arg| self.infer_expr(*arg)).collect();
        if arg_tys.len() != intrinsic.arity() {
            self.error(
                "intrinsic-arg-count",
                format!(
                    "Intrinsic '{}' expects {} argument{}, got {}",
                    intrinsic.name(),
                    intrinsic.arity(),
                    if intrinsic.arity() == 1 { "" } else { "s" },
                    arg_tys.len()
                ),
                span,
            );
            return Type::Error;
        }
        if arg_tys.iter().any(Type::is_error) {
            return Type::Error;
        }

        match intrinsic {
            UpdateIntrinsic::Apply => {
                let record = self.intrinsic_record_argument(intrinsic, 0, &arg_tys[0], span);
                let update = self.intrinsic_update_argument(intrinsic, 1, &arg_tys[1], span);
                let (Some(record), Some((update, target))) = (record, update) else {
                    return Type::Error;
                };
                if !self.update_patches_record(&update, &target, &record)
                    || update.args() != record.args()
                {
                    let expected = Type::Named(
                        NamedType::new(nx_hir::update_record_name(record.name.as_str()), None)
                            .with_args(record.args().to_vec()),
                    );
                    self.error(
                        "intrinsic-target-mismatch",
                        format!(
                            "Intrinsic 'apply' takes a record and its own update record: '{}' is not '{}'",
                            Type::Named(update.clone()),
                            expected
                        ),
                        span,
                    );
                    return Type::Error;
                }
                Type::Named(record)
            }
            UpdateIntrinsic::Merge => {
                let first = self.intrinsic_update_argument(intrinsic, 0, &arg_tys[0], span);
                let second = self.intrinsic_update_argument(intrinsic, 1, &arg_tys[1], span);
                let (Some((first, _)), Some((second, _))) = (first, second) else {
                    return Type::Error;
                };
                if first != second {
                    self.error(
                        "intrinsic-target-mismatch",
                        format!(
                            "Intrinsic 'merge' takes two updates of one record: '{}' and '{}' target different records",
                            Type::Named(first.clone()),
                            Type::Named(second.clone())
                        ),
                        span,
                    );
                    return Type::Error;
                }
                Type::Named(first)
            }
            UpdateIntrinsic::Diff => {
                let before = self.intrinsic_record_argument(intrinsic, 0, &arg_tys[0], span);
                let after = self.intrinsic_record_argument(intrinsic, 1, &arg_tys[1], span);
                let (Some(before), Some(after)) = (before, after) else {
                    return Type::Error;
                };
                if before != after {
                    self.error(
                        "intrinsic-target-mismatch",
                        format!(
                            "Intrinsic 'diff' takes two records of one type: '{}' is not '{}'",
                            Type::Named(after.clone()),
                            Type::Named(before.clone())
                        ),
                        span,
                    );
                    return Type::Error;
                }
                if self
                    .record_definition_for(&before)
                    .is_some_and(|record| record.is_abstract)
                {
                    self.error(
                        "intrinsic-argument-type",
                        format!(
                            "Intrinsic 'diff' takes concrete records, and '{}' is abstract",
                            before.name
                        ),
                        span,
                    );
                    return Type::Error;
                }
                let update = self.derived_type_of(&before, nx_hir::update_record_name);
                Self::carry_type_arguments(&before, update)
            }
            UpdateIntrinsic::Changed => {
                let Some((update, target)) =
                    self.intrinsic_update_argument(intrinsic, 0, &arg_tys[0], span)
                else {
                    return Type::Error;
                };
                let property_union = self.property_union_of_update(&update, &target);
                if property_union.is_error() {
                    self.error(
                        "unknown-property-union",
                        format!(
                            "Intrinsic 'changed' yields '{}' cases, but that property union could not be reached from here",
                            nx_hir::property_union_name(target.as_str())
                        ),
                        span,
                    );
                    return Type::Error;
                }
                Type::zero_or_more(property_union)
            }
        }
    }

    /// The record-shaped, non-derived type an intrinsic argument must have, or a diagnostic.
    fn intrinsic_record_argument(
        &mut self,
        intrinsic: UpdateIntrinsic,
        index: usize,
        ty: &Type,
        span: TextSpan,
    ) -> Option<NamedType> {
        if let Type::Named(named) = ty {
            if let Some(record) = self.record_definition_for(named) {
                if record.update_target().is_none() {
                    return Some(named.clone());
                }
            }
        }
        self.error(
            "intrinsic-argument-type",
            format!(
                "Argument {} to intrinsic '{}' must be a record or action; found '{}'",
                index + 1,
                intrinsic.name(),
                ty
            ),
            span,
        );
        None
    }

    /// The update-record type an intrinsic argument must have, with the name of the declaration
    /// it patches, or a diagnostic.
    fn intrinsic_update_argument(
        &mut self,
        intrinsic: UpdateIntrinsic,
        index: usize,
        ty: &Type,
        span: TextSpan,
    ) -> Option<(NamedType, Name)> {
        if let Type::Named(named) = ty {
            if let Some(target) = self
                .record_definition_for(named)
                .and_then(|record| record.update_target().cloned())
            {
                return Some((named.clone(), target));
            }
        }
        self.error(
            "intrinsic-argument-type",
            format!(
                "Argument {} to intrinsic '{}' must be an update record; found '{}'",
                index + 1,
                intrinsic.name(),
                ty
            ),
            span,
        );
        None
    }

    /// Returns true when `update` is the update record of exactly the declaration `record` is.
    ///
    /// <para>The update record's target is a name its own module wrote, so it is resolved there
    /// and compared to the record by declaration rather than by spelling.</para>
    fn update_patches_record(&self, update: &NamedType, target: &Name, record: &NamedType) -> bool {
        let Some(update_origin) = update.origin() else {
            return update.name.as_str()
                == nx_hir::update_record_name(record.name.as_str()).as_str();
        };
        let target_origin = self
            .module
            .resolve_in_module(
                PreparedNamespace::Type,
                update_origin.module_identity(),
                target,
            )
            .map(|resolved| resolved.declaring_origin());
        nx_hir::same_declaration(
            target_origin.as_ref(),
            target,
            record.origin(),
            &record.name,
        )
    }

    /// The type of a declaration derived from `record` — its update record or property union —
    /// reached through the module that declared the record, so it resolves whether or not this
    /// module can spell it.
    fn derived_type_of(&mut self, record: &NamedType, derive: fn(&str) -> Name) -> Type {
        let derived = derive(record.name.as_str());
        if let Some(origin) = record.origin() {
            let module_identity = origin.module_identity().to_string();
            if let Some(ty) = self.nominal_type_in_module(&module_identity, &derived) {
                return ty;
            }
        }
        let mut seen = FxHashSet::default();
        self.resolve_named_type(&derived, &mut seen)
    }

    /// Gives `derived` the type arguments `record` was instantiated with.
    ///
    /// <para>A generic record's update companion declares the same parameters, so `<Range T=int/>`
    /// diffs to `<Range.Update T=int/>` by carrying the arguments across rather than resolving
    /// them again. A non-generic record carries none and the type is unchanged.</para>
    fn carry_type_arguments(record: &NamedType, derived: Type) -> Type {
        match derived {
            Type::Named(named) if record.args().is_empty() => Type::Named(named),
            Type::Named(named) => Type::Named(named.with_args(record.args().to_vec())),
            other => other,
        }
    }

    /// The property union of the declaration `update` patches, resolved in the update record's
    /// own module.
    fn property_union_of_update(&mut self, update: &NamedType, target: &Name) -> Type {
        let property_union = nx_hir::property_union_name(target.as_str());
        if let Some(origin) = update.origin() {
            let module_identity = origin.module_identity().to_string();
            if let Some(ty) = self.nominal_type_in_module(&module_identity, &property_union) {
                return ty;
            }
        }
        match self.union_defs.get(&property_union) {
            Some(entry) => Type::Union(entry.shape()),
            None => Type::Error,
        }
    }

    fn check_element_bindings_against_function(
        &mut self,
        element_id: ElementId,
        element: &nx_hir::Element,
        function: &nx_hir::Function,
        span: TextSpan,
        declaring_module: Option<&str>,
    ) {
        let spec = self.build_element_binding_spec_in(
            declaring_module,
            function.params.iter().map(|param| {
                (
                    &param.name,
                    &param.ty,
                    param.is_content,
                    !param.is_omissible(),
                    param.optional,
                )
            }),
        );
        self.check_element_bindings(element_id, element, span, &spec);
    }

    /// Checks a use site of a component: its type arguments first, then its value bindings
    /// against the contract those arguments instantiate.
    ///
    /// <para>A type argument is a plain `Name=Type` binding under one of the target's type
    /// parameters. Each is resolved as a type name here, and the contract's prop types are then
    /// resolved with every parameter denoting its argument — or, for a parameter the site left
    /// unbound, the bottom type, so that a site binding nothing typed by the parameter needs no
    /// argument. The arguments are recorded for removal from the element afterwards; below the
    /// checker, nothing sees them.</para>
    ///
    /// <para>Substituting before any binding is checked is deliberate: it is the shape inference
    /// slots into later, by filling the gaps in the argument map from the bound values, with an
    /// explicit argument always winning.</para>
    fn check_element_bindings_against_component(
        &mut self,
        element_id: ElementId,
        element: &nx_hir::Element,
        component: &nx_hir::Component,
        span: TextSpan,
        declaring_module: Option<&str>,
    ) {
        let effective_contract = self
            .effective_component_contract(&component.name)
            .ok()
            .flatten();
        let type_param_names: Vec<Name> = match effective_contract.as_ref() {
            Some(contract) => contract
                .type_params
                .iter()
                .map(|param| param.name.clone())
                .collect(),
            None => component
                .type_params
                .iter()
                .map(|param| param.name.clone())
                .collect(),
        };
        let owner = self.component_origins.get(&component.name).cloned();
        let arguments = (!type_param_names.is_empty()).then(|| {
            self.resolve_type_arguments(element, &component.name, &type_param_names, owner.clone())
        });
        let mut spec = self.under_type_arguments(arguments.as_ref(), |this| {
            if let Some(contract) = effective_contract.as_ref() {
                this.build_element_binding_spec_in(
                    declaring_module,
                    contract.props.iter().map(|field| {
                        (
                            &field.name,
                            &field.ty,
                            field.is_content,
                            field.is_required,
                            field.optional,
                        )
                    }),
                )
            } else {
                this.build_element_binding_spec_in(
                    declaring_module,
                    component.props.iter().map(|field| {
                        (
                            &field.name,
                            &field.ty,
                            field.is_content,
                            field.default.is_none() && !field.optional,
                            field.optional,
                        )
                    }),
                )
            }
        });
        if let Some(arguments) = arguments {
            // A prop typed by a parameter the site left unbound is checked against a type no
            // value can be assumed to have, and remembers the parameter so a failure can be
            // reported by its name. Where the prop *produces* a value of the parameter — a
            // sequence of items — that is the bottom type, so only the empty value binds. Where
            // the prop *consumes* one — a template's `Item` parameter, which the host will call
            // with whatever the items are — it is the top type, so a template that assumes any
            // particular item type fails and the diagnostic asks for the argument.
            let unspecified = arguments.unspecified;
            let is_unspecified = |param: &TypeParameterRef| {
                param.owner == owner && unspecified.contains(&param.name)
            };
            for entry in spec.properties.values_mut() {
                let Some(param) = entry.ty.find_parameter(&is_unspecified).cloned() else {
                    continue;
                };
                entry.unspecified_parameter = Some(param.name);
                entry.ty = entry
                    .ty
                    .substitute_parameters_by_variance(true, &|param, covariant| {
                        is_unspecified(param).then(|| {
                            if covariant {
                                Type::never()
                            } else {
                                Type::named("object")
                            }
                        })
                    });
            }
            spec.type_parameters = type_param_names.iter().cloned().collect();
            self.consumed_type_arguments.extend(arguments.consumed);
            self.resolved_type_arguments
                .insert(element_id, arguments.resolved);
        }
        let emit_names: Vec<&Name> = match effective_contract.as_ref() {
            Some(contract) => contract.emits.iter().map(|emit| &emit.emit.name).collect(),
            None => component.emits.iter().map(|emit| &emit.name).collect(),
        };
        spec.handler_properties.extend(
            emit_names
                .into_iter()
                .map(|name| Name::new(&handler_prop_name(name.as_str()))),
        );
        self.check_element_bindings(element_id, element, span, &spec);
    }

    /// Runs `build` with each of the target's type parameters standing for the argument the use
    /// site bound it to, then restores the scope.
    ///
    /// <para>This is the whole of what a generic use site shares: every field type of the target's
    /// contract is resolved once, with the arguments already substituted, which is what makes a
    /// binding check against the instantiated type rather than against the parameter.</para>
    fn under_type_arguments<R>(
        &mut self,
        arguments: Option<&ResolvedTypeArguments>,
        build: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let previous_scope = arguments.map(|arguments| {
            std::mem::replace(&mut self.type_parameter_scope, arguments.scope.clone())
        });
        let result = build(self);
        if let Some(previous_scope) = previous_scope {
            self.type_parameter_scope = previous_scope;
        }
        result
    }

    /// Resolves the type arguments a record construction bound, reporting each parameter it left
    /// unbound once, by name and in the form to write.
    ///
    /// <para>A record has no bottom-type fallback the way a component does: the constructed
    /// value's own type is what the argument decides, so an unbound parameter is an error rather
    /// than an erasure.</para>
    fn resolve_record_type_arguments(
        &mut self,
        bindings: Vec<(Name, ExprId, TextSpan)>,
        record_name: &Name,
        type_params: &[Name],
        owner: Option<DeclaringOrigin>,
        span: TextSpan,
    ) -> ResolvedTypeArguments {
        let arguments =
            self.resolve_type_argument_bindings(bindings, record_name, type_params, owner);
        for param in type_params {
            if arguments.unspecified.contains(param) {
                self.error(
                    "type-parameter-not-specified",
                    format!(
                        "Type parameter '{}' of record '{}' was not specified; write {}=<type>",
                        param, record_name, param
                    ),
                    span,
                );
            }
        }
        arguments
    }

    /// Blanks out every field whose type mentions a parameter the site left unbound, and records
    /// the arguments for removal from the construction.
    ///
    /// <para>The field is neither required nor checked: the author's one real problem is the
    /// missing argument, already reported, and checking `start` against a type nothing satisfies
    /// would bury it.</para>
    fn apply_record_type_arguments(
        &mut self,
        spec: &mut ElementBindingSpec,
        arguments: ResolvedTypeArguments,
        type_params: &[Name],
        owner: Option<&DeclaringOrigin>,
    ) -> Vec<(Name, Type)> {
        let unspecified = arguments.unspecified;
        let is_unspecified = |param: &TypeParameterRef| {
            param.owner.as_ref() == owner && unspecified.contains(&param.name)
        };
        for entry in spec.properties.values_mut() {
            if entry.ty.find_parameter(&is_unspecified).is_some() {
                entry.ty = Type::Error;
                entry.is_required = false;
            }
        }
        spec.type_parameters = type_params.iter().cloned().collect();
        self.consumed_type_arguments.extend(arguments.consumed);
        arguments.resolved
    }

    /// Resolves the type arguments a use site binds for `type_params`, the target's effective
    /// type parameters, and reports every binding that is not a plain bare type name.
    fn resolve_type_arguments(
        &mut self,
        element: &nx_hir::Element,
        component_name: &Name,
        type_params: &[Name],
        owner: Option<DeclaringOrigin>,
    ) -> ResolvedTypeArguments {
        let mut bindings = Vec::new();
        for entry in element.property_entries() {
            match entry {
                PropertyEntry::Value(property) if type_params.contains(&property.key) => {
                    bindings.push((property.key.clone(), property.value, property.span));
                }
                PropertyEntry::Value(_) => {}
                entry => self.report_conditional_type_arguments(entry, component_name, type_params),
            }
        }
        self.resolve_type_argument_bindings(bindings, component_name, type_params, owner)
    }

    /// Resolves the type arguments a use site bound, whatever shape the site is.
    ///
    /// <para>A component element and a record element write their arguments as property entries; a
    /// content-free same-module record construction lowers to a record literal and writes them as
    /// literal properties. Both arrive here as `(parameter, value expression, span)`, so the rules
    /// — bare name only, first binding wins, unbound parameters recorded — are one set.</para>
    fn resolve_type_argument_bindings(
        &mut self,
        bindings: Vec<(Name, ExprId, TextSpan)>,
        target_name: &Name,
        type_params: &[Name],
        owner: Option<DeclaringOrigin>,
    ) -> ResolvedTypeArguments {
        let component_name = target_name;
        let mut bound: FxHashMap<Name, Type> = FxHashMap::default();
        let mut consumed = Vec::new();

        for (key, value, span) in bindings {
            consumed.push(value);
            // An argument that was written but rejected still binds its parameter, as the error
            // type: the rejection is the one problem, and the parameter is not also unspecified.
            let ty = self
                .resolve_type_argument(value, &key, component_name, span)
                .unwrap_or(Type::Error);
            // A second binding of one parameter is reported as a duplicate property on the
            // same terms as any other; the first is the one that counts.
            bound.entry(key).or_insert(ty);
        }

        let mut scope = FxHashMap::default();
        let mut unspecified = FxHashSet::default();
        let mut resolved = Vec::new();
        for (ordinal, name) in type_params.iter().enumerate() {
            match bound.remove(name) {
                Some(ty) => {
                    resolved.push((name.clone(), ty.clone()));
                    scope.insert(name.clone(), ty);
                }
                None => {
                    unspecified.insert(name.clone());
                    scope.insert(
                        name.clone(),
                        Type::parameter(name.clone(), owner.clone(), ordinal),
                    );
                }
            }
        }

        ResolvedTypeArguments {
            scope,
            unspecified,
            resolved,
            consumed,
        }
    }

    /// Resolves one type argument: a bare name, resolved against the types visible here and never
    /// against value bindings. Anything else is reported with the bare form to write instead.
    fn resolve_type_argument(
        &mut self,
        value: ExprId,
        param: &Name,
        component_name: &Name,
        span: TextSpan,
    ) -> Option<Type> {
        let expr = self.module.raw_module().expr(value).clone();
        let (name, occurrence) = match &expr {
            ast::Expr::ContextualName {
                name, occurrence, ..
            } => (name.clone(), *occurrence),
            other => {
                let suggested = match other {
                    ast::Expr::Literal(ast::Literal::String(text)) => text.to_string(),
                    _ => self
                        .flattened_expr_name(value)
                        .map(|name| name.as_str().to_string())
                        .unwrap_or_else(|| "<type>".to_string()),
                };
                self.error(
                    "type-argument-not-a-type-name",
                    format!(
                        "Type parameter '{}' on '{}' expects a bare type name; write {}={}",
                        param, component_name, param, suggested
                    ),
                    span,
                );
                return None;
            }
        };

        if self.is_visible_type_name(&name) {
            // The argument is a name this use site wrote, not a declaration's, so a problem with
            // it — a generic record named without its own arguments, say — is reported here, at
            // the element that wrote it.
            let ty = self.type_from_type_ref_at(span, &ast::TypeRef::Name(name.clone()));
            // A suffix written on the argument (`T=int?`) is reported on the same terms as one an
            // alias carries (`T=Maybe`), naming the argument as written.
            if let Some(occurrence) = occurrence {
                let written = Name::new(&format!("{name}{occurrence}"));
                let suffixed = ty.with_occurrence(occurrence);
                self.reject_occurrence_type_argument(&suffixed, Some(&written), span);
                return None;
            }
            if self.reject_occurrence_type_argument(&ty, Some(&name), span) {
                return None;
            }
            return Some(ty);
        }

        let candidates = self.visible_type_names();
        let suggestion = Self::closest_candidate(&name, &candidates)
            .map(|candidate| format!("; did you mean `{}`?", candidate))
            .unwrap_or_default();
        self.error(
            "unresolved-type-argument",
            format!(
                "Type parameter '{}' on '{}' expects a type name, and '{}' is not a visible type{}",
                param, component_name, name, suggestion
            ),
            span,
        );
        None
    }

    /// Reports every type-argument binding inside a conditional property fragment.
    fn report_conditional_type_arguments(
        &mut self,
        entry: &PropertyEntry,
        component_name: &Name,
        type_params: &[Name],
    ) {
        let nested: Vec<&PropertyEntry> = match entry {
            PropertyEntry::Value(_) => Vec::new(),
            PropertyEntry::If {
                then_entries,
                else_entries,
                ..
            } => then_entries.iter().chain(else_entries).collect(),
            PropertyEntry::ConditionList {
                arms, else_entries, ..
            } => arms
                .iter()
                .flat_map(|arm| arm.entries.iter())
                .chain(else_entries)
                .collect(),
            PropertyEntry::Match {
                arms, else_entries, ..
            } => arms
                .iter()
                .flat_map(|arm| arm.entries.iter())
                .chain(else_entries)
                .collect(),
        };
        for nested in nested {
            match nested {
                PropertyEntry::Value(property) if type_params.contains(&property.key) => {
                    self.error(
                        "conditional-type-argument",
                        format!(
                            "Type parameter '{}' on '{}' cannot be bound conditionally; write {}=<type> as a plain property",
                            property.key, component_name, property.key
                        ),
                        property.span,
                    );
                }
                PropertyEntry::Value(_) => {}
                nested => {
                    self.report_conditional_type_arguments(nested, component_name, type_params)
                }
            }
        }
    }

    /// True when `name` denotes a type here: a primitive, an alias, a union, a record, a component,
    /// or a type parameter in scope. Value bindings are never consulted.
    fn is_visible_type_name(&self, name: &Name) -> bool {
        self.type_parameter_scope.contains_key(name)
            || PRIMITIVE_TYPE_NAMES.contains(&name.as_str())
            || self.type_aliases.contains_key(name)
            || self.union_defs.contains_key(name)
            || self.record_origins.contains_key(name)
            || self.component_origins.contains_key(name)
            || nx_syntax::BUILTIN_TYPE_NAMES.contains(&name.as_str())
    }

    /// Every name `is_visible_type_name` answers for, for a did-you-mean.
    fn visible_type_names(&self) -> Vec<Name> {
        let mut names: Vec<Name> = PRIMITIVE_TYPE_NAMES.into_iter().map(Name::new).collect();
        names.extend(self.type_parameter_scope.keys().cloned());
        names.extend(self.type_aliases.keys().cloned());
        names.extend(self.union_defs.keys().cloned());
        names.extend(self.record_origins.keys().cloned());
        names.extend(self.component_origins.keys().cloned());
        names.extend(nx_syntax::BUILTIN_TYPE_NAMES.into_iter().map(Name::new));
        names.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        names.dedup();
        names
    }

    fn check_element_bindings_against_record(
        &mut self,
        element_id: ElementId,
        element: &nx_hir::Element,
        record_def: &nx_hir::RecordDef,
        span: TextSpan,
        declaring_module: Option<&str>,
    ) -> Type {
        let effective_shape = self.effective_record_shape(&record_def.name).ok().flatten();
        let type_params: Vec<Name> = record_def
            .type_params
            .iter()
            .map(|param| param.name.clone())
            .collect();
        let owner = self.record_origins.get(&element.tag).cloned();
        let arguments = (!type_params.is_empty()).then(|| {
            let bindings = element
                .property_entries()
                .iter()
                .filter_map(|entry| match entry {
                    PropertyEntry::Value(property) if type_params.contains(&property.key) => {
                        Some((property.key.clone(), property.value, property.span))
                    }
                    _ => None,
                })
                .collect();
            for entry in element.property_entries() {
                if !matches!(entry, PropertyEntry::Value(_)) {
                    self.report_conditional_type_arguments(entry, &element.tag, &type_params);
                }
            }
            self.resolve_record_type_arguments(
                bindings,
                &element.tag,
                &type_params,
                owner.clone(),
                span,
            )
        });
        let mut spec = self.under_type_arguments(arguments.as_ref(), |this| {
            if let Some(shape) = effective_shape.as_ref() {
                this.build_element_binding_spec_in(
                    declaring_module,
                    shape.fields.iter().map(|field| {
                        (
                            &field.name,
                            &field.ty,
                            field.is_content,
                            field.is_required,
                            field.optional,
                        )
                    }),
                )
            } else {
                this.build_element_binding_spec_in(
                    declaring_module,
                    record_def.properties.iter().map(|field| {
                        (
                            &field.name,
                            &field.ty,
                            field.is_content,
                            field.default.is_none() && !field.optional,
                            field.optional,
                        )
                    }),
                )
            }
        });
        let resolved = arguments.map(|arguments| {
            self.apply_record_type_arguments(&mut spec, arguments, &type_params, owner.as_ref())
        });
        self.check_record_element_bindings(element_id, element, span, &spec);
        match (self.nominal_named_type(&element.tag), resolved) {
            (Type::Named(named), Some(resolved)) => Type::Named(named.with_args(resolved)),
            (ty, _) => ty,
        }
    }

    fn check_record_element_bindings(
        &mut self,
        element_id: ElementId,
        element: &nx_hir::Element,
        span: TextSpan,
        spec: &ElementBindingSpec,
    ) {
        let property_paths = self.property_paths_for_entries(element.property_entries());
        self.report_duplicate_property_paths(&property_paths, &element.tag);

        let content_from_body = if !element.content.is_empty() {
            if let Some(content_name) = spec.content_property.as_ref() {
                if property_paths.iter().any(|path| {
                    path.properties
                        .iter()
                        .any(|property| property.key == *content_name)
                }) {
                    self.error(
                        "content-binding-conflict",
                        format!(
                            "Record '{}' passes content for '{}' both as a property and as body content",
                            element.tag, content_name
                        ),
                        span,
                    );
                    false
                } else if let Some(expected) = spec.properties.get(content_name) {
                    self.check_content_binding(
                        element_id,
                        &element.content,
                        &expected.ty,
                        span,
                        format!("Content for '{}' binds to '{}'", element.tag, content_name),
                    );
                    true
                } else {
                    false
                }
            } else {
                self.error(
                    "missing-content-property",
                    format!(
                        "Record '{}' passes body content, but '{}' does not declare a content field",
                        element.tag, element.tag
                    ),
                    span,
                );
                false
            }
        } else {
            false
        };

        self.check_property_path_bindings(
            &property_paths,
            spec,
            &element.tag,
            content_from_body,
            span,
            "record-field-type-mismatch",
            "unknown-record-field",
            "missing-property",
        );
    }

    /// Checks `<Union.case ... />` against the case's declared fields.
    ///
    /// <para>`declaring_module` is the module that wrote the union. Its base and its field types
    /// are names that module wrote, so both are resolved there — an imported union's `n:Size` means
    /// its module's `Size`, not a same-named record visible here.</para>
    fn check_element_bindings_against_union_case(
        &mut self,
        element_id: ElementId,
        element: &nx_hir::Element,
        union_def: &UnionDef,
        case: &UnionCaseDef,
        span: TextSpan,
        declaring_module: Option<&str>,
    ) {
        let mut content_property: Option<Name> = None;
        let mut properties = FxHashMap::<Name, ElementPropertySpec>::default();

        if let Some(base_name) = union_def.base.as_ref() {
            let base_origin = declaring_module
                .and_then(|module_identity| self.record_origin_in(module_identity, base_name));
            let shape = match base_origin.as_ref() {
                Some(origin) => nx_hir::effective_record_shape_at(self.module, origin),
                None => effective_record_shape_for_name(self.module, base_name),
            };
            if let Ok(Some(shape)) = shape {
                for field in shape.fields {
                    let ty =
                        self.type_from_type_ref_in_quietly(Some(&field.module_identity), &field.ty);
                    if field.is_content {
                        content_property = Some(field.name.clone());
                    }
                    properties.insert(
                        field.name,
                        ElementPropertySpec::new(read_type(&ty, field.optional), field.is_required),
                    );
                }
            }
        }

        for field in &case.fields {
            let ty = self.type_from_type_ref_in_quietly(declaring_module, &field.ty);
            let is_required = field.default.is_none() && !field.optional;
            if field.is_content {
                content_property = Some(field.name.clone());
            }
            properties.insert(
                field.name.clone(),
                ElementPropertySpec::new(read_type(&ty, field.optional), is_required),
            );
        }

        let spec = ElementBindingSpec {
            content_property,
            properties,
            handler_properties: FxHashSet::default(),
            type_parameters: FxHashSet::default(),
        };
        let property_paths = self.property_paths_for_entries(element.property_entries());
        self.report_duplicate_property_paths(&property_paths, &element.tag);

        let content_from_body = if !element.content.is_empty() {
            if let Some(content_name) = spec.content_property.as_ref() {
                if property_paths
                    .iter()
                    .any(|path| path.properties.iter().any(|prop| prop.key == *content_name))
                {
                    self.error(
                        "content-binding-conflict",
                        format!(
                            "Union case '{}.{}' passes content for '{}' both as a property and as body content",
                            union_def.name, case.name, content_name
                        ),
                        span,
                    );
                    false
                } else if let Some(expected) = spec.properties.get(content_name) {
                    self.check_content_binding(
                        element_id,
                        &element.content,
                        &expected.ty,
                        span,
                        format!(
                            "Content for '{}.{}' binds to '{}'",
                            union_def.name, case.name, content_name
                        ),
                    );
                    true
                } else {
                    false
                }
            } else {
                self.error(
                    "missing-content-property",
                    format!(
                        "Union case '{}.{}' receives body content but does not declare a content field",
                        union_def.name, case.name
                    ),
                    span,
                );
                false
            }
        } else {
            false
        };

        self.check_property_path_bindings(
            &property_paths,
            &spec,
            &element.tag,
            content_from_body,
            span,
            "union-case-field-type-mismatch",
            "unknown-union-case-field",
            "missing-union-case-field",
        );
    }

    /// Resolves a nominal name written by a declaration in `module_identity`.
    ///
    /// A declaration's type references are written in its own namespace: `fit: Fit` in a library
    /// means that library's `Fit`. Resolving it in the consumer's scope is what leaves the type as
    /// an unresolved `Type::Named`, and what lets an unrelated local `Fit` stand in for it.
    fn nominal_type_in_module(&mut self, module_identity: &str, name: &Name) -> Option<Type> {
        if module_identity == self.module.module_identity() {
            return None;
        }

        // The declaring module's own namespace, which includes what it imported. A type it named
        // but did not declare is reached here and nowhere else. Records and unions live in the
        // type namespace, components in the element namespace, and a property may be typed by any
        // of the three.
        let peer_entry = self
            .module
            .peer_entry(PreparedNamespace::Type, module_identity, name)
            .or_else(|| {
                self.module
                    .peer_entry(PreparedNamespace::Element, module_identity, name)
            });
        if let Some(entry) = peer_entry {
            let origin = entry.clone();
            if let Some(item) = self
                .module
                .peer_module(origin.module_identity())
                .and_then(|module| module.item_by_definition(origin.definition_id()))
                .cloned()
            {
                if let Some(ty) = self.foreign_nominal_type(name, &item, origin) {
                    return Some(ty);
                }
            }
        }

        // A workspace peer keeps its whole lowered module, so its own definitions are readable
        // directly, whether or not the consumer imported them. This is what a module analyzed
        // outside a graph has, where no peer namespace was registered.
        if let Some((definition_id, item)) = self
            .module
            .peer_module(module_identity)
            .and_then(|peer| peer.find_item_with_definition(name.as_str()))
            .map(|(definition_id, item)| (definition_id, item.clone()))
        {
            let origin = DeclaringOrigin::new(module_identity, definition_id);
            if let Some(ty) = self.foreign_nominal_type(name, &item, origin) {
                return Some(ty);
            }
        }

        None
    }

    /// Builds the nominal type one foreign declaration denotes, under the name the reference used.
    ///
    /// The name is the contract's spelling and need not be the name the declaration was given —
    /// the declaring module may have imported it under an alias — so it is display information
    /// only. The origin decides what type this is.
    fn foreign_nominal_type(
        &mut self,
        name: &Name,
        item: &Item,
        origin: DeclaringOrigin,
    ) -> Option<Type> {
        match item {
            Item::Union(union_def) => {
                let mut union_def = union_def.clone();
                union_def.name = name.clone();
                let entry = UnionEntry {
                    def: union_def,
                    origin: Some(origin.clone()),
                };
                let ty = Type::Union(entry.shape());
                self.foreign_union_defs.insert(origin, entry);
                Some(ty)
            }
            Item::Record(_) | Item::Component(_) => {
                Some(Type::named_at(name.clone(), Some(origin)))
            }
            // An alias is a type reference the declaring module wrote, so what it names is
            // resolved there too. Stopping at the alias would hand the reference back to the
            // consumer's namespace, where an unrelated same-named declaration answers for it.
            Item::TypeAlias(alias) => {
                if !self.foreign_alias_stack.insert(origin.clone()) {
                    return Some(Type::Error);
                }
                let target = alias.ty.clone();
                let ty =
                    self.type_from_type_ref_in_quietly(Some(origin.module_identity()), &target);
                self.foreign_alias_stack.remove(&origin);
                Some(ty)
            }
            _ => None,
        }
    }

    /// Converts a type reference written by a declaration owned by `declaring_module`.
    ///
    /// The declaring module is tried first, so an unrelated local type that merely shares the
    /// spelling cannot stand in for the one the declaration actually named.
    fn type_from_type_ref_in(
        &mut self,
        declaring_module: Option<&str>,
        type_ref: &ast::TypeRef,
    ) -> Type {
        let mut seen = FxHashSet::default();
        self.type_from_type_ref_walk(declaring_module, type_ref, &mut seen, true)
    }

    /// Reports an occurrence suffix applied to something that already carries one, once per name.
    ///
    /// <para>A second suffix written directly on a spelled one -- `string??`, `(string+)*` -- is
    /// post-parse validation's to reject, at the suffix itself. Saying it again from here would
    /// say the same thing twice, in vaguer words and at a whole declaration's span, so only what
    /// the parser cannot see is reported: a name that resolves to a suffixed type.</para>
    fn report_second_occurrence(&mut self, inner: &ast::TypeRef) {
        if matches!(inner, ast::TypeRef::Seq { .. }) {
            return;
        }
        let span = self.type_ref_span;
        let message = match inner {
            ast::TypeRef::Name(name) => format!(
                "Type already carries an occurrence; '{name}' says how many values it admits, \
                 so no suffix can be applied to it"
            ),
            _ => "Type already carries an occurrence".to_string(),
        };
        self.error("second-occurrence-suffix", message, span);
    }

    /// Converts a type reference one layer at a time, so that an applied type is reached wherever
    /// one can be written and resolved where the declarations it names are known.
    ///
    /// <para>`scoped` is whether a bare name may denote a type parameter currently in scope. A
    /// declaration's own annotation may; the target of a module-level type alias may not, since
    /// the alias was written outside the declaration whose parameters those are.</para>
    fn type_from_type_ref_walk(
        &mut self,
        declaring_module: Option<&str>,
        type_ref: &ast::TypeRef,
        seen: &mut FxHashSet<Name>,
        scoped: bool,
    ) -> Type {
        match type_ref {
            ast::TypeRef::Name(name) => {
                if scoped {
                    if let Some(ty) = self.type_parameter_scope.get(name) {
                        return ty.clone();
                    }
                }
                if let Some(ty) = crate::semantics::builtin_type(name) {
                    return ty;
                }
                if let Some(module_identity) = declaring_module {
                    let module_identity = module_identity.to_string();
                    if let Some(ty) = self.nominal_type_in_module(&module_identity, name) {
                        return self.require_type_arguments(name, ty);
                    }
                }
                let ty = self.resolve_named_type(name, seen);
                self.require_type_arguments(name, ty)
            }
            ast::TypeRef::Applied { name, args } => {
                self.applied_type(declaring_module, name, args, seen, scoped)
            }
            ast::TypeRef::Seq { inner, occ } => {
                let item = self.type_from_type_ref_walk(declaring_module, inner, seen, scoped);
                // A suffix on nothing usable is nothing usable, not a sequence. Wrapping the
                // error would make an enclosing suffix believe the inner reference named a
                // suffixed type, so `type A = A+` would report its cycle and then claim `A`
                // already carries an occurrence -- on top of a name that is not a type at all.
                if item.is_error() {
                    return Type::Error;
                }
                if matches!(item, Type::Seq { .. }) {
                    self.report_second_occurrence(inner);
                    return Type::Error;
                }
                Type::seq(item, *occ)
            }
            ast::TypeRef::Function {
                params,
                return_type,
            } => {
                let params = params
                    .iter()
                    .map(|param| {
                        let ty =
                            self.type_from_type_ref_walk(declaring_module, &param.ty, seen, scoped);
                        // A function type's parameter is a property definition, so it takes the
                        // `?` mark and refuses `?` and `*` in its type slot on the same terms.
                        let ty = self.check_property_slot(&param.name, &param.ty, ty);
                        FunctionParam {
                            name: param.name.clone(),
                            ty,
                            is_content: param.is_content,
                            optional: param.optional,
                        }
                    })
                    .collect();
                let ret = self.type_from_type_ref_walk(declaring_module, return_type, seen, scoped);
                Type::function(params, ret)
            }
        }
    }

    /// The type parameters of the record `ty` names, empty when it names no generic record.
    fn record_type_params_of(&self, ty: &Type) -> Vec<Name> {
        let Type::Named(named) = ty else {
            return Vec::new();
        };
        let record = match named.origin() {
            Some(origin) => nx_hir::resolve_record_definition_at(self.module, origin),
            None => nx_hir::resolve_record_definition(self.module, &named.name),
        };
        record
            .map(|record| {
                record
                    .type_params
                    .iter()
                    .map(|param| param.name.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Rejects a generic record's name written with no arguments: it is not a type on its own.
    ///
    /// <para>This is what keeps an argument-less instantiation from existing at all. `NamedType`
    /// equality includes the arguments, so such a type would be unequal to every real one and
    /// every later mismatch would be reported far from the annotation that caused it.</para>
    fn require_type_arguments(&mut self, name: &Name, ty: Type) -> Type {
        // An alias whose target is an applied type arrives already instantiated.
        if matches!(&ty, Type::Named(named) if !named.args().is_empty()) {
            return ty;
        }
        let params = self.record_type_params_of(&ty);
        if params.is_empty() {
            return ty;
        }
        let span = self.type_ref_span;
        self.error(
            "type-parameter-not-specified",
            format!(
                "Type parameter '{}' of record '{}' was not specified; write {}",
                params[0],
                name,
                nx_hir::ast::spell_applied_type(
                    name.as_str(),
                    params
                        .iter()
                        .map(|param| (param.as_str(), "...".to_string()))
                        .collect::<Vec<_>>(),
                )
            ),
            span,
        );
        Type::Error
    }

    /// The occurrence suffix glued to `expr`, when it is a contextual name written with one.
    fn contextual_name_occurrence(&self, expr: ExprId) -> Option<Occurrence> {
        match self.module.raw_module().expr(expr) {
            ast::Expr::ContextualName { occurrence, .. } => *occurrence,
            _ => None,
        }
    }

    /// Rejects a type argument that carries an occurrence, naming the alias when one was written.
    ///
    /// <para>A type parameter stands where an item type stands: a record declaring `items:T+`
    /// instantiated with `T=int?` would have a field whose item admits zero, which no NX type
    /// is. Checking the resolved argument covers `T=int?`, `T=Maybe` and a parameter forwarded
    /// from an enclosing declaration alike, since all three arrive here as a `Type`. Optionality
    /// belongs to the slot: `value?:T`.</para>
    fn reject_occurrence_type_argument(
        &mut self,
        ty: &Type,
        written: Option<&Name>,
        span: TextSpan,
    ) -> bool {
        if !matches!(ty, Type::Seq { .. }) {
            return false;
        }
        self.error(
            "occurrence-type-argument",
            match written {
                Some(name) => format!(
                    "A type argument must be exactly one value; '{name}' carries an occurrence"
                ),
                None => "A type argument must be exactly one value".to_string(),
            },
            span,
        );
        true
    }

    /// Resolves `<Range T=int/>`: the tag to a generic record, each argument to a type, and the
    /// two into one instantiation whose arguments stand in the record's declaration order.
    ///
    /// <para>Every way of getting it wrong is reported by record and argument name, and every one
    /// of them yields `Type::Error`, so one mistake is one diagnostic rather than a cascade from a
    /// half-built type.</para>
    fn applied_type(
        &mut self,
        declaring_module: Option<&str>,
        name: &Name,
        args: &[(Name, ast::TypeRef)],
        seen: &mut FxHashSet<Name>,
        scoped: bool,
    ) -> Type {
        let span = self.type_ref_span;
        // The tag names a record, never a type parameter or a primitive, so it skips both.
        let base = match declaring_module
            .map(|module_identity| module_identity.to_string())
            .and_then(|module_identity| self.nominal_type_in_module(&module_identity, name))
        {
            Some(ty) => ty,
            None => self.resolve_named_type(name, seen),
        };
        let params = self.record_type_params_of(&base);
        if params.is_empty() {
            self.error(
                "applied-type-not-generic",
                match &base {
                    Type::Named(named) if named.origin().is_some() => format!(
                        "Record '{}' has no type parameters, so it cannot be applied",
                        name
                    ),
                    _ => format!(
                        "'{}' is not a generic record, so it cannot be applied",
                        name
                    ),
                },
                span,
            );
            return Type::Error;
        }

        let mut bound: FxHashMap<Name, Type> = FxHashMap::default();
        let mut failed = false;
        for (param, arg) in args {
            if !params.contains(param) {
                self.error(
                    "unknown-type-argument",
                    format!(
                        "'{}' is not a type parameter of record '{}'; it declares {}",
                        param,
                        name,
                        params
                            .iter()
                            .map(|param| param.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    span,
                );
                failed = true;
                continue;
            }
            if bound.contains_key(param) {
                self.error(
                    "duplicate-type-argument",
                    format!(
                        "Type parameter '{}' of record '{}' is already bound",
                        param, name
                    ),
                    span,
                );
                failed = true;
                continue;
            }
            let ty = self.type_from_type_ref_walk(declaring_module, arg, seen, scoped);
            if ty.is_error() {
                failed = true;
            }
            let written = match arg {
                ast::TypeRef::Name(arg_name) => Some(arg_name.clone()),
                _ => None,
            };
            if self.reject_occurrence_type_argument(&ty, written.as_ref(), span) {
                failed = true;
            }
            // A bare name that reached no declaration is a misspelling, not a type. Nothing else
            // reports it: an unresolved `Type::Named` is otherwise carried along unremarked.
            if let ast::TypeRef::Name(arg_name) = arg {
                if matches!(&ty, Type::Named(named) if named.origin().is_none())
                    && !self.is_visible_type_name(arg_name)
                {
                    let candidates = self.visible_type_names();
                    let suggestion = Self::closest_candidate(arg_name, &candidates)
                        .map(|candidate| format!("; did you mean `{}`?", candidate))
                        .unwrap_or_default();
                    self.error(
                        "unresolved-type-argument",
                        format!(
                            "Type parameter '{}' of record '{}' expects a type, and '{}' is not a visible type{}",
                            param, name, arg_name, suggestion
                        ),
                        span,
                    );
                    failed = true;
                }
            }
            bound.insert(param.clone(), ty);
        }
        // A rejected argument is most often the missing one misspelled or misplaced, so the
        // parameters it leaves unbound are not reported on top of it.
        if failed {
            return Type::Error;
        }

        let mut resolved = Vec::with_capacity(params.len());
        for param in &params {
            match bound.remove(param) {
                Some(ty) => resolved.push((param.clone(), ty)),
                None => {
                    self.error(
                        "type-parameter-not-specified",
                        format!(
                            "Type parameter '{}' of record '{}' was not specified; write {}",
                            param,
                            name,
                            nx_hir::ast::spell_applied_type(
                                name.as_str(),
                                params
                                    .iter()
                                    .map(|param| (param.as_str(), "...".to_string()))
                                    .collect::<Vec<_>>()
                            )
                        ),
                        span,
                    );
                    failed = true;
                }
            }
        }

        if failed {
            return Type::Error;
        }
        match base {
            Type::Named(named) => Type::Named(named.with_args(resolved)),
            other => other,
        }
    }

    /// Builds the binding contract of a use site from `(name, declared type, is content, is
    /// required, is optional)` per property. A written value is checked against the property's
    /// read type, so an optional property accepts `{}` and a required or defaulted one does not.
    fn build_element_binding_spec_in<'b, I>(
        &mut self,
        declaring_module: Option<&str>,
        bindings: I,
    ) -> ElementBindingSpec
    where
        I: IntoIterator<Item = (&'b Name, &'b ast::TypeRef, bool, bool, bool)>,
    {
        let mut content_property = None;
        let mut properties = FxHashMap::default();

        for (name, ty_ref, is_content, is_required, optional) in bindings {
            let ty = self.type_from_type_ref_in_quietly(declaring_module, ty_ref);
            // A declared type that admits zero was rejected at the declaration, whose fix-it is to
            // mark the name. It is read as marked, so a use that leaves it out is not a second
            // report of the same mistake. A type argument is exactly one, so nothing else admits
            // zero here.
            let is_required = is_required && !ty.admits_zero();
            if is_content {
                content_property = Some(name.clone());
            }
            properties.insert(
                name.clone(),
                ElementPropertySpec::new(read_type(&ty, optional), is_required),
            );
        }

        ElementBindingSpec {
            content_property,
            properties,
            handler_properties: FxHashSet::default(),
            type_parameters: FxHashSet::default(),
        }
    }

    fn check_element_bindings(
        &mut self,
        element_id: ElementId,
        element: &nx_hir::Element,
        span: TextSpan,
        spec: &ElementBindingSpec,
    ) {
        let property_paths = self.property_paths_for_entries(element.property_entries());
        self.report_duplicate_property_paths(&property_paths, &element.tag);

        let content_from_body = if !element.content.is_empty() {
            if let Some(content_name) = spec.content_property.as_ref() {
                if property_paths.iter().any(|path| {
                    path.properties
                        .iter()
                        .any(|property| property.key == *content_name)
                }) {
                    self.error(
                        "content-binding-conflict",
                        format!(
                            "Element '{}' passes content for '{}' both as a property and as body content",
                            element.tag, content_name
                        ),
                        span,
                    );
                    false
                } else if let Some(expected) = spec.properties.get(content_name) {
                    self.check_content_binding(
                        element_id,
                        &element.content,
                        &expected.ty,
                        span,
                        format!("Content for '{}' binds to '{}'", element.tag, content_name),
                    );
                    true
                } else {
                    false
                }
            } else {
                self.error(
                    "missing-content-property",
                    format!(
                        "Element '{}' passes body content, but '{}' does not declare a content property",
                        element.tag, element.tag
                    ),
                    span,
                );
                false
            }
        } else {
            false
        };

        self.check_property_path_bindings(
            &property_paths,
            spec,
            &element.tag,
            content_from_body,
            span,
            "property-type-mismatch",
            "unknown-property",
            "missing-property",
        );
    }

    fn property_paths_for_entries(&mut self, entries: &[PropertyEntry]) -> Vec<PropertyPath> {
        let mut paths = vec![PropertyPath {
            properties: Vec::new(),
        }];

        for entry in entries {
            let alternatives = self.property_paths_for_entry(entry);
            let mut next_paths = Vec::new();
            for path in &paths {
                for alternative in &alternatives {
                    let mut properties = path.properties.clone();
                    properties.extend(alternative.properties.clone());
                    next_paths.push(PropertyPath { properties });
                }
            }
            paths = next_paths;
        }

        paths
    }

    fn property_paths_for_entry(&mut self, entry: &PropertyEntry) -> Vec<PropertyPath> {
        match entry {
            PropertyEntry::Value(property) => vec![PropertyPath {
                properties: vec![PropertyPathBinding {
                    key: property.key.clone(),
                    ty: self.infer_expr(property.value),
                    span: property.span,
                    value: property.value,
                }],
            }],
            PropertyEntry::If {
                condition,
                then_entries,
                else_entries,
                span,
            } => {
                self.check_boolean_condition(*condition, *span, "property-list if condition");
                // A presence test narrows here exactly as it does in an expression `if`.
                let then_narrowings = self.condition_narrowings(*condition, true);
                let depth = self.push_narrowings(then_narrowings);
                let mut paths = self.property_paths_for_entries(then_entries);
                self.pop_narrowings(depth);
                let else_narrowings = self.condition_narrowings(*condition, false);
                let depth = self.push_narrowings(else_narrowings);
                paths.extend(self.property_paths_for_entries(else_entries));
                self.pop_narrowings(depth);
                paths
            }
            PropertyEntry::ConditionList {
                arms, else_entries, ..
            } => {
                // Arms are tried in order, as nested `if`/`else`: an arm is reached only when every
                // earlier condition was false, so it and the `else` see each earlier condition's
                // false-branch narrowings as well as their own.
                let mut paths = Vec::new();
                let outer = self.narrowings.len();
                for arm in arms {
                    self.check_boolean_condition(
                        arm.condition,
                        arm.span,
                        "property-list condition arm",
                    );
                    let narrowings = self.condition_narrowings(arm.condition, true);
                    let depth = self.push_narrowings(narrowings);
                    paths.extend(self.property_paths_for_entries(&arm.entries));
                    self.pop_narrowings(depth);
                    let refuted = self.condition_narrowings(arm.condition, false);
                    self.push_narrowings(refuted);
                }
                paths.extend(self.property_paths_for_entries(else_entries));
                self.pop_narrowings(outer);
                paths
            }
            PropertyEntry::Match {
                scrutinee,
                arms,
                else_entries,
                span,
            } => self.property_paths_for_match(*scrutinee, arms, else_entries, *span),
        }
    }

    fn property_paths_for_match(
        &mut self,
        scrutinee: ExprId,
        arms: &[nx_hir::PropertyMatchArm],
        else_entries: &[PropertyEntry],
        span: TextSpan,
    ) -> Vec<PropertyPath> {
        let scrutinee_ty = self.infer_expr(scrutinee);
        let scrutinee_path = self.expr_path(scrutinee);
        let union_ty = match scrutinee_ty.item() {
            Type::Union(union_ty) => Some(union_ty.clone()),
            _ => None,
        };

        let mut covered_cases = FxHashSet::default();
        let mut covered_empty = false;
        let mut paths = Vec::new();

        for arm in arms {
            let present_narrowings = self.present_scrutinee_narrowings(
                scrutinee_path.as_deref(),
                &scrutinee_ty,
                covered_empty,
            );
            let pattern_tys = arm
                .patterns
                .iter()
                .map(|pattern| {
                    let pattern_ty = self.infer_match_pattern(*pattern, &scrutinee_ty);
                    self.check_match_pattern(
                        &scrutinee_ty,
                        union_ty.as_ref(),
                        &pattern_ty,
                        &mut covered_cases,
                        &mut covered_empty,
                        self.module.raw_module().expr(*pattern).span(),
                    );
                    pattern_ty
                })
                .collect::<Vec<_>>();

            let narrowed_case = self.match_arm_narrowed_case(union_ty.as_ref(), &pattern_tys);
            let mut narrowings = present_narrowings;
            if let (Some(path), Some(case_ty)) = (scrutinee_path.as_ref(), narrowed_case) {
                narrowings.push((path.clone(), Type::UnionCase(case_ty)));
            }
            let depth = self.push_narrowings(narrowings);
            paths.extend(self.property_paths_for_entries(&arm.entries));
            self.pop_narrowings(depth);
        }

        let is_exhaustive = union_ty.as_ref().is_some_and(|union_ty| {
            union_ty
                .cases
                .iter()
                .all(|case| covered_cases.contains(case))
                && (covered_empty || !scrutinee_ty.admits_zero())
        });

        if !else_entries.is_empty() {
            let narrowings = self.present_scrutinee_narrowings(
                scrutinee_path.as_deref(),
                &scrutinee_ty,
                covered_empty,
            );
            let depth = self.push_narrowings(narrowings);
            paths.extend(self.property_paths_for_entries(else_entries));
            self.pop_narrowings(depth);
        } else if let Some(union_ty) = union_ty.as_ref() {
            if !is_exhaustive {
                let mut missing = union_ty
                    .cases
                    .iter()
                    .filter(|case| !covered_cases.contains(*case))
                    .map(|case| case.as_str().to_string())
                    .collect::<Vec<_>>();
                if scrutinee_ty.admits_zero() && !covered_empty {
                    missing.push("{}".to_string());
                }
                let missing = missing.join(", ");
                self.error(
                    "non-exhaustive-union-match",
                    format!(
                        "Union match on '{}' is missing cases: {}",
                        union_ty.name, missing
                    ),
                    span,
                );
                paths.push(PropertyPath {
                    properties: Vec::new(),
                });
            }
        } else {
            paths.push(PropertyPath {
                properties: Vec::new(),
            });
        }

        paths
    }

    fn check_boolean_condition(&mut self, condition: ExprId, span: TextSpan, context: &str) {
        let condition_ty = self.infer_expr(condition);
        if !condition_ty.is_error()
            && !self.type_satisfies_expected(&condition_ty, &Type::boolean())
        {
            self.error(
                "type-mismatch",
                format!("{} expects boolean, found {}", context, condition_ty),
                span,
            );
        }
    }

    fn report_duplicate_property_paths(&mut self, paths: &[PropertyPath], element_name: &Name) {
        let mut reported = FxHashSet::<(Name, usize, usize)>::default();
        for path in paths {
            let mut seen = FxHashSet::<Name>::default();
            for property in &path.properties {
                if !seen.insert(property.key.clone()) {
                    let start: usize = property.span.start().into();
                    let end: usize = property.span.end().into();
                    if reported.insert((property.key.clone(), start, end)) {
                        self.error(
                            "duplicate-property",
                            format!(
                                "Property '{}' on '{}' can be supplied more than once on the same path",
                                property.key, element_name
                            ),
                            property.span,
                        );
                    }
                }
            }
        }
    }

    // The three diagnostic codes are what make this long: the same walk reports for an element, a
    // record literal and an update, and each names its own codes.
    #[allow(clippy::too_many_arguments)]
    fn check_property_path_bindings(
        &mut self,
        paths: &[PropertyPath],
        spec: &ElementBindingSpec,
        element_name: &Name,
        content_from_body: bool,
        span: TextSpan,
        type_mismatch_code: &str,
        unknown_property_code: &str,
        missing_property_code: &str,
    ) {
        let mut reported_unknown = FxHashSet::<(Name, usize, usize)>::default();

        for path in paths {
            for property in &path.properties {
                if let Some(expected) = spec.properties.get(&property.key) {
                    // A property whose type an unspecified parameter fixed is checked quietly
                    // first: the empty value still binds. Only a failure is reported, and it
                    // names the parameter and the form that supplies it, because `never*` is not
                    // something the author can act on.
                    if let Some(param) = expected.unspecified_parameter.as_ref() {
                        if !self.type_satisfies_expected(&property.ty, &expected.ty) {
                            self.error(
                                "type-parameter-not-specified",
                                format!(
                                    "Property '{}' on '{}' is typed by '{}', which was not specified; add {}=<type>",
                                    property.key, element_name, param, param
                                ),
                                property.span,
                            );
                            continue;
                        }
                    }
                    self.check_typed_binding_for(
                        Some(property.value),
                        &property.ty,
                        &expected.ty,
                        property.span,
                        type_mismatch_code,
                        format!("Property '{}' on '{}'", property.key, element_name),
                    );
                } else if spec.handler_properties.contains(&property.key)
                    || spec.type_parameters.contains(&property.key)
                {
                    continue;
                } else {
                    let start: usize = property.span.start().into();
                    let end: usize = property.span.end().into();
                    if reported_unknown.insert((property.key.clone(), start, end)) {
                        self.error(
                            unknown_property_code,
                            format!(
                                "Element '{}' has no property '{}'",
                                element_name, property.key
                            ),
                            property.span,
                        );
                    }
                }
            }
        }

        for (name, expected) in &spec.properties {
            if !expected.is_required {
                continue;
            }

            let supplied_by_body = content_from_body
                && spec
                    .content_property
                    .as_ref()
                    .is_some_and(|prop| prop == name);
            if supplied_by_body {
                continue;
            }

            if paths
                .iter()
                .any(|path| !path.properties.iter().any(|property| property.key == *name))
            {
                self.error(
                    missing_property_code,
                    format!("Element '{}' requires property '{}'", element_name, name),
                    span,
                );
            }
        }
    }

    /// The type of several content expressions collected into one sequence: the join of their
    /// item types under the sum of their occurrences.
    fn normalized_sequence_type(&mut self, exprs: &[ExprId], span: TextSpan) -> Type {
        if exprs.is_empty() {
            return Type::empty();
        }

        if exprs.len() == 1 {
            return self.infer_expr(exprs[0]);
        }

        // Content is spliced, so what an item contributes is its item type and its occurrence.
        // The empty value contributes `never`, which the join discards in favour of its siblings,
        // and `?`, which adds nothing to the sum.
        let contributions: Vec<_> = exprs
            .iter()
            .map(|expr_id| self.item_contribution(*expr_id))
            .collect();
        let item_types: Vec<_> = contributions.iter().map(|(ty, _)| ty.clone()).collect();
        let occ = Self::summed_occurrence(contributions.iter().map(|(_, occ)| *occ));

        Type::seq(self.common_sequence_item_type(&item_types, span), occ)
    }

    /// The occurrence of a collection of items with these occurrences: the sum, folded left,
    /// with a single item left unchanged.
    fn summed_occurrence(occurrences: impl IntoIterator<Item = Occurrence>) -> Occurrence {
        let mut occurrences = occurrences.into_iter();
        let first = occurrences.next().unwrap_or(Occurrence::OPTIONAL);
        occurrences.fold(first, Occurrence::sum)
    }

    /// What an item contributes to the sequence it sits in, typing the item first.
    ///
    /// <para>Every item contributes its item type and its occurrence: a sequence-typed item its
    /// items, an optional one zero or one, anything else exactly one. An `object` item contributes
    /// `object`: it may hold a sequence, but it is one item to the type system.</para>
    fn item_contribution(&mut self, expr_id: ExprId) -> (Type, Occurrence) {
        self.infer_expr(expr_id);
        self.item_contribution_of(expr_id)
    }

    /// The contribution of an item whose type has already been inferred.
    fn item_contribution_of(&self, expr_id: ExprId) -> (Type, Occurrence) {
        let ty = self
            .env
            .get_expr_type(expr_id)
            .cloned()
            .unwrap_or(Type::Error);
        let (item, occ) = ty.split();
        (item.clone(), occ)
    }

    fn common_sequence_item_type(&self, item_types: &[Type], _span: TextSpan) -> Type {
        let mut current = item_types
            .first()
            .cloned()
            .unwrap_or_else(|| Type::named("object"));

        for ty in item_types.iter().skip(1) {
            current = self.common_supertype(&current, ty);
        }

        current
    }

    /// Strips the occurrence a contextual name is allowed to resolve through, so `Fit?` and
    /// `Fit+` both resolve against `Fit` and the one-level lift applies to the resolved value.
    fn contextual_target(expected: &Type) -> &Type {
        expected.item()
    }

    /// Suggests the closest candidate to `name`, for a did-you-mean on an unresolved bare name.
    fn closest_candidate(name: &Name, candidates: &[Name]) -> Option<Name> {
        candidates
            .iter()
            .map(|candidate| {
                (
                    Self::edit_distance(name.as_str(), candidate.as_str()),
                    candidate,
                )
            })
            .filter(|(distance, candidate)| *distance <= 2.max(candidate.as_str().len() / 3))
            .min_by_key(|(distance, _)| *distance)
            .map(|(_, candidate)| candidate.clone())
    }

    fn edit_distance(a: &str, b: &str) -> usize {
        let b_chars: Vec<char> = b.chars().collect();
        let mut prev: Vec<usize> = (0..=b_chars.len()).collect();
        let mut current = vec![0usize; b_chars.len() + 1];
        for (i, a_char) in a.chars().enumerate() {
            current[0] = i + 1;
            for (j, b_char) in b_chars.iter().enumerate() {
                let cost = usize::from(a_char != *b_char);
                current[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(current[j] + 1);
            }
            std::mem::swap(&mut prev, &mut current);
        }
        prev[b_chars.len()]
    }

    fn candidate_list(candidates: &[Name]) -> String {
        candidates
            .iter()
            .map(|c| c.as_str().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Resolves a bare name against the expected type of its binding site.
    ///
    /// The single entry point for contextual literal resolution, shared by every binding site so
    /// the rule cannot drift between them. Resolves only against the closed nominal set — the
    /// constant cases of a union — and never falls back to treating the name as a string.
    fn resolve_contextual_name(
        &mut self,
        expr: ExprId,
        name: &Name,
        expected: &Type,
        span: TextSpan,
        context: &str,
    ) -> Option<Type> {
        self.resolve_contextual_name_in(expr, name, expected, span, context, false)
    }

    /// Resolves a bare name, optionally admitting union cases that carry a payload.
    ///
    /// A pattern matches on the discriminator, so `failed` is a valid pattern for a payload case
    /// even though it is not a valid way to construct one.
    fn resolve_contextual_name_in(
        &mut self,
        expr: ExprId,
        name: &Name,
        expected: &Type,
        span: TextSpan,
        context: &str,
        allow_payload_cases: bool,
    ) -> Option<Type> {
        let target = Self::contextual_target(expected).clone();

        // One nominal kind. What `enum` declared is a union whose cases are all constant, so a
        // bare name resolves against one closed set rather than two.
        //
        // Only a resolved `Type::Union` counts. A `Type::Named` reaching here is a name resolution
        // did not find a declaration for, and looking it up again in *this* module's namespace is
        // what let a same-named local declaration stand in for a foreign one.
        let union_info = match &target {
            Type::Union(info) => Some(info.clone()),
            _ => None,
        };
        if let Some(info) = union_info {
            let union_name = info.name.clone();

            // The resolved type's own cases are authoritative. Looking the name up again here
            // would find whatever declaration *this* module binds to that spelling, which need
            // not be the same type — that is how a same-named local declaration used to stand in
            // for a foreign one.
            if !info.cases.iter().any(|case| case == name) {
                let suggestion = Self::closest_candidate(name, &info.cases)
                    .map(|s| format!("; did you mean `{}`?", s))
                    .unwrap_or_default();
                self.error(
                    "unresolved-contextual-name",
                    format!(
                        "'{}' is not a case of union '{}'{} Cases: {}",
                        name,
                        self.display_union_name(&info),
                        suggestion,
                        Self::candidate_list(&info.cases)
                    ),
                    span,
                );
                return Some(Type::Error);
            }

            // The declaration reached under that name, and only when it really is that type.
            // Whether this module can *name* the union does not matter: the case it resolved to
            // carries the union's declaring origin from here on, so nothing below type checking
            // has to find the declaration again by a spelling that is visible here.
            let entry = self.union_entry_for(&union_name, info.origin()).cloned();

            let Some(entry) = entry else {
                // The expected type resolved to this union, so its definition was reached to build
                // it. Not finding it again is an internal inconsistency, not an authoring error.
                self.error(
                    "unresolved-contextual-name",
                    format!(
                        "'{}' resolves to '{}.{}', but the definition of '{}' could not be reached",
                        name, union_name, name, union_name
                    ),
                    span,
                );
                return Some(Type::Error);
            };
            let case = entry
                .def
                .cases
                .iter()
                .find(|case| case.name == *name)
                .expect("the resolved type lists this case");
            let case_name = case.name.clone();
            // A fieldless case of a union with an abstract base still constructs — it takes the
            // base's fields and their defaults — so nameability, not constant-ness, is what the
            // bare form needs here.
            let is_fieldless = case.fields.is_empty();

            if is_fieldless || allow_payload_cases {
                self.resolved_contextual_names.insert(
                    expr,
                    ContextualResolution {
                        type_name: union_name.clone(),
                        member: case_name.clone(),
                        origin: entry.origin.clone(),
                    },
                );
                let resolved = entry.case_type(case_name);
                self.env.set_expr_type(expr, resolved.clone());
                return Some(resolved);
            }

            self.error(
                "payload-union-case-requires-constructor",
                format!(
                    "Union case '{}.{}' requires element-style payload construction; write \
                     `<{}.{} ... />`",
                    union_name, case_name, union_name, case_name
                ),
                span,
            );
            return Some(Type::Error);
        }

        // Not a nominal type: a bare name never falls back to being a string. Whatever fix the
        // message names has to be one this site would take — offering `"r"` at an `int` site sends
        // the author to a second error — so a binding in scope is pointed at in its braced form
        // only when its own type satisfies the site, the quoted form is offered only where a
        // string satisfies the site, and neither being true still leaves the accepted shape to
        // name.
        let binding_fits = self
            .env
            .lookup(name)
            .is_some_and(|bound| self.type_satisfies_expected(bound, expected));
        let fix = if name.as_str() == "null" && !binding_fits {
            // NX has no `null`; quoting it would send the author to a string.
            "; NX has no `null`: the absent value is written `{}`, and an optional property is \
             simply omitted"
                .to_string()
        } else if binding_fits {
            format!("; to use the value bound to '{}' write {{{}}}", name, name)
        } else if self
            .type_satisfies_expected(&Type::Primitive(crate::ty::Primitive::String), expected)
        {
            format!("; for a string value write \"{}\"", name)
        } else {
            // Nothing can be pointed at by this name, but the form the site takes is still worth
            // saying: a value that is not a literal reaches a site in braces.
            "; a value here is written as a literal, or in braces as `{...}`".to_string()
        };
        self.error(
            "contextual-name-requires-nominal-type",
            format!(
                "{} expects {}, and a bare name resolves only against a union's cases{}",
                context, expected, fix
            ),
            span,
        );
        Some(Type::Error)
    }

    /// Suggests the bare form when a string was written at a site that wants a nominal value.
    ///
    /// A quoted string never resolves to a union case, so the fix is the bare
    /// spelling rather than a different string.
    fn bare_form_hint(&self, actual: &Type, expected: &Type) -> String {
        if !matches!(actual, Type::Primitive(crate::ty::Primitive::String)) {
            return String::new();
        }
        let target = Self::contextual_target(expected);
        let candidates: Vec<Name> = match target {
            Type::Union(info) => info.cases.clone(),
            _ => Vec::new(),
        };
        if candidates.is_empty() {
            return String::new();
        }
        format!(
            "; a quoted string is never a member of {}, so write the bare form, one of: {}",
            expected,
            Self::candidate_list(&candidates)
        )
    }

    /// Checks body content against the type its content property declares.
    ///
    /// <para>Content is a sequence of expressions with no expression of its own, so a rule that
    /// attaches to an expression — a contextual name, an integer literal at a float site — cannot
    /// reach it the way it reaches a property binding. At a `string` property a body of several
    /// pieces, or a single text run, number or boolean, is joined into one string; any other single
    /// content expression is checked as itself, and several anywhere else are checked as the
    /// elements of the declared list.</para>
    fn check_content_binding(
        &mut self,
        element_id: ElementId,
        content: &[ExprId],
        expected: &Type,
        span: TextSpan,
        context: String,
    ) -> bool {
        if content.len() == 1 {
            let actual = self.infer_expr(content[0]);
            // A lone braced number or boolean at a `string` property is a join of one piece, and so
            // is a lone text run, whose layout the join removes.
            let lone_text_run = self.module.raw_module().element(element_id).text_runs == [0];
            if expected.item() == &Type::string()
                && (lone_text_run || Self::stringifiable_primitive(&actual).is_some())
            {
                let accepted = self.check_string_content_piece(content[0], &actual, span, &context);
                if accepted {
                    self.string_conversions.joined_bodies.insert(element_id);
                }
                return accepted;
            }
            return self.check_typed_binding_for(
                Some(content[0]),
                &actual,
                expected,
                span,
                "content-type-mismatch",
                context,
            );
        }

        if expected.item() == &Type::string() {
            return self.check_string_content_join(element_id, content, span, &context);
        }

        let actual = self.normalized_sequence_type(content, span);

        if expected.admits_many() {
            match self.convert_literals_in(content, expected.item(), span, &context) {
                Some(true) => return true,
                // The diagnostic is already reported.
                Some(false) => return false,
                None => {}
            }
        }

        if self.type_satisfies_expected(&actual, expected) {
            return true;
        }

        self.check_typed_binding(&actual, expected, span, "content-type-mismatch", context)
    }

    /// Checks a text body that binds to a `string` content property as a join of its pieces.
    ///
    /// <para>The body binds as one string: each text run as written and each braced value in its
    /// canonical text form, so every braced piece must be a string or a stringifiable primitive.
    /// The join itself is recorded for the rewrite after analysis, which replaces the content list
    /// with a `Concat` chain; below the checker a joined body is just a string expression bound to
    /// the property.</para>
    fn check_string_content_join(
        &mut self,
        element_id: ElementId,
        content: &[ExprId],
        span: TextSpan,
        context: &str,
    ) -> bool {
        let mut accepted = true;
        for piece in content {
            let ty = self.infer_expr(*piece);
            accepted &= self.check_string_content_piece(*piece, &ty, span, context);
        }
        if accepted {
            self.string_conversions.joined_bodies.insert(element_id);
        }
        accepted
    }

    /// Checks one piece of a joined `string` body, already inferred as `ty`, recording its text
    /// conversion when it is a number or a boolean.
    fn check_string_content_piece(
        &mut self,
        piece: ExprId,
        ty: &Type,
        span: TextSpan,
        context: &str,
    ) -> bool {
        if ty.is_error() {
            return false;
        }
        if *ty == Type::string() {
            return true;
        }
        match Self::stringifiable_primitive(ty) {
            Some(primitive) => {
                self.string_conversions
                    .text_conversions
                    .insert(piece, primitive);
                true
            }
            None => {
                self.error(
                    "content-type-mismatch",
                    format!(
                        "{}: a value of type {} cannot be written into text; only a string, \
                         a number or a boolean has a text form",
                        context, ty
                    ),
                    span,
                );
                false
            }
        }
    }

    fn check_typed_binding(
        &mut self,
        actual: &Type,
        expected: &Type,
        span: TextSpan,
        code: &str,
        context: String,
    ) -> bool {
        self.check_typed_binding_for(None, actual, expected, span, code, context)
    }

    /// Checks a binding, recording the resolution when `expr` is a contextual name.
    ///
    /// Sites that know which expression they are checking pass it, so a resolved contextual name
    /// can be rewritten to its qualified form after analysis. Sites that do not pass `None`, and a
    /// contextual name reaching one of those is reported rather than silently accepted.
    fn check_typed_binding_for(
        &mut self,
        expr: Option<ExprId>,
        actual: &Type,
        expected: &Type,
        span: TextSpan,
        code: &str,
        context: String,
    ) -> bool {
        // A pending contextual name resolves here, where the expected type is finally known.
        if let Type::ContextualName(name) = actual {
            let name = name.clone();
            if let Some(occurrence) = expr.and_then(|expr| self.contextual_name_occurrence(expr)) {
                self.error(
                    "occurrence-suffix-on-value",
                    format!(
                        "{}: '{}{}' puts an occurrence suffix on a value; only a type takes one, \
                         so write '{}'",
                        context, name, occurrence, name
                    ),
                    span,
                );
                return false;
            }
            let Some(expr) = expr else {
                self.error(
                    "contextual-name-without-expected-type",
                    format!(
                        "{}: a bare name is only allowed where the declared type is known;                          write the qualified form in braces instead",
                        context
                    ),
                    span,
                );
                return false;
            };
            let resolved = self.resolve_contextual_name(expr, &name, expected, span, &context);
            return match resolved {
                Some(Type::Error) => false,
                Some(resolved) => {
                    self.check_typed_binding_for(None, &resolved, expected, span, code, context)
                }
                None => false,
            };
        }

        // A numeric literal takes the numeric type its site declares. Tried before ordinary
        // satisfaction so that `1` at an `int64` site is recorded at `int64`, not left an `int`
        // that merely widens; `numeric_literal_target` declines `object` and an undecided type
        // variable, so a site that accepts any value keeps the literal's default type.
        if let Some(expr) = expr {
            if let Some(converted) = self.convert_literals(expr, expected, span, &context) {
                return converted;
            }
        }

        if self.type_satisfies_expected(actual, expected) {
            return true;
        }

        // Two same-named types are told apart by their declaring modules; one nominal type in
        // a message is left unqualified. The empty type renders as `{}`, the form the author
        // wrote, rather than naming the bottom type.
        let (expected_display, actual_display) = crate::display_type_pair(expected, actual);
        // A sequence where something else was expected gets the plain message and none of the
        // hints below, which are about scalars: a bare-form hint, a function signature's reason,
        // and a lossy numeric conversion are all beside the point when the mismatch is a sequence.
        // The suffix in the type already says it is a sequence, so the message does not repeat it.
        let message = if actual.admits_many() && !expected.admits_many() {
            format!(
                "{} expects {}, found {}",
                context, expected_display, actual_display
            )
        } else {
            let mut hint = self.bare_form_hint(actual, expected);
            // Two function signatures side by side leave the reader to compare them; the reason
            // names the parameter that decided it.
            if let Err(reason) = self.function_satisfies_expected(actual, expected.item()) {
                hint = format!("{hint}; {reason}");
            }
            let lossy = match (actual, expected.item()) {
                (Type::Primitive(found), Type::Primitive(wanted))
                    if found.is_numeric() && wanted.is_numeric() =>
                {
                    format!(
                        "; {} does not convert to {} without loss, so the conversion is not implicit",
                        found, wanted
                    )
                }
                _ => String::new(),
            };
            format!(
                "{} expects {}, found {}{}{}",
                context, expected_display, actual_display, hint, lossy
            )
        };
        self.error(code, message, span);
        false
    }

    /// Types the numeric literals `expr` is made of by the numeric type expected of them.
    ///
    /// <para>Returns `None` when the rule does not reach this expression — it is not a numeric
    /// literal or constant expression, the site expects no numeric type, or the value already has
    /// the type — so the caller checks the binding as it would any other; `Some(true)` when every
    /// literal converted, and `Some(false)` when one could not take the type and the diagnostic has
    /// already been reported.</para>
    ///
    /// <para>An integer literal takes any numeric width: an integer one after a range check, a
    /// floating-point one after an exactness check. A real literal takes `float32` by rounding, as
    /// a `float32` literal does in any language, and takes no integer type. The recorded type moves
    /// with the value, so the literal is typed as the width the site chose and the rewrite after
    /// analysis gives it that width in the module.</para>
    ///
    /// <para>A constant expression, arithmetic over numeric literals only, is folded first at the
    /// types its literals have on their own, so `7 / 2` is `3` wherever it is written. The folded
    /// value then takes the site's type on a literal's terms: `1.5 * 2` binds at `float32`, and
    /// `1000000 * 3000` is out of range for `int32`.</para>
    ///
    /// <para>A list is walked because its elements are each written at the element type, and the
    /// binding site names only the list. A single literal at a list-typed site is reached the same
    /// way, since a scalar binds there by coercion.</para>
    fn convert_literals(
        &mut self,
        expr: ExprId,
        expected: &Type,
        span: TextSpan,
        context: &str,
    ) -> Option<bool> {
        match self.module.raw_module().expr(expr).clone() {
            ast::Expr::Literal(literal) => {
                self.convert_numeric_literal(expr, &literal, expected, span, context)
            }
            ast::Expr::BinaryOp { .. } | ast::Expr::UnaryOp { .. } => {
                // Nothing to fold unless the site would give the value another type: an `int`
                // expression at an `int` or `int64` site, or a `float64` one anywhere but a
                // `float32` site, is left for evaluation, as a literal is left as written.
                let target = numeric_literal_target(expected)?;
                match self.env.get_expr_type(expr)? {
                    Type::Primitive(Primitive::Int)
                        if matches!(target, Primitive::Int | Primitive::Int64) =>
                    {
                        return None;
                    }
                    Type::Primitive(Primitive::Float64) if target != Primitive::Float32 => {
                        return None;
                    }
                    Type::Primitive(Primitive::Int | Primitive::Float64) => {}
                    _ => return None,
                }
                let folded = match self.fold_constant(expr)? {
                    Ok(folded) => folded,
                    Err(problem) => {
                        let (code, message) = match problem {
                            ConstantProblem::DivisionByZero => (
                                "constant-division-by-zero",
                                format!("{}: the constant expression divides by zero", context),
                            ),
                            ConstantProblem::Overflow => (
                                "constant-overflow",
                                format!(
                                    "{}: the constant expression overflows {}",
                                    context,
                                    Primitive::Int
                                ),
                            ),
                        };
                        self.error(code, message, span);
                        return Some(false);
                    }
                };
                let converted =
                    self.convert_numeric_literal(expr, &folded, expected, span, context)?;
                if converted {
                    self.folded_constants.insert(expr, folded);
                }
                Some(converted)
            }
            ast::Expr::Range { start, end, .. } => {
                // The site's `T` is what the operands are checked against, so `range={0..1}` at a
                // `<Range T=float64/>` field binds two `float64` bounds, exactly as `start={0}`
                // would. The recorded type is rebuilt for the same reason a list's is: it was
                // inferred from operands that were still integers.
                let argument = self.prelude_range_argument(expected.item())?;
                if !self.convert_literals_in(&[start, end], &argument, span, context)? {
                    return Some(false);
                }
                let Type::Named(named) = expected.item() else {
                    return None;
                };
                self.env.set_expr_type(expr, Type::Named(named.clone()));
                Some(true)
            }
            ast::Expr::Array { elements, .. } => {
                if !expected.admits_many() || elements.len() < 2 {
                    return None;
                }
                if !self.convert_literals_in(&elements, expected.item(), span, context)? {
                    return Some(false);
                }
                // The braced value's own recorded type was inferred from elements that were still
                // integers, so it says `int+` over elements that are now floats. Recomputing it
                // the way inference would have is what keeps the value indistinguishable from one
                // whose elements were written as real literals.
                let contributions: Vec<_> = elements
                    .iter()
                    .map(|element| {
                        self.env
                            .get_expr_type(*element)
                            .cloned()
                            .unwrap_or(Type::Error)
                            .split()
                            .0
                            .clone()
                    })
                    .collect();
                let occ = Self::summed_occurrence(elements.iter().map(|element| {
                    self.env
                        .get_expr_type(*element)
                        .map(Type::occurrence)
                        .unwrap_or(Occurrence::ONE)
                }));
                let item_ty = self.common_sequence_item_type(&contributions, span);
                self.env.set_expr_type(expr, Type::seq(item_ty, occ));
                Some(true)
            }
            _ => None,
        }
    }

    /// Types one numeric literal, written as `expr` or folded from it, by the numeric type
    /// expected of it. Returns what [`Self::convert_literals`] does.
    fn convert_numeric_literal(
        &mut self,
        expr: ExprId,
        literal: &ast::Literal,
        expected: &Type,
        span: TextSpan,
        context: &str,
    ) -> Option<bool> {
        match literal {
            ast::Literal::Int(value) => {
                let value = *value;
                let target = numeric_literal_target(expected)?;
                match target {
                    // Its own type already.
                    Primitive::Int => return None,
                    Primitive::Int32 => {
                        if i32::try_from(value).is_err() {
                            self.error(
                                "integer-literal-out-of-range",
                                format!(
                                    "{}: {} is out of range for {}, which holds {} to {}",
                                    context,
                                    value,
                                    target,
                                    i32::MIN,
                                    i32::MAX
                                ),
                                span,
                            );
                            return Some(false);
                        }
                    }
                    Primitive::Int64 => {}
                    _ => {
                        if !target.represents_integer_exactly(value) {
                            self.error(
                                "float-literal-not-exact",
                                format!(
                                    "{}: {} is not exactly representable as {}; write the value \
                                     you mean as a {} literal",
                                    context, value, target, target
                                ),
                                span,
                            );
                            return Some(false);
                        }
                    }
                }
                self.env.set_expr_type(expr, Type::Primitive(target));
                self.converted_literals.insert(expr, target);
                Some(true)
            }
            ast::Literal::Float(_) => {
                if numeric_literal_target(expected)? != Primitive::Float32 {
                    return None;
                }
                self.env.set_expr_type(expr, Type::float32());
                self.converted_literals.insert(expr, Primitive::Float32);
                Some(true)
            }
            _ => None,
        }
    }

    /// Folds a constant expression: arithmetic whose operands are all numeric literals.
    ///
    /// <para>`None` when `expr` is not one. Otherwise the value, computed at the types the literals
    /// have on their own, as evaluation would compute it: two integers as an `int`, with `/`
    /// truncating, and anything with a real operand as a `float64`. A division by zero, or an
    /// integer result an `int` cannot hold, is a problem rather than a value.</para>
    fn fold_constant(&self, expr: ExprId) -> Option<Result<ast::Literal, ConstantProblem>> {
        use ast::{BinOp, Literal};

        match self.module.raw_module().expr(expr) {
            ast::Expr::Literal(literal @ (Literal::Int(_) | Literal::Float(_))) => {
                Some(Ok(literal.clone()))
            }
            ast::Expr::UnaryOp {
                op: ast::UnOp::Neg,
                expr: operand,
                ..
            } => Some(match self.fold_constant(*operand)? {
                Ok(Literal::Int(value)) => value
                    .checked_neg()
                    .map(Literal::Int)
                    .ok_or(ConstantProblem::Overflow),
                Ok(Literal::Float(value)) => Ok(Literal::Float(ast::OrderedFloat(-value.0))),
                other => other,
            }),
            ast::Expr::BinaryOp { lhs, op, rhs, .. } => {
                let (lhs, op, rhs) = (*lhs, *op, *rhs);
                if !matches!(
                    op,
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
                ) {
                    return None;
                }
                let lhs = self.fold_constant(lhs)?;
                let rhs = self.fold_constant(rhs)?;
                Some(lhs.and_then(|lhs| fold_binary(op, lhs, rhs?)))
            }
            _ => None,
        }
    }

    /// Types the numeric literals in a sequence of expressions by the type expected of each one.
    ///
    /// <para>Every element has to end up satisfying the element type. One that is not a convertible
    /// literal must already do so on its own, or the sequence as a whole does not bind and the
    /// caller's mismatch is the right diagnostic.</para>
    fn convert_literals_in(
        &mut self,
        elements: &[ExprId],
        element_expected: &Type,
        span: TextSpan,
        context: &str,
    ) -> Option<bool> {
        let mut converted_any = false;
        for element in elements {
            match self.convert_literals(*element, element_expected, span, context) {
                Some(true) => converted_any = true,
                Some(false) => return Some(false),
                None => {
                    let actual = self.env.get_expr_type(*element)?.clone();
                    if !self.type_satisfies_expected(&actual, element_expected) {
                        return None;
                    }
                }
            }
        }
        if converted_any {
            Some(true)
        } else {
            None
        }
    }

    /// Records a type error.
    fn error(&mut self, code: &str, message: String, span: nx_diagnostics::TextSpan) {
        self.error_involving(code, message, span, None);
    }

    /// Records a warning: something the author should look at that does not make the program
    /// wrong, such as a presence test on a value that is always present.
    fn warn(&mut self, code: &str, message: String, span: nx_diagnostics::TextSpan) {
        self.diagnostics.push(
            Diagnostic::warning(code)
                .with_message(message)
                .with_label(Label::primary(self.file_name.clone(), span))
                .build(),
        );
    }

    /// Resolves a property definition's type and enforces the rule of its slot: the type admits
    /// zero only through the `?` mark on the name.
    ///
    /// <para>The slot is checked once the type is resolved, because that is the only place an
    /// alias to a `?` or `*` type is visible, and the message is the same whether the author wrote
    /// `string?` or `Maybe`. A resolved `?` or `*` is rejected with the `name?:` form — with the
    /// same base type, and with `+` where the type was `*`, since that is what `name?:T+` reads
    /// as. The other slot rule, that an optional property has no default, is checked on the
    /// syntax tree, where the default can be quoted as written.</para>
    fn property_slot_type(&mut self, span: TextSpan, name: &Name, type_ref: &ast::TypeRef) -> Type {
        self.property_slot_type_in(span, None, name, type_ref)
    }

    /// [`Self::property_slot_type`] for a definition written by `declaring_module`.
    fn property_slot_type_in(
        &mut self,
        span: TextSpan,
        declaring_module: Option<&str>,
        name: &Name,
        type_ref: &ast::TypeRef,
    ) -> Type {
        let ty = self.type_from_type_ref_in_at(span, declaring_module, type_ref);
        let previous = std::mem::replace(&mut self.type_ref_span, span);
        let ty = self.check_property_slot(name, type_ref, ty);
        self.type_ref_span = previous;
        ty
    }

    /// Enforces the property-slot rules on an already resolved type; see
    /// [`Self::property_slot_type`]. Reported at `type_ref_span`.
    fn check_property_slot(&mut self, name: &Name, written: &ast::TypeRef, ty: Type) -> Type {
        let span = self.type_ref_span;
        if ty.admits_zero() {
            let base = ty.item().to_string();
            let fix = if ty.admits_many() {
                format!("{name}?:{base}+")
            } else {
                format!("{name}?:{base}")
            };
            // A bare name whose resolved type carries the occurrence is an alias: name it, since
            // the suffix the rule is about is written at the alias, not here.
            let carrier = match written {
                ast::TypeRef::Name(alias) => format!("{} (an alias to {})", alias, ty),
                _ => ty.to_string(),
            };
            self.error(
                "optional-in-type-slot",
                format!(
                    "Property '{}' has type {}, which admits zero; a property admits zero only \
                     through the `?` mark on its name: write `{}`",
                    name, carrier, fix
                ),
                span,
            );
            return Type::Error;
        }
        ty
    }

    /// Reports an error whose message may not name every type it is about.
    ///
    /// `involved` is a type the error concerns beyond those the message names, such as the base
    /// of a member access, and is considered by the swapped-loop-names hint.
    fn error_involving(
        &mut self,
        code: &str,
        message: String,
        span: nx_diagnostics::TextSpan,
        involved: Option<&Type>,
    ) {
        let help = self.swapped_loop_names_help(&message, span, involved);
        let mut builder = Diagnostic::error(code)
            .with_message(message)
            .with_label(Label::primary(self.file_name.clone(), span));
        if let Some(help) = help {
            builder = builder.with_help(help);
        }
        self.diagnostics.push(builder.build());
    }

    /// Notes a use of an indexed loop's item or index name, when the name resolves to the loop.
    fn record_loop_name_use(&mut self, expr_id: ExprId, name: &Name) {
        let Some(depth) = self.env.binding_depth(name) else {
            return;
        };
        let Some(indexed_loop) = self
            .indexed_loops
            .iter_mut()
            .rev()
            .find(|indexed_loop| indexed_loop.scope_depth == depth)
        else {
            return;
        };
        let span = self.module.raw_module().expr_span(expr_id);
        if *name == indexed_loop.item {
            indexed_loop.item_uses.push(span);
        } else if *name == indexed_loop.index {
            indexed_loop.index_uses.push(span);
        }
    }

    /// Says which name is the item and which the index, for an error that reads like the two
    /// were swapped.
    ///
    /// <para>`for index, row in rows` binds `index` to each row and `row` to its position, the
    /// reverse of what the names say. The mistake surfaces far from the loop, as an operator or
    /// member access on the wrong type, so the hint is given when the error's span uses one of the
    /// loop's names and the error is about the type that name really has alongside the type the
    /// other name has: the item beside an `int`, or the index where the item was expected. A loop
    /// over `int` items is skipped, since there a swap is not a type error.</para>
    fn swapped_loop_names_help(
        &self,
        message: &str,
        span: TextSpan,
        involved: Option<&Type>,
    ) -> Option<String> {
        let int = Type::int();
        let concerns = |ty: &Type| involved == Some(ty) || mentions_word(message, &ty.to_string());
        let uses_within =
            |uses: &[TextSpan]| uses.iter().any(|use_span| span.contains_range(*use_span));
        let indexed_loop = self.indexed_loops.iter().rev().find(|indexed_loop| {
            let item_ty = &indexed_loop.item_ty;
            if *item_ty == int || *item_ty == Type::Error {
                return false;
            }
            let item_as_index =
                uses_within(&indexed_loop.item_uses) && concerns(item_ty) && concerns(&int);
            let index_as_item = uses_within(&indexed_loop.index_uses) && concerns(&int);
            item_as_index || index_as_item
        })?;
        Some(format!(
            "In `for {item}, {index} in ...`, `{item}` is each item ({item_ty}) and `{index}` is \
             its zero-based index (int): the item comes first",
            item = indexed_loop.item,
            index = indexed_loop.index,
            item_ty = indexed_loop.item_ty,
        ))
    }

    /// Returns the contextual names resolved during analysis, as `expr → (type, member)`.
    pub fn resolved_contextual_names(&self) -> &FxHashMap<ExprId, ContextualResolution> {
        &self.resolved_contextual_names
    }

    /// The value expression of every plain type-argument binding the checker consumed.
    ///
    /// Read after analysis to remove each binding from its element, so nothing below the checker
    /// sees a type argument.
    pub fn range_for_expressions(&self) -> &FxHashSet<ExprId> {
        &self.range_for_expressions
    }

    pub fn consumed_type_arguments(&self) -> &FxHashSet<ExprId> {
        &self.consumed_type_arguments
    }

    /// Elements that call a function-typed value, by element id, each with the name of the
    /// callee type's content parameter when it has one.
    pub fn function_value_calls(&self) -> &FxHashMap<ElementId, Option<Name>> {
        &self.function_value_calls
    }

    /// The type each use site bound to each of its target's type parameters, by element.
    pub fn resolved_type_arguments(&self) -> &FxHashMap<ElementId, Vec<(Name, Type)>> {
        &self.resolved_type_arguments
    }

    /// Returns the numeric literals that took a numeric type from their site, and the type each
    /// took.
    pub fn converted_literals(&self) -> &FxHashMap<ExprId, Primitive> {
        &self.converted_literals
    }

    /// Returns the constant expressions that took a numeric type from their site, and the literal
    /// each folded to before taking it.
    pub fn folded_constants(&self) -> &FxHashMap<ExprId, ast::Literal> {
        &self.folded_constants
    }

    /// Returns the string conversions analysis decided on: the additions that concatenate, the
    /// operands rendered as text, and the text bodies joined into one string.
    pub fn string_conversions(&self) -> &StringConversions {
        &self.string_conversions
    }

    /// The branches of a join that widen, and the numeric type each widens to.
    pub fn lifted_joins(&self) -> &FxHashSet<ExprId> {
        &self.lifted_joins
    }

    pub fn widened_joins(&self) -> &FxHashMap<ExprId, Primitive> {
        &self.widened_joins
    }

    /// Returns the collected diagnostics.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Returns the type environment.
    pub fn env(&self) -> &TypeEnvironment {
        &self.env
    }

    /// Consumes the context and returns the environment and diagnostics.
    pub fn finish(self) -> (TypeEnvironment, Vec<Diagnostic>) {
        (self.env, self.diagnostics)
    }

    fn register_type_definitions(&mut self) {
        let bindings = self
            .module
            .bindings(PreparedNamespace::Type)
            .cloned()
            .collect::<Vec<_>>();

        for binding in bindings {
            let Some(resolved) = self.module.resolve_prepared_item(&binding) else {
                continue;
            };
            // The declaration this name reaches, which is what makes the type it denotes the same
            // type wherever else it is reached — under another name, or from another module.
            let origin = DeclaringOrigin::new(resolved.module_identity(), resolved.definition_id());

            match resolved {
                ResolvedPreparedItem::Raw {
                    item: Item::TypeAlias(ref alias),
                    ..
                } => {
                    self.type_aliases.insert(
                        binding.visible_name.clone(),
                        TypeAliasInfo {
                            target: alias.ty.clone(),
                        },
                    );
                }
                ResolvedPreparedItem::Imported { ref item, .. } => {
                    if let Some(alias) = interface_type_alias(item) {
                        self.type_aliases.insert(
                            binding.visible_name.clone(),
                            TypeAliasInfo { target: alias.ty },
                        );
                    } else if let Some(mut union_def) = interface_union(item) {
                        union_def.name = binding.visible_name.clone();
                        self.union_defs.insert(
                            binding.visible_name.clone(),
                            UnionEntry {
                                def: union_def,
                                origin: Some(origin),
                            },
                        );
                    } else if matches!(item.item, InterfaceItemKind::Record { .. }) {
                        self.record_origins
                            .insert(binding.visible_name.clone(), origin);
                    }
                }
                ResolvedPreparedItem::Raw {
                    item: Item::Record(_),
                    ..
                } => {
                    self.record_origins
                        .insert(binding.visible_name.clone(), origin);
                }
                ResolvedPreparedItem::Raw {
                    item: Item::Union(ref union_def),
                    ..
                } => {
                    let mut union_def = union_def.clone();
                    union_def.name = binding.visible_name.clone();
                    self.union_defs.insert(
                        binding.visible_name.clone(),
                        UnionEntry {
                            def: union_def,
                            origin: Some(origin),
                        },
                    );
                }
                _ => {}
            }
        }

        let element_bindings = self
            .module
            .bindings(PreparedNamespace::Element)
            .cloned()
            .collect::<Vec<_>>();

        for binding in element_bindings {
            if binding.kind != PreparedItemKind::Component {
                continue;
            }
            let Some(resolved) = self.module.resolve_prepared_item(&binding) else {
                continue;
            };
            self.component_origins
                .insert(binding.visible_name.clone(), resolved.declaring_origin());
        }
    }

    /// Checks every locally declared record's own field types and defaults.
    ///
    /// <para>A generic record's fields are checked under a rigid scope, where each of its type
    /// parameters denotes its own type and nothing else: that is what makes `value:T = "text"` a
    /// mismatch and `items:T[]` fine. Resolving each field type, default or not, is also what
    /// reports a malformed applied type where it was written.</para>
    fn validate_local_record_defaults(&mut self) {
        let local_items = self.module.raw_module().items().to_vec();
        for item in local_items {
            if let Item::Record(record_def) = item {
                // A derived update record's fields are copies of its target's, with defaults
                // stripped, so there is nothing here that the target's own pass does not already
                // cover — and resolving them again would report each malformed field type twice.
                if record_def.update_target().is_some() {
                    continue;
                }
                let owner = self.record_origins.get(&record_def.name).cloned();
                let type_param_names: Vec<Name> = record_def
                    .type_params
                    .iter()
                    .map(|param| param.name.clone())
                    .collect();
                let previous_scope = std::mem::replace(
                    &mut self.type_parameter_scope,
                    Self::rigid_type_parameter_scope(&type_param_names, owner),
                );
                for prop in &record_def.properties {
                    let expected = self.property_slot_type(prop.span, &prop.name, &prop.ty);
                    if let Some(default_expr) = prop.default {
                        let actual = self.infer_expr(default_expr);
                        self.check_typed_binding_for(
                            Some(default_expr),
                            &actual,
                            &expected,
                            prop.span,
                            "record-default-type-mismatch",
                            format!("Default value for record property '{}'", prop.name),
                        );
                    }
                }
                self.type_parameter_scope = previous_scope;
            }
        }
    }

    /// Resolves each local type alias's target, so a target that is not a type is reported where
    /// the alias was written rather than at whichever use site reaches it first.
    ///
    /// <para>`type Rows = Names*` where `Names` already carries an occurrence is the case this
    /// exists for: the parse validator sees no second suffix to object to, and an alias nobody
    /// uses would otherwise never be resolved at all.</para>
    fn validate_local_type_aliases(&mut self) {
        let local_items = self.module.raw_module().items().to_vec();
        // Recorded before any of them is resolved, because resolving one alias can reach another
        // declared after it, and that one's report belongs on its own declaration too.
        for item in &local_items {
            if let Item::TypeAlias(alias) = item {
                self.local_type_alias_spans
                    .insert(alias.name.clone(), alias.span);
            }
        }
        for item in local_items {
            if let Item::TypeAlias(alias) = item {
                // An alias an earlier one already reached is resolved and reported already: this
                // loop walks declarations in source order, and an alias may name one declared
                // after it. Walking it again here would report whatever is wrong with its target
                // a second time, and whether that happened would depend on declaration order.
                if self.resolved_type_aliases.contains_key(&alias.name) {
                    continue;
                }
                // The answer is cached under the alias's name, so this is the resolution every
                // use of the alias reads. Resolving the target here and again at the first use
                // would report whatever is wrong with it twice, once at each span.
                let ty = self.resolve_local_alias_target(&alias.name, alias.span, &alias.ty);
                self.resolved_type_aliases.insert(alias.name.clone(), ty);
            }
        }
    }

    /// Resolves one local alias's target exactly as a use site reaching that alias would.
    ///
    /// <para>`seen` starts with the alias's own name and the target resolves unscoped, which is
    /// what [`Self::resolve_named_type`] does, so the eager pass and a use site cannot disagree
    /// about the answer they cache. It matters most for a cycle: entering the walk at the target
    /// instead of at the alias leaves the alias out of `seen` until the loop comes back round, so
    /// the cycle closes on whichever name completes it -- which may be one another module
    /// declared, leaving the report naming that alias while underlining this one.</para>
    fn resolve_local_alias_target(
        &mut self,
        name: &Name,
        span: TextSpan,
        target: &ast::TypeRef,
    ) -> Type {
        let mut seen = FxHashSet::default();
        seen.insert(name.clone());
        let enclosing_span = std::mem::replace(&mut self.type_ref_span, span);
        let ty = self.type_from_type_ref_walk(None, target, &mut seen, false);
        self.type_ref_span = enclosing_span;
        ty
    }

    fn validate_local_union_defaults(&mut self) {
        let local_items = self.module.raw_module().items().to_vec();
        for item in local_items {
            if let Item::Union(union_def) = item {
                for case in &union_def.cases {
                    for field in &case.fields {
                        // Resolved whether or not it has a default, so a type reference that names
                        // nothing usable — a generic record with no arguments, say — is reported
                        // where it was written.
                        let expected = self.property_slot_type(field.span, &field.name, &field.ty);
                        if let Some(default_expr) = field.default {
                            let actual = self.infer_expr(default_expr);
                            self.check_typed_binding_for(
                                Some(default_expr),
                                &actual,
                                &expected,
                                field.span,
                                "union-case-default-type-mismatch",
                                format!(
                                    "Default value for union case field '{}.{}.{}'",
                                    union_def.name, case.name, field.name
                                ),
                            );
                        }
                    }
                }
            }
        }
    }

    fn register_function_signatures(&mut self) {
        let bindings = self
            .module
            .bindings(PreparedNamespace::Value)
            .cloned()
            .collect::<Vec<_>>();

        for binding in bindings {
            let Some(resolved) = self.module.resolve_prepared_item(&binding) else {
                continue;
            };

            // A function's parameter and return annotations are names the *declaring* module
            // wrote, so they are resolved there. Resolving them here would let an unrelated local
            // declaration sharing the spelling become an imported function's signature.
            let declaring_module = resolved.module_identity().to_string();

            match resolved {
                ResolvedPreparedItem::Raw {
                    item: Item::Function(func),
                    origin,
                    ..
                } => {
                    if matches!(origin, PreparedBindingOrigin::Local) {
                        self.reject_reserved_intrinsic_name(&func.name, "function", func.span);
                    }
                    let return_type = if let Some(ty) = func.return_type.as_ref() {
                        self.type_from_type_ref_in_quietly(Some(&declaring_module), ty)
                    } else {
                        let placeholder = self.fresh_var();
                        if matches!(origin, PreparedBindingOrigin::Local) {
                            self.function_return_placeholders
                                .insert(binding.visible_name.clone(), placeholder.clone());
                        }
                        placeholder
                    };

                    self.bind_function_signature_from_parts(
                        binding.visible_name.clone(),
                        &func.params,
                        return_type,
                        Some(&declaring_module),
                    );
                }
                ResolvedPreparedItem::Imported { item, .. } => {
                    if let Some((_name, _visibility, _form, params, return_type, _span)) =
                        interface_function_signature(&item)
                    {
                        let param_types = params
                            .iter()
                            .map(|param| FunctionParam {
                                name: param.name.clone(),
                                ty: self.type_from_type_ref_in_quietly(
                                    Some(&declaring_module),
                                    &param.ty,
                                ),
                                is_content: param.is_content,
                                optional: param.optional,
                            })
                            .collect::<Vec<_>>();
                        let return_type = self
                            .type_from_type_ref_in_quietly(Some(&declaring_module), &return_type);
                        self.env.bind(
                            binding.visible_name.clone(),
                            Type::function(param_types, return_type),
                        );
                    }
                }
                _ => {}
            }
        }
    }

    /// Rejects a top-level declaration that would bind an intrinsic's name.
    ///
    /// <para>The intrinsic resolves first at every call, so the declaration could never be reached
    /// through that name; saying so here is what keeps the author from writing it and wondering
    /// why it does nothing.</para>
    fn reject_reserved_intrinsic_name(&mut self, name: &Name, kind: &str, span: TextSpan) {
        if nx_hir::is_update_intrinsic(name.as_str()) {
            self.error(
                "intrinsic-name-reserved",
                format!(
                    "'{}' is an intrinsic function and cannot be declared as a {}",
                    name, kind
                ),
                span,
            );
        }
    }

    fn register_value_bindings(&mut self) {
        let mut bindings = self
            .module
            .bindings(PreparedNamespace::Value)
            .cloned()
            .collect::<Vec<_>>();
        // A local value's initializer may read a value declared before it, so local values are
        // inferred in declaration order; everything reached from elsewhere carries its own type.
        bindings.sort_by_key(|binding| match &binding.target {
            nx_hir::PreparedBindingTarget::Local { definition_id } => (0, definition_id.index()),
            _ => (1, 0),
        });

        for binding in bindings {
            let Some(resolved) = self.module.resolve_prepared_item(&binding) else {
                continue;
            };

            // An imported value's annotation is a name the *declaring* module wrote, so it is
            // resolved there. Resolving it here would let an unrelated local declaration sharing
            // the spelling become the imported value's type.
            let declaring_module = resolved.module_identity().to_string();

            match resolved {
                ResolvedPreparedItem::Raw {
                    module_identity,
                    item: Item::Value(value),
                    ..
                } => {
                    let binding_ty = if module_identity == self.module.module_identity() {
                        self.reject_reserved_intrinsic_name(&value.name, "value", value.span);
                        let actual = self.infer_expr(value.value);
                        if let Some(ty_ref) = value.ty.as_ref() {
                            let expected = self.type_from_type_ref_at(value.span, ty_ref);
                            self.check_typed_binding_for(
                                Some(value.value),
                                &actual,
                                &expected,
                                value.span,
                                "value-type-mismatch",
                                format!("Initializer for value '{}'", value.name),
                            );
                            expected
                        } else if let Type::ContextualName(name) = &actual {
                            // A bare name has no type of its own: it resolves only against a
                            // declared type, which an unannotated binding does not supply.
                            let message = if name.as_str() == "null" {
                                "`null` is not a value; the absent value is written `{}`"
                                    .to_string()
                            } else {
                                format!(
                                    "'{}' is a bare name, which resolves only against a declared \
                                     type; annotate '{}' with the type it should resolve against, \
                                     or write the value in braces",
                                    name, value.name
                                )
                            };
                            self.error(
                                "contextual-name-without-expected-type",
                                message,
                                value.span,
                            );
                            Type::Error
                        } else if Self::mentions_never(&actual) {
                            // `{}` has a real type and this binding could simply take it. It is
                            // reported anyway, and only here, where the binding has a name to put
                            // in the message: a binding whose type is fixed by the empty value says
                            // nothing about what it holds, and the next reader has no way to find
                            // out. The annotation is required for legibility, not because the
                            // system cannot type it, so the binding keeps the type it has rather
                            // than poisoning to `Error` — exactly as the function-return arm in
                            // `infer_function` does. A legibility rule reports once and leaves the
                            // program otherwise typed.
                            self.error(
                                "empty-value-type-unknown",
                                format!(
                                    "Cannot determine the item type of the empty value bound to \
                                     '{}'; annotate the binding with the type you mean, as in \
                                     'let {}: string* = {{}}'",
                                    value.name, value.name
                                ),
                                value.span,
                            );
                            actual
                        } else {
                            actual
                        }
                    } else {
                        value
                            .ty
                            .as_ref()
                            .map(|ty_ref| {
                                self.type_from_type_ref_in_quietly(Some(&declaring_module), ty_ref)
                            })
                            .unwrap_or(Type::Error)
                    };

                    self.env.bind(binding.visible_name.clone(), binding_ty);
                }
                ResolvedPreparedItem::Imported { item, .. } => {
                    if let InterfaceItemKind::Value { ty, .. } = &item.item {
                        let ty = ty.clone();
                        let binding_ty =
                            self.type_from_type_ref_in_quietly(Some(&declaring_module), &ty);
                        self.env.bind(binding.visible_name.clone(), binding_ty);
                    }
                }
                _ => {}
            }
        }
    }

    fn resolve_function_definition(&self, name: &Name) -> Option<ResolvedPreparedItem> {
        self.module
            .resolve_binding(PreparedNamespace::Element, name)
            .and_then(|binding| self.module.resolve_prepared_item(binding))
            .and_then(|resolved| match &resolved {
                ResolvedPreparedItem::Raw {
                    item: Item::Function(_),
                    ..
                } => Some(resolved),
                ResolvedPreparedItem::Imported { item, .. }
                    if matches!(item.item, InterfaceItemKind::Function { .. }) =>
                {
                    Some(resolved)
                }
                _ => None,
            })
    }

    fn resolve_component_definition(&self, name: &Name) -> Option<ResolvedPreparedItem> {
        self.module
            .resolve_binding(PreparedNamespace::Element, name)
            .and_then(|binding| self.module.resolve_prepared_item(binding))
            .and_then(|resolved| match &resolved {
                ResolvedPreparedItem::Raw {
                    item: Item::Component(_),
                    ..
                } => Some(resolved),
                ResolvedPreparedItem::Imported { item, .. }
                    if matches!(item.item, InterfaceItemKind::Component { .. }) =>
                {
                    Some(resolved)
                }
                _ => None,
            })
    }

    fn resolve_record_definition(&self, name: &Name) -> Option<nx_hir::RecordDef> {
        nx_hir::resolve_record_definition(self.module, name)
    }

    fn resolve_record_definition_with_origin(
        &self,
        name: &Name,
    ) -> Option<(String, nx_hir::RecordDef)> {
        nx_hir::resolve_record_definition_with_module(self.module, name)
    }

    /// The union an expression names, when it is a bare or dotted type name such as `Status` or
    /// `User.Property`.
    fn union_info_for_expr(&self, expr_id: ExprId) -> Option<UnionType> {
        let name = self.flattened_expr_name(expr_id)?;
        let mut seen = FxHashSet::default();
        self.union_info_from_name(&name, &mut seen)
    }

    /// The union a type name denotes here, following type aliases.
    fn union_info_from_name(&self, name: &Name, seen: &mut FxHashSet<Name>) -> Option<UnionType> {
        if let Some(entry) = self.union_defs.get(name) {
            return Some(entry.shape());
        }

        if let Some(alias) = self.type_aliases.get(name) {
            if !seen.insert(name.clone()) {
                return None;
            }
            if let ast::TypeRef::Name(target) = &alias.target {
                let target_info = self.union_info_from_name(target, seen);
                seen.remove(name);
                return target_info;
            }
            seen.remove(name);
        }

        None
    }

    /// Types a member access against a union whose type is already resolved.
    fn union_case_by_member(
        &mut self,
        union_info: &UnionType,
        member: &Name,
        span: TextSpan,
    ) -> Type {
        if union_info.cases.iter().any(|case| case == member) {
            return Type::union_case_type(
                union_info.name.clone(),
                member.clone(),
                union_info.origin().cloned(),
            );
        }
        self.report_unknown_union_case(union_info, member, span);
        Type::Error
    }

    /// Reports a member access that names something the union does not declare.
    fn report_unknown_union_case(&mut self, union_info: &UnionType, case: &Name, span: TextSpan) {
        let suggestion = Self::closest_candidate(case, &union_info.cases)
            .map(|s| format!("; did you mean `{}`?", s))
            .unwrap_or_default();
        self.error(
            "undefined-union-case",
            format!(
                "Union '{}' has no case named '{}'{} Cases: {}",
                self.display_union_name(union_info),
                case,
                suggestion,
                Self::candidate_list(&union_info.cases)
            ),
            span,
        );
    }

    /// Renders a union's name for a diagnostic, qualified when another union here shares it.
    ///
    /// <para>A message that says `union 'Fit'` while a different `Fit` is declared in the module
    /// the author is reading describes the wrong declaration to them: theirs does have the case the
    /// message says does not exist. Naming the declaring module is what tells the two apart, and it
    /// is added only when there are two to tell apart.</para>
    fn display_union_name(&self, union_info: &UnionType) -> String {
        let Some(origin) = union_info.origin() else {
            return union_info.name.to_string();
        };
        let shares_the_name_here = self
            .union_defs
            .get(&union_info.name)
            .is_some_and(|entry| entry.origin.as_ref() != Some(origin));
        if shares_the_name_here {
            format!("{}:{}", origin.module_identity(), union_info.name)
        } else {
            union_info.name.to_string()
        }
    }

    fn union_case_from_qualified_name<'info>(
        &'info self,
        name: &Name,
    ) -> Option<(&'info UnionEntry, &'info UnionCaseDef)> {
        let (union_name, case_name) = name.as_str().rsplit_once('.')?;
        let union_name = Name::new(union_name);
        let case_name = Name::new(case_name);
        let entry = self.union_defs.get(&union_name)?;
        let case = entry.def.cases.iter().find(|case| case.name == case_name)?;
        Some((entry, case))
    }

    /// Converts a type reference, reporting a problem with the reference itself at `span`.
    ///
    /// <para>Every caller that knows where the reference was written goes through this, so that an
    /// applied type missing an argument or a bare generic record name underlines the annotation
    /// rather than the start of the file. See [`Self::type_ref_span`].</para>
    fn type_from_type_ref_at(&mut self, span: TextSpan, type_ref: &ast::TypeRef) -> Type {
        self.type_from_type_ref_in_at(span, None, type_ref)
    }

    /// Converts a type reference without reporting a problem with the reference itself.
    ///
    /// <para>For the signature pre-pass, which resolves every function's annotations before any
    /// body is inferred. A local function's annotations are resolved again by
    /// [`Self::infer_function`], which knows each one's span, so only that pass reports and one
    /// malformed annotation is one diagnostic underlined where it was written. An imported
    /// declaration's annotations are diagnosed by the check of the module that wrote them, where
    /// their spans mean something.</para>
    fn type_from_type_ref_in_quietly(
        &mut self,
        declaring_module: Option<&str>,
        type_ref: &ast::TypeRef,
    ) -> Type {
        let before = self.diagnostics.len();
        // A quiet resolution has to leave no trace: an alias it resolved for the first time would
        // otherwise answer every later use from the cache, with the diagnostics that resolution
        // produced already thrown away. So the cache is rolled back with them.
        let resolved_before = self.resolved_type_aliases.clone();
        let ty = self.type_from_type_ref_in(declaring_module, type_ref);
        self.diagnostics.truncate(before);
        self.resolved_type_aliases = resolved_before;
        ty
    }

    /// [`Self::type_from_type_ref_at`] for a reference written by `declaring_module`.
    fn type_from_type_ref_in_at(
        &mut self,
        span: TextSpan,
        declaring_module: Option<&str>,
        type_ref: &ast::TypeRef,
    ) -> Type {
        let previous = std::mem::replace(&mut self.type_ref_span, span);
        let ty = self.type_from_type_ref_in(declaring_module, type_ref);
        self.type_ref_span = previous;
        ty
    }

    /// Resolves a type reference's own names through the type-parameter scope first.
    ///
    /// <para>Only the names the reference spells directly are looked up there. Everything
    /// else — an alias's target, a foreign declaration's own signature — goes through the ordinary
    /// resolver, so a parameter shadows a type only where the author wrote the name.</para>
    fn resolve_named_type(&mut self, name: &Name, seen: &mut FxHashSet<Name>) -> Type {
        if let Some(alias) = self.type_aliases.get(name) {
            if let Some(resolved) = self.resolved_type_aliases.get(name) {
                return resolved.clone();
            }
            if !seen.insert(name.clone()) {
                // The alias a cycle closes on is not always one this module wrote: the eager
                // alias pass enters the cycle from a local alias, so it closes on whichever name
                // completes the loop, which may be an imported one. That alias's span belongs to
                // the file that declared it and cannot underline anything here -- it can even run
                // past the end of this one. Where the name is not local, the reference that
                // reached it is the best span this file has.
                let span = self
                    .local_type_alias_spans
                    .get(name)
                    .copied()
                    .unwrap_or(self.type_ref_span);
                self.error(
                    "type-alias-cycle",
                    format!("Type alias '{}' forms a cycle", name),
                    span,
                );
                return Type::Error;
            }

            // An alias target was written outside any declaration, so no type parameter is in
            // scope for it; it may still be an applied type, which the walker resolves.
            //
            // A problem with the target belongs to the alias that wrote it, not to whatever
            // reference reached the alias first — which may be another alias, or a binding on a
            // line that names neither the target nor what is wrong with it. So the span is the
            // alias's for the length of the walk, and the caller's is put back afterwards.
            let target = alias.target.clone();
            let enclosing_span = self
                .local_type_alias_spans
                .get(name)
                .copied()
                .map(|alias_span| std::mem::replace(&mut self.type_ref_span, alias_span));
            let ty = self.type_from_type_ref_walk(None, &target, seen, false);
            if let Some(enclosing_span) = enclosing_span {
                self.type_ref_span = enclosing_span;
            }
            seen.remove(name);
            self.resolved_type_aliases.insert(name.clone(), ty.clone());
            return ty;
        }

        if let Some(entry) = self.union_defs.get(name) {
            return Type::Union(entry.shape());
        }

        self.nominal_named_type(name)
    }

    /// The nominal type one visible name denotes, carrying the declaration it reaches.
    ///
    /// A record or component name reaches a declaration; `Element`, `object`, and a name that
    /// reaches nothing do not, and stay origin-less.
    fn nominal_named_type(&self, name: &Name) -> Type {
        let origin = self
            .record_origins
            .get(name)
            .or_else(|| self.component_origins.get(name))
            .cloned();
        Type::named_at(name.clone(), origin)
    }

    /// Binds a function's type from its parts, resolving each parameter annotation in
    /// `declaring_module` — the module that wrote the signature, or `None` for this one.
    fn bind_function_signature_from_parts(
        &mut self,
        name: Name,
        params: &[nx_hir::Param],
        return_type: Type,
        declaring_module: Option<&str>,
    ) {
        let param_types = params
            .iter()
            .map(|param| FunctionParam {
                name: param.name.clone(),
                ty: self.type_from_type_ref_in_quietly(declaring_module, &param.ty),
                is_content: param.is_content,
                optional: param.optional,
            })
            .collect::<Vec<_>>();
        self.env
            .bind(name, Type::function(param_types, return_type));
    }

    /// The function type a tag or callee `name` denotes as a *value*: a parameter, a prop, a local
    /// or top-level `let` of function type, or a lexical binding that shadows a declared function.
    ///
    /// <para>A declared function reached under its own name is not a value here: `<Row />` on a
    /// declaration `Row` is the declaration's call, checked against the declaration itself, and
    /// `add(1, 2)` on a declared paren function is the positional call it always was.</para>
    fn function_typed_value(&self, name: &Name) -> Option<(Vec<FunctionParam>, Type)> {
        let ty = self.env.lookup(name)?;
        let (params, ret) = ty.function_parts()?;
        let shadows_declaration = self.env.binding_depth(name).is_some_and(|depth| depth > 1);
        if self.resolve_function_definition(name).is_some() && !shadows_declaration {
            return None;
        }
        Some((params.to_vec(), ret.clone()))
    }

    /// The name and parameters of a call's callee when it is a function-typed value.
    fn function_value_callee(&self, callee: ExprId) -> Option<(Name, Vec<FunctionParam>)> {
        let ast::Expr::Ident(name) = self.module.raw_module().expr(callee) else {
            return None;
        };
        let (params, _) = self.function_typed_value(name)?;
        Some((name.clone(), params))
    }

    /// The form and parameters of a call's callee when it names a declared function under its
    /// own name, each parameter as its name and whether a caller may omit it.
    fn declared_function_callee(&mut self, callee: ExprId) -> Option<DeclaredCallee> {
        let ast::Expr::Ident(name) = self.module.raw_module().expr(callee) else {
            return None;
        };
        if self.env.binding_depth(name).is_some_and(|depth| depth > 1) {
            return None;
        }
        let resolved = self.resolve_function_definition(name)?;
        let (form, params) = match &resolved {
            ResolvedPreparedItem::Raw {
                item: Item::Function(function),
                ..
            } => (
                function.form,
                function
                    .params
                    .iter()
                    .map(|param| (param.name.clone(), param.ty.clone(), param.is_omissible()))
                    .collect::<Vec<_>>(),
            ),
            ResolvedPreparedItem::Imported { item, .. } => {
                let (_, _, form, params, _, _) = interface_function_signature(item)?;
                (
                    form,
                    params
                        .into_iter()
                        .map(|param| {
                            let omissible = param.is_omissible();
                            (param.name, param.ty, omissible)
                        })
                        .collect::<Vec<_>>(),
                )
            }
            _ => return None,
        };
        // A parameter whose declared type admits zero was rejected at the declaration and reads as
        // marked, as an element's property does, so a call that leaves it off is not a second
        // report of the same mistake.
        let module_identity = resolved.module_identity().to_string();
        let params = params
            .into_iter()
            .map(|(param_name, ty, omissible)| {
                let omissible = omissible
                    || self
                        .type_from_type_ref_in_quietly(Some(&module_identity), &ty)
                        .admits_zero();
                (param_name, omissible)
            })
            .collect();
        Some(DeclaredCallee {
            name: name.clone(),
            form,
            params,
        })
    }

    // The `Err` is large for the reason `RecordResolutionError` records: it carries the spans its
    // diagnostic prints.
    #[allow(clippy::result_large_err)]
    fn effective_record_shape(
        &self,
        name: &Name,
    ) -> Result<Option<nx_hir::EffectiveRecordShape>, nx_hir::RecordResolutionError> {
        effective_record_shape_for_name(self.module, name)
    }

    fn effective_component_contract(
        &self,
        name: &Name,
    ) -> Result<Option<nx_hir::EffectiveComponentContract>, nx_hir::ComponentResolutionError> {
        effective_component_contract_for_name(self.module, name)
    }

    fn record_type_satisfies_expected(&self, actual: &NamedType, expected: &NamedType) -> bool {
        // Applied types are invariant: an instantiation satisfies an expectation only when both
        // bind every parameter to the same type. A generic record has no ancestors, so the
        // arguments and the declaration are the whole comparison.
        if actual.args() != expected.args() {
            return false;
        }
        is_record_subtype(
            self.module,
            &actual.name,
            actual.origin(),
            &expected.name,
            expected.origin(),
        )
        .unwrap_or(false)
    }

    fn component_type_satisfies_expected(&self, actual: &NamedType, expected: &NamedType) -> bool {
        nx_hir::is_component_subtype(
            self.module,
            &actual.name,
            actual.origin(),
            &expected.name,
            expected.origin(),
        )
        .unwrap_or(false)
    }

    fn named_type_satisfies_expected(&self, actual: &NamedType, expected: &NamedType) -> bool {
        self.record_type_satisfies_expected(actual, expected)
            || self.component_type_satisfies_expected(actual, expected)
    }

    /// Decides whether a union satisfies an expected record type through its abstract base.
    ///
    /// The base is a name the *union's* module wrote, so it is resolved there. Resolving it here
    /// would let an unrelated local record of that name make the union satisfy a foreign base.
    fn union_type_satisfies_record(
        &self,
        union_name: &Name,
        union_origin: Option<&DeclaringOrigin>,
        expected: &NamedType,
    ) -> bool {
        let Some(entry) = self.union_entry_for(union_name, union_origin) else {
            return false;
        };
        let Some(base) = entry.def.base.as_ref() else {
            return false;
        };

        let base_origin = entry
            .origin
            .as_ref()
            .and_then(|origin| self.record_origin_in(origin.module_identity(), base));

        is_record_subtype(
            self.module,
            base,
            base_origin.as_ref(),
            &expected.name,
            expected.origin(),
        )
        .unwrap_or(false)
    }

    /// Returns the declaration a record name reaches in `module_identity`'s own namespace.
    fn record_origin_in(&self, module_identity: &str, name: &Name) -> Option<DeclaringOrigin> {
        if module_identity == self.module.module_identity() {
            return self.record_origins.get(name).cloned();
        }

        self.module
            .resolve_in_module(PreparedNamespace::Type, module_identity, name)
            .filter(|resolved| resolved.kind() == PreparedItemKind::Record)
            .map(|resolved| resolved.declaring_origin())
    }

    fn named_type_is_element_like(&self, name: &Name) -> bool {
        if name.as_str() == "Element" {
            return true;
        }

        if self
            .module
            .resolve_binding(PreparedNamespace::Element, name)
            .is_some()
        {
            true
        } else {
            self.module
                .resolve_binding(PreparedNamespace::Type, name)
                .or_else(|| self.module.resolve_binding(PreparedNamespace::Value, name))
                .is_none()
        }
    }

    fn type_satisfies_expected(&self, actual: &Type, expected: &Type) -> bool {
        if generic_type_satisfies_expected(actual, expected) {
            return true;
        }

        match (actual, expected) {
            // The bottom type satisfies every expectation, which is what makes it the bottom.
            (Type::Primitive(Primitive::Never), _) => true,
            // Occurrences follow the lattice and items follow this relation. An exactly-one
            // value at a suffixed site is a sequence of one — the one-level lift — and a value
            // that admits zero or many never satisfies an exactly-one site.
            (Type::Seq { .. }, Type::Seq { .. }) | (_, Type::Seq { .. }) => {
                let (item, occ) = actual.split();
                let (expected_item, expected_occ) = expected.split();
                occ.satisfies(expected_occ) && self.type_satisfies_expected(item, expected_item)
            }
            (Type::Seq { .. }, _) => false,
            (Type::Named(actual_name), Type::Named(expected_name))
                if expected_name.name.as_str() == "Element" =>
            {
                self.named_type_is_element_like(&actual_name.name)
            }
            (Type::Named(actual_name), Type::Named(expected_name)) => {
                self.named_type_satisfies_expected(actual_name, expected_name)
            }
            // The case must be a case of *that* union — the one declared at the same origin.
            // Matching on the name alone, or on the name and the case list, would let a same-named
            // local declaration's case stand in for a foreign union's.
            (Type::UnionCase(case), Type::Union(union)) => case.is_case_of(union),
            (Type::UnionCase(case), Type::Named(expected_name)) => {
                self.union_type_satisfies_record(&case.union, case.origin(), expected_name)
            }
            (Type::Union(union), Type::Named(expected_name)) => {
                self.union_type_satisfies_record(&union.name, union.origin(), expected_name)
            }
            (Type::Function { .. }, Type::Function { .. }) => {
                self.function_satisfies_expected(actual, expected).is_ok()
            }
            _ => false,
        }
    }

    /// Checks a function's type against a function type by parameter name, under this checker's
    /// own relation, so a parameter typed by a record or a component subtype pairs the way any
    /// other binding does.
    // The `Err` is large for the reason `check_function_satisfies` records.
    #[allow(clippy::result_large_err)]
    fn function_satisfies_expected(
        &self,
        actual: &Type,
        expected: &Type,
    ) -> Result<(), FunctionMismatch> {
        let (Some((actual_params, actual_ret)), Some((expected_params, expected_ret))) =
            (actual.function_parts(), expected.function_parts())
        else {
            return Ok(());
        };
        check_function_satisfies(
            actual_params,
            actual_ret,
            expected_params,
            expected_ret,
            &mut |value, target| self.type_satisfies_expected(value, target),
        )
    }

    /// True when `never` occurs anywhere in this type.
    ///
    /// <para>Only the empty value puts it there, so this asks whether the type was fixed by one.
    /// It looks through a function's return type as well as through an occurrence, because an
    /// unannotated `let f(x) = {}` is a binding whose type the empty value decided just as much as
    /// `let a = {}` is.</para>
    fn mentions_never(ty: &Type) -> bool {
        match ty {
            Type::Primitive(Primitive::Never) => true,
            Type::Seq { item, .. } => Self::mentions_never(item),
            Type::Function { ret, .. } => Self::mentions_never(ret),
            _ => false,
        }
    }

    /// The join of two types: the least type above both.
    ///
    /// <para>The occurrence is split off first and joined by the lattice, so the arms of an `if`
    /// join their item types however they are wrapped: `int` with `int+` is `int+`, `int?` with
    /// `int+` is `int*`, and `{}` — the bottom item type under `?` — with `T` is `T?`, which is
    /// what types a conditional with no `else`. The item types join by the record and component
    /// lineages, which only the inference context knows; the structural join in `semantics`
    /// answers `object` for two item types that are not equal.</para>
    fn common_supertype(&self, lhs: &Type, rhs: &Type) -> Type {
        if lhs.is_error() || rhs.is_error() {
            return Type::Error;
        }
        let (lhs_item, lhs_occ) = lhs.split();
        let (rhs_item, rhs_occ) = rhs.split();
        let item = self.common_item_supertype(lhs_item, rhs_item);
        Type::seq(item, lhs_occ.join(rhs_occ))
    }

    /// The join of two item types; see [`Self::common_supertype`].
    fn common_item_supertype(&self, lhs: &Type, rhs: &Type) -> Type {
        match (lhs, rhs) {
            // The bottom type is the identity of the join: it is below the other side already, so
            // the other side is the least type above both.
            (Type::Primitive(Primitive::Never), other) => other.clone(),
            (other, Type::Primitive(Primitive::Never)) => other.clone(),

            (Type::UnionCase(lhs_case), Type::UnionCase(rhs_case))
                if lhs_case.shares_union_with(rhs_case) =>
            {
                self.union_entry_for(&lhs_case.union, lhs_case.origin())
                    .map(|entry| Type::Union(entry.shape()))
                    .unwrap_or_else(|| common_item_supertype(lhs, rhs))
            }
            (Type::UnionCase(case), Type::Union(union))
            | (Type::Union(union), Type::UnionCase(case))
                if case.is_same_union_as(union) =>
            {
                Type::Union(union.clone())
            }
            // An applied type is invariant: two instantiations are one type exactly when they bind
            // every parameter alike, and two that differ share nothing below `object`, which every
            // applied type satisfies. Neither answer can come from the lineage walks below. A
            // generic record takes no part in inheritance, so the only ancestor they could find is
            // the declaration itself, and they name an ancestor by name alone — which for two
            // `<Box T=int/>` would be a bare `Box`, a spelling NX does not accept as a type and
            // whose fields still carry the declaration's own parameters.
            (Type::Named(lhs_name), Type::Named(rhs_name))
                if !lhs_name.args().is_empty() || !rhs_name.args().is_empty() =>
            {
                if lhs_name == rhs_name {
                    lhs.clone()
                } else {
                    common_item_supertype(lhs, rhs)
                }
            }
            (Type::Named(lhs_name), Type::Named(rhs_name)) => self
                .common_record_supertype(lhs_name, rhs_name)
                .or_else(|| self.common_component_supertype(lhs_name, rhs_name))
                .unwrap_or_else(|| common_item_supertype(lhs, rhs)),
            _ => common_item_supertype(lhs, rhs),
        }
    }

    /// The nearest record both lineages share, compared by declaration rather than by spelling.
    ///
    /// <para>The ancestor is named by name and origin, with no type arguments, which is why the
    /// caller answers a pair where either side is an applied type before reaching here: an
    /// instantiation's ancestor could only be its own declaration, and naming that bare would lose
    /// the arguments.</para>
    fn common_record_supertype(&self, lhs: &NamedType, rhs: &NamedType) -> Option<Type> {
        let lhs_shape = self.record_shape_of(lhs).ok().flatten()?;
        let rhs_shape = self.record_shape_of(rhs).ok().flatten()?;

        let lhs_lineage = record_lineage(&lhs_shape);
        let rhs_lineage = record_lineage(&rhs_shape);

        lhs_lineage
            .into_iter()
            .find(|candidate| {
                rhs_lineage.iter().any(|other| {
                    nx_hir::same_declaration(
                        candidate.origin.as_ref(),
                        &candidate.name,
                        other.origin.as_ref(),
                        &other.name,
                    )
                })
            })
            .map(|ancestor| Type::named_at(ancestor.name, ancestor.origin))
    }

    /// Returns the shape of the record a nominal type denotes.
    ///
    /// A type that carries an origin is read from the declaration it names, so a foreign record
    /// resolves whether or not the asking module can spell it.
    // The `Err` is large for the reason `RecordResolutionError` records.
    #[allow(clippy::result_large_err)]
    fn record_shape_of(
        &self,
        named: &NamedType,
    ) -> Result<Option<nx_hir::EffectiveRecordShape>, nx_hir::RecordResolutionError> {
        match named.origin() {
            Some(origin) => nx_hir::effective_record_shape_at(self.module, origin),
            None => self.effective_record_shape(&named.name),
        }
    }

    /// The nearest component both lineages share, compared by declaration rather than by spelling.
    fn common_component_supertype(&self, lhs: &NamedType, rhs: &NamedType) -> Option<Type> {
        let lhs_contract = self.component_contract_of(lhs).ok().flatten()?;
        let rhs_contract = self.component_contract_of(rhs).ok().flatten()?;

        let lhs_lineage = component_lineage(&lhs_contract);
        let rhs_lineage = component_lineage(&rhs_contract);

        lhs_lineage
            .into_iter()
            .find(|candidate| {
                rhs_lineage.iter().any(|other| {
                    nx_hir::same_declaration(
                        candidate.origin.as_ref(),
                        &candidate.name,
                        other.origin.as_ref(),
                        &other.name,
                    )
                })
            })
            .map(|ancestor| Type::named_at(ancestor.name, ancestor.origin))
    }

    /// Returns the contract of the component a nominal type denotes.
    fn component_contract_of(
        &self,
        named: &NamedType,
    ) -> Result<Option<nx_hir::EffectiveComponentContract>, nx_hir::ComponentResolutionError> {
        match named.origin() {
            Some(origin) => nx_hir::effective_component_contract_at(self.module, origin),
            None => self.effective_component_contract(&named.name),
        }
    }
}

/// High-level type inference entry point.
pub struct TypeInference;

impl TypeInference {
    /// Infers types for all expressions in a module.
    pub fn infer_module(module: &PreparedModule) -> (TypeEnvironment, Vec<Diagnostic>) {
        let ctx = InferenceContext::new(module);

        // TODO: Process all items and their expressions
        // For now, just return empty results

        ctx.finish()
    }
}

/// Whether `text` contains `word` with no identifier character on either side.
fn mentions_word(text: &str, word: &str) -> bool {
    let is_ident = |c: char| c.is_alphanumeric() || c == '_';
    text.match_indices(word).any(|(start, _)| {
        let before = text[..start].chars().next_back();
        let after = text[start + word.len()..].chars().next();
        !before.is_some_and(is_ident) && !after.is_some_and(is_ident)
    })
}

/// The numeric type a value of `member` type widens to when it is joined into `joined`, or `None`
/// when it does not widen.
fn join_widening(member: &Type, joined: &Type) -> Option<Primitive> {
    match (member, joined) {
        (Type::Primitive(from), Type::Primitive(to)) => {
            (from != to && from.is_numeric() && from.widens_to(*to)).then_some(*to)
        }
        // A branch widens to the join's item type whatever the occurrences: in
        // `if c { 1 } else { floats }` the `1` becomes the one-item `float64+` `[1.0]`.
        (Type::Seq { item: member, .. }, Type::Seq { item: joined, .. }) => {
            join_widening(member, joined)
        }
        (member, Type::Seq { item: joined, .. }) => join_widening(member, joined),
        _ => None,
    }
}

/// The type of a value of type `ty` once its numbers are widened to `target`.
pub(crate) fn widened_type(ty: &Type, target: nx_hir::ast::PrimitiveType) -> Type {
    fn widen(ty: &Type, target: Primitive) -> Type {
        match ty {
            Type::Primitive(primitive) if primitive.is_numeric() => Type::Primitive(target),
            Type::Seq { item, occ } => Type::seq(widen(item, target), *occ),
            other => other.clone(),
        }
    }
    widen(ty, Primitive::from_hir_type(target))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nx_diagnostics::{TextSize, TextSpan};
    use nx_hir::{
        ast::BinOp, ast::Expr, ast::Literal, ast::TypeRef, Function, Item, LoweredModule, Name,
        Param, PreparedModule, SourceId, TypeAlias,
    };

    fn prepared(module: &LoweredModule) -> PreparedModule {
        PreparedModule::standalone("test.nx", module.clone())
    }

    #[test]
    fn test_infer_int_literal() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let expr_id = module.alloc_expr(Expr::Literal(Literal::Int(42)));

        let prepared = prepared(&module);
        let mut ctx = InferenceContext::new(&prepared);
        let ty = ctx.infer_expr(expr_id);

        assert_eq!(ty, Type::int());
        assert!(ctx.diagnostics().is_empty());
    }

    #[test]
    fn test_infer_string_literal() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let expr_id = module.alloc_expr(Expr::Literal(Literal::String("hello".into())));

        let prepared = prepared(&module);
        let mut ctx = InferenceContext::new(&prepared);
        let ty = ctx.infer_expr(expr_id);

        assert_eq!(ty, Type::string());
    }

    #[test]
    fn test_infer_bool_literal() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let expr_id = module.alloc_expr(Expr::Literal(Literal::Boolean(true)));

        let prepared = prepared(&module);
        let mut ctx = InferenceContext::new(&prepared);
        let ty = ctx.infer_expr(expr_id);

        assert_eq!(ty, Type::boolean());
    }

    #[test]
    fn test_element_supertype_requires_exact_case() {
        let module = LoweredModule::new(SourceId::new(0));
        let prepared = prepared(&module);
        let ctx = InferenceContext::new(&prepared);

        assert!(ctx.type_satisfies_expected(
            &Type::named(Name::new("div")),
            &Type::named(Name::new("Element"))
        ));
        assert!(!ctx.type_satisfies_expected(
            &Type::named(Name::new("div")),
            &Type::named(Name::new("element"))
        ));
    }

    #[test]
    fn test_converted_literals_records_only_the_literal_that_took_a_float_type() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(0));

        // One literal at a declared float return type, one with nothing expecting anything.
        let converted_body = module.alloc_expr(Expr::Literal(Literal::Int(42)));
        let untouched_body = module.alloc_expr(Expr::Literal(Literal::Int(7)));

        module.add_item(Item::Function(Function {
            name: Name::new("declared"),
            visibility: nx_hir::Visibility::Export,
            form: nx_hir::FunctionForm::Paren,
            params: vec![],
            return_type: Some(TypeRef::name("float64")),
            body: converted_body,
            span,
        }));
        module.add_item(Item::Function(Function {
            name: Name::new("inferred"),
            visibility: nx_hir::Visibility::Export,
            form: nx_hir::FunctionForm::Paren,
            params: vec![],
            return_type: None,
            body: untouched_body,
            span,
        }));

        let prepared = prepared(&module);
        let mut ctx = InferenceContext::new(&prepared);
        for item in module.items() {
            if let Item::Function(func) = item {
                ctx.infer_function(func);
            }
        }

        assert_eq!(
            ctx.converted_literals().get(&converted_body),
            Some(&Primitive::Float64),
            "the literal at the declared float type should be recorded"
        );
        assert!(
            !ctx.converted_literals().contains_key(&untouched_body),
            "a literal with no float expectation should not be recorded"
        );
        assert!(ctx.diagnostics().is_empty());
    }

    #[test]
    fn test_infer_function_parameter_reference() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(0));

        let body = module.alloc_expr(Expr::Ident(Name::new("text")));
        let param = Param::new(Name::new("text"), TypeRef::name("string"), span);

        let function = Function {
            name: Name::new("Button"),
            visibility: nx_hir::Visibility::Export,
            form: nx_hir::FunctionForm::Paren,
            params: vec![param],
            return_type: None,
            body,
            span,
        };

        module.add_item(Item::Function(function));

        let prepared = prepared(&module);
        let mut ctx = InferenceContext::new(&prepared);

        if let Item::Function(func) = &module.items()[0] {
            ctx.infer_function(func);
        } else {
            panic!("Expected function item");
        }

        let (env, diagnostics) = ctx.finish();
        assert!(diagnostics.is_empty());
        let name = Name::new("text");
        assert!(env.lookup(&name).is_none());
    }

    #[test]
    fn test_infers_return_type_for_unannotated_function() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(0));

        let body = module.alloc_expr(Expr::Ident(Name::new("value")));
        let function = Function {
            name: Name::new("identity"),
            visibility: nx_hir::Visibility::Export,
            form: nx_hir::FunctionForm::Paren,
            params: vec![Param::new(Name::new("value"), TypeRef::name("int"), span)],
            return_type: None,
            body,
            span,
        };
        module.add_item(Item::Function(function));

        let prepared = prepared(&module);
        let mut ctx = InferenceContext::new(&prepared);
        if let Item::Function(func) = &module.items()[0] {
            ctx.infer_function(func);
        }

        let (env, diagnostics) = ctx.finish();
        assert!(
            diagnostics.is_empty(),
            "Unexpected diagnostics: {:?}",
            diagnostics
        );

        let func_ty = env
            .lookup(&Name::new("identity"))
            .expect("Function binding should exist");
        match func_ty {
            Type::Function { params, ret } => {
                assert_eq!(params.len(), 1);
                assert_eq!(params[0].ty, Type::int());
                assert_eq!(**ret, Type::int());
            }
            other => panic!("Expected function type, got {:?}", other),
        }
    }

    #[test]
    fn test_infer_paren_function_call() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(0));

        // add(a:int, b:int): int = a + b
        let add_lhs = module.alloc_expr(Expr::Ident(Name::new("a")));
        let add_rhs = module.alloc_expr(Expr::Ident(Name::new("b")));
        let add_body = module.alloc_expr(Expr::BinaryOp {
            lhs: add_lhs,
            op: BinOp::Add,
            rhs: add_rhs,
            span,
        });
        let add_fn = Function {
            name: Name::new("add"),
            visibility: nx_hir::Visibility::Export,
            form: nx_hir::FunctionForm::Paren,
            params: vec![
                Param::new(Name::new("a"), TypeRef::name("int"), span),
                Param::new(Name::new("b"), TypeRef::name("int"), span),
            ],
            return_type: Some(TypeRef::name("int")),
            body: add_body,
            span,
        };
        module.add_item(Item::Function(add_fn));

        // double(value:int): int = add(value, value)
        let double_callee = module.alloc_expr(Expr::Ident(Name::new("add")));
        let double_arg1 = module.alloc_expr(Expr::Ident(Name::new("value")));
        let double_arg2 = module.alloc_expr(Expr::Ident(Name::new("value")));
        let double_body = module.alloc_expr(Expr::Call {
            func: double_callee,
            args: vec![double_arg1, double_arg2],
            span,
        });
        let double_fn = Function {
            name: Name::new("double"),
            visibility: nx_hir::Visibility::Export,
            form: nx_hir::FunctionForm::Paren,
            params: vec![Param::new(Name::new("value"), TypeRef::name("int"), span)],
            return_type: Some(TypeRef::name("int")),
            body: double_body,
            span,
        };
        module.add_item(Item::Function(double_fn));

        // compute(n:int): int = double(add(n, 1))
        let inner_add_callee = module.alloc_expr(Expr::Ident(Name::new("add")));
        let inner_arg_n = module.alloc_expr(Expr::Ident(Name::new("n")));
        let inner_arg_one = module.alloc_expr(Expr::Literal(Literal::Int(1)));
        let inner_call = module.alloc_expr(Expr::Call {
            func: inner_add_callee,
            args: vec![inner_arg_n, inner_arg_one],
            span,
        });
        let outer_callee = module.alloc_expr(Expr::Ident(Name::new("double")));
        let compute_body = module.alloc_expr(Expr::Call {
            func: outer_callee,
            args: vec![inner_call],
            span,
        });
        let compute_fn = Function {
            name: Name::new("compute"),
            visibility: nx_hir::Visibility::Export,
            form: nx_hir::FunctionForm::Paren,
            params: vec![Param::new(Name::new("n"), TypeRef::name("int"), span)],
            return_type: Some(TypeRef::name("int")),
            body: compute_body,
            span,
        };
        module.add_item(Item::Function(compute_fn));

        let prepared = prepared(&module);
        let mut ctx = InferenceContext::new(&prepared);
        for item in module.items() {
            if let Item::Function(func) = item {
                ctx.infer_function(func);
            }
        }

        let (env, diagnostics) = ctx.finish();
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics, got {:?}",
            diagnostics
        );

        let add_ty = env.lookup(&Name::new("add")).expect("add type binding");
        match add_ty {
            Type::Function { params, ret } => {
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].name.as_str(), "a");
                assert_eq!(params[0].ty, Type::int());
                assert_eq!(params[1].name.as_str(), "b");
                assert_eq!(params[1].ty, Type::int());
                assert_eq!(**ret, Type::int());
            }
            _ => panic!("expected function type"),
        }
    }

    #[test]
    fn test_infer_union_case_access() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(0));
        let union_def = UnionDef {
            name: Name::new("Direction"),
            visibility: nx_hir::Visibility::Export,
            base: None,
            cases: vec![
                UnionCaseDef {
                    name: Name::new("north"),
                    fields: Vec::new(),
                    span,
                },
                UnionCaseDef {
                    name: Name::new("south"),
                    fields: Vec::new(),
                    span,
                },
            ],
            property_target: None,
            span,
        };
        module.add_item(Item::Union(union_def));

        let base = module.alloc_expr(Expr::Ident(Name::new("Direction")));
        let expr_id = module.alloc_expr(Expr::Member {
            base,
            member: Name::new("north"),
            span,
        });

        let prepared = prepared(&module);
        let mut ctx = InferenceContext::new(&prepared);
        let ty = ctx.infer_expr(expr_id);

        match ty {
            Type::UnionCase(case_ty) => {
                assert_eq!(case_ty.union.as_str(), "Direction");
                assert_eq!(case_ty.case.as_str(), "north");
            }
            other => panic!("Expected enum type, got {:?}", other),
        }
        assert!(
            ctx.diagnostics().is_empty(),
            "Enum member access should not emit diagnostics"
        );
    }

    #[test]
    fn test_infer_invalid_union_case() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(0));
        let union_def = UnionDef {
            name: Name::new("Status"),
            visibility: nx_hir::Visibility::Export,
            base: None,
            cases: vec![UnionCaseDef {
                name: Name::new("active"),
                fields: Vec::new(),
                span,
            }],
            property_target: None,
            span,
        };
        module.add_item(Item::Union(union_def));

        let base = module.alloc_expr(Expr::Ident(Name::new("Status")));
        let expr_id = module.alloc_expr(Expr::Member {
            base,
            member: Name::new("pending_review"),
            span,
        });

        let prepared = prepared(&module);
        let mut ctx = InferenceContext::new(&prepared);
        let ty = ctx.infer_expr(expr_id);

        assert!(ty.is_error());
        assert_eq!(ctx.diagnostics().len(), 1);
    }

    #[test]
    fn test_union_case_access_via_alias() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(0));
        let union_def = UnionDef {
            name: Name::new("Status"),
            visibility: nx_hir::Visibility::Export,
            base: None,
            cases: vec![UnionCaseDef {
                name: Name::new("active"),
                fields: Vec::new(),
                span,
            }],
            property_target: None,
            span,
        };
        module.add_item(Item::Union(union_def));
        let alias = TypeAlias {
            name: Name::new("State"),
            visibility: nx_hir::Visibility::Export,
            ty: ast::TypeRef::name("Status"),
            span,
        };
        module.add_item(Item::TypeAlias(alias));

        let base = module.alloc_expr(Expr::Ident(Name::new("State")));
        let expr_id = module.alloc_expr(Expr::Member {
            base,
            member: Name::new("active"),
            span,
        });

        let prepared = prepared(&module);
        let mut ctx = InferenceContext::new(&prepared);
        let ty = ctx.infer_expr(expr_id);

        match ty {
            Type::UnionCase(case_ty) => assert_eq!(case_ty.union.as_str(), "Status"),
            other => panic!("Expected enum type, got {:?}", other),
        }
        assert!(ctx.diagnostics().is_empty());
    }

    #[test]
    fn test_function_signature_uses_union_type() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(0));
        let union_def = UnionDef {
            name: Name::new("Direction"),
            visibility: nx_hir::Visibility::Export,
            base: None,
            cases: vec![UnionCaseDef {
                name: Name::new("north"),
                fields: Vec::new(),
                span,
            }],
            property_target: None,
            span,
        };
        module.add_item(Item::Union(union_def));

        let base = module.alloc_expr(Expr::Ident(Name::new("Direction")));
        let member = module.alloc_expr(Expr::Member {
            base,
            member: Name::new("north"),
            span,
        });
        let func = Function {
            name: Name::new("north"),
            visibility: nx_hir::Visibility::Export,
            form: nx_hir::FunctionForm::Paren,
            params: vec![],
            return_type: None,
            body: member,
            span,
        };
        module.add_item(Item::Function(func));

        let prepared = prepared(&module);
        let mut ctx = InferenceContext::new(&prepared);
        if let Item::Function(func) = &module.items()[1] {
            ctx.infer_function(func);
        }
        let (env, diagnostics) = ctx.finish();
        assert!(diagnostics.is_empty());

        let func_ty = env.lookup(&Name::new("north")).expect("function type");
        match func_ty {
            Type::Function { ret, .. } => match ret.as_ref() {
                Type::UnionCase(case_ty) => assert_eq!(case_ty.union.as_str(), "Direction"),
                other => panic!("Expected enum return type, got {:?}", other),
            },
            other => panic!("Expected function type, got {:?}", other),
        }
    }
}
