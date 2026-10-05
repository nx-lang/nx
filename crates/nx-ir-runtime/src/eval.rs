//! Evaluation over the node table.

use crate::error::{Diagnostic, Limit, NxIrRuntimeError, Result, SourceSpan, RESOURCE_LIMIT};
use crate::module::{
    BinaryOp, CaseConstruct, Construct, DeclarationKind, ElementNode, Field, ForNode, FunctionDecl,
    HandlerNode, Intrinsic, Node, Param, Properties, Ref, TextType, UnaryOp,
};
use crate::normalize::{integral, require_record, Labeled, Path};
use crate::program::ProgramData;
use crate::text::{float32_text, float64_text};
use crate::update;
use crate::usage::Usage;
use crate::value::{
    get_field, set_field, values_equal, CaseValue, Fields, FunctionRef, Handler, Value,
};
pub(crate) use meter::Meter;
use std::cell::Cell;
use std::fmt;
use std::sync::Arc;

/// The limits an evaluation runs under, and where it reports what it used.
#[derive(Debug, Clone)]
pub struct RuntimeOptions {
    /// How deeply function calls may nest. 100 by default.
    pub max_call_depth: u32,
    /// The most integers one range may hold when a loop iterates it. One million by default.
    ///
    /// <para>A range makes an enormous loop one token long, so the count is checked before the
    /// body runs at all rather than discovered part-way through.</para>
    pub max_range_length: u64,
    /// The most operations one call may cost, counted as `docs/nx-ir-format.md` defines an
    /// operation. `None`, the default, is unlimited.
    ///
    /// <para>One budget covers one call of an evaluation method and everything it evaluates: for
    /// [`Program::dispatch_component_actions`](crate::Program::dispatch_component_actions), every
    /// handler of the batch and the render after it. A host that evaluates code it did not write
    /// should set it: the call depth and the range length bound neither work nor allocation.</para>
    pub max_operations: Option<u64>,
    /// The largest input one call may be given, measured as `docs/nx-ir-format.md` defines the
    /// input size of a call and as [`input_size`](crate::input_size) measures one value. `None`,
    /// the default, is unlimited, and then nothing is measured.
    ///
    /// <para>It covers every value the host passes to one call: arguments, props, content, a
    /// state, the entries of a batch and a state patch. An instance is not input. A call whose
    /// input is larger fails with `nx-ir-resource-limit` naming `maxInputSize` before the program
    /// is looked at, and measuring stops as soon as the size passes the limit. The limit is
    /// separate from the budget: measuring charges no operation.</para>
    pub max_input_size: Option<u64>,
    /// Where a call reports what it used, when the host wants to know. `None` by default.
    ///
    /// <para>The runtime clears the report when a call begins and fills it when the call ends,
    /// whether it succeeds or fails: the operations when [`max_operations`](Self::max_operations)
    /// is set, and the input size when [`max_input_size`](Self::max_input_size) is set and the
    /// input was within it. Nothing is counted for the report alone.</para>
    pub usage: Option<Arc<Usage>>,
}

/// The default of [`RuntimeOptions::max_call_depth`].
pub const NX_DEFAULT_MAX_CALL_DEPTH: u32 = 100;
/// The default of [`RuntimeOptions::max_range_length`].
pub const NX_DEFAULT_MAX_RANGE_LENGTH: u64 = 1_000_000;

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            max_call_depth: NX_DEFAULT_MAX_CALL_DEPTH,
            max_range_length: NX_DEFAULT_MAX_RANGE_LENGTH,
            max_operations: None,
            max_input_size: None,
            usage: None,
        }
    }
}

/// How deeply expressions may nest across every call of one evaluation.
///
/// <para>The evaluator recurses on the native stack, and a native stack overflow aborts the
/// process rather than returning an error. The call-depth limit is the host's to raise, so it
/// cannot be what protects the stack; this bound is fixed and counts every nested node, so no
/// image and no option reaches the end of the stack.</para>
pub(crate) const MAX_NESTING: u32 = 1000;

/// How much native stack one evaluation may use, measured from where it began.
///
/// <para>A count of nested nodes bounds the stack only as well as the size of a frame is known,
/// and an unoptimized build's frames are several times an optimized one's. So the stack actually
/// used is checked too, and whichever limit is met first ends the evaluation with a diagnostic.
/// A thread that runs the evaluator needs this much stack free.</para>
const MAX_STACK_BYTES: usize = 1 << 20;

/// The limit a diagnostic names when an evaluation ran out of native stack.
pub(crate) const STACK_LIMIT: Limit = Limit {
    name: "maxStackBytes",
    value: Some(MAX_STACK_BYTES as u64),
};

/// The limit a diagnostic names when expressions nested too deeply.
const NESTING_LIMIT: Limit = Limit {
    name: "maxExpressionNesting",
    value: Some(MAX_NESTING as u64),
};

/// Roughly where the native stack is now: the address of a local of the caller's frame.
#[inline(never)]
fn stack_position() -> usize {
    let marker = 0u8;
    std::ptr::from_ref(&marker) as usize
}

/// The most local slots one frame may hold. A slot is a cell of the image, so an altered image
/// can name any; a frame is allocated to its highest slot, so the highest is bounded.
const MAX_SLOTS: usize = 1 << 16;

/// Where evaluation is: the module whose node table the indices belong to, the declaration they
/// belong to, and how many calls deep it is.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Cx {
    pub module: u32,
    pub declaration: u32,
    pub depth: u32,
}

/// A declaration's locals, by slot. A slot nothing has bound is `None`.
pub(crate) type Frame = Vec<Option<Value>>;

/// Binds a slot, growing the frame to reach it.
pub(crate) fn bind(frame: &mut Frame, slot: usize, value: Value) -> Result<()> {
    if slot >= MAX_SLOTS {
        return crate::error::fail(
            "nx-ir-slot",
            format!("Local slot {slot} is past the {MAX_SLOTS} slots a frame may hold."),
        );
    }
    if frame.len() <= slot {
        frame.resize(slot.saturating_add(1), None);
    }
    if let Some(entry) = frame.get_mut(slot) {
        *entry = Some(value);
    }
    Ok(())
}

/// Who made a call of a function: a host, through `evaluate_function` or `call_function`, or the
/// program, from a body or a default. A failure in an argument of the host's call names the
/// argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Caller {
    Host,
    Program,
}

/// Whether a diagnostic with this code is about a value passed for a parameter: a boundary
/// failure, a missing required argument or a `Function` record that names no function.
fn is_about_argument(code: &str) -> bool {
    code == "nx-ir-arguments"
        || code == "nx-ir-function-value"
        || code.starts_with("nx-ir-boundary-")
}

/// One evaluation against one program under one set of limits.
pub(crate) struct Machine<'p> {
    pub program: &'p ProgramData,
    pub options: &'p RuntimeOptions,
    nesting: Cell<u32>,
    /// The operations the budget has left: `u64::MAX` when the host set none.
    remaining: Cell<u64>,
    /// Where the native stack stood when the evaluation began.
    stack_base: usize,
    /// Whether a default failed, by its expression or by its value not fitting its type. A
    /// failure ends the evaluation, so once this is set the failure that leaves is that one, and
    /// it is not in an argument even when the default was filled in while an argument was checked.
    default_failed: Cell<bool>,
}

impl<'p> Machine<'p> {
    pub(crate) fn new(program: &'p ProgramData, options: &'p RuntimeOptions) -> Self {
        Self {
            program,
            options,
            nesting: Cell::new(0),
            remaining: Cell::new(options.max_operations.unwrap_or(u64::MAX)),
            stack_base: stack_position(),
            default_failed: Cell::new(false),
        }
    }

    /// Records that a default failed, on the way out with its failure.
    pub(crate) fn default_failed(&self, error: NxIrRuntimeError) -> NxIrRuntimeError {
        self.default_failed.set(true);
        error
    }

    /// The failure of the host's call in the binding of `argument`, with the argument named on
    /// each diagnostic that is about the value the host passed. A default that failed while the
    /// value was checked is not about that value whatever its code, and neither is a limit or a
    /// failure of the image, which have other codes.
    #[cold]
    fn naming_argument(&self, mut error: NxIrRuntimeError, argument: &str) -> NxIrRuntimeError {
        if !self.default_failed.get() {
            for diagnostic in &mut error.diagnostics {
                if is_about_argument(diagnostic.code) {
                    diagnostic.argument = Some(argument.to_string());
                }
            }
        }
        error
    }

    /// A diagnostic that names the declaration evaluation is in and, when the image carries its
    /// debug section, the span of the node.
    pub(crate) fn error(
        &self,
        cx: Option<Cx>,
        node: Option<u32>,
        code: &'static str,
        message: impl Into<String>,
    ) -> NxIrRuntimeError {
        let mut diagnostic = Diagnostic::new(code, message);
        if let Some(linked) = cx.and_then(|cx| self.program.linked(cx.module)) {
            let module = &linked.module;
            let name = cx
                .and_then(|cx| module.declaration(cx.declaration))
                .map(|declaration| &*declaration.name)
                .unwrap_or("");
            diagnostic.declaration = Some(format!("{}::{name}", module.identity));
            diagnostic.source =
                node.and_then(|node| module.image().node_span(node))
                    .map(|(start, end)| SourceSpan {
                        identity: module.identity.to_string(),
                        start,
                        end,
                    });
        }
        NxIrRuntimeError {
            diagnostics: vec![diagnostic],
        }
    }

    fn fail<T>(
        &self,
        cx: Cx,
        node: u32,
        code: &'static str,
        message: impl Into<String>,
    ) -> Result<T> {
        Err(self.error(Some(cx), Some(node), code, message))
    }

    fn node(&self, cx: Cx, index: u32) -> Result<&'p Node> {
        let linked = self.program.linked(cx.module).ok_or_else(|| {
            self.error(
                None,
                None,
                "nx-ir-reference",
                "The program has no such module.",
            )
        })?;
        linked.module.node(index).map_err(|message| {
            self.error(
                Some(cx),
                None,
                "nx-ir-malformed",
                format!("NX IR image is malformed: {message}."),
            )
        })
    }

    fn resolve(
        &self,
        cx: Cx,
        node: u32,
        reference: &Ref,
    ) -> Result<(u32, u32, &'p crate::module::Declaration)> {
        self.program
            .resolve(cx.module, reference)
            .map_err(|message| self.error(Some(cx), Some(node), "nx-ir-reference", message))
    }

    /// Evaluates node `index`.
    ///
    /// <para>This and `eval_node` are the frames every level of nesting pays for, so they hold
    /// nothing but the dispatch: each kind's work, and every diagnostic's formatting, is in a
    /// function of its own.</para>
    ///
    /// <para>The node costs one operation, charged before anything else, and nests one level
    /// deeper. Every limit is tested in one branch, so the evaluation that meets none pays for
    /// one; which limit was met is the cold path's to work out.</para>
    pub(crate) fn eval(&self, cx: Cx, frame: &mut Frame, index: u32) -> Result<Value> {
        let remaining = self.remaining.get();
        let nesting = self.nesting.get();
        if remaining == 0
            || nesting >= MAX_NESTING
            || stack_position().abs_diff(self.stack_base) > MAX_STACK_BYTES
        {
            return self.limit_reached(cx, index);
        }
        self.remaining.set(remaining.wrapping_sub(1));
        self.nesting.set(nesting.wrapping_add(1));
        let result = self.eval_node(cx, frame, index);
        self.nesting.set(nesting);
        result
    }

    /// The failure of an evaluation that met a limit before its node: the budget, which is
    /// charged first, then the nesting count, then the stack.
    #[cold]
    #[inline(never)]
    fn limit_reached(&self, cx: Cx, index: u32) -> Result<Value> {
        if self.remaining.get() == 0 {
            self.budget_exhausted(cx, Some(index))?;
        }
        if self.nesting.get() >= MAX_NESTING {
            return self.nesting_exceeded(cx, index);
        }
        self.stack_exceeded(cx, index)
    }

    /// The operations charged so far, when the host set a budget; with none, nothing is counted.
    /// A charge the budget refused was not made, so it is not among them.
    pub(crate) fn used(&self) -> Option<u64> {
        self.options
            .max_operations
            .map(|budget| budget.saturating_sub(self.remaining.get()))
    }

    /// Whether the host set a budget. A charge whose amount takes work to find is not measured
    /// when it did not: the charge cannot fail and nothing reads the count.
    pub(crate) fn is_limited(&self) -> bool {
        self.options.max_operations.is_some()
    }

    /// Who pays for a walk over a value made on behalf of node `node` of the declaration of `cx`,
    /// or of that declaration alone when the walk belongs to no node. With no budget nobody
    /// does: the charges could not fail and nothing reads the count, so the walk skips them.
    pub(crate) fn meter(&self, cx: Cx, node: Option<u32>) -> Meter<'_, 'p> {
        if self.is_limited() {
            Meter::charged(self, cx, node)
        } else {
            Meter::free()
        }
    }

    /// Whether the evaluation is still within its share of the native stack. The walks that
    /// recurse over a value rather than over nodes ask this.
    pub(crate) fn within_stack(&self) -> bool {
        stack_position().abs_diff(self.stack_base) <= MAX_STACK_BYTES
    }

    /// Charges `amount` operations, failing before the operation is performed when the budget
    /// does not cover it. The failure names the declaration `cx` is in and, when the charge is
    /// for a node, that node's span.
    #[inline]
    pub(crate) fn charge(&self, cx: Cx, node: Option<u32>, amount: u64) -> Result<()> {
        let remaining = self.remaining.get();
        if remaining < amount {
            return self.budget_exhausted(cx, node);
        }
        self.remaining.set(remaining.wrapping_sub(amount));
        Ok(())
    }

    #[cold]
    #[inline(never)]
    fn budget_exhausted(&self, cx: Cx, node: Option<u32>) -> Result<()> {
        let budget = self.options.max_operations.unwrap_or(u64::MAX);
        self.fail_limit(
            cx,
            node,
            Limit {
                name: "maxOperations",
                value: Some(budget),
            },
            format!("The evaluation exceeded its budget of {budget} operations."),
        )
    }

    #[cold]
    #[inline(never)]
    fn nesting_exceeded(&self, cx: Cx, index: u32) -> Result<Value> {
        self.fail_limit(
            cx,
            Some(index),
            NESTING_LIMIT,
            format!("Maximum NX IR expression nesting was exceeded: expressions nest more than {MAX_NESTING} deep."),
        )
    }

    #[cold]
    #[inline(never)]
    fn stack_exceeded(&self, cx: Cx, index: u32) -> Result<Value> {
        self.fail_limit(
            cx,
            Some(index),
            STACK_LIMIT,
            format!("Maximum NX IR expression nesting was exceeded: evaluation used more than {MAX_STACK_BYTES} bytes of stack."),
        )
    }

    /// Fails with an `nx-ir-resource-limit` diagnostic in the declaration of `cx`, at node `node`
    /// when the limit was met at one, that carries `limit`.
    fn fail_limit<T>(&self, cx: Cx, node: Option<u32>, limit: Limit, message: String) -> Result<T> {
        let mut error = self.error(Some(cx), node, RESOURCE_LIMIT, message);
        if let Some(diagnostic) = error.diagnostics.first_mut() {
            diagnostic.limit = Some(limit);
        }
        Err(error)
    }

    fn eval_node(&self, cx: Cx, frame: &mut Frame, index: u32) -> Result<Value> {
        match self.node(cx, index)? {
            Node::Bool(value) => Ok(Value::Bool(*value)),
            Node::String(text) => Ok(Value::Str(Arc::clone(text))),
            Node::Number(value) => Ok(value.clone()),
            Node::Slot { slot, name } => match frame.get(*slot as usize) {
                Some(Some(value)) => Ok(value.clone()),
                _ => self.unbound(cx, index, name),
            },
            Node::Reference(reference) => self.eval_reference(cx, index, reference),
            Node::Binary { op, lhs, rhs } => self.eval_binary(cx, frame, index, *op, *lhs, *rhs),
            Node::Unary { op, operand } => self.eval_unary(cx, frame, index, *op, *operand),
            Node::Text { operand, ty } => self.eval_text(cx, frame, index, *operand, *ty),
            Node::Call { callee, args } => self.eval_call(cx, frame, index, *callee, args),
            Node::NamedCall { callee, args } => {
                self.eval_named_call(cx, frame, index, *callee, args)
            }
            Node::Intrinsic {
                intrinsic,
                args,
                names,
            } => self.eval_intrinsic(cx, frame, index, *intrinsic, args, names),
            Node::If {
                condition,
                then,
                otherwise,
            } => self.eval_if(cx, frame, index, *condition, *then, *otherwise),
            Node::IfIs {
                scrutinee,
                arms,
                otherwise,
            } => self.eval_if_is(cx, frame, *scrutinee, arms, *otherwise),
            Node::Array(items) => self.eval_array(cx, frame, index, items),
            Node::For(node) => self.eval_for(cx, frame, index, node),
            Node::Member {
                optional,
                base,
                name,
            } => self.eval_member(cx, frame, index, *optional, *base, name),
            Node::Exists(operand) => self.eval_exists(cx, frame, *operand),
            Node::Coalesce(left, right) => self.eval_coalesce(cx, frame, *left, *right),
            Node::Record(construct) => self.eval_record(cx, frame, index, construct),
            Node::UnionCase(construct) => self.eval_union_case(cx, frame, index, construct),
            Node::Element(element) => self.eval_element(cx, frame, index, element),
            Node::Component(construct) => self.eval_component(cx, frame, index, construct),
            Node::ActionHandler(handler) => self.eval_handler(cx, frame, index, handler),
        }
    }

    #[cold]
    #[inline(never)]
    fn unbound(&self, cx: Cx, index: u32, name: &str) -> Result<Value> {
        self.fail(
            cx,
            index,
            "nx-ir-slot",
            format!("Local slot '{name}' was not bound."),
        )
    }

    #[inline(never)]
    fn eval_unary(
        &self,
        cx: Cx,
        frame: &mut Frame,
        index: u32,
        op: UnaryOp,
        operand: u32,
    ) -> Result<Value> {
        let operand = self.eval(cx, frame, operand)?;
        match (op, operand) {
            (UnaryOp::Neg, Value::Int(value)) => Ok(Value::Int(value.wrapping_neg())),
            (UnaryOp::Neg, Value::Float(value)) => Ok(Value::Float(-value)),
            (UnaryOp::Neg, _) => self.fail(
                cx,
                index,
                "nx-ir-number",
                "Operator 'neg' requires a numeric operand.",
            ),
            (UnaryOp::Not, operand) => Ok(Value::Bool(!self.boolean(cx, index, &operand, "not")?)),
        }
    }

    #[inline(never)]
    fn eval_text(
        &self,
        cx: Cx,
        frame: &mut Frame,
        index: u32,
        operand: u32,
        ty: TextType,
    ) -> Result<Value> {
        let operand = self.eval(cx, frame, operand)?;
        self.text(cx, index, &operand, ty)
    }

    /// A missing `else` is an `else { }`, so a conditional that takes no branch is the empty
    /// value: it splices away where items are collected, and is what a type that admits zero
    /// promises where one value is expected.
    #[inline(never)]
    fn eval_if(
        &self,
        cx: Cx,
        frame: &mut Frame,
        index: u32,
        condition: u32,
        then: u32,
        otherwise: Option<u32>,
    ) -> Result<Value> {
        let condition = self.eval(cx, frame, condition)?;
        let branch = if self.boolean(cx, index, &condition, "if condition")? {
            Some(then)
        } else {
            otherwise
        };
        drop(condition);
        match branch {
            Some(branch) => self.eval(cx, frame, branch),
            None => Ok(Value::empty()),
        }
    }

    /// The arm an `if … is` takes, or its `else`.
    #[inline(never)]
    fn taken_arm(
        &self,
        cx: Cx,
        frame: &mut Frame,
        scrutinee: u32,
        arms: &[crate::module::Arm],
        otherwise: Option<u32>,
    ) -> Result<Option<u32>> {
        let scrutinee = self.eval(cx, frame, scrutinee)?;
        for arm in arms {
            for pattern in arm.patterns.iter() {
                let node = *pattern;
                let pattern = self.eval_pattern(cx, frame, node)?;
                if pattern_matches(&scrutinee, &pattern, &self.meter(cx, Some(node)))? {
                    return Ok(Some(arm.body));
                }
            }
        }
        Ok(otherwise)
    }

    #[inline(never)]
    fn eval_if_is(
        &self,
        cx: Cx,
        frame: &mut Frame,
        scrutinee: u32,
        arms: &[crate::module::Arm],
        otherwise: Option<u32>,
    ) -> Result<Value> {
        match self.taken_arm(cx, frame, scrutinee, arms, otherwise)? {
            Some(branch) => self.eval(cx, frame, branch),
            None => Ok(Value::empty()),
        }
    }

    #[inline(never)]
    fn eval_array(&self, cx: Cx, frame: &mut Frame, node: u32, items: &[u32]) -> Result<Value> {
        Ok(Value::seq(self.items(cx, frame, node, items)?))
    }

    #[inline(never)]
    fn eval_member(
        &self,
        cx: Cx,
        frame: &mut Frame,
        index: u32,
        optional: bool,
        base: u32,
        name: &str,
    ) -> Result<Value> {
        let base = self.eval(cx, frame, base)?;
        if !optional {
            return self.read_member(cx, index, &base, name);
        }
        // `x?.m`: the receiver is evaluated once; when it is empty so is the result.
        match optional_item(&base) {
            None => Ok(Value::empty()),
            Some(receiver) => self.read_member(cx, index, receiver, name),
        }
    }

    #[inline(never)]
    fn eval_exists(&self, cx: Cx, frame: &mut Frame, operand: u32) -> Result<Value> {
        Ok(Value::Bool(!self.eval(cx, frame, operand)?.is_empty()))
    }

    /// `x ?? y`: the right operand runs only when the left is empty.
    #[inline(never)]
    fn eval_coalesce(&self, cx: Cx, frame: &mut Frame, left: u32, right: u32) -> Result<Value> {
        let left = self.eval(cx, frame, left)?;
        if left.is_empty() {
            self.eval(cx, frame, right)
        } else {
            Ok(left)
        }
    }

    /// Evaluates nodes as the items of one sequence.
    ///
    /// <para>A list value contributes its elements and every other value contributes itself.
    /// That is the whole rule: a conditional that takes no branch evaluates to the empty value,
    /// which is the empty sequence, so it splices away like any other list-valued item.</para>
    ///
    /// <para>`node` is the node building the sequence, which pays for every item placed.</para>
    fn items(&self, cx: Cx, frame: &mut Frame, node: u32, nodes: &[u32]) -> Result<Vec<Value>> {
        let mut items = Vec::with_capacity(nodes.len());
        for item in nodes {
            let value = self.eval(cx, frame, *item)?;
            self.place(cx, node, &mut items, value)?;
        }
        Ok(items)
    }

    /// Appends what one item contributes to the sequence node `node` builds, charging one
    /// operation per item placed before placing any.
    #[inline]
    fn place(&self, cx: Cx, node: u32, items: &mut Vec<Value>, value: Value) -> Result<()> {
        match value {
            Value::Seq(elements) => {
                self.charge(cx, Some(node), elements.len() as u64)?;
                items.extend(elements.iter().cloned());
            }
            other => {
                self.charge(cx, Some(node), 1)?;
                items.push(other);
            }
        }
        Ok(())
    }

    /// Evaluates written properties into fields, in the order they were written.
    fn properties(&self, cx: Cx, frame: &mut Frame, properties: &Properties) -> Result<Fields> {
        let mut fields = Vec::with_capacity(properties.len());
        for (name, node) in properties.iter() {
            let value = self.eval(cx, frame, *node)?;
            set_field(&mut fields, Arc::clone(name), value);
        }
        Ok(fields)
    }

    fn boolean(&self, cx: Cx, node: u32, value: &Value, what: &str) -> Result<bool> {
        match value {
            Value::Bool(value) => Ok(*value),
            other => self.fail(
                cx,
                node,
                "nx-ir-type",
                format!("Expected a boolean for '{what}', got {}.", other.describe()),
            ),
        }
    }

    #[inline(never)]
    fn eval_reference(&self, cx: Cx, node: u32, reference: &Ref) -> Result<Value> {
        let (module, index, declaration) = self.resolve(cx, node, reference)?;
        match &declaration.kind {
            DeclarationKind::Function(_) => Ok(Value::Function(Arc::new(FunctionRef {
                module: self.program.identity(module),
                name: Arc::clone(&declaration.name),
            }))),
            DeclarationKind::Value { value, ty } => {
                let declared = Cx {
                    module,
                    declaration: index,
                    depth: cx.depth,
                };
                let result = self.eval(declared, &mut Vec::new(), *value)?;
                match ty {
                    None => Ok(result),
                    Some(ty) => self.normalize(
                        declared,
                        ty,
                        result,
                        &Path::Root(&Quoted("value '", &declaration.name, "'")),
                    ),
                }
            }
            _ => self.fail(
                cx,
                node,
                "nx-ir-reference",
                format!(
                    "Declaration '{}' cannot be used as a value.",
                    reference.name
                ),
            ),
        }
    }

    #[inline(never)]
    fn eval_binary(
        &self,
        cx: Cx,
        frame: &mut Frame,
        node: u32,
        op: BinaryOp,
        lhs: u32,
        rhs: u32,
    ) -> Result<Value> {
        // `and` and `or` are the only non-strict operators: the left operand decides whether the
        // right one runs at all, so a guard such as `d != 0 && n / d > 1` never divides by zero.
        if matches!(op, BinaryOp::And | BinaryOp::Or) {
            let left = self.eval(cx, frame, lhs)?;
            let left = self.boolean(cx, node, &left, op.name())?;
            if left == (op == BinaryOp::Or) {
                return Ok(Value::Bool(left));
            }
            let right = self.eval(cx, frame, rhs)?;
            return Ok(Value::Bool(self.boolean(cx, node, &right, op.name())?));
        }
        let left = self.eval(cx, frame, lhs)?;
        let right = self.eval(cx, frame, rhs)?;
        if op == BinaryOp::Concat {
            self.charge_concat(cx, node, &left, &right)?;
        }
        // Equality walks its operands, and pays for each pair of values it compares.
        if matches!(op, BinaryOp::Eq | BinaryOp::Ne) {
            let equal = values_equal(&left, &right, &self.meter(cx, Some(node)))?;
            return Ok(Value::Bool(equal == (op == BinaryOp::Eq)));
        }
        match binary(op, &left, &right) {
            Ok(value) => Ok(value),
            Err(BinaryError::NotNumeric) => self.fail(
                cx,
                node,
                "nx-ir-number",
                format!("Operator '{}' requires numeric operands.", op.name()),
            ),
            Err(BinaryError::NotInteger) => self.fail(
                cx,
                node,
                "nx-ir-number",
                format!(
                    "Operator '{}' requires integer operands for integer results.",
                    op.name()
                ),
            ),
            Err(BinaryError::DivisionByZero) => {
                self.fail(cx, node, "nx-ir-division-by-zero", "Division by zero.")
            }
            Err(BinaryError::NotStrings) => self.fail(
                cx,
                node,
                "nx-ir-operator",
                "Operator 'concat' requires string operands.",
            ),
        }
    }

    /// Charges a `concat` one operation per 64 UTF-16 code units of the string it produces,
    /// before the string is allocated. Operands that are not both strings produce no string.
    ///
    /// <para>With no budget the charge cannot fail and nothing reads the count, so the operands
    /// are not measured at all: a host that sets none pays nothing here.</para>
    fn charge_concat(&self, cx: Cx, node: u32, left: &Value, right: &Value) -> Result<()> {
        if !self.is_limited() {
            return Ok(());
        }
        let (Some(left), Some(right)) = (left.as_text(), right.as_text()) else {
            return Ok(());
        };
        // A string has no more UTF-16 code units than UTF-8 bytes, so a short one is not counted.
        if left.len().saturating_add(right.len()) < TEXT_UNITS {
            return Ok(());
        }
        let units = utf16_len(left).saturating_add(utf16_len(right));
        self.charge(cx, Some(node), (units / TEXT_UNITS) as u64)
    }

    /// A `text` node: the canonical text form of a primitive value, by the static type the node
    /// names.
    fn text(&self, cx: Cx, node: u32, value: &Value, ty: TextType) -> Result<Value> {
        let text = match (ty, value) {
            (TextType::Boolean, Value::Bool(value)) => value.to_string(),
            (TextType::Float32, Value::Float(value)) => float32_text(*value),
            (TextType::Float32, Value::Int(value)) => float32_text(*value as f64),
            (TextType::Float64, Value::Float(value)) => float64_text(*value),
            (TextType::Int, Value::Float(value)) => float64_text(*value),
            (TextType::Float64 | TextType::Int, Value::Int(value)) => value.to_string(),
            _ => {
                let name = match ty {
                    TextType::Boolean => "boolean",
                    TextType::Float32 => "float32",
                    TextType::Float64 => "float64",
                    TextType::Int => "int",
                };
                return self.fail(
                    cx,
                    node,
                    "nx-ir-operator",
                    format!(
                        "A text conversion from '{name}' cannot render {}.",
                        value.describe()
                    ),
                );
            }
        };
        Ok(Value::Str(Arc::from(text)))
    }

    /// The function a callee node evaluates to.
    fn callee(
        &self,
        cx: Cx,
        frame: &mut Frame,
        node: u32,
        callee: u32,
        what: &str,
    ) -> Result<(u32, u32)> {
        // A callee written as a reference is resolved where it stands, without building a value,
        // and costs what evaluating it would have: one operation.
        if let Node::Reference(reference) = self.node(cx, callee)? {
            let (module, index, declaration) = self.resolve(cx, callee, reference)?;
            if matches!(declaration.kind, DeclarationKind::Function(_)) {
                self.charge(cx, Some(callee), 1)?;
                return Ok((module, index));
            }
        }
        match self.eval(cx, frame, callee)? {
            Value::Function(function) => self
                .resolve_function(&function, &"A call")
                .map_err(|error| self.error(Some(cx), Some(node), "nx-ir-call", error.to_string())),
            _ => self.fail(
                cx,
                node,
                "nx-ir-call",
                format!("NX IR {what} callee did not evaluate to a function value."),
            ),
        }
    }

    #[inline(never)]
    fn eval_call(
        &self,
        cx: Cx,
        frame: &mut Frame,
        node: u32,
        callee: u32,
        args: &[Option<u32>],
    ) -> Result<Value> {
        let (module, index) = self.callee(cx, frame, node, callee, "call")?;
        // An argument the call left out is passed on as `None` for the function to fill.
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            values.push(match arg {
                Some(arg) => Some(self.eval(cx, frame, *arg)?),
                None => None,
            });
        }
        self.invoke(
            module,
            index,
            values,
            cx.depth.saturating_add(1),
            Caller::Program,
        )
    }

    /// A call of a function-typed value by name: the arguments are bound to that function's
    /// parameters under the subset rule.
    #[inline(never)]
    fn eval_named_call(
        &self,
        cx: Cx,
        frame: &mut Frame,
        node: u32,
        callee: u32,
        args: &Properties,
    ) -> Result<Value> {
        let (module, index) = self.callee(cx, frame, node, callee, "named call")?;
        let args = self.properties(cx, frame, args)?;
        self.invoke_by_name(
            module,
            index,
            &args,
            cx.depth.saturating_add(1),
            Caller::Program,
        )
    }

    fn function(&self, module: u32, index: u32) -> Result<(&'p Arc<str>, &'p FunctionDecl)> {
        match self
            .program
            .linked(module)
            .and_then(|linked| linked.module.declaration(index))
        {
            Some(declaration) => match &declaration.kind {
                DeclarationKind::Function(function) => Ok((&declaration.name, function)),
                _ => crate::error::fail(
                    "nx-ir-call",
                    format!("'{}' is not a function.", declaration.name),
                ),
            },
            None => crate::error::fail("nx-ir-call", "The call names no declaration."),
        }
    }

    /// Invokes a function with arguments by name. The caller supplied every parameter of the
    /// function type it holds, so an argument the declaration does not name is dropped, and a
    /// parameter the declaration names must be present unless the function can fill it.
    pub(crate) fn invoke_by_name(
        &self,
        module: u32,
        index: u32,
        args: &[(Arc<str>, Value)],
        depth: u32,
        caller: Caller,
    ) -> Result<Value> {
        let (_, function) = self.function(module, index)?;
        let positional = function
            .params
            .iter()
            .map(|param| get_field(args, &param.name).cloned())
            .collect();
        self.invoke(module, index, positional, depth, caller)
    }

    /// Calls a function with its arguments by position. An argument that is `None`, or past the
    /// end of `args`, was left out: the function fills its parameter with the default it
    /// declares, evaluated here after the parameters before it, or with empty when the parameter
    /// is optional.
    pub(crate) fn invoke(
        &self,
        module: u32,
        index: u32,
        args: Vec<Option<Value>>,
        depth: u32,
        caller: Caller,
    ) -> Result<Value> {
        if depth > self.options.max_call_depth {
            return crate::error::fail_limit(
                Limit {
                    name: "maxCallDepth",
                    value: Some(u64::from(self.options.max_call_depth)),
                },
                format!(
                    "Maximum NX IR call depth {} was exceeded.",
                    self.options.max_call_depth
                ),
            );
        }
        let (name, function) = self.function(module, index)?;
        if args.len() > function.params.len() {
            return crate::error::fail(
                "nx-ir-arguments",
                format!(
                    "Function '{name}' expected at most {} arguments, got {}.",
                    function.params.len(),
                    args.len()
                ),
            );
        }
        let cx = Cx {
            module,
            declaration: index,
            depth,
        };
        let mut frame: Frame = Vec::with_capacity(function.params.len());
        let mut args = args.into_iter();
        for param in &function.params {
            let arg = args.next().flatten();
            // A parameter the host gave nothing for and that has a default is the function's own
            // to fill: what its default raises is not in anything the host passed.
            let names = caller == Caller::Host && (arg.is_some() || param.default.is_none());
            let value = match self.bind_parameter(cx, &mut frame, name, param, arg) {
                Err(error) if names => return Err(self.naming_argument(error, &param.name)),
                bound => bound?,
            };
            frame.push(Some(value));
        }
        let result = self.eval(cx, &mut frame, function.body)?;
        match &function.result {
            None => Ok(result),
            Some(ty) => self.normalize(
                cx,
                ty,
                result,
                &Path::Root(&Quoted("return value for '", name, "'")),
            ),
        }
    }

    /// The value a parameter of `function` is bound to: the argument, the default the parameter
    /// declares, evaluated after the parameters before it, or empty for an optional parameter,
    /// checked against the parameter's type.
    #[inline]
    fn bind_parameter(
        &self,
        cx: Cx,
        frame: &mut Frame,
        function: &str,
        param: &Param,
        arg: Option<Value>,
    ) -> Result<Value> {
        let value = match (arg, param.default) {
            // Body content reaches the content parameter as the list of children gathered,
            // and a child that is itself a list is spliced, as for a component's content.
            // The list is built anew for the call, so each item of the argument is paid for
            // by the function it is bound in: one, or one for each item it contributes when
            // it is itself a sequence of several. An empty item contributes nothing and still
            // costs one, since the list is as long to go through whatever it holds.
            (Some(Value::Seq(items)), _) if param.is_content => {
                let mut spliced = Vec::with_capacity(items.len());
                for item in items.iter() {
                    match item {
                        Value::Seq(elements) => {
                            self.charge(cx, None, elements.len().max(1) as u64)?;
                            spliced.extend(elements.iter().cloned());
                        }
                        other => {
                            self.charge(cx, None, 1)?;
                            spliced.push(other.clone());
                        }
                    }
                }
                Value::seq(spliced)
            }
            (Some(value), _) => value,
            (None, Some(default)) => self.eval(cx, frame, default)?,
            (None, None) if param.is_optional => Value::empty(),
            (None, None) => {
                return crate::error::fail(
                    "nx-ir-arguments",
                    format!("Function '{function}' requires argument '{}'.", param.name),
                )
            }
        };
        self.normalize(cx, &param.ty, value, &Path::Root(&&*param.name))
    }

    #[inline(never)]
    fn eval_for(&self, cx: Cx, frame: &mut Frame, node: u32, for_node: &ForNode) -> Result<Value> {
        let iterable = self.eval(cx, frame, for_node.iterable)?;
        let item_slot = for_node.item_slot as usize;
        let index_slot = for_node.index_slot.map(|slot| slot as usize);
        let mut results = Vec::new();
        if for_node.is_range {
            let Some((start, count)) = integer_range(&iterable) else {
                return self.fail(
                    cx,
                    node,
                    "nx-ir-for",
                    "For expression iterable must evaluate to a Range record with integer bounds.",
                );
            };
            // The count is computed once and checked before the body runs at all.
            let limit = self.options.max_range_length;
            if count > u128::from(limit) {
                return self.fail_limit(
                    cx,
                    Some(node),
                    Limit {
                        name: "maxRangeLength",
                        value: Some(limit),
                    },
                    format!(
                        "Iterating this range would run the loop body {count} times, above the maxRangeLength limit of {limit}."
                    ),
                );
            }
            for position in 0..count as i64 {
                bind(frame, item_slot, Value::Int(start.wrapping_add(position)))?;
                if let Some(index_slot) = index_slot {
                    bind(frame, index_slot, Value::Int(position))?;
                }
                let value = self.eval(cx, frame, for_node.body)?;
                self.place(cx, node, &mut results, value)?;
            }
            return Ok(Value::seq(results));
        }
        // The iterable is read as its items: a `+` or `*` value is a list of them, and a `?`
        // value is the empty value or the one item it holds. An optional that holds an item runs
        // the body once, and the body's value is the loop's value unchanged: an item stays an
        // item rather than becoming a one-element list.
        let items = match iterable {
            Value::Seq(items) => items,
            item => {
                bind(frame, item_slot, item)?;
                if let Some(index_slot) = index_slot {
                    bind(frame, index_slot, Value::Int(0))?;
                }
                return self.eval(cx, frame, for_node.body);
            }
        };
        // A `for` concatenates what its body yields, so each iteration contributes on the same
        // terms as an item of a braced value list.
        for (position, item) in items.iter().enumerate() {
            bind(frame, item_slot, item.clone())?;
            if let Some(index_slot) = index_slot {
                bind(frame, index_slot, Value::Int(position as i64))?;
            }
            let value = self.eval(cx, frame, for_node.body)?;
            self.place(cx, node, &mut results, value)?;
        }
        Ok(Value::seq(results))
    }

    /// Reads a member of a record value. A record stores no entry for an optional field that is
    /// empty, so a declared field that is not stored reads as the empty value; an update
    /// record's fields are all of that kind, since an absent one is unchanged.
    fn read_member(&self, cx: Cx, node: u32, base: &Value, member: &str) -> Result<Value> {
        let record = require_record(base, &"member access")?;
        if let Some(value) = record.get(member) {
            return Ok(value.clone());
        }
        let reads_as_empty = record.type_name().is_some_and(|discriminator| {
            self.program.shapes(discriminator).any(|(_, shape)| {
                shape.fields.iter().any(|field| {
                    &*field.name == member
                        && (shape.update_target.is_some() || field.ty.admits_empty())
                })
            })
        });
        if reads_as_empty {
            return Ok(Value::empty());
        }
        self.fail(
            cx,
            node,
            "nx-ir-member",
            format!("Object does not contain member '{member}'."),
        )
    }

    /// A match arm's pattern as a value. A pattern naming a union case with fields stands for
    /// every record of that case, so it is the case's `$type` alone: building the record would
    /// ask for fields a pattern never supplies. Every other pattern is the expression it is.
    ///
    /// <para>The shortcut costs what evaluating the node would have: one operation.</para>
    fn eval_pattern(&self, cx: Cx, frame: &mut Frame, index: u32) -> Result<Value> {
        if let Node::UnionCase(construct) = self.node(cx, index)? {
            let (_, _, declaration) = self.resolve(cx, index, &construct.union)?;
            if let DeclarationKind::Union(union) = &declaration.kind {
                if union
                    .cases
                    .iter()
                    .any(|case| case.name == construct.case && !case.is_constant)
                {
                    self.charge(cx, Some(index), 1)?;
                    return Ok(Value::record(
                        Some(Arc::from(format!(
                            "{}.{}",
                            construct.union.name, construct.case
                        ))),
                        Vec::new(),
                    ));
                }
            }
        }
        self.eval(cx, frame, index)
    }

    /// The properties and the content of a construction, with the body bound to the content
    /// field. `node` is the construction, which pays for each item of the content.
    #[allow(clippy::too_many_arguments)]
    fn written(
        &self,
        cx: Cx,
        frame: &mut Frame,
        node: u32,
        properties: &Properties,
        content: &[u32],
        fields: &[Field],
        path: &dyn fmt::Display,
    ) -> Result<Fields> {
        let mut input = self.properties(cx, frame, properties)?;
        let content_values = self.items(cx, frame, node, content)?;
        bind_content(
            &mut input,
            fields,
            content_values,
            path,
            !content.is_empty(),
        )?;
        Ok(input)
    }

    #[inline(never)]
    fn eval_record(
        &self,
        cx: Cx,
        frame: &mut Frame,
        node: u32,
        construct: &Construct,
    ) -> Result<Value> {
        let name = &construct.target.name;
        let (module, index, declaration) = self.resolve(cx, node, &construct.target)?;
        let DeclarationKind::Record(record) = &declaration.kind else {
            return self.fail(
                cx,
                node,
                "nx-ir-record",
                format!("'{name}' is not a record."),
            );
        };
        let input = self.written(
            cx,
            frame,
            node,
            &construct.properties,
            &construct.content,
            &record.fields,
            name,
        )?;
        let declared = Cx {
            module,
            declaration: index,
            depth: cx.depth,
        };
        let fields = if record.update_target.is_some() {
            self.normalize_patch_fields(declared, &record.fields, &input, name)?
        } else {
            self.normalize_fields(
                declared,
                &record.fields,
                &input,
                &mut Vec::new(),
                name,
                false,
            )?
        };
        Ok(Value::record(Some(Arc::clone(name)), fields))
    }

    #[inline(never)]
    fn eval_union_case(
        &self,
        cx: Cx,
        frame: &mut Frame,
        node: u32,
        construct: &CaseConstruct,
    ) -> Result<Value> {
        let union_name = &construct.union.name;
        let (module, index, declaration) = self.resolve(cx, node, &construct.union)?;
        let DeclarationKind::Union(union) = &declaration.kind else {
            return self.fail(
                cx,
                node,
                "nx-ir-union",
                format!("'{union_name}' is not a union."),
            );
        };
        let Some(case) = union.cases.iter().find(|case| case.name == construct.case) else {
            return self.fail(
                cx,
                node,
                "nx-ir-union",
                format!("'{union_name}' has no case '{}'.", construct.case),
            );
        };
        // A constant case carries nothing beyond its own name.
        if case.is_constant {
            return Ok(Value::Case(Arc::new(CaseValue {
                union: Arc::clone(union_name),
                case: Arc::clone(&case.name),
            })));
        }
        let path: Arc<str> = Arc::from(format!("{union_name}.{}", construct.case));
        let input = self.written(
            cx,
            frame,
            node,
            &construct.properties,
            &construct.content,
            &case.fields,
            &path,
        )?;
        let declared = Cx {
            module,
            declaration: index,
            depth: cx.depth,
        };
        let fields = self.normalize_fields(
            declared,
            &case.fields,
            &input,
            &mut Vec::new(),
            &path,
            false,
        )?;
        Ok(Value::record(Some(path), fields))
    }

    #[inline(never)]
    fn eval_element(
        &self,
        cx: Cx,
        frame: &mut Frame,
        node: u32,
        element: &ElementNode,
    ) -> Result<Value> {
        let mut fields = self.properties(cx, frame, &element.properties)?;
        let mut content = self.items(cx, frame, node, &element.content)?;
        // An element with no declared content field: one child is bound as itself, several as a
        // list.
        if content.len() == 1 {
            if let Some(only) = content.pop() {
                set_field(&mut fields, Arc::from("content"), only);
            }
        } else if content.len() > 1 {
            set_field(&mut fields, Arc::from("content"), Value::seq(content));
        }
        Ok(Value::record(Some(Arc::clone(&element.tag)), fields))
    }

    #[inline(never)]
    fn eval_component(
        &self,
        cx: Cx,
        frame: &mut Frame,
        node: u32,
        construct: &Construct,
    ) -> Result<Value> {
        let name = &construct.target.name;
        let (module, index, declaration) = self.resolve(cx, node, &construct.target)?;
        let DeclarationKind::Component(component) = &declaration.kind else {
            return self.fail(
                cx,
                node,
                "nx-ir-component",
                format!("'{name}' is not a component."),
            );
        };
        let properties = self.properties(cx, frame, &construct.properties)?;
        let content = self.items(cx, frame, node, &construct.content)?;
        let path = Labeled(name, " props");
        let (mut props, handlers) =
            self.split_handler_properties(module, index, properties, &path)?;
        bind_content(
            &mut props,
            &component.props,
            content,
            name,
            !construct.content.is_empty(),
        )?;
        let declared = Cx {
            module,
            declaration: index,
            depth: cx.depth,
        };
        let mut fields = self.normalize_fields(
            declared,
            &component.props,
            &props,
            &mut Vec::new(),
            &path,
            false,
        )?;
        for (name, handler) in handlers {
            set_field(&mut fields, name, Value::Handler(handler));
        }
        Ok(Value::record(Some(Arc::clone(name)), fields))
    }

    /// A handler node: the component and emit the handler answers, the action record it accepts
    /// and its owner. Nothing is evaluated beyond copying the frame, which is the by-value
    /// capture.
    #[inline(never)]
    fn eval_handler(
        &self,
        cx: Cx,
        frame: &Frame,
        node: u32,
        handler: &HandlerNode,
    ) -> Result<Value> {
        self.handler(cx, node, handler, frame.clone())
            .map(|handler| Value::Handler(Arc::new(handler)))
    }

    /// The handler value of handler node `node` of the declaration `cx` names, over `captured`.
    /// Everything but the capture is read from the node, so a restored handler is rebuilt by
    /// this as well.
    pub(crate) fn handler(
        &self,
        cx: Cx,
        node: u32,
        handler: &HandlerNode,
        captured: Frame,
    ) -> Result<Handler> {
        let (_, _, action) = self.resolve(cx, node, &handler.action)?;
        self.resolve(cx, node, &handler.component)?;
        let owner = match &handler.owner {
            Some(owner) => {
                self.resolve(cx, node, owner)?;
                Some(Arc::from(self.program.key(cx.module, owner)))
            }
            None => None,
        };
        let declaration = self
            .program
            .linked(cx.module)
            .and_then(|linked| linked.module.declaration(cx.declaration))
            .map(|declaration| Arc::clone(&declaration.name))
            .unwrap_or_else(|| Arc::from(""));
        Ok(Handler {
            module: self.program.identity(cx.module),
            declaration,
            node,
            component: Arc::from(self.program.key(cx.module, &handler.component)),
            component_name: Arc::clone(&handler.component.name),
            emit: Arc::clone(&handler.emit),
            action_name: Arc::clone(&action.name),
            owner,
            captured,
        })
    }

    #[inline(never)]
    fn eval_intrinsic(
        &self,
        cx: Cx,
        frame: &mut Frame,
        node: u32,
        intrinsic: Intrinsic,
        args: &[u32],
        names: &[Arc<str>],
    ) -> Result<Value> {
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            values.push(self.eval(cx, frame, *arg)?);
        }
        let name = intrinsic.name();
        let arity = if intrinsic == Intrinsic::Changed {
            1
        } else {
            2
        };
        if values.len() != arity {
            return self.fail(
                cx,
                node,
                "nx-ir-intrinsic",
                format!(
                    "Intrinsic '{name}' expects {arity} arguments, got {}.",
                    values.len()
                ),
            );
        }
        let first = update::record_argument(values.first(), name)?;
        match intrinsic {
            Intrinsic::Changed => {
                if names.is_empty() {
                    update::changed(first, &update::declared_order(self.program, first)?)
                } else {
                    update::changed(first, names)
                }
            }
            Intrinsic::Apply => update::apply(first, update::record_argument(values.get(1), name)?),
            Intrinsic::Merge => update::merge(first, update::record_argument(values.get(1), name)?),
            Intrinsic::Diff => update::diff(
                first,
                update::record_argument(values.get(1), name)?,
                &self.meter(cx, Some(node)),
            ),
        }
    }
}

/// The UTF-16 code units of a string per operation it costs, where its length is charged: when a
/// `concat` produces it and when it is written for the host.
pub(crate) const TEXT_UNITS: usize = 64;

/// The length of `text` in UTF-16 code units, which is the unit the cost of a string is counted
/// in, so that it is the same number the TypeScript runtime reads from `length`. ASCII text has
/// one code unit per byte, which is checked without decoding it.
pub(crate) fn utf16_len(text: &str) -> usize {
    if text.is_ascii() {
        text.len()
    } else {
        text.chars().map(char::len_utf16).sum()
    }
}

/// A name between two fixed fragments, for a diagnostic's path.
pub(crate) struct Quoted<'a>(pub &'static str, pub &'a str, pub &'static str);

impl fmt::Display for Quoted<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}{}{}", self.0, self.1, self.2)
    }
}

/// The item an optional value holds, or `None` when it is empty. A `?` value that holds an item
/// is the item itself; a one-element sequence reaching an optional receiver is read as that
/// element.
fn optional_item(value: &Value) -> Option<&Value> {
    match value {
        Value::Seq(items) => match &**items {
            [] => None,
            [only] => Some(only),
            _ => Some(value),
        },
        other => Some(other),
    }
}

/// Whether a match arm's pattern matches the scrutinee. The `{}` pattern matches exactly the
/// empty value; a record pattern matches by `$type`; anything else by the language's equality,
/// which is the one of the three that walks the values and so the one `meter` pays for.
fn pattern_matches(value: &Value, pattern: &Value, meter: &Meter<'_, '_>) -> Result<bool> {
    if pattern.is_empty() || value.is_empty() {
        return Ok(pattern.is_empty() && value.is_empty());
    }
    if let (Value::Record(value), Value::Record(pattern)) = (value, pattern) {
        if pattern.type_name.is_some() {
            return Ok(value.type_name == pattern.type_name);
        }
    }
    values_equal(value, pattern, meter)
}

/// Binds an element's body to its content field.
///
/// <para>`has_body` is whether a body was written, which is not the same as whether it produced
/// anything. An element with no body leaves the content field to its declared default; a body
/// that was written and produced nothing binds the empty value. A content field declared `+` or
/// `*` binds a list however many children were supplied, one included; one declared exactly-one
/// or `?` binds a single child as the child itself.</para>
pub(crate) fn bind_content(
    input: &mut Fields,
    fields: &[Field],
    mut content: Vec<Value>,
    path: &dyn fmt::Display,
    has_body: bool,
) -> Result<()> {
    if content.is_empty() && !has_body {
        return Ok(());
    }
    let Some(field) = fields.iter().find(|field| field.is_content) else {
        return crate::error::fail(
            "nx-ir-boundary-field",
            format!("{path} does not accept content."),
        );
    };
    if get_field(input, &field.name).is_some() {
        return crate::error::fail(
            "nx-ir-boundary-field",
            format!(
                "{path} field '{}' was supplied both as a property and as content.",
                field.name
            ),
        );
    }
    let value = if !field.ty.admits_many() && content.len() == 1 {
        content.pop().unwrap_or_else(Value::empty)
    } else {
        Value::seq(content)
    };
    input.push((Arc::clone(&field.name), value));
    Ok(())
}

/// The start and the number of integers of a `Range` record, or `None` when the value is not
/// one with integer bounds. The count is computed once, so a closed range ending at the largest
/// integer terminates.
fn integer_range(value: &Value) -> Option<(i64, u128)> {
    let record = value.as_record()?;
    if record.type_name() != Some("Range") {
        return None;
    }
    let bound = |name: &str| match record.get(name)? {
        Value::Int(value) => Some(*value),
        Value::Float(value) => integral(*value),
        _ => None,
    };
    let start = bound("start")?;
    let end = bound("end")?;
    let Value::Bool(inclusive) = record.get("endInclusive")? else {
        return None;
    };
    let span = i128::from(end).saturating_sub(i128::from(start));
    let count = if span > 0 {
        span.saturating_add(i128::from(*inclusive))
    } else if span == 0 && *inclusive {
        1
    } else {
        0
    };
    Some((start, count.unsigned_abs()))
}

enum BinaryError {
    NotNumeric,
    NotInteger,
    DivisionByZero,
    NotStrings,
}

enum Numbers {
    Ints(i64, i64),
    Floats(f64, f64),
}

fn numbers(left: &Value, right: &Value) -> std::result::Result<Numbers, BinaryError> {
    Ok(match (left, right) {
        (Value::Int(left), Value::Int(right)) => Numbers::Ints(*left, *right),
        (Value::Int(left), Value::Float(right)) => Numbers::Floats(*left as f64, *right),
        (Value::Float(left), Value::Int(right)) => Numbers::Floats(*left, *right as f64),
        (Value::Float(left), Value::Float(right)) => Numbers::Floats(*left, *right),
        _ => return Err(BinaryError::NotNumeric),
    })
}

fn floats(left: &Value, right: &Value) -> std::result::Result<(f64, f64), BinaryError> {
    Ok(match numbers(left, right)? {
        Numbers::Ints(left, right) => (left as f64, right as f64),
        Numbers::Floats(left, right) => (left, right),
    })
}

fn integers(left: &Value, right: &Value) -> std::result::Result<(i64, i64), BinaryError> {
    match numbers(left, right)? {
        Numbers::Ints(left, right) => Ok((left, right)),
        Numbers::Floats(left, right) => match (integral(left), integral(right)) {
            (Some(left), Some(right)) => Ok((left, right)),
            _ => Err(BinaryError::NotInteger),
        },
    }
}

/// `-0` as `0`: a remainder never prints a sign the operands' magnitudes do not have.
fn unsigned_zero(value: f64) -> f64 {
    if value == 0.0 {
        0.0
    } else {
        value
    }
}

/// Every operator but `and` and `or`, which are evaluated without evaluating both operands, and
/// `eq` and `ne`, which are charged for what they compare.
///
/// <para>Integer arithmetic is 64-bit and wraps; a width narrower than that is checked where a
/// value meets a typed site, not here. A `float32` operation computed on `float64` operands and
/// rounded to the nearest `float32` is the `float32` operation itself.</para>
fn binary(op: BinaryOp, left: &Value, right: &Value) -> std::result::Result<Value, BinaryError> {
    let narrow = |value: f64| Value::Float(f64::from(value as f32));
    Ok(match op {
        BinaryOp::Add => match numbers(left, right)? {
            Numbers::Ints(left, right) => Value::Int(left.wrapping_add(right)),
            Numbers::Floats(left, right) => Value::Float(left + right),
        },
        BinaryOp::Sub => match numbers(left, right)? {
            Numbers::Ints(left, right) => Value::Int(left.wrapping_sub(right)),
            Numbers::Floats(left, right) => Value::Float(left - right),
        },
        BinaryOp::Mul => match numbers(left, right)? {
            Numbers::Ints(left, right) => Value::Int(left.wrapping_mul(right)),
            Numbers::Floats(left, right) => Value::Float(left * right),
        },
        BinaryOp::Div | BinaryOp::Fdiv32 => {
            let (left, right) = floats(left, right)?;
            if right == 0.0 {
                return Err(BinaryError::DivisionByZero);
            }
            if op == BinaryOp::Div {
                Value::Float(left / right)
            } else {
                narrow(left / right)
            }
        }
        BinaryOp::Fadd32 => {
            let (left, right) = floats(left, right)?;
            narrow(left + right)
        }
        BinaryOp::Fsub32 => {
            let (left, right) = floats(left, right)?;
            narrow(left - right)
        }
        BinaryOp::Fmul32 => {
            let (left, right) = floats(left, right)?;
            narrow(left * right)
        }
        BinaryOp::Idiv | BinaryOp::Imod => {
            let (left, right) = integers(left, right)?;
            if right == 0 {
                return Err(BinaryError::DivisionByZero);
            }
            Value::Int(if op == BinaryOp::Idiv {
                // The one overflow, the least integer over `-1`, wraps to itself.
                left.checked_div(right).unwrap_or(left)
            } else {
                left.checked_rem(right).unwrap_or(0)
            })
        }
        BinaryOp::Mod => match numbers(left, right)? {
            Numbers::Ints(_, 0) => return Err(BinaryError::DivisionByZero),
            Numbers::Ints(left, right) => Value::Int(left.checked_rem(right).unwrap_or(0)),
            Numbers::Floats(left, right) => {
                if right == 0.0 {
                    return Err(BinaryError::DivisionByZero);
                }
                Value::Float(unsigned_zero(left % right))
            }
        },
        // Strings only: the emitter wraps an operand that is not one in a `text` node, which is
        // where a primitive's text form is decided.
        BinaryOp::Concat => match (left.as_text(), right.as_text()) {
            (Some(left), Some(right)) => {
                let mut text = String::with_capacity(left.len().saturating_add(right.len()));
                text.push_str(left);
                text.push_str(right);
                Value::Str(Arc::from(text))
            }
            _ => return Err(BinaryError::NotStrings),
        },
        BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
            let ordering = match numbers(left, right)? {
                Numbers::Ints(left, right) => Some(left.cmp(&right)),
                Numbers::Floats(left, right) => left.partial_cmp(&right),
            };
            Value::Bool(ordering.is_some_and(|ordering| match op {
                BinaryOp::Lt => ordering.is_lt(),
                BinaryOp::Le => ordering.is_le(),
                BinaryOp::Gt => ordering.is_gt(),
                _ => ordering.is_ge(),
            }))
        }
        // `eval_binary` applies these itself: `and` and `or` without evaluating both operands,
        // and the equalities under the budget.
        BinaryOp::And | BinaryOp::Or | BinaryOp::Eq | BinaryOp::Ne => {
            return Err(BinaryError::NotNumeric)
        }
    })
}

mod meter {
    use super::{Cx, Machine};
    use crate::error::Result;

    /// Who pays for a walk over a value: an evaluation with a budget, which each value visited
    /// is charged to, or nobody, for an evaluation with none and for the helpers a host calls on
    /// values it holds.
    #[derive(Clone, Copy)]
    pub(crate) struct Meter<'m, 'p>(Option<(&'m Machine<'p>, Cx, Option<u32>)>);

    impl<'m, 'p> Meter<'m, 'p> {
        pub(super) fn charged(machine: &'m Machine<'p>, cx: Cx, node: Option<u32>) -> Self {
            Self(Some((machine, cx, node)))
        }

        /// The meter of a walk nobody pays for.
        pub(crate) fn free() -> Meter<'m, 'p> {
            Meter(None)
        }

        /// Whether a budget pays for the walk. A walk nobody pays for skips whatever only a count
        /// needs: it does not measure a string, and it stops comparing two records at the first field
        /// that differs.
        /// It never does anything that could change the walk's result.
        pub(crate) fn is_charged(&self) -> bool {
            self.0.is_some()
        }

        /// Charges `amount` operations for values the walk is about to visit.
        #[inline]
        pub(crate) fn charge(&self, amount: u64) -> Result<()> {
            match self.0 {
                Some((machine, cx, node)) => machine.charge(cx, node, amount),
                None => Ok(()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LinkOptions, PreparedModule, Program};

    /// The nesting bound, checked where an unoptimized build can reach it. Such a build's frames
    /// use the stack budget up before a thousand nodes nest, so a test that nests them for real
    /// meets `maxStackBytes` first; this one starts the count near the bound instead.
    #[test]
    fn expressions_nest_exactly_a_thousand_deep() {
        let image = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../specs/ir-conformance/evaluation-cost/expected/main.nx.nxir");
        let module = PreparedModule::prepare(std::fs::read(image).unwrap()).unwrap();
        let program = Program::link(&module, |_| None, &LinkOptions::default()).unwrap();
        let entry = &program.data.entry;
        // `directCall` is `one()`: its `call` node, and nested in that the literal body of `one`.
        let (index, _) = entry
            .entrypoint(&entry.function_entrypoints, "directCall")
            .unwrap();
        let options = RuntimeOptions::default();
        let run = |nested: u32| {
            let machine = Machine::new(&program.data, &options);
            machine.nesting.set(nested);
            machine.invoke(0, index, Vec::new(), 0, Caller::Program)
        };

        assert!(matches!(run(MAX_NESTING - 2), Ok(Value::Int(1))));
        let error = run(MAX_NESTING - 1).unwrap_err();
        let diagnostic = &error.diagnostics[0];
        assert_eq!(diagnostic.code, "nx-ir-resource-limit");
        assert_eq!(
            diagnostic.limit,
            Some(Limit {
                name: "maxExpressionNesting",
                value: Some(1000),
            })
        );
        assert_eq!(diagnostic.declaration.as_deref(), Some("main.nx::one"));
    }

    /// An item is charged before it is placed: a placement the budget does not cover fails with
    /// the sequence as it was. A charge made after appending would fail at the same node with
    /// the same count, so this is the one place the order shows.
    #[test]
    fn a_refused_placement_leaves_the_sequence_untouched() {
        let image = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../specs/ir-conformance/evaluation-cost/expected/main.nx.nxir");
        let module = PreparedModule::prepare(std::fs::read(image).unwrap()).unwrap();
        let program = Program::link(&module, |_| None, &LinkOptions::default()).unwrap();
        let options = RuntimeOptions {
            max_operations: Some(3),
            ..RuntimeOptions::default()
        };
        let machine = Machine::new(&program.data, &options);
        let cx = Cx {
            module: 0,
            declaration: 0,
            depth: 0,
        };
        let list = |length: i64| Value::seq((0..length).map(Value::Int).collect());
        let exhausted = |result: Result<()>| {
            result.unwrap_err().diagnostics[0]
                .limit
                .map(|limit| limit.name)
        };
        let mut items = Vec::new();

        // Five items against a budget of three: refused, and none of them placed.
        assert_eq!(
            exhausted(machine.place(cx, 0, &mut items, list(5))),
            Some("maxOperations")
        );
        assert!(items.is_empty());
        // Three are covered exactly, and then not one more.
        assert!(machine.place(cx, 0, &mut items, list(3)).is_ok());
        assert_eq!(items.len(), 3);
        assert_eq!(
            exhausted(machine.place(cx, 0, &mut items, Value::Int(7))),
            Some("maxOperations")
        );
        assert_eq!(items.len(), 3);
    }
}
