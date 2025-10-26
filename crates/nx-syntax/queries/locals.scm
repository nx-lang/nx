;; NX Scope Analysis Queries
;; Defines scopes and local variable bindings

;; Scopes
(function_definition) @local.scope

;; Definitions
(function_definition
  name: (function_signature
    (markup_signature
      tag: (qualified_markup_name
        (identifier) @local.definition))))

(param
  name: (identifier) @local.definition)

(type_definition
  name: (identifier) @local.definition)

;; References
(identifier_expression
  (identifier) @local.reference)

(user_defined_type
  (identifier) @local.reference)
