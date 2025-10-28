//! CST → HIR lowering.
//!
//! This module converts the tree-sitter Concrete Syntax Tree (CST) into
//! our typed High-level Intermediate Representation (HIR).

use crate::ast::{BinOp, Expr, Literal, OrderedFloat, Stmt, TypeRef, UnOp};
use crate::{Element, ExprId, Function, Item, Module, Name, Param, Property, SourceId};
use nx_diagnostics::{TextSize, TextSpan};
use nx_syntax::{SyntaxKind, SyntaxNode};
use smol_str::SmolStr;

/// Context for lowering operations.
///
/// Maintains the module being built and provides helper methods for
/// allocating expressions and handling errors.
pub struct LoweringContext {
    module: Module,
}

impl LoweringContext {
    /// Creates a new lowering context for the given source file.
    pub fn new(source_id: SourceId) -> Self {
        Self {
            module: Module::new(source_id),
        }
    }

    /// Consumes the context and returns the completed module.
    pub fn finish(self) -> Module {
        self.module
    }

    /// Allocates an expression in the module arena.
    fn alloc_expr(&mut self, expr: Expr) -> ExprId {
        self.module.alloc_expr(expr)
    }

    /// Creates an error expression for malformed CST nodes.
    fn error_expr(&mut self, span: TextSpan) -> ExprId {
        self.alloc_expr(Expr::Error(span))
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
                self.alloc_expr(Expr::Literal(Literal::String(SmolStr::new(s))))
            }

            SyntaxKind::INT_LITERAL => {
                let text = node.text();
                match text.parse::<i64>() {
                    Ok(value) => self.alloc_expr(Expr::Literal(Literal::Int(value))),
                    Err(_) => self.error_expr(node.span()),
                }
            }

            SyntaxKind::HEX_LITERAL => {
                let text = node.text();
                let digits = text.trim_start_matches("0x").trim_start_matches("0X");
                match i64::from_str_radix(digits, 16) {
                    Ok(value) => self.alloc_expr(Expr::Literal(Literal::Int(value))),
                    Err(_) => self.error_expr(node.span()),
                }
            }

            SyntaxKind::NUMBER_LITERAL
            | SyntaxKind::NUMBER_EXPRESSION
            | SyntaxKind::REAL_LITERAL => {
                let text = node.text();
                if let Ok(value) = text.parse::<i64>() {
                    self.alloc_expr(Expr::Literal(Literal::Int(value)))
                } else if let Ok(value) = text.parse::<f64>() {
                    self.alloc_expr(Expr::Literal(Literal::Float(OrderedFloat(value))))
                } else {
                    self.error_expr(node.span())
                }
            }

            SyntaxKind::BOOLEAN_LITERAL
            | SyntaxKind::BOOL_LITERAL
            | SyntaxKind::BOOLEAN_EXPRESSION => {
                let text = node.text();
                let value = text == "true";
                self.alloc_expr(Expr::Literal(Literal::Bool(value)))
            }

            SyntaxKind::NULL_LITERAL | SyntaxKind::NULL_EXPRESSION => {
                self.alloc_expr(Expr::Literal(Literal::Null))
            }

            // Identifier
            SyntaxKind::IDENTIFIER | SyntaxKind::IDENTIFIER_EXPRESSION => {
                // For identifier expressions, get the actual identifier child
                if let Some(id_node) = node
                    .child_by_field("name")
                    .or_else(|| node.children().find(|n| n.kind() == SyntaxKind::IDENTIFIER))
                {
                    let name = Name::new(id_node.text());
                    self.alloc_expr(Expr::Ident(name))
                } else {
                    let name = Name::new(node.text());
                    self.alloc_expr(Expr::Ident(name))
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

                if let Some(op) = op {
                    self.alloc_expr(Expr::BinaryOp {
                        lhs,
                        op,
                        rhs,
                        span: node.span(),
                    })
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

                let op = if node.text().starts_with('!') {
                    UnOp::Not
                } else {
                    UnOp::Neg
                };

                self.alloc_expr(Expr::UnaryOp {
                    op,
                    expr,
                    span: node.span(),
                })
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
            SyntaxKind::VALUE_EXPRESSION | SyntaxKind::VALUE_EXPR | SyntaxKind::RHS_EXPRESSION => {
                node.children()
                    .next()
                    .map(|n| self.lower_expr(n))
                    .unwrap_or_else(|| self.error_expr(node.span()))
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
            _ => TypeRef::name("unknown"),
        }
    }

    /// Lowers a function definition.
    ///
    /// Parses: `let <Name param1:type1 param2:type2=default /> = body`
    pub fn lower_function(&mut self, node: SyntaxNode) -> Function {
        let span = node.span();

        // Extract function name from the element_name field
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

                let param_type = child
                    .child_by_field("type")
                    .map(|n| self.lower_type(n))
                    .unwrap_or_else(|| TypeRef::name("unknown"));

                let param_span = child.span();

                params.push(Param::new(param_name, param_type, param_span));

                // Note: Default values are part of property_definition grammar
                // but we don't store them in Param yet (future enhancement)
            }
        }

        // Lower the body expression
        let body = node
            .child_by_field("body")
            .map(|n| self.lower_expr(n))
            .unwrap_or_else(|| self.error_expr(span));

        Function {
            name,
            params,
            return_type: None, // NX doesn't have explicit return types in this syntax
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
                SyntaxKind::ELEMENT => {
                    let element = self.lower_element(child);
                    let element_id = self.module.alloc_element(element);
                    self.module.add_item(Item::Element(element_id));
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
    fn test_lower_simple_element() {
        let source = "<button />";
        let parse_result = parse_str(source, "test.nx");

        let tree = parse_result.tree.unwrap();
        let root = tree.root();
        let module = lower(root, SourceId::new(0));

        // Should have one element item
        assert_eq!(module.items().len(), 1);

        match &module.items()[0] {
            Item::Element(element_id) => {
                let element = module.element(*element_id);
                assert_eq!(element.tag.as_str(), "button");
                assert_eq!(element.properties.len(), 0);
                assert_eq!(element.children.len(), 0);
            }
            _ => panic!("Expected Element item"),
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
            Item::Element(element_id) => {
                let element = module.element(*element_id);
                assert_eq!(element.tag.as_str(), "button");
                assert_eq!(element.properties.len(), 2);
                assert_eq!(element.properties[0].key.as_str(), "class");
                assert_eq!(element.properties[1].key.as_str(), "disabled");
            }
            _ => panic!("Expected Element item"),
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
            Item::Element(element_id) => {
                let element = module.element(*element_id);
                assert_eq!(element.tag.as_str(), "div");
                assert_eq!(element.children.len(), 1);

                // Check nested button element
                let child = module.element(element.children[0]);
                assert_eq!(child.tag.as_str(), "button");
            }
            _ => panic!("Expected Element item"),
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
}
