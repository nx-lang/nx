;; NX Syntax Highlighting Queries
;; For use with tree-sitter

;; Keywords
[
  "let"
  "type"
  "enum"
  "import"
  "if"
  "else"
  "for"
  "in"
  "is"
  "raw"
] @keyword

;; Types
(primitive_type) @type.builtin

(user_defined_type
  (qualified_name
    (identifier) @type))

(enum_definition
  name: (identifier) @type)

(record_definition
  name: (identifier) @type)

;; Variables
(value_definition
  name: (identifier) @variable)

;; Functions
(function_definition
  name: (element_name) @function)

(function_definition
  name: (identifier) @function)

;; Parameters
(function_definition
  (property_definition
    name: (markup_identifier) @variable.parameter))

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
  "=>"
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
(int_literal) @number
(real_literal) @number
(hex_literal) @number
(bool_literal) @constant.builtin
(null_literal) @constant.builtin

;; Identifiers
(identifier_expression
  (identifier) @variable)

;; Property names
(property_value
  name: (qualified_markup_name) @property)

(record_definition
  (property_definition
    name: (markup_identifier) @property))

;; Element tags
(element
  name: (element_name) @tag)

(element
  close_name: (element_name) @tag)

(text_child_element
  name: (element_name) @tag)

(text_child_element
  close_name: (element_name) @tag)

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
