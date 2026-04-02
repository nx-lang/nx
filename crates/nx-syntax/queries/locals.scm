;; NX Scope Analysis Queries
;; Defines scopes and local variable bindings

;; Scopes
(function_definition) @local.scope
(component_definition) @local.scope
(value_for_expression) @local.scope
(elements_for_expression) @local.scope

;; Definitions
(function_definition
  name: (element_name
    (qualified_markup_name
      (identifier) @local.definition)))

(function_definition
  name: (identifier) @local.definition)

(component_signature
  name: (element_name
    (qualified_markup_name
      (identifier) @local.definition)))

(function_definition
  (property_definition
    name: (markup_identifier) @local.definition))

(component_signature
  (property_definition
    name: (markup_identifier) @local.definition))

(state_group
  (property_definition
    name: (markup_identifier) @local.definition))

(type_definition
  name: (identifier) @local.definition)

(action_definition
  name: (identifier) @local.definition)

(enum_definition
  name: (identifier) @local.definition)

(record_definition
  name: (identifier) @local.definition)

(value_definition
  name: (identifier) @local.definition)

(wildcard_import
  alias: (identifier) @local.definition)

((selective_import
   name: (identifier) @local.definition) @import.binding
  (#not-match? @import.binding "\\bas\\b"))

(selective_import
  alias: (qualified_name
    (identifier) @local.definition))

(value_for_expression
  item: (identifier) @local.definition)

(value_for_expression
  index: (identifier) @local.definition)

(elements_for_expression
  item: (identifier) @local.definition)

(elements_for_expression
  index: (identifier) @local.definition)

;; References
(identifier_expression
  (identifier) @local.reference)

(user_defined_type
  (qualified_name
    (identifier) @local.reference))

(emit_reference
  (qualified_name
    (identifier) @local.reference))
