/**
 * NX Language Grammar for tree-sitter
 *
 * This grammar implements the NX language specification from nx-grammar-spec.md.
 * NX is a markup-centric language combining XML-like elements with expressions and logic.
 */

module.exports = grammar({
  name: 'nx',

  extras: $ => [
    /\s/,
    $.line_comment,
    $.block_comment,
    $.html_block_comment,
  ],

  externals: $ => [
    $.text_chunk,
    $.embed_text_chunk,
    $.entity,
    $.escaped_lbrace,
    $.escaped_rbrace,
    $.escaped_at,
  ],

  conflicts: $ => [
    [$.value_if_expression, $.elements_if_expression],
    [$.property_list_if_expression],
    [$.value_expression, $.elements_expression],
    [$.value_expression, $.value_list_item_expression],
    [$.value_expression, $._value_list_expression],
    [$.elements_expression],
    [$.mixed_content],
    [$.property_list],
    [$.property_list_if_condition_arm],
    [$.property_list_if_match_arm],
    [$.property_list_if_simple_expression],
    [$.property_list_if_match_expression],
    [$.property_list_if_condition_list_expression],
    [$.value_if_condition_arm],
    [$.value_if_match_arm],
    [$.value_if_simple_expression],
    [$.value_if_match_expression],
    [$.value_if_condition_list_expression],
    [$.value_if_condition_arm, $.property_list_if_condition_arm],
    [$.value_if_match_arm, $.property_list_if_match_arm],
    [$.value_if_simple_expression, $.property_list_if_simple_expression],
    [$.elements_if_condition_arm],
    [$.elements_if_match_arm],
    [$.elements_if_simple_expression],
    [$.elements_if_match_expression],
    [$.elements_if_condition_list_expression],
    [$.text_run],
    [$.text_child_element],
    [$.text_content],
    [$.embed_text_run],
    [$.identifier_expression, $.qualified_markup_name],
    [$.identifier_expression, $.qualified_name],
    [$.value_definition, $.function_definition],
    [$.property_definition],
    [$._component_property_definition],
  ],

  word: $ => $.identifier,

  rules: {
    // ===== Module Definition =====
    module_definition: $ => seq(
      repeat($.import_statement),
      repeat(choice(
        $.record_definition,
        $.action_definition,
        $.union_definition,
        $.type_definition,
        $.value_definition,
        $.function_definition,
        $.component_definition,
      )),
      optional($.element),
    ),

    // ===== Imports =====
    import_statement: $ => choice(
      seq(
        'import',
        field('kind', $.wildcard_import),
      ),
      seq(
        'import',
        field('kind', $.selective_import_list),
        'from',
        field('path', $.library_path),
      ),
    ),

    wildcard_import: $ => seq(
      field('path', $.library_path),
      optional(seq(
        'as',
        field('alias', $.identifier),
      )),
    ),

    selective_import_list: $ => seq(
      '{',
      optional(seq(
        $.selective_import,
        repeat(seq(',', $.selective_import)),
        optional(','),
      )),
      '}',
    ),

    selective_import: $ => seq(
      field('name', $.identifier),
      optional(seq(
        'as',
        field('alias', $.qualified_name),
      )),
    ),

    // Semantic wrapper around string_literal so import paths have
    // a stable node kind for downstream lowering and queries.
    library_path: $ => seq(
      field('value', $.string_literal),
    ),

    visibility_modifier: $ => choice(
      'private',
      'export',
    ),

    // ===== Type Definitions =====
    record_definition: $ => seq(
      optional(field('visibility', $.visibility_modifier)),
      optional(field('abstract', 'abstract')),
      'type',
      field('name', $.identifier),
      optional(seq(
        'extends',
        field('base', $.qualified_name),
      )),
      '=',
      '{',
      repeat(field('properties', $.property_definition)),
      '}',
    ),

    action_definition: $ => seq(
      optional(field('visibility', $.visibility_modifier)),
      optional(field('abstract', 'abstract')),
      'action',
      field('name', $.identifier),
      optional(seq(
        'extends',
        field('base', $.qualified_name),
      )),
      '=',
      '{',
      repeat(field('properties', $.property_definition)),
      '}',
    ),

    type_definition: $ => seq(
      optional(field('visibility', $.visibility_modifier)),
      'type',
      field('name', $.identifier),
      '=',
      field('type', $.type),
    ),

    union_definition: $ => seq(
      optional(field('visibility', $.visibility_modifier)),
      'type',
      field('name', $.identifier),
      optional(seq(
        'extends',
        field('base', $.qualified_name),
      )),
      '=',
      field('cases', $.union_case_list),
    ),

    // The leading `|` is optional for a list of two or more cases, and required for a single
    // case. A single bare case would be ambiguous with `type_definition`'s alias form
    // (`type A = B`); two or more cannot be, because an alias's right-hand side is one `$.type`
    // and cannot contain `|`.
    union_case_list: $ => choice(
      repeat1($.union_case),
      seq(
        alias($._bare_union_case, $.union_case),
        repeat1($.union_case),
      ),
    ),

    union_case: $ => seq(
      '|',
      $._union_case_name_and_body,
    ),

    _bare_union_case: $ => $._union_case_name_and_body,

    _union_case_name_and_body: $ => seq(
      field('name', $.identifier),
      optional(seq(
        '{',
        repeat(field('properties', $.property_definition)),
        '}',
      )),
    ),

    // ===== Value Definitions =====
    value_definition: $ => seq(
      optional(field('visibility', $.visibility_modifier)),
      'let',
      field('name', $.identifier),
      optional(seq(
        ':',
        field('type', $.type),
      )),
      '=',
      field('value', $.rhs_expression),
    ),

    // A base type followed by at most one occurrence suffix: `?` zero or one, `+` one or more,
    // `*` zero or more. `prec.right` makes a suffix after a function type's result bind to the
    // result, so `<function />: string?` is a function returning `string?`; a suffix on the
    // function type itself is written `(<function />: string)?`. Every suffix is admitted here,
    // and `[]` still parses, so that post-parse validation can report a second suffix and a `[]`
    // by name rather than as a bare parse error.
    type: $ => prec.right(seq(
      choice(
        $.primitive_type,
        $.user_defined_type,
        $.function_type,
        $.applied_type,
        $.parenthesized_type,
      ),
      repeat(choice(
        '?',           // zero or one
        '+',           // one or more
        '*',           // zero or more
        seq('[', ']'), // removed; validation names `*` and `+`
      )),
    )),

    // A function type is an element function's signature with `function` in the name slot and
    // the result type after `/>`: `<function Item:Contact Index:int />: DrawnNode`. `function` is
    // a keyword only here: tree-sitter offers it to the lexer in this state alone, so the word
    // stays an identifier everywhere else. Parameters reuse `property_definition`; validation
    // rejects a default, a `type` parameter and a second `content` parameter.
    function_type: $ => seq(
      '<',
      'function',
      repeat(field('parameters', $.property_definition)),
      '/',
      '>',
      ':',
      field('result', $.type),
    ),

    parenthesized_type: $ => seq(
      '(',
      field('type', $.type),
      ')',
    ),

    // An applied type names one instantiation of a generic record, spelled as the element that
    // constructs it with only its type arguments: `<Range T=int/>`. The tag may be qualified, so
    // the update companion is named `<Range.Update T=int/>`. `repeat`, not `repeat1`, so a missing
    // argument is a type-checker diagnostic naming the parameter rather than a parse error.
    applied_type: $ => seq(
      '<',
      field('name', $.qualified_name),
      repeat(field('arguments', $.type_argument)),
      '/',
      '>',
    ),

    // A type argument binds a parameter by name to any type. A construction site writes the same
    // binding through `property_value`, where only a bare name reads unambiguously as a type.
    type_argument: $ => seq(
      field('name', $.identifier),
      '=',
      field('type', $.type),
    ),

    primitive_type: $ => choice(
      'string',
      'int',
      'int32',
      'int64',
      'float32',
      'float64',
      'boolean',
      'object',
    ),

    user_defined_type: $ => $.qualified_name,

    // ===== Function Definitions =====
    function_definition: $ => seq(
      optional(field('visibility', $.visibility_modifier)),
      'let',
      choice(
        seq(
          '<',
          field('name', $.element_name),
          repeat($.property_definition),
          '/',
          '>'
        ),
        seq(
          field('name', $.identifier),
          '(',
          optional(seq(
            $.property_definition,
            repeat(seq(',', $.property_definition)),
          )),
          ')',
        ),
      ),
      optional(seq(
        ':',
        field('return_type', $.type),
      )),
      '=',
      field('body', $.rhs_expression),
    ),

    component_definition: $ => seq(
      optional(field('visibility', $.visibility_modifier)),
      optional(field('abstract', 'abstract')),
      optional(field('external', 'external')),
      'component',
      field('signature', $.component_signature),
      optional(seq(
        '=',
        field('body', $.component_body),
      )),
    ),

    component_signature: $ => seq(
      '<',
      field('name', $.element_name),
      optional(seq(
        'extends',
        field('base', $.qualified_name),
      )),
      repeat(field('properties', alias($._component_property_definition, $.property_definition))),
      optional(field('emits', $.emits_group)),
      '/',
      '>',
    ),

    // Components require at least one emitted action when the emits block is
    // present, but each action payload may be empty.
    emits_group: $ => seq(
      'emits',
      '{',
      repeat1(field('entries', choice(
        $.emit_definition,
        $.emit_reference,
      ))),
      '}',
    ),

    emit_definition: $ => seq(
      field('name', $.identifier),
      optional(seq(
        'extends',
        field('base', $.qualified_name),
      )),
      '{',
      repeat(field('properties', alias($._component_property_definition, $.property_definition))),
      '}',
    ),

    emit_reference: $ => field('name', $.qualified_name),

    component_body: $ => seq(
      '{',
      choice(
        seq(
          field('state', $.state_group),
          optional(field('body', $.value_expression)),
        ),
        field('body', $.value_expression),
      ),
      '}',
    ),

    state_group: $ => seq(
      'state',
      '{',
      repeat(field('properties', alias($._component_property_definition, $.property_definition))),
      '}',
    ),

    // Reuse PROPERTY_DEFINITION nodes for components while accepting the token
    // stream tree-sitter produces in component signatures and nested emits/state
    // blocks. Using the shared property_definition rule directly regresses plain
    // identifier props like `text:string` under `component`.
    // The `type` keyword in type position declares a component type parameter (`TItem:type`).
    // The grammar accepts it in every property list; validation restricts it to leading
    // definitions of a component signature so the rejection can name the definition.
    _component_property_definition: $ => choice(
      seq(
        field('name', alias($._component_field_name, $.markup_identifier)),
        optional(field('optional', '?')),
        ':',
        field('type', $._property_type),
        optional(seq(
          '=',
          field('default', $.rhs_expression),
        )),
      ),
      seq(
        field('modifier', alias($._component_field_name, $.markup_identifier)),
        field('name', alias($._component_field_name, $.markup_identifier)),
        optional(field('optional', '?')),
        ':',
        field('type', $._property_type),
        optional(seq(
          '=',
          field('default', $.rhs_expression),
        )),
      ),
    ),

    _property_type: $ => choice(
      $.type,
      'type',
    ),

    _component_field_name: $ => choice(
      $.identifier,
      $.markup_identifier,
    ),

    // `name?:type` marks a property optional: it may be omitted at construction and reads as a
    // type that admits zero. The mark sits on the name, never in the type slot, so `?` and `*`
    // there are the type checker's to reject with the `name?:` fix-it.
    property_definition: $ => choice(
      seq(
        field('name', $.markup_identifier),
        optional(field('optional', '?')),
        ':',
        field('type', $._property_type),
        optional(seq(
          '=',
          field('default', $.rhs_expression),
        )),
      ),
      seq(
        field('modifier', $.markup_identifier),
        field('name', $.markup_identifier),
        optional(field('optional', '?')),
        ':',
        field('type', $._property_type),
        optional(seq(
          '=',
          field('default', $.rhs_expression),
        )),
      ),
    ),

    // ===== Expressions =====
    // An unbraced value is always a literal, never an expression. `literal` comes first so
    // `true` and `false` keep lexing as bool literals rather than contextual names.
    // `contextual_name` is a single identifier and deliberately never a qualified_name: admitting
    // `fit=Fit.cover` would also admit `fit=obj.field`, and the invariant would be gone.
    rhs_expression: $ => choice(
      $.element,
      $.literal,
      $.signed_numeric_literal,
      $.contextual_name,
      $.values_braced_expression,
    ),

    // A `-` directly before a numeric literal, accepted only where a literal is grammatically
    // required. Tokenization is unchanged and `prefix_unary_expression` is untouched, so binary
    // subtraction keeps its meaning in every expression context.
    signed_numeric_literal: $ => seq(
      '-',
      choice($.int_literal, $.real_literal, $.hex_literal),
    ),

    // An occurrence suffix may follow the name with no space, so `T=int?` at a construction site
    // parses and the checker can say that a type argument is exactly one value. Only the glued form
    // is admitted: an unbraced value is never an operator expression, and `a=n + 1` or
    // `a=c ? 1 : 2` is the author reaching for one, not writing a suffix.
    contextual_name: $ => seq(
      $.identifier,
      repeat(field('suffix', choice(
        token.immediate('?'),
        token.immediate('+'),
        token.immediate('*'),
      ))),
    ),

    // Zero items is admitted so the empty value has a spelling: `{}`. It stays off
    // `elements_braced_expression` and `embed_braced_expression`, which still require at least one
    // item; `_empty_braced_expression` admits it as an item and an operand.
    values_braced_expression: $ => seq(
      '{',
      optional(choice(
        prec.dynamic(2, $.value_expression),
        prec.dynamic(1, $._value_list_expression),
      )),
      '}',
    ),

    // Lists require at least two items so `{value}` stays on the singleton
    // `value_expression` path instead of becoming an ambiguous one-item list.
    _value_list_expression: $ => prec.dynamic(1, seq(
      $.value_list_item_expression,
      repeat1($.value_list_item_expression),
    )),

    value_list_item_expression: $ => choice(
      $.element,
      $.value_if_expression,
      $.value_for_expression,
      $.call_expression,
      $.member_access_expression,
      $.optional_member_expression,
      $.exists_expression,
      $.literal,
      $.identifier_expression,
      $.unit_literal,
      $.parenthesized_expression,
      alias($._empty_braced_expression, $.values_braced_expression),
    ),

    // The empty value `{}` is an item and an operand too: `{ "a" {} }`, `{ x == {} }`,
    // `if c { {} } else { 1 }`. Only the zero-item brace is admitted here, so a non-empty list
    // is still not an item of a list. It shares the `values_braced_expression` node, so lowering
    // reads it as the same empty value. Where a full braced value is also admitted, the lower
    // precedence lets that reading win.
    _empty_braced_expression: $ => prec(-1, seq('{', '}')),

    value_expression: $ => choice(
      $.value_list_item_expression,
      $.prefix_unary_expression,
      $.binary_expression,
    ),

    identifier_expression: $ => $.identifier,

    unit_literal: $ => seq('(', ')'),

    parenthesized_expression: $ => seq(
      '(',
      $.value_expression,
      ')',
    ),

    // There is no conditional operator: `c ? a : b` is written `if c { a } else { b }`, and a
    // `?` after an expression is the presence test. Validation reports the ternary shape with
    // that fix-it.
    binary_expression: $ => {
      const operators = [
        // `??` supplies a fallback for an empty value. It binds above arithmetic, so
        // `"a" + x ?? "b"` is `"a" + (x ?? "b")`, and associates to the right.
        [prec.right, 125, '??'],
        [prec.left, 120, '*'],
        [prec.left, 120, '/'],
        [prec.left, 120, '%'],
        [prec.left, 110, '+'],
        [prec.left, 110, '-'],
        // The range operators sit between the additive and relational levels, so an arithmetic
        // operand needs no parentheses and a comparison of two ranges reads left to right.
        [prec.left, 100, '..='],
        [prec.left, 100, '..'],
        [prec.left, 90, '<'],
        [prec.left, 90, '>'],
        [prec.left, 90, '<='],
        [prec.left, 90, '>='],
        [prec.left, 80, '=='],
        [prec.left, 80, '!='],
        [prec.left, 40, '&&'],
        [prec.left, 30, '||'],
      ];

      return choice(...operators.map(([assoc, precedence, operator]) =>
        assoc(precedence, seq(
          field('left', $.value_expression),
          field('operator', operator),
          field('right', $.value_expression),
        ))
      ));
    },

    prefix_unary_expression: $ => prec.right(130, seq(
      field('operator', choice('-', '!')),
      field('operand', $.value_expression),
    )),

    call_expression: $ => prec.left(140, seq(
      field('callee', $.value_expression),
      token.immediate('('),
      optional(seq(
        $._call_argument,
        repeat(seq(',', $._call_argument)),
      )),
      ')',
    )),

    // An argument may be a braced value, so a function is passed a list the same way a property
    // is bound one: `f({})`, `f({a})`, `f({a b})`. `value_list_item_expression` admits only the
    // empty brace, so a non-empty list is still not an item of a list. Hidden, so the argument's
    // own node reaches lowering directly.
    _call_argument: $ => choice(
      $.value_expression,
      $.values_braced_expression,
    ),

    member_access_expression: $ => prec.left(140, seq(
      field('target', $.value_expression),
      '.',
      field('member', $.identifier),
    )),

    // `x?.m` steps through a receiver that may be empty: `{}` when `x` is empty, `x.m` otherwise.
    // `?.` is one token, so the lexer's longest match keeps it from reading as a presence test
    // followed by `.`.
    optional_member_expression: $ => prec.left(140, seq(
      field('target', $.value_expression),
      '?.',
      field('member', $.identifier),
    )),

    // Postfix `?` tests presence: `x?` is `true` when `x` holds an item and `false` when it is
    // the empty value. It binds as tightly as member access.
    exists_expression: $ => prec.left(140, seq(
      field('operand', $.value_expression),
      '?',
    )),

    // ===== Literals =====
    literal: $ => choice(
      $.string_literal,
      $.int_literal,
      $.real_literal,
      $.hex_literal,
      $.bool_literal,
    ),

    string_literal: $ => token(seq(
      '"',
      repeat(choice(
        /[^"\\]/,
        seq('\\', /./)
      )),
      '"'
    )),

    int_literal: $ => /[0-9]+/,
    real_literal: $ => /[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?/,
    hex_literal: $ => /0[xX][0-9a-fA-F]+/,
    bool_literal: $ => choice('true', 'false'),

    // ===== Value If Expressions =====
    value_if_expression: $ => choice(
      $.value_if_simple_expression,
      $.value_if_match_expression,
      $.value_if_condition_list_expression,
    ),

    value_if_simple_expression: $ => seq(
      'if',
      field('condition', $.value_expression),
      field('then', $.values_braced_expression),
      optional(seq(
        'else',
        field('else', $.values_braced_expression),
      )),
    ),

    value_if_match_expression: $ => seq(
      'if',
      field('scrutinee', $.value_expression),
      'is',
      '{',
      repeat1($.value_if_match_arm),
      optional(seq(
        'else',
        '=>',
        field('else', choice(
          $.value_expression,
          $.values_braced_expression,
        )),
      )),
      '}',
    ),

    value_if_match_arm: $ => seq(
      $.pattern,
      repeat(seq(',', $.pattern)),
      '=>',
      field('body', choice(
        $.value_expression,
        $.values_braced_expression,
      )),
    ),

    value_if_condition_list_expression: $ => seq(
      'if',
      '{',
      repeat1($.value_if_condition_arm),
      optional(seq(
        'else',
        '=>',
        field('else', choice(
          $.value_expression,
          $.values_braced_expression,
        )),
      )),
      '}',
    ),

    value_if_condition_arm: $ => seq(
      field('condition', $.value_expression),
      '=>',
      field('body', choice(
        $.value_expression,
        $.values_braced_expression,
      )),
    ),

    // ===== Value For Expressions =====
    value_for_expression: $ => seq(
      'for',
      field('item', $.identifier),
      optional(seq(',', field('index', $.identifier))),
      'in',
      field('iterable', $.value_expression),
      field('body', $.values_braced_expression),
    ),

    // ===== Elements Expression =====
    // Keep bare if/for/else prefixes available for control-flow items in mixed content. Text
    // that would otherwise collide with those prefixes can still be written via braces.
    _mixed_text_run: $ => alias(token(prec(-1, /(?:[^\s<{ife][^<{]*|i[^f\s<{][^<{]*|if[^\s<{][^<{]*|f[^o\s<{][^<{]*|fo[^r\s<{][^<{]*|for[^\s<{][^<{]*|e[^l\s<{][^<{]*|el[^s\s<{][^<{]*|els[^e\s<{][^<{]*|else[^\s<{][^<{]*)/)), $.text_run),

    mixed_content: $ => repeat1(choice(
      $._mixed_text_run,
      $.element,
      $.elements_if_expression,
      $.elements_for_expression,
      $.values_braced_expression,
    )),

    elements_expression: $ => repeat1(choice(
      $.element,
      $.elements_if_expression,
      $.elements_for_expression,
      $.values_braced_expression,
    )),

    elements_braced_expression: $ => seq(
      '{',
      $.elements_expression,
      '}',
    ),

    elements_if_expression: $ => choice(
      $.elements_if_simple_expression,
      $.elements_if_match_expression,
      $.elements_if_condition_list_expression,
    ),

    elements_if_simple_expression: $ => seq(
      'if',
      field('condition', $.value_expression),
      field('then', $.elements_braced_expression),
      optional(seq(
        'else',
        field('else', $.elements_braced_expression),
      )),
    ),

    elements_if_match_expression: $ => seq(
      'if',
      field('scrutinee', $.value_expression),
      'is',
      '{',
      repeat1($.elements_if_match_arm),
      optional(seq(
        'else',
        '=>',
        field('else', choice(
          $.element,
          $.elements_braced_expression,
        )),
      )),
      '}',
    ),

    elements_if_match_arm: $ => seq(
      $.pattern,
      repeat(seq(',', $.pattern)),
      '=>',
      field('body', choice(
        $.element,
        $.elements_braced_expression,
      )),
    ),

    elements_if_condition_list_expression: $ => seq(
      'if',
      '{',
      repeat1($.elements_if_condition_arm),
      optional(seq(
        'else',
        '=>',
        field('else', choice(
          $.element,
          $.elements_braced_expression,
        )),
      )),
      '}',
    ),

    elements_if_condition_arm: $ => seq(
      field('condition', $.value_expression),
      '=>',
      field('body', choice(
        $.element,
        $.elements_braced_expression,
      )),
    ),

    // ===== Elements For Expression =====
    elements_for_expression: $ => seq(
      'for',
      field('item', $.identifier),
      optional(seq(',', field('index', $.identifier))),
      'in',
      field('iterable', $.value_expression),
      field('body', $.elements_braced_expression),
    ),

    // ===== Elements (Markup) =====
    element: $ => seq(
      '<',
      field('name', $.element_name),
      choice(
        seq(
          field('properties', optional($.property_list)),
          choice(
            seq('/', '>'),  // self-closing
            seq(
              '>',
              field('content', optional($.mixed_content)),
              '<',
              '/',
              field('close_name', $.element_name),
              '>',
            ),
          ),
        ),
        seq(
          ':',
          choice(
            seq(
              field('properties', optional($.property_list)),
              '>',
              field('content', $.text_content),
              '<',
              '/',
              field('close_name', $.element_name),
              '>'
            ),
            seq(
              'raw',
              field('properties', optional($.property_list)),
              '>',
              field('content', $.raw_text_run),
              '<',
              '/',
              field('close_name', $.element_name),
              '>'
            ),
            seq(
              field('text_type', $.identifier),
              choice(
                seq(
                  field('properties', optional($.property_list)),
                  '>',
                  field('content', $.embed_text_content),
                  '<',
                  '/',
                  field('close_name', $.element_name),
                  '>'
                ),
                seq(
                  'raw',
                  field('properties', optional($.property_list)),
                  '>',
                  field('content', $.raw_text_run),
                  '<',
                  '/',
                  field('close_name', $.element_name),
                  '>'
                ),
              ),
            ),
          ),
        ),
      ),
    ),

    element_name: $ => $.qualified_markup_name,

    // ===== Property Lists =====
    property_list: $ => repeat1(choice(
      $.property_value,
      $.property_list_if_expression,
    )),

    property_value: $ => seq(
      field('name', $.qualified_markup_name),
      '=',
      field('value', $.rhs_expression),
    ),

    property_list_if_expression: $ => choice(
      $.property_list_if_simple_expression,
      $.property_list_if_match_expression,
      $.property_list_if_condition_list_expression,
    ),

    property_list_if_simple_expression: $ => seq(
      'if',
      field('condition', $.value_expression),
      '{',
      field('then', optional($.property_list)),
      '}',
      optional(seq(
        'else',
        '{',
        field('else', optional($.property_list)),
        '}',
      )),
    ),

    property_list_if_match_expression: $ => seq(
      'if',
      field('scrutinee', $.value_expression),
      'is',
      '{',
      repeat1($.property_list_if_match_arm),
      optional(seq(
        'else',
        '=>',
        field('else', optional($.property_list)),
      )),
      '}',
    ),

    property_list_if_match_arm: $ => seq(
      $.pattern,
      repeat(seq(',', $.pattern)),
      '=>',
      optional($.property_list),
    ),

    property_list_if_condition_list_expression: $ => seq(
      'if',
      '{',
      repeat1($.property_list_if_condition_arm),
      optional(seq(
        'else',
        '=>',
        field('else', optional($.property_list)),
      )),
      '}',
    ),

    property_list_if_condition_arm: $ => seq(
      field('condition', $.value_expression),
      '=>',
      field('body', optional($.property_list)),
    ),

    // ===== Text Content =====
    text_content: $ => repeat1(choice(
      $.text_run,
      $.text_child_element,
      $.values_braced_expression,
    )),

    // TextChildElement allows nested elements inside text content
    // e.g., <p:>Hello <b>world</b>!</p>
    text_child_element: $ => seq(
      '<',
      field('name', $.element_name),
      field('properties', optional($.property_list)),
      choice(
        seq('/', '>'),  // self-closing
        seq(
          '>',
          field('content', $.text_content),
          '<',
          '/',
          field('close_name', $.element_name),
          '>',
        ),
      ),
    ),

    embed_text_content: $ => repeat1(choice(
      $.embed_text_run,
      $.embed_braced_expression,
    )),

    embed_braced_expression: $ => seq(
      '@{',
      choice(
        prec.dynamic(2, $.value_expression),
        prec.dynamic(1, $._value_list_expression),
      ),
      '}',
    ),

    text_run: $ => repeat1(choice(
      $.text_chunk,
      $.entity,
      $.escaped_lbrace,
      $.escaped_rbrace,
    )),

    embed_text_run: $ => repeat1(choice(
      $.embed_text_chunk,
      $.entity,
      $.escaped_lbrace,
      $.escaped_rbrace,
      $.escaped_at,
    )),

    raw_text_run: $ => repeat1($.raw_text_chunk),
    raw_text_chunk: $ => token(/[^<]+/),

    // ===== Patterns =====
    // `{}` matches the empty value, so `if x is { {} => ... else => ... }` tests absence.
    pattern: $ => choice(
      $.literal,
      $.signed_numeric_literal,
      $.qualified_name,
      seq('{', '}'),
    ),

    // ===== Names =====
    qualified_name: $ => seq(
      $.identifier,
      repeat(seq('.', $.identifier)),
    ),

    qualified_markup_name: $ => seq(
      $.identifier,
      repeat(seq('.', $.markup_identifier)),
    ),

    identifier: $ => /[a-zA-Z_][a-zA-Z0-9_]*/,
    markup_identifier: $ => /[a-zA-Z_][a-zA-Z0-9_\-]*/,

    // ===== Comments =====
    line_comment: $ => token(seq('//', /.*/)),

    block_comment: $ => token(seq(
      '/*',
      repeat(choice(
        /[^*]/,
        /\*[^/]/
      )),
      '*/'
    )),

    html_block_comment: $ => token(seq(
      '<!--',
      repeat(choice(
        /[^-]/,
        /-[^-]/,
        /--[^>]/
      )),
      '-->'
    )),
  }
});
