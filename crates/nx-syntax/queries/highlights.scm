;; NX Syntax Highlighting Queries
;; For use with tree-sitter

;; Keywords
[
  "let"
  "type"
  "import"
  "if"
  "else"
  "for"
  "in"
  "match"
] @keyword

;; Types
(primitive_type) @type.builtin

(user_defined_type
  (identifier) @type)

;; Functions
(function_definition
  name: (function_signature
    (markup_signature
      tag: (qualified_markup_name) @function)))

;; Parameters
(param
  name: (identifier) @variable.parameter)

;; Operators
[
  "+"
  "-"
  "*"
  "/"
  "%"
  "=="
  "!="
  "<"
  ">"
  "<="
  ">="
  "&&"
  "||"
  "!"
  "?"
  ":"
] @operator

;; Punctuation
[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "<"
  ">"
  "/"
] @punctuation.bracket

[
  ","
  "."
  ":"
  "="
] @punctuation.delimiter

;; Literals
(string_literal) @string
(number_literal) @number
(boolean_literal) @constant.builtin

;; Identifiers
(identifier_expression
  (identifier) @variable)

;; Property names
(property
  name: (identifier) @property)

;; Element tags
(element
  open_tag: (open_tag
    name: (qualified_markup_name) @tag))

(element
  close_tag: (close_tag
    name: (qualified_markup_name) @tag))

(self_closing_element
  name: (qualified_markup_name) @tag)

;; Comments
(line_comment) @comment
(block_comment) @comment
(html_block_comment) @comment

;; Qualified names
(qualified_name
  (identifier) @namespace
  "."
  (identifier) @variable)

;; Error nodes
(ERROR) @error
