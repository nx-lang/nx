//! CST → HIR lowering.
//!
//! This module converts the tree-sitter Concrete Syntax Tree (CST) into
//! our typed High-level Intermediate Representation (HIR).

use crate::ast::{BinOp, Expr, Literal, OrderedFloat, Stmt, TypeRef, UnOp};
use crate::{
    Element, EnumDef, EnumMember, ExprId, Function, Item, Module, Name, Param, Property, RecordDef,
    RecordField, SourceId, TypeAlias,
};
use nx_diagnostics::{TextSize, TextSpan};
use nx_syntax::{SyntaxKind, SyntaxNode};
use rustc_hash::FxHashMap;
use smol_str::SmolStr;

/// Context for lowering operations.
///
/// Maintains the module being built and provides helper methods for
/// allocating expressions and handling errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeTag {
    Int,
    Float,
    Boolean,
    String,
    Null,
    Unknown,
}

impl TypeTag {
    fn from_type_ref(ty: &TypeRef) -> Self {
        match ty {
            TypeRef::Name(name) => {
                let lower = name.as_str().to_ascii_lowercase();
                match lower.as_str() {
                    "string" => TypeTag::String,
                    "int" | "long" => TypeTag::Int,
                    "float" | "double" => TypeTag::Float,
                    "boolean" | "bool" => TypeTag::Boolean,
                    _ => TypeTag::Unknown,
                }
            }
            TypeRef::Nullable(inner) => TypeTag::from_type_ref(inner),
            _ => TypeTag::Unknown,
        }
    }

    fn combine_numeric(lhs: TypeTag, rhs: TypeTag) -> TypeTag {
        match (lhs, rhs) {
            (TypeTag::Float, _) | (_, TypeTag::Float) => TypeTag::Float,
            (TypeTag::Int, TypeTag::Int) => TypeTag::Int,
            _ => TypeTag::Unknown,
        }
    }

    fn is_string(self) -> bool {
        matches!(self, TypeTag::String)
    }
}

pub struct LoweringContext {
    module: Module,
    expr_types: FxHashMap<ExprId, TypeTag>,
    scope_stack: Vec<FxHashMap<Name, TypeTag>>,
}

impl LoweringContext {
    /// Creates a new lowering context for the given source file.
    pub fn new(source_id: SourceId) -> Self {
        Self {
            module: Module::new(source_id),
            expr_types: FxHashMap::default(),
            scope_stack: vec![FxHashMap::default()],
        }
    }

    /// Consumes the context and returns the completed module.
    pub fn finish(self) -> Module {
        self.module
    }

    /// Allocates an expression in the module arena.
    fn alloc_expr(&mut self, expr: Expr) -> ExprId {
        let id = self.module.alloc_expr(expr);
        self.expr_types.insert(id, TypeTag::Unknown);
        id
    }

    /// Creates an error expression for malformed CST nodes.
    fn error_expr(&mut self, span: TextSpan) -> ExprId {
        self.alloc_expr(Expr::Error(span))
    }

    fn push_scope(&mut self) {
        self.scope_stack.push(FxHashMap::default());
    }

    fn pop_scope(&mut self) {
        self.scope_stack.pop();
    }

    fn define_name(&mut self, name: &Name, ty: TypeTag) {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.insert(name.clone(), ty);
        }
    }

    fn lookup_name(&self, name: &Name) -> TypeTag {
        for scope in self.scope_stack.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return *ty;
            }
        }
        TypeTag::Unknown
    }

    fn set_expr_type(&mut self, expr: ExprId, ty: TypeTag) {
        self.expr_types.insert(expr, ty);
    }

    fn expr_type(&self, expr: ExprId) -> TypeTag {
        self.expr_types
            .get(&expr)
            .copied()
            .unwrap_or(TypeTag::Unknown)
    }

    /// Lowers a SyntaxNode to an expression.
    pub fn lower_expr(&mut self, node: SyntaxNode) -> ExprId {
        if node.is_error() {
            return self.error_expr(node.span());
        }

        match node.kind() {
            // Literals
            SyntaxKind::STRING_LITERAL | SyntaxKind::STRING_EXPRESSION => {
                let text = node.text();
                // Remove quotes
                let s = if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
                    &text[1..text.len() - 1]
                } else {
                    text
                };
                let expr = self.alloc_expr(Expr::Literal(Literal::String(SmolStr::new(s))));
                self.set_expr_type(expr, TypeTag::String);
                expr
            }

            SyntaxKind::INT_LITERAL => {
                let text = node.text();
                match text.parse::<i64>() {
                    Ok(value) => {
                        let expr = self.alloc_expr(Expr::Literal(Literal::Int(value)));
                        self.set_expr_type(expr, TypeTag::Int);
                        expr
                    }
                    Err(_) => self.error_expr(node.span()),
                }
            }

            SyntaxKind::HEX_LITERAL => {
                let text = node.text();
                let digits = text.trim_start_matches("0x").trim_start_matches("0X");
                match i64::from_str_radix(digits, 16) {
                    Ok(value) => {
                        let expr = self.alloc_expr(Expr::Literal(Literal::Int(value)));
                        self.set_expr_type(expr, TypeTag::Int);
                        expr
                    }
                    Err(_) => self.error_expr(node.span()),
                }
            }

            SyntaxKind::NUMBER_LITERAL
            | SyntaxKind::NUMBER_EXPRESSION
            | SyntaxKind::REAL_LITERAL => {
                let text = node.text();
                if let Ok(value) = text.parse::<i64>() {
                    let expr = self.alloc_expr(Expr::Literal(Literal::Int(value)));
                    self.set_expr_type(expr, TypeTag::Int);
                    expr
                } else if let Ok(value) = text.parse::<f64>() {
                    let expr = self.alloc_expr(Expr::Literal(Literal::Float(OrderedFloat(value))));
                    self.set_expr_type(expr, TypeTag::Float);
                    expr
                } else {
                    self.error_expr(node.span())
                }
            }

            SyntaxKind::BOOLEAN_LITERAL
            | SyntaxKind::BOOL_LITERAL
            | SyntaxKind::BOOLEAN_EXPRESSION => {
                let text = node.text();
                let value = text == "true";
                let expr = self.alloc_expr(Expr::Literal(Literal::Boolean(value)));
                self.set_expr_type(expr, TypeTag::Boolean);
                expr
            }

            SyntaxKind::NULL_LITERAL | SyntaxKind::NULL_EXPRESSION => {
                let expr = self.alloc_expr(Expr::Literal(Literal::Null));
                self.set_expr_type(expr, TypeTag::Null);
                expr
            }

            // Identifier
            SyntaxKind::IDENTIFIER | SyntaxKind::IDENTIFIER_EXPRESSION => {
                // For identifier expressions, get the actual identifier child
                if let Some(id_node) = node
                    .child_by_field("name")
                    .or_else(|| node.children().find(|n| n.kind() == SyntaxKind::IDENTIFIER))
                {
                    let name = Name::new(id_node.text());
                    let expr = self.alloc_expr(Expr::Ident(name.clone()));
                    let ty = self.lookup_name(&name);
                    self.set_expr_type(expr, ty);
                    expr
                } else {
                    let name = Name::new(node.text());
                    let expr = self.alloc_expr(Expr::Ident(name.clone()));
                    let ty = self.lookup_name(&name);
                    self.set_expr_type(expr, ty);
                    expr
                }
            }

            // Binary operations
            SyntaxKind::BINARY_EXPRESSION => {
                let lhs = node
                    .child_by_field("left")
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()));

                let rhs = node
                    .child_by_field("right")
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()));

                // Find operator
                let op = node.children_with_tokens().find_map(|n| match n.kind() {
                    SyntaxKind::PLUS => Some(BinOp::Add),
                    SyntaxKind::MINUS => Some(BinOp::Sub),
                    SyntaxKind::STAR => Some(BinOp::Mul),
                    SyntaxKind::SLASH => Some(BinOp::Div),
                    SyntaxKind::PERCENT => Some(BinOp::Mod),
                    SyntaxKind::EQ_EQ => Some(BinOp::Eq),
                    SyntaxKind::BANG_EQ => Some(BinOp::Ne),
                    SyntaxKind::LT => Some(BinOp::Lt),
                    SyntaxKind::GT => Some(BinOp::Gt),
                    SyntaxKind::LT_EQ => Some(BinOp::Le),
                    SyntaxKind::GT_EQ => Some(BinOp::Ge),
                    SyntaxKind::AMP_AMP => Some(BinOp::And),
                    SyntaxKind::PIPE_PIPE => Some(BinOp::Or),
                    _ => None,
                });

                if let Some(mut op) = op {
                    if matches!(op, BinOp::Add) {
                        let lhs_ty = self.expr_type(lhs);
                        let rhs_ty = self.expr_type(rhs);
                        if lhs_ty.is_string() && rhs_ty.is_string() {
                            op = BinOp::Concat;
                        }
                    }

                    let result_ty = match op {
                        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                            TypeTag::combine_numeric(self.expr_type(lhs), self.expr_type(rhs))
                        }
                        BinOp::Eq
                        | BinOp::Ne
                        | BinOp::Lt
                        | BinOp::Gt
                        | BinOp::Le
                        | BinOp::Ge
                        | BinOp::And
                        | BinOp::Or => TypeTag::Boolean,
                        BinOp::Concat => TypeTag::String,
                    };

                    let expr = self.alloc_expr(Expr::BinaryOp {
                        lhs,
                        op,
                        rhs,
                        span: node.span(),
                    });
                    self.set_expr_type(expr, result_ty);
                    expr
                } else {
                    self.error_expr(node.span())
                }
            }

            // Unary operations
            SyntaxKind::UNARY_EXPRESSION | SyntaxKind::PREFIX_UNARY_EXPRESSION => {
                let expr_node = node
                    .child_by_field("operand")
                    .or_else(|| node.children().last())
                    .unwrap();
                let expr = self.lower_expr(expr_node);

                let op = node
                    .child_by_field("operator")
                    .map(|n| match n.kind() {
                        SyntaxKind::BANG => UnOp::Not,
                        SyntaxKind::MINUS => UnOp::Neg,
                        _ => {
                            if n.text() == "!" {
                                UnOp::Not
                            } else {
                                UnOp::Neg
                            }
                        }
                    })
                    .or_else(|| {
                        node.children_with_tokens().find_map(|n| match n.kind() {
                            SyntaxKind::BANG => Some(UnOp::Not),
                            SyntaxKind::MINUS => Some(UnOp::Neg),
                            _ => None,
                        })
                    })
                    .unwrap_or_else(|| {
                        let text = node.text().trim_start();
                        if text.starts_with('!') {
                            UnOp::Not
                        } else {
                            UnOp::Neg
                        }
                    });

                let expr_id = self.alloc_expr(Expr::UnaryOp {
                    op,
                    expr,
                    span: node.span(),
                });

                let operand_ty = self.expr_type(expr);
                let result_ty = match op {
                    UnOp::Not => TypeTag::Boolean,
                    UnOp::Neg => match operand_ty {
                        TypeTag::Int | TypeTag::Float => operand_ty,
                        _ => TypeTag::Unknown,
                    },
                };

                self.set_expr_type(expr_id, result_ty);
                expr_id
            }

            // Call expression
            SyntaxKind::CALL_EXPRESSION => {
                let func = node
                    .child_by_field("function")
                    .or_else(|| node.children().next())
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()));

                let args = node
                    .children()
                    .skip(1)
                    .filter(|n| {
                        !matches!(
                            n.kind(),
                            SyntaxKind::LPAREN | SyntaxKind::RPAREN | SyntaxKind::COMMA
                        )
                    })
                    .map(|n| self.lower_expr(n))
                    .collect();

                self.alloc_expr(Expr::Call {
                    func,
                    args,
                    span: node.span(),
                })
            }

            // Member access
            SyntaxKind::MEMBER_EXPRESSION | SyntaxKind::MEMBER_ACCESS_EXPRESSION => {
                let base = node
                    .child_by_field("object")
                    .or_else(|| node.children().next())
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()));

                let member = node
                    .child_by_field("property")
                    .or_else(|| node.children().nth(1))
                    .map(|n| Name::new(n.text()))
                    .unwrap_or_else(|| Name::new(""));

                self.alloc_expr(Expr::Member {
                    base,
                    member,
                    span: node.span(),
                })
            }

            SyntaxKind::ELEMENT => {
                let span = node.span();
                let element = self.lower_element(node);

                if element.children.is_empty() {
                    if let Some(Item::Record(record_def)) =
                        self.module.find_item(element.tag.as_str())
                    {
                        let props = element
                            .properties
                            .iter()
                            .map(|p| (p.key.clone(), p.value))
                            .collect();
                        return self.alloc_expr(Expr::RecordLiteral {
                            record: record_def.name.clone(),
                            properties: props,
                            span,
                        });
                    }
                }

                let element_id = self.module.alloc_element(element);
                self.alloc_expr(Expr::Element {
                    element: element_id,
                    span,
                })
            }

            // Sequence (array) expression
            SyntaxKind::SEQUENCE_EXPRESSION => {
                let elements = node
                    .children()
                    .filter(|n| {
                        !matches!(
                            n.kind(),
                            SyntaxKind::LBRACKET | SyntaxKind::RBRACKET | SyntaxKind::COMMA
                        )
                    })
                    .map(|n| self.lower_expr(n))
                    .collect();

                self.alloc_expr(Expr::Array {
                    elements,
                    span: node.span(),
                })
            }

            // Parenthesized expression - unwrap
            SyntaxKind::PARENTHESIZED_EXPRESSION => node
                .children()
                .find(|n| !matches!(n.kind(), SyntaxKind::LPAREN | SyntaxKind::RPAREN))
                .map(|n| self.lower_expr(n))
                .unwrap_or_else(|| self.error_expr(node.span())),

            // Value expression wrappers - unwrap
            SyntaxKind::LITERAL => node
                // Tree-sitter wraps actual literal nodes (string/int/etc.) in a
                // `literal` parent. Unwrap so downstream code keeps seeing the
                // concrete literal expression.
                .children()
                .next()
                .map(|n| self.lower_expr(n))
                .unwrap_or_else(|| self.error_expr(node.span())),

            SyntaxKind::VALUE_EXPRESSION | SyntaxKind::VALUE_EXPR | SyntaxKind::RHS_EXPRESSION => {
                node.children()
                    .next()
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()))
            }

            SyntaxKind::INTERPOLATION_EXPRESSION => node
                .children()
                .find(|n| !matches!(n.kind(), SyntaxKind::LBRACE | SyntaxKind::RBRACE))
                .map(|n| self.lower_expr(n))
                .unwrap_or_else(|| self.error_expr(node.span())),
            SyntaxKind::EMBED_INTERPOLATION_EXPRESSION => node
                .children()
                .find(|n| !matches!(n.kind(), SyntaxKind::LBRACE | SyntaxKind::RBRACE))
                .map(|n| self.lower_expr(n))
                .unwrap_or_else(|| self.error_expr(node.span())),

            // For loop expression
            SyntaxKind::VALUE_FOR_EXPRESSION => {
                // Get item identifier
                let item = node
                    .child_by_field("item")
                    .map(|n| Name::new(n.text()))
                    .unwrap_or_else(|| Name::new("_"));

                // Get optional index identifier
                let index = node.child_by_field("index").map(|n| Name::new(n.text()));

                // Get iterable expression
                let iterable = node
                    .child_by_field("iterable")
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()));

                // Get body expression
                let body = node
                    .child_by_field("body")
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()));

                self.alloc_expr(Expr::For {
                    item,
                    index,
                    iterable,
                    body,
                    span: node.span(),
                })
            }

            // Ternary expression: condition ? consequent : alternative
            SyntaxKind::CONDITIONAL_EXPRESSION => {
                let condition = node
                    .child_by_field("condition")
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()));

                let then_branch = node
                    .child_by_field("consequent")
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()));

                let else_branch = node
                    .child_by_field("alternative")
                    .map(|n| self.lower_expr(n));

                self.alloc_expr(Expr::If {
                    condition,
                    then_branch,
                    else_branch,
                    span: node.span(),
                })
            }

            // Value if expression wrapper
            SyntaxKind::VALUE_IF_EXPRESSION => node
                .children()
                .next()
                .map(|n| self.lower_expr(n))
                .unwrap_or_else(|| self.error_expr(node.span())),

            // If-else expression: if condition { then } else { else }
            SyntaxKind::VALUE_IF_SIMPLE_EXPRESSION => {
                let condition = node
                    .child_by_field("condition")
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()));

                let then_branch = node
                    .child_by_field("then")
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()));

                let else_branch = node.child_by_field("else").map(|n| self.lower_expr(n));

                self.alloc_expr(Expr::If {
                    condition,
                    then_branch,
                    else_branch,
                    span: node.span(),
                })
            }

            // Condition list expression: if { cond1 => expr1, cond2 => expr2, else => default }
            // We lower this to nested if-else expressions
            SyntaxKind::VALUE_IF_CONDITION_LIST_EXPRESSION => {
                // Collect all condition arms
                let mut arms: Vec<(ExprId, ExprId)> = Vec::new();
                let mut else_expr: Option<ExprId> = None;

                for child in node.children() {
                    match child.kind() {
                        SyntaxKind::VALUE_IF_CONDITION_ARM => {
                            let condition = child
                                .child_by_field("condition")
                                .map(|n| self.lower_expr(n))
                                .unwrap_or_else(|| self.error_expr(child.span()));
                            let body = child
                                .child_by_field("body")
                                .map(|n| self.lower_expr(n))
                                .unwrap_or_else(|| self.error_expr(child.span()));
                            arms.push((condition, body));
                        }
                        _ => {
                            // Check for else branch by looking for the field
                            if let Some(else_node) = node.child_by_field("else") {
                                else_expr = Some(self.lower_expr(else_node));
                            }
                        }
                    }
                }

                // Build nested if-else from the arms (in reverse order)
                // if { a => x, b => y, else => z } becomes: if a { x } else { if b { y } else { z } }
                let mut result = else_expr;
                for (condition, body) in arms.into_iter().rev() {
                    result = Some(self.alloc_expr(Expr::If {
                        condition,
                        then_branch: body,
                        else_branch: result,
                        span: node.span(),
                    }));
                }

                result.unwrap_or_else(|| self.error_expr(node.span()))
            }

            // Match expression: if scrutinee is { pattern => expr, ... }
            // We lower this to: let $match = scrutinee in (nested if-else with equality checks)
            // This ensures the scrutinee is evaluated only once.
            SyntaxKind::VALUE_IF_MATCH_EXPRESSION => {
                // Get the scrutinee expression
                let scrutinee_expr = node
                    .child_by_field("scrutinee")
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()));

                // Generate a unique name for the scrutinee binding
                let scrutinee_name = Name::new("$match");
                let scrutinee_ref = self.alloc_expr(Expr::Ident(scrutinee_name.clone()));

                // Collect all match arms
                let mut arms: Vec<(Vec<ExprId>, ExprId)> = Vec::new();
                let mut else_expr: Option<ExprId> = None;

                for child in node.children() {
                    if child.kind() == SyntaxKind::VALUE_IF_MATCH_ARM {
                        // Each arm can have multiple patterns (comma-separated)
                        let mut patterns: Vec<ExprId> = Vec::new();
                        let mut body: Option<ExprId> = None;

                        for arm_child in child.children() {
                            match arm_child.kind() {
                                SyntaxKind::PATTERN => {
                                    // Lower the pattern as an expression (literal or qualified name)
                                    let pattern_expr = arm_child
                                        .children()
                                        .next()
                                        .map(|n| self.lower_expr(n))
                                        .unwrap_or_else(|| self.lower_expr(arm_child));
                                    patterns.push(pattern_expr);
                                }
                                _ => {
                                    // The last non-pattern child is the body
                                    if !arm_child.text().is_empty()
                                        && arm_child.text() != "=>"
                                        && arm_child.text() != ","
                                    {
                                        body = Some(self.lower_expr(arm_child));
                                    }
                                }
                            }
                        }

                        if !patterns.is_empty() {
                            arms.push((
                                patterns,
                                body.unwrap_or_else(|| self.error_expr(child.span())),
                            ));
                        }
                    }
                }

                // Check for else branch
                if let Some(else_node) = node.child_by_field("else") {
                    else_expr = Some(self.lower_expr(else_node));
                }

                // Build nested if-else from the arms (in reverse order)
                // if x is { 1 => a, 2, 3 => b, else => c } becomes:
                // let $match = x in (if $match == 1 { a } else { if $match == 2 || $match == 3 { b } else { c } })
                let mut result = else_expr;
                for (patterns, body) in arms.into_iter().rev() {
                    // Build OR condition for multiple patterns: $match == p1 || $match == p2 ...
                    let condition = if patterns.len() == 1 {
                        self.alloc_expr(Expr::BinaryOp {
                            lhs: scrutinee_ref,
                            op: BinOp::Eq,
                            rhs: patterns[0],
                            span: node.span(),
                        })
                    } else {
                        // Multiple patterns: build OR chain
                        let mut or_expr = self.alloc_expr(Expr::BinaryOp {
                            lhs: scrutinee_ref,
                            op: BinOp::Eq,
                            rhs: patterns[0],
                            span: node.span(),
                        });
                        for pattern in patterns.into_iter().skip(1) {
                            let eq_expr = self.alloc_expr(Expr::BinaryOp {
                                lhs: scrutinee_ref,
                                op: BinOp::Eq,
                                rhs: pattern,
                                span: node.span(),
                            });
                            or_expr = self.alloc_expr(Expr::BinaryOp {
                                lhs: or_expr,
                                op: BinOp::Or,
                                rhs: eq_expr,
                                span: node.span(),
                            });
                        }
                        or_expr
                    };

                    result = Some(self.alloc_expr(Expr::If {
                        condition,
                        then_branch: body,
                        else_branch: result,
                        span: node.span(),
                    }));
                }

                // Wrap in Let expression to evaluate scrutinee once
                let match_body = result.unwrap_or_else(|| self.error_expr(node.span()));
                self.alloc_expr(Expr::Let {
                    name: scrutinee_name,
                    value: scrutinee_expr,
                    body: match_body,
                    span: node.span(),
                })
            }

            // Default: create error
            _ => self.error_expr(node.span()),
        }
    }

    /// Lowers a SyntaxNode to a statement.
    pub fn lower_stmt(&mut self, _node: SyntaxNode) -> Stmt {
        // TODO: Implement statement lowering
        // For now, return a placeholder
        Stmt::Expr(
            self.error_expr(TextSpan::new(TextSize::from(0), TextSize::from(0))),
            TextSpan::new(TextSize::from(0), TextSize::from(0)),
        )
    }

    /// Lowers a type reference.
    pub fn lower_type(&self, node: SyntaxNode) -> TypeRef {
        if node.is_error() {
            return TypeRef::name("error");
        }

        match node.kind() {
            SyntaxKind::PRIMITIVE_TYPE | SyntaxKind::IDENTIFIER => TypeRef::name(node.text()),
            SyntaxKind::USER_DEFINED_TYPE => node
                .children()
                .next()
                .map(|child| self.lower_type(child))
                .unwrap_or_else(|| TypeRef::name(node.text())),
            SyntaxKind::QUALIFIED_NAME => TypeRef::name(node.text()),
            SyntaxKind::TYPE => node
                .children()
                .next()
                .map(|child| self.lower_type(child))
                .unwrap_or_else(|| TypeRef::name("unknown")),
            _ => TypeRef::name("unknown"),
        }
    }

    /// Lowers a type alias definition node.
    pub fn lower_type_alias(&self, node: SyntaxNode) -> TypeAlias {
        let name = node
            .child_by_field("name")
            .map(|n| Name::new(n.text()))
            .unwrap_or_else(|| Name::new("unknown"));
        let type_node = node.child_by_field("type").unwrap_or(node);
        let ty = self.lower_type(type_node);

        TypeAlias {
            name,
            ty,
            span: node.span(),
        }
    }

    /// Lowers a record definition node.
    pub fn lower_record_definition(&mut self, node: SyntaxNode) -> RecordDef {
        let name = node
            .child_by_field("name")
            .map(|n| Name::new(n.text()))
            .unwrap_or_else(|| Name::new("unknown"));

        let mut properties = Vec::new();
        for prop in node
            .children()
            .filter(|child| child.kind() == SyntaxKind::PROPERTY_DEFINITION)
        {
            let field_name = prop
                .child_by_field("name")
                .map(|n| Name::new(n.text()))
                .unwrap_or_else(|| Name::new("_"));
            let ty_node = prop.child_by_field("type").unwrap_or(prop);
            let ty = self.lower_type(ty_node);
            let default = prop
                .child_by_field("default")
                .map(|default_node| self.lower_expr(default_node));

            properties.push(RecordField {
                name: field_name,
                ty,
                default,
                span: prop.span(),
            });
        }

        RecordDef {
            name,
            properties,
            span: node.span(),
        }
    }

    /// Lowers an enum definition node.
    pub fn lower_enum_definition(&self, node: SyntaxNode) -> EnumDef {
        let name = node
            .child_by_field("name")
            .map(|n| Name::new(n.text()))
            .unwrap_or_else(|| Name::new("unknown"));

        let members = node
            .child_by_field("members")
            .map(|list| {
                list.children()
                    .filter(|child| child.kind() == SyntaxKind::ENUM_MEMBER)
                    .map(|child| EnumMember {
                        name: Name::new(child.text()),
                        span: child.span(),
                    })
                    .collect()
            })
            .unwrap_or_else(Vec::new);

        EnumDef {
            name,
            members,
            span: node.span(),
        }
    }

    /// Lowers a function definition.
    ///
    /// Supports both element-style (`let <Name props... />`) and paren-style (`let name(params)`)
    /// declarations with an optional `: Type` return annotation.
    pub fn lower_function(&mut self, node: SyntaxNode) -> Function {
        let span = node.span();

        // Extract function name (`element_name` for markup functions, `identifier` for paren forms)
        let name = node
            .child_by_field("name")
            .map(|n| Name::new(n.text()))
            .unwrap_or_else(|| Name::new("anonymous"));

        // Parse parameters from property_definition nodes
        let mut params = Vec::new();
        for child in node.children() {
            if child.kind() == SyntaxKind::PROPERTY_DEFINITION {
                // property_definition: name ':' type ['=' default]
                let param_name = child
                    .child_by_field("name")
                    .map(|n| Name::new(n.text()))
                    .unwrap_or_else(|| Name::new("_"));

                let type_node = child
                    .child_by_field("type")
                    .or_else(|| child.children().find(|n| n.kind() == SyntaxKind::TYPE));

                let param_type = type_node
                    .map(|n| self.lower_type(n))
                    .unwrap_or_else(|| TypeRef::name("unknown"));

                let param_span = child.span();

                params.push(Param::new(param_name, param_type, param_span));

                // Note: Default values are part of property_definition grammar
                // but we don't store them in Param yet (future enhancement)
            }
        }

        // Track parameter types in a new scope so expression lowering can infer operand kinds.
        self.push_scope();
        for param in &params {
            let ty = TypeTag::from_type_ref(&param.ty);
            self.define_name(&param.name, ty);
        }

        // Lower the optional return type annotation if present
        let return_type = node
            .child_by_field("return_type")
            .map(|n| self.lower_type(n));

        // Lower the body expression
        let body = node
            .child_by_field("body")
            .map(|n| self.lower_expr(n))
            .unwrap_or_else(|| self.error_expr(span));

        self.pop_scope();

        Function {
            name,
            params,
            return_type,
            body,
            span,
        }
    }

    /// Recursively extracts element children from content nodes.
    ///
    /// Content can be wrapped in MIXED_CONTENT, ELEMENTS_EXPRESSION, etc.
    /// This recursively searches for actual ELEMENT nodes.
    fn lower_element_children(&mut self, node: SyntaxNode, children: &mut Vec<crate::ElementId>) {
        match node.kind() {
            SyntaxKind::ELEMENT => {
                let child_element = self.lower_element(node);
                let child_id = self.module.alloc_element(child_element);
                children.push(child_id);
            }
            // These are container nodes - recurse into their children
            SyntaxKind::MIXED_CONTENT | SyntaxKind::ELEMENTS_EXPRESSION | SyntaxKind::CONTENT => {
                for child in node.children() {
                    self.lower_element_children(child, children);
                }
            }
            // Skip other nodes (text, interpolations, etc.)
            _ => {}
        }
    }

    /// Lowers an element.
    ///
    /// Parses: `<tag prop1=val1 prop2={expr}>...children...</tag>`
    /// Or self-closing: `<tag prop1=val1 />`
    pub fn lower_element(&mut self, node: SyntaxNode) -> Element {
        let span = node.span();

        // Extract tag name
        let tag = node
            .child_by_field("name")
            .map(|n| Name::new(n.text()))
            .unwrap_or_else(|| Name::new("unknown"));

        // Parse properties from property_list
        let mut properties = Vec::new();
        if let Some(prop_list) = node.child_by_field("properties") {
            for child in prop_list.children() {
                if child.kind() == SyntaxKind::PROPERTY_VALUE {
                    // property_value: name '=' expression
                    let key = child
                        .child_by_field("name")
                        .map(|n| Name::new(n.text()))
                        .unwrap_or_else(|| Name::new("_"));

                    // The value can be various expression types
                    let value = child
                        .children()
                        .find(|n| {
                            matches!(
                                n.kind(),
                                SyntaxKind::STRING_LITERAL
                                    | SyntaxKind::INTERPOLATION_EXPRESSION
                                    | SyntaxKind::VALUE_EXPRESSION
                                    | SyntaxKind::RHS_EXPRESSION
                            )
                        })
                        .map(|n| self.lower_expr(n))
                        .unwrap_or_else(|| self.error_expr(child.span()));

                    let prop_span = child.span();
                    properties.push(Property {
                        key,
                        value,
                        span: prop_span,
                    });
                }
            }
        }

        // Parse child elements from content
        let mut children = Vec::new();
        if let Some(content) = node.child_by_field("content") {
            // Content can be MIXED_CONTENT, ELEMENTS_EXPRESSION, etc.
            // We need to recursively search for ELEMENT nodes
            self.lower_element_children(content, &mut children);
        }

        // Extract closing tag name for validation
        let close_name = node
            .child_by_field("close_name")
            .map(|n| Name::new(n.text()));

        Element {
            tag,
            properties,
            children,
            close_name,
            span,
        }
    }

    /// Lowers a module (source file).
    pub fn lower_module(&mut self, root: SyntaxNode) {
        // Process all top-level items
        for child in root.children() {
            match child.kind() {
                SyntaxKind::FUNCTION_DEFINITION => {
                    let func = self.lower_function(child);
                    self.module.add_item(Item::Function(func));
                }
                SyntaxKind::TYPE_DEFINITION => {
                    let alias = self.lower_type_alias(child);
                    self.module.add_item(Item::TypeAlias(alias));
                }
                SyntaxKind::RECORD_DEFINITION => {
                    let record = self.lower_record_definition(child);
                    self.module.add_item(Item::Record(record));
                }
                SyntaxKind::ENUM_DEFINITION => {
                    let enum_def = self.lower_enum_definition(child);
                    self.module.add_item(Item::Enum(enum_def));
                }
                SyntaxKind::ELEMENT => {
                    // Top-level element becomes an implicit 'root' function
                    let span = child.span();
                    let element = self.lower_element(child);
                    let element_id = self.module.alloc_element(element);

                    // Create an Expr::Element that references the element
                    let body = self.alloc_expr(Expr::Element {
                        element: element_id,
                        span,
                    });

                    // Create the implicit 'root' function
                    let root_func = Function {
                        name: Name::new("root"),
                        params: vec![],
                        return_type: None,
                        body,
                        span,
                    };

                    self.module.add_item(Item::Function(root_func));
                }
                _ => {
                    // Skip other node types for now
                }
            }
        }
    }
}

/// Lower a CST root node to a HIR Module.
pub fn lower(root: SyntaxNode, source_id: SourceId) -> Module {
    let mut ctx = LoweringContext::new(source_id);
    ctx.lower_module(root);
    ctx.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nx_syntax::parse_str;

    #[test]
    fn test_lowering_context_creation() {
        let ctx = LoweringContext::new(SourceId::new(0));
        let module = ctx.finish();
        assert_eq!(module.items().len(), 0);
    }

    #[test]
    fn test_lower_simple_function() {
        // Parse a simple function definition
        let source = "let <Button text:string /> = <button>{text}</button>";
        let parse_result = parse_str(source, "test.nx");

        assert!(parse_result.tree.is_some());
        let tree = parse_result.tree.unwrap();
        let root = tree.root();

        // Lower to HIR
        let module = lower(root, SourceId::new(0));

        // Should have one function item
        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "Button");
                assert_eq!(func.params.len(), 1);
                assert_eq!(func.params[0].name.as_str(), "text");
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_lower_function_with_multiple_params() {
        let source = "let <Button text:string disabled:boolean /> = <button />";
        let parse_result = parse_str(source, "test.nx");

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "Button");
                assert_eq!(func.params.len(), 2);
                assert_eq!(func.params[0].name.as_str(), "text");
                assert_eq!(func.params[1].name.as_str(), "disabled");
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_lower_paren_function_with_return_type() {
        let source = "let add(a:int, b:int): int = { a + b }";
        let parse_result = parse_str(source, "test.nx");

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "add");
                assert_eq!(func.params.len(), 2);
                assert_eq!(func.params[0].name.as_str(), "a");
                assert_eq!(func.params[1].name.as_str(), "b");

                let ret = func
                    .return_type
                    .as_ref()
                    .expect("Function should capture return type annotation");
                match ret {
                    TypeRef::Name(name) => assert_eq!(name.as_str(), "int"),
                    _ => panic!("Expected simple return type"),
                }
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_lower_element_function_with_return_type() {
        let source = r#"let <Button text:string />: Element = <button>{text}</button>"#;
        let parse_result = parse_str(source, "test.nx");

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "Button");
                assert_eq!(func.params.len(), 1);

                let ret = func
                    .return_type
                    .as_ref()
                    .expect("Element-style function should retain return type");
                match ret {
                    TypeRef::Name(name) => assert_eq!(name.as_str(), "Element"),
                    _ => panic!("Expected simple return type"),
                }
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_lower_type_alias_and_enum() {
        let source = r#"
            type UserId = string
            enum Direction = | North | South | East | West
        "#;
        let parse_result = parse_str(source, "types.nx");
        let tree = parse_result.tree.expect("Should parse enum/type defs");
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 2);

        match &module.items()[0] {
            Item::TypeAlias(alias) => {
                assert_eq!(alias.name.as_str(), "UserId");
            }
            other => panic!("Expected type alias, got {:?}", other),
        }

        match &module.items()[1] {
            Item::Enum(enum_def) => {
                assert_eq!(enum_def.name.as_str(), "Direction");
                let names: Vec<_> = enum_def
                    .members
                    .iter()
                    .map(|member| member.name.as_str())
                    .collect();
                assert!(names.contains(&"North"));
                assert!(names.contains(&"West"));
            }
            other => panic!("Expected enum definition, got {:?}", other),
        }
    }

    #[test]
    fn test_lower_record_definition() {
        let source = r#"
            type User = {
              name: string
              age: int = 0
            }
        "#;
        let parse_result = parse_str(source, "record.nx");
        let tree = parse_result.tree.expect("Should parse record");
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        let record = module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Record(def) => Some(def),
                _ => None,
            })
            .expect("Should lower record definition");

        assert_eq!(record.name.as_str(), "User");
        assert_eq!(record.properties.len(), 2);
        let defaults = record
            .properties
            .iter()
            .filter(|f| f.default.is_some())
            .count();
        assert_eq!(defaults, 1, "One field should carry a default value");
    }

    #[test]
    fn test_lower_record_literal_expression() {
        let source = r#"
            type User = { name: string age: int = 30 }
            let make(): User = { <User name="Bob" /> }
        "#;

        let parse_result = parse_str(source, "record-literal.nx");
        let tree = parse_result.tree.expect("Should parse record literal");
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        let func = match module.items()[1] {
            Item::Function(ref f) => f,
            _ => panic!("expected function item"),
        };

        match module.expr(func.body) {
            Expr::RecordLiteral {
                record, properties, ..
            } => {
                assert_eq!(record.as_str(), "User");
                assert_eq!(properties.len(), 1);
                assert_eq!(properties[0].0.as_str(), "name");
            }
            other => panic!("expected record literal expr, got {:?}", other),
        }
    }

    #[test]
    fn test_lower_enum_with_leading_pipe() {
        let source = r#"
            enum Orientation = | Horizontal | Vertical
        "#;
        let parse_result = parse_str(source, "enum-lead.nx");
        let tree = parse_result.tree.expect("Should parse enum");
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        let enums: Vec<_> = module
            .items()
            .iter()
            .filter_map(|item| match item {
                Item::Enum(def) => Some(def),
                _ => None,
            })
            .collect();
        assert_eq!(enums.len(), 1);
        let enum_def = enums[0];
        assert_eq!(enum_def.name.as_str(), "Orientation");
        assert_eq!(enum_def.members.len(), 2);
        assert_eq!(enum_def.members[0].name.as_str(), "Horizontal");
        assert_eq!(enum_def.members[1].name.as_str(), "Vertical");
    }

    #[test]
    fn test_lower_simple_element() {
        let source = "<button />";
        let parse_result = parse_str(source, "test.nx");

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        // Should have one function item named 'root' (implicit from top-level element)
        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "root");
                assert!(func.params.is_empty());

                // The body should be an Element expression
                let body_expr = module.expr(func.body);
                match body_expr {
                    Expr::Element { element, .. } => {
                        let elem = module.element(*element);
                        assert_eq!(elem.tag.as_str(), "button");
                        assert_eq!(elem.properties.len(), 0);
                        assert_eq!(elem.children.len(), 0);
                    }
                    _ => panic!("Expected Element expression as body"),
                }
            }
            _ => panic!("Expected Function item for implicit root"),
        }
    }

    #[test]
    fn test_lower_embed_interpolation_inside_text_element() {
        let source = r#"
            <Root>
              <markdown:text>@{user}</markdown:text>
            </Root>
        "#;
        let parse_result = parse_str(source, "text.nx");
        let tree = parse_result.tree.expect("Should parse typed text element");
        let root = tree.root();

        fn find_kind(node: SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
            if node.kind() == kind {
                return Some(node);
            }
            for child in node.children() {
                if let Some(found) = find_kind(child, kind) {
                    return Some(found);
                }
            }
            None
        }

        let embed_interp = find_kind(root, SyntaxKind::EMBED_INTERPOLATION_EXPRESSION)
            .expect("expected embed interpolation node");

        let mut ctx = LoweringContext::new(SourceId::new(0));
        let expr_id = ctx.lower_expr(embed_interp);
        let expr = ctx.module.expr(expr_id);

        match expr {
            Expr::Ident(name) => assert_eq!(name.as_str(), "user"),
            other => panic!(
                "Expected identifier from embed interpolation, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_lower_element_with_properties() {
        let source = r#"<button class="btn" disabled="true" />"#;
        let parse_result = parse_str(source, "test.nx");

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "root");

                // The body should be an Element expression
                let body_expr = module.expr(func.body);
                match body_expr {
                    Expr::Element { element, .. } => {
                        let elem = module.element(*element);
                        assert_eq!(elem.tag.as_str(), "button");
                        assert_eq!(elem.properties.len(), 2);
                        assert_eq!(elem.properties[0].key.as_str(), "class");
                        assert_eq!(elem.properties[1].key.as_str(), "disabled");
                    }
                    _ => panic!("Expected Element expression as body"),
                }
            }
            _ => panic!("Expected Function item for implicit root"),
        }
    }

    #[test]
    fn test_lower_nested_elements() {
        let source = "<div><button /></div>";
        let parse_result = parse_str(source, "test.nx");

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "root");

                // The body should be an Element expression
                let body_expr = module.expr(func.body);
                match body_expr {
                    Expr::Element { element, .. } => {
                        let elem = module.element(*element);
                        assert_eq!(elem.tag.as_str(), "div");
                        assert_eq!(elem.children.len(), 1);

                        // Check nested button element
                        let child = module.element(elem.children[0]);
                        assert_eq!(child.tag.as_str(), "button");
                    }
                    _ => panic!("Expected Element expression as body"),
                }
            }
            _ => panic!("Expected Function item for implicit root"),
        }
    }

    #[test]
    fn test_lower_function_body_element_expression() {
        use crate::ast::Expr;

        let source = "let <Button text:string /> = <button />";
        let parse_result = parse_str(source, "test.nx");

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        match &module.items()[0] {
            Item::Function(func) => {
                let body_expr = module.expr(func.body);
                match body_expr {
                    Expr::Element { element, .. } => {
                        let element_ref = module.element(*element);
                        assert_eq!(element_ref.tag.as_str(), "button");
                    }
                    _ => panic!("Expected element expression in function body"),
                }
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_string_addition_lowers_to_concat() {
        let source = r#"let <concat a:string b:string /> = { a + b }"#;
        let parse_result = parse_str(source, "test.nx");
        assert!(
            parse_result.errors.is_empty(),
            "parse errors: {:?}",
            parse_result.errors
        );
        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 1);
        let func = match &module.items()[0] {
            Item::Function(f) => f,
            _ => panic!("expected function item"),
        };

        let assert_concat = |expr_id: ExprId| match module.expr(expr_id) {
            Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::Concat),
            Expr::Block {
                expr: Some(final_expr),
                ..
            } => match module.expr(*final_expr) {
                Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::Concat),
                other => panic!("expected binary op in block, found {:?}", other),
            },
            other => panic!("expected binary op, found {:?}", other),
        };

        assert_concat(func.body);
    }

    #[test]
    fn test_lower_for_loop_simple() {
        let source = "let <ForSimple items:object /> = {for item in items { item * 2 }}";
        let parse_result = parse_str(source, "test.nx");

        assert!(
            parse_result.errors.is_empty(),
            "parse errors: {:?}",
            parse_result.errors
        );

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        // Should have one function item
        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "ForSimple");

                // Function body should be a block containing a for loop
                let body_expr = module.expr(func.body);

                // Navigate through potential block wrapper
                let for_expr = match body_expr {
                    Expr::For { .. } => body_expr,
                    Expr::Block { expr: Some(e), .. } => module.expr(*e),
                    other => panic!("Expected For or Block expression, got {:?}", other),
                };

                // Verify it's a for loop
                match for_expr {
                    Expr::For {
                        item,
                        index,
                        iterable,
                        body,
                        ..
                    } => {
                        assert_eq!(item.as_str(), "item");
                        assert!(index.is_none());

                        // Verify iterable is an identifier
                        match module.expr(*iterable) {
                            Expr::Ident(name) => assert_eq!(name.as_str(), "items"),
                            other => panic!("Expected Ident for iterable, got {:?}", other),
                        }

                        // Verify body is a binary operation
                        match module.expr(*body) {
                            Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::Mul),
                            other => panic!("Expected BinaryOp for body, got {:?}", other),
                        }
                    }
                    other => panic!("Expected For expression, got {:?}", other),
                }
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_lower_for_loop_with_index() {
        let source =
            "let <ForWithIndex items:object /> = {for item, index in items { item + index }}";
        let parse_result = parse_str(source, "test.nx");

        assert!(
            parse_result.errors.is_empty(),
            "parse errors: {:?}",
            parse_result.errors
        );

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                let body_expr = module.expr(func.body);

                let for_expr = match body_expr {
                    Expr::For { .. } => body_expr,
                    Expr::Block { expr: Some(e), .. } => module.expr(*e),
                    other => panic!("Expected For or Block expression, got {:?}", other),
                };

                match for_expr {
                    Expr::For { item, index, .. } => {
                        assert_eq!(item.as_str(), "item");
                        assert!(index.is_some());
                        assert_eq!(index.as_ref().unwrap().as_str(), "index");
                    }
                    other => panic!("Expected For expression, got {:?}", other),
                }
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_lower_ternary_expression() {
        // Ternary: condition ? consequent : alternative
        let source = "let choose(cond:bool): int = { cond ? 1 : 0 }";
        let parse_result = parse_str(source, "test.nx");

        assert!(
            parse_result.errors.is_empty(),
            "parse errors: {:?}",
            parse_result.errors
        );

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "choose");

                // Navigate through potential block wrapper
                let if_expr = match module.expr(func.body) {
                    Expr::If { .. } => module.expr(func.body),
                    Expr::Block { expr: Some(e), .. } => module.expr(*e),
                    other => panic!("Expected If or Block expression, got {:?}", other),
                };

                match if_expr {
                    Expr::If {
                        condition,
                        then_branch,
                        else_branch,
                        ..
                    } => {
                        // Verify condition is an identifier
                        match module.expr(*condition) {
                            Expr::Ident(name) => assert_eq!(name.as_str(), "cond"),
                            other => panic!("Expected Ident for condition, got {:?}", other),
                        }

                        // Verify then branch is literal 1
                        match module.expr(*then_branch) {
                            Expr::Literal(Literal::Int(1)) => (),
                            other => {
                                panic!("Expected Literal(Int(1)) for then_branch, got {:?}", other)
                            }
                        }

                        // Verify else branch is literal 0
                        assert!(else_branch.is_some());
                        match module.expr(else_branch.unwrap()) {
                            Expr::Literal(Literal::Int(0)) => (),
                            other => {
                                panic!("Expected Literal(Int(0)) for else_branch, got {:?}", other)
                            }
                        }
                    }
                    other => panic!("Expected If expression, got {:?}", other),
                }
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_lower_if_else_expression() {
        // If-else: if condition { then } else { else }
        let source = "let max(a:int, b:int): int = { if a > b { a } else { b } }";
        let parse_result = parse_str(source, "test.nx");

        assert!(
            parse_result.errors.is_empty(),
            "parse errors: {:?}",
            parse_result.errors
        );

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "max");

                // Navigate through potential block wrapper
                let if_expr = match module.expr(func.body) {
                    Expr::If { .. } => module.expr(func.body),
                    Expr::Block { expr: Some(e), .. } => module.expr(*e),
                    other => panic!("Expected If or Block expression, got {:?}", other),
                };

                match if_expr {
                    Expr::If {
                        condition,
                        then_branch,
                        else_branch,
                        ..
                    } => {
                        // Verify condition is a > b
                        match module.expr(*condition) {
                            Expr::BinaryOp { op, .. } => assert_eq!(*op, BinOp::Gt),
                            other => panic!("Expected BinaryOp for condition, got {:?}", other),
                        }

                        // Verify then branch is identifier 'a'
                        match module.expr(*then_branch) {
                            Expr::Ident(name) => assert_eq!(name.as_str(), "a"),
                            other => panic!("Expected Ident for then_branch, got {:?}", other),
                        }

                        // Verify else branch is identifier 'b'
                        assert!(else_branch.is_some());
                        match module.expr(else_branch.unwrap()) {
                            Expr::Ident(name) => assert_eq!(name.as_str(), "b"),
                            other => panic!("Expected Ident for else_branch, got {:?}", other),
                        }
                    }
                    other => panic!("Expected If expression, got {:?}", other),
                }
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_lower_if_without_else() {
        // If without else: if condition { then }
        let source = "let maybe(x:int): int = { if x > 0 { x } }";
        let parse_result = parse_str(source, "test.nx");

        assert!(
            parse_result.errors.is_empty(),
            "parse errors: {:?}",
            parse_result.errors
        );

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "maybe");

                // Navigate through potential block wrapper
                let if_expr = match module.expr(func.body) {
                    Expr::If { .. } => module.expr(func.body),
                    Expr::Block { expr: Some(e), .. } => module.expr(*e),
                    other => panic!("Expected If or Block expression, got {:?}", other),
                };

                match if_expr {
                    Expr::If { else_branch, .. } => {
                        // Verify no else branch
                        assert!(else_branch.is_none(), "Expected no else branch");
                    }
                    other => panic!("Expected If expression, got {:?}", other),
                }
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_lower_nested_ternary() {
        // Nested ternary: x > 0 ? "positive" : x < 0 ? "negative" : "zero"
        let source =
            r#"let classify(x:int): string = { x > 0 ? "positive" : x < 0 ? "negative" : "zero" }"#;
        let parse_result = parse_str(source, "test.nx");

        assert!(
            parse_result.errors.is_empty(),
            "parse errors: {:?}",
            parse_result.errors
        );

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.as_str(), "classify");

                // Navigate through potential block wrapper
                let if_expr = match module.expr(func.body) {
                    Expr::If { .. } => module.expr(func.body),
                    Expr::Block { expr: Some(e), .. } => module.expr(*e),
                    other => panic!("Expected If or Block expression, got {:?}", other),
                };

                // Verify it's a nested if expression
                match if_expr {
                    Expr::If { else_branch, .. } => {
                        assert!(else_branch.is_some());
                        // The else branch should be another if expression
                        match module.expr(else_branch.unwrap()) {
                            Expr::If {
                                else_branch: inner_else,
                                ..
                            } => {
                                assert!(inner_else.is_some());
                            }
                            other => panic!("Expected nested If expression, got {:?}", other),
                        }
                    }
                    other => panic!("Expected If expression, got {:?}", other),
                }
            }
            _ => panic!("Expected Function item"),
        }
    }
}
