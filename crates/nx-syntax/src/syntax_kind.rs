//! Syntax node and token kinds for the NX language.

use std::fmt;

/// Represents the kind of a syntax node or token in the NX CST.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub enum SyntaxKind {
    // === Structural ===
    MODULE_DEFINITION,
    ERROR,

    // === Imports ===
    IMPORT_STATEMENT,

    // === Type Definitions ===
    TYPE_DEFINITION,
    RECORD_DEFINITION,
    ENUM_DEFINITION,
    ENUM_MEMBER_LIST,
    ENUM_MEMBER,
    VALUE_DEFINITION,
    TYPE,
    PRIMITIVE_TYPE,
    USER_DEFINED_TYPE,

    // === Functions ===
    FUNCTION_DEFINITION,
    FUNCTION_SIGNATURE,
    MARKUP_SIGNATURE,
    PARAM,
    PARAM_LIST,

    // === Elements ===
    ELEMENT,
    SELF_CLOSING_ELEMENT,
    OPEN_TAG,
    CLOSE_TAG,
    TEXT_CONTENT,
    TEXT_CHILD_ELEMENT,
    EMBED_TEXT_CONTENT,
    MIXED_CONTENT,
    TEXT_RUN,
    EMBED_TEXT_RUN,
    RAW_TEXT_RUN,
    TEXT_CHUNK,
    EMBED_TEXT_CHUNK,
    RAW_TEXT_CHUNK,
    TEXT_PART,
    ENTITY,
    CONTENT,
    EMBED_CONTENT,

    // === Properties ===
    PROPERTY,
    PROPERTY_LIST,
    PROPERTY_DEFINITION,
    PROPERTY_VALUE,

    // === Expressions ===
    VALUE_EXPRESSION,
    VALUE_EXPR,
    RHS_EXPRESSION,
    ELEMENTS_EXPRESSION,
    IDENTIFIER_EXPRESSION,
    NUMBER_EXPRESSION,
    STRING_EXPRESSION,
    BOOLEAN_EXPRESSION,
    NULL_EXPRESSION,
    MEMBER_EXPRESSION,
    MEMBER_ACCESS_EXPRESSION,
    CALL_EXPRESSION,
    UNARY_EXPRESSION,
    PREFIX_UNARY_EXPRESSION,
    BINARY_EXPRESSION,
    TERNARY_EXPRESSION,
    CONDITIONAL_EXPRESSION,
    SEQUENCE_EXPRESSION,
    INTERPOLATION_EXPRESSION,
    EMBED_INTERPOLATION_EXPRESSION,
    PARENTHESIZED_EXPRESSION,

    // === Control Flow ===
    VALUE_IF_EXPRESSION,
    ELEMENTS_IF_EXPRESSION,
    PROPERTY_LIST_IF_EXPRESSION,
    VALUE_FOR_EXPRESSION,
    ELEMENTS_FOR_EXPRESSION,
    VALUE_MATCH_EXPRESSION,
    ELEMENTS_MATCH_EXPRESSION,
    PATTERN,

    // === If Expression Parts ===
    VALUE_IF_SIMPLE_EXPRESSION,
    VALUE_IF_CONDITION_LIST_EXPRESSION,
    VALUE_IF_MATCH_EXPRESSION,
    VALUE_IF_CONDITION_ARM,
    VALUE_IF_MATCH_ARM,

    ELEMENTS_IF_SIMPLE_EXPRESSION,
    ELEMENTS_IF_CONDITION_LIST_EXPRESSION,
    ELEMENTS_IF_MATCH_EXPRESSION,
    ELEMENTS_IF_CONDITION_ARM,
    ELEMENTS_IF_MATCH_ARM,

    PROPERTY_LIST_IF_SIMPLE_EXPRESSION,
    PROPERTY_LIST_IF_CONDITION_LIST_EXPRESSION,
    PROPERTY_LIST_IF_MATCH_EXPRESSION,
    PROPERTY_LIST_IF_CONDITION_ARM,
    PROPERTY_LIST_IF_MATCH_ARM,

    // === Literals ===
    LITERAL,
    STRING_LITERAL,
    NUMBER_LITERAL,
    INT_LITERAL,
    REAL_LITERAL,
    HEX_LITERAL,
    UNIT_LITERAL,
    BOOLEAN_LITERAL,
    BOOL_LITERAL,
    NULL_LITERAL,

    // === Names and Identifiers ===
    IDENTIFIER,
    QUALIFIED_NAME,
    QUALIFIED_MARKUP_NAME,
    ELEMENT_NAME,
    MARKUP_IDENTIFIER,

    // === Comments ===
    LINE_COMMENT,
    BLOCK_COMMENT,
    HTML_BLOCK_COMMENT,

    // === Keywords ===
    LET,
    TYPE_KW,
    ENUM,
    IMPORT,
    IF,
    ELSE,
    FOR,
    IN,
    MATCH,
    TRUE,
    FALSE,
    NULL_KW,

    // === Operators ===
    PLUS,
    MINUS,
    STAR,
    SLASH,
    PERCENT,
    EQ_EQ,
    BANG_EQ,
    LT,
    GT,
    LT_EQ,
    GT_EQ,
    AMP_AMP,
    PIPE_PIPE,
    PIPE,
    BANG,
    QUESTION,
    FAT_ARROW,
    COLON,

    // === Punctuation ===
    LPAREN,
    RPAREN,
    LBRACE,
    RBRACE,
    LBRACKET,
    RBRACKET,
    LT_TAG,
    GT_TAG,
    SLASH_GT,
    LT_SLASH,
    COMMA,
    DOT,
    EQ,
    ESCAPED_LBRACE,
    ESCAPED_RBRACE,
    ESCAPED_AT,

    // === Special ===
    WHITESPACE,
    NEWLINE,
    EOF,
}

impl SyntaxKind {
    /// Returns true if this kind represents a token (leaf node).
    pub fn is_token(self) -> bool {
        matches!(
            self,
            SyntaxKind::IDENTIFIER
                | SyntaxKind::STRING_LITERAL
                | SyntaxKind::NUMBER_LITERAL
                | SyntaxKind::BOOLEAN_LITERAL
                | SyntaxKind::NULL_LITERAL
                | SyntaxKind::LET
                | SyntaxKind::TYPE_KW
                | SyntaxKind::IMPORT
                | SyntaxKind::ENUM
                | SyntaxKind::IF
                | SyntaxKind::ELSE
                | SyntaxKind::FOR
                | SyntaxKind::IN
                | SyntaxKind::MATCH
                | SyntaxKind::TRUE
                | SyntaxKind::FALSE
                | SyntaxKind::NULL_KW
                | SyntaxKind::PLUS
                | SyntaxKind::MINUS
                | SyntaxKind::STAR
                | SyntaxKind::SLASH
                | SyntaxKind::PERCENT
                | SyntaxKind::EQ_EQ
                | SyntaxKind::BANG_EQ
                | SyntaxKind::LT
                | SyntaxKind::GT
                | SyntaxKind::LT_EQ
                | SyntaxKind::GT_EQ
                | SyntaxKind::AMP_AMP
                | SyntaxKind::PIPE_PIPE
                | SyntaxKind::PIPE
                | SyntaxKind::BANG
                | SyntaxKind::QUESTION
                | SyntaxKind::FAT_ARROW
                | SyntaxKind::COLON
                | SyntaxKind::LPAREN
                | SyntaxKind::RPAREN
                | SyntaxKind::LBRACE
                | SyntaxKind::RBRACE
                | SyntaxKind::LBRACKET
                | SyntaxKind::RBRACKET
                | SyntaxKind::LT_TAG
                | SyntaxKind::GT_TAG
                | SyntaxKind::SLASH_GT
                | SyntaxKind::LT_SLASH
                | SyntaxKind::COMMA
                | SyntaxKind::DOT
                | SyntaxKind::EQ
                | SyntaxKind::WHITESPACE
                | SyntaxKind::NEWLINE
                | SyntaxKind::TEXT_CHUNK
                | SyntaxKind::EMBED_TEXT_CHUNK
                | SyntaxKind::RAW_TEXT_CHUNK
                | SyntaxKind::ENTITY
                | SyntaxKind::ESCAPED_LBRACE
                | SyntaxKind::ESCAPED_RBRACE
                | SyntaxKind::ESCAPED_AT
                | SyntaxKind::LINE_COMMENT
                | SyntaxKind::BLOCK_COMMENT
                | SyntaxKind::HTML_BLOCK_COMMENT
        )
    }

    /// Returns true if this kind represents a keyword.
    pub fn is_keyword(self) -> bool {
        matches!(
            self,
            SyntaxKind::LET
                | SyntaxKind::TYPE_KW
                | SyntaxKind::IMPORT
                | SyntaxKind::IF
                | SyntaxKind::ELSE
                | SyntaxKind::FOR
                | SyntaxKind::IN
                | SyntaxKind::MATCH
                | SyntaxKind::TRUE
                | SyntaxKind::FALSE
                | SyntaxKind::NULL_KW
                | SyntaxKind::ENUM
        )
    }

    /// Returns true if this kind represents a comment.
    pub fn is_comment(self) -> bool {
        matches!(
            self,
            SyntaxKind::LINE_COMMENT | SyntaxKind::BLOCK_COMMENT | SyntaxKind::HTML_BLOCK_COMMENT
        )
    }

    /// Returns true if this kind represents trivia (whitespace, comments, etc.).
    pub fn is_trivia(self) -> bool {
        matches!(self, SyntaxKind::WHITESPACE | SyntaxKind::NEWLINE) || self.is_comment()
    }
}

impl fmt::Display for SyntaxKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Converts a tree-sitter node kind string to a SyntaxKind.
pub fn syntax_kind_from_str(kind: &str) -> SyntaxKind {
    match kind {
        "module_definition" => SyntaxKind::MODULE_DEFINITION,
        "ERROR" => SyntaxKind::ERROR,
        "import_statement" => SyntaxKind::IMPORT_STATEMENT,
        "type_definition" => SyntaxKind::TYPE_DEFINITION,
        "record_definition" => SyntaxKind::RECORD_DEFINITION,
        "enum_definition" => SyntaxKind::ENUM_DEFINITION,
        "enum_member_list" => SyntaxKind::ENUM_MEMBER_LIST,
        "enum_member" => SyntaxKind::ENUM_MEMBER,
        "value_definition" => SyntaxKind::VALUE_DEFINITION,
        "type" => SyntaxKind::TYPE,
        "primitive_type" => SyntaxKind::PRIMITIVE_TYPE,
        "user_defined_type" => SyntaxKind::USER_DEFINED_TYPE,
        "function_definition" => SyntaxKind::FUNCTION_DEFINITION,
        "function_signature" => SyntaxKind::FUNCTION_SIGNATURE,
        "markup_signature" => SyntaxKind::MARKUP_SIGNATURE,
        "param" => SyntaxKind::PARAM,
        "param_list" => SyntaxKind::PARAM_LIST,
        "element" => SyntaxKind::ELEMENT,
        "self_closing_element" => SyntaxKind::SELF_CLOSING_ELEMENT,
        "open_tag" => SyntaxKind::OPEN_TAG,
        "close_tag" => SyntaxKind::CLOSE_TAG,
        "text_content" => SyntaxKind::TEXT_CONTENT,
        "text_child_element" => SyntaxKind::TEXT_CHILD_ELEMENT,
        "embed_text_content" => SyntaxKind::EMBED_TEXT_CONTENT,
        "mixed_content" => SyntaxKind::MIXED_CONTENT,
        "text_run" => SyntaxKind::TEXT_RUN,
        "embed_text_run" => SyntaxKind::EMBED_TEXT_RUN,
        "raw_text_run" => SyntaxKind::RAW_TEXT_RUN,
        "text_chunk" => SyntaxKind::TEXT_CHUNK,
        "embed_text_chunk" => SyntaxKind::EMBED_TEXT_CHUNK,
        "raw_text_chunk" => SyntaxKind::RAW_TEXT_CHUNK,
        "text_part" => SyntaxKind::TEXT_PART,
        "entity" => SyntaxKind::ENTITY,
        "content" => SyntaxKind::CONTENT,
        "embed_content" => SyntaxKind::EMBED_CONTENT,
        "property" => SyntaxKind::PROPERTY,
        "property_list" => SyntaxKind::PROPERTY_LIST,
        "property_definition" => SyntaxKind::PROPERTY_DEFINITION,
        "property_value" => SyntaxKind::PROPERTY_VALUE,
        "value_expression" => SyntaxKind::VALUE_EXPRESSION,
        "value_expr" => SyntaxKind::VALUE_EXPR,
        "rhs_expression" => SyntaxKind::RHS_EXPRESSION,
        "elements_expression" => SyntaxKind::ELEMENTS_EXPRESSION,
        "identifier_expression" => SyntaxKind::IDENTIFIER_EXPRESSION,
        "number_expression" => SyntaxKind::NUMBER_EXPRESSION,
        "string_expression" => SyntaxKind::STRING_EXPRESSION,
        "boolean_expression" => SyntaxKind::BOOLEAN_EXPRESSION,
        "null_expression" => SyntaxKind::NULL_EXPRESSION,
        "member_expression" => SyntaxKind::MEMBER_EXPRESSION,
        "member_access_expression" => SyntaxKind::MEMBER_ACCESS_EXPRESSION,
        "call_expression" => SyntaxKind::CALL_EXPRESSION,
        "unary_expression" => SyntaxKind::UNARY_EXPRESSION,
        "prefix_unary_expression" => SyntaxKind::PREFIX_UNARY_EXPRESSION,
        "binary_expression" => SyntaxKind::BINARY_EXPRESSION,
        "ternary_expression" => SyntaxKind::TERNARY_EXPRESSION,
        "conditional_expression" => SyntaxKind::CONDITIONAL_EXPRESSION,
        "sequence_expression" => SyntaxKind::SEQUENCE_EXPRESSION,
        "interpolation_expression" => SyntaxKind::INTERPOLATION_EXPRESSION,
        "embed_interpolation_expression" => SyntaxKind::EMBED_INTERPOLATION_EXPRESSION,
        "parenthesized_expression" => SyntaxKind::PARENTHESIZED_EXPRESSION,
        "value_if_expression" => SyntaxKind::VALUE_IF_EXPRESSION,
        "elements_if_expression" => SyntaxKind::ELEMENTS_IF_EXPRESSION,
        "property_list_if_expression" => SyntaxKind::PROPERTY_LIST_IF_EXPRESSION,
        "value_for_expression" => SyntaxKind::VALUE_FOR_EXPRESSION,
        "elements_for_expression" => SyntaxKind::ELEMENTS_FOR_EXPRESSION,
        "value_match_expression" => SyntaxKind::VALUE_MATCH_EXPRESSION,
        "elements_match_expression" => SyntaxKind::ELEMENTS_MATCH_EXPRESSION,
        "pattern" => SyntaxKind::PATTERN,
        "literal" => SyntaxKind::LITERAL,
        "string_literal" => SyntaxKind::STRING_LITERAL,
        "number_literal" => SyntaxKind::NUMBER_LITERAL,
        "int_literal" => SyntaxKind::INT_LITERAL,
        "real_literal" => SyntaxKind::REAL_LITERAL,
        "hex_literal" => SyntaxKind::HEX_LITERAL,
        "unit_literal" => SyntaxKind::UNIT_LITERAL,
        "boolean_literal" => SyntaxKind::BOOLEAN_LITERAL,
        "bool_literal" => SyntaxKind::BOOL_LITERAL,
        "null_literal" => SyntaxKind::NULL_LITERAL,
        "identifier" => SyntaxKind::IDENTIFIER,
        "qualified_name" => SyntaxKind::QUALIFIED_NAME,
        "qualified_markup_name" => SyntaxKind::QUALIFIED_MARKUP_NAME,
        "element_name" => SyntaxKind::ELEMENT_NAME,
        "markup_identifier" => SyntaxKind::MARKUP_IDENTIFIER,
        "escaped_lbrace" => SyntaxKind::ESCAPED_LBRACE,
        "escaped_rbrace" => SyntaxKind::ESCAPED_RBRACE,
        "escaped_at" => SyntaxKind::ESCAPED_AT,
        "line_comment" => SyntaxKind::LINE_COMMENT,
        "block_comment" => SyntaxKind::BLOCK_COMMENT,
        "html_block_comment" => SyntaxKind::HTML_BLOCK_COMMENT,
        "let" => SyntaxKind::LET,
        // Note: "type" already matched earlier as TYPE
        "import" => SyntaxKind::IMPORT,
        "enum" => SyntaxKind::ENUM,
        "if" => SyntaxKind::IF,
        "else" => SyntaxKind::ELSE,
        "for" => SyntaxKind::FOR,
        "in" => SyntaxKind::IN,
        "match" => SyntaxKind::MATCH,
        "true" => SyntaxKind::TRUE,
        "false" => SyntaxKind::FALSE,
        "null" => SyntaxKind::NULL_KW,
        "+" => SyntaxKind::PLUS,
        "-" => SyntaxKind::MINUS,
        "*" => SyntaxKind::STAR,
        "/" => SyntaxKind::SLASH,
        "%" => SyntaxKind::PERCENT,
        "==" => SyntaxKind::EQ_EQ,
        "!=" => SyntaxKind::BANG_EQ,
        "<" => SyntaxKind::LT,
        ">" => SyntaxKind::GT,
        "<=" => SyntaxKind::LT_EQ,
        ">=" => SyntaxKind::GT_EQ,
        "&&" => SyntaxKind::AMP_AMP,
        "||" => SyntaxKind::PIPE_PIPE,
        "|" => SyntaxKind::PIPE,
        "!" => SyntaxKind::BANG,
        "?" => SyntaxKind::QUESTION,
        "=>" => SyntaxKind::FAT_ARROW,
        ":" => SyntaxKind::COLON,
        "(" => SyntaxKind::LPAREN,
        ")" => SyntaxKind::RPAREN,
        "{" => SyntaxKind::LBRACE,
        "}" => SyntaxKind::RBRACE,
        "[" => SyntaxKind::LBRACKET,
        "]" => SyntaxKind::RBRACKET,
        "," => SyntaxKind::COMMA,
        "." => SyntaxKind::DOT,
        "=" => SyntaxKind::EQ,
        _ => SyntaxKind::ERROR,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_token() {
        assert!(SyntaxKind::IDENTIFIER.is_token());
        assert!(SyntaxKind::STRING_LITERAL.is_token());
        assert!(!SyntaxKind::FUNCTION_DEFINITION.is_token());
    }

    #[test]
    fn test_is_keyword() {
        assert!(SyntaxKind::LET.is_keyword());
        assert!(SyntaxKind::IF.is_keyword());
        assert!(!SyntaxKind::IDENTIFIER.is_keyword());
    }

    #[test]
    fn test_is_comment() {
        assert!(SyntaxKind::LINE_COMMENT.is_comment());
        assert!(SyntaxKind::BLOCK_COMMENT.is_comment());
        assert!(!SyntaxKind::IDENTIFIER.is_comment());
    }

    #[test]
    fn test_syntax_kind_from_str() {
        assert_eq!(
            syntax_kind_from_str("module_definition"),
            SyntaxKind::MODULE_DEFINITION
        );
        assert_eq!(syntax_kind_from_str("identifier"), SyntaxKind::IDENTIFIER);
        assert_eq!(syntax_kind_from_str("unknown"), SyntaxKind::ERROR);
    }
}
