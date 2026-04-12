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
        $.type_definition,
        $.enum_definition,
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

    enum_definition: $ => seq(
      optional(field('visibility', $.visibility_modifier)),
      'enum',
      field('name', $.identifier),
      '=',
      field('members', $.enum_member_list),
    ),

    enum_member_list: $ => seq(
      optional('|'),
      $.enum_member,
      repeat(seq('|', $.enum_member)),
    ),

    enum_member: $ => field('name', $.identifier),

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

    type: $ => seq(
      choice(
        $.primitive_type,
        $.user_defined_type,
      ),
      optional(choice(
        '?',          // nullable
        seq('[', ']'), // sequence/list
      )),
    ),

    primitive_type: $ => choice(
      'string',
      'i32',
      'i64',
      'int',
      'f32',
      'f64',
      'float',
      'bool',
      'void',
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
      'component',
      field('signature', $.component_signature),
      '=',
      field('body', $.component_body),
    ),

    component_signature: $ => seq(
      '<',
      field('name', $.element_name),
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
      optional(field('state', $.state_group)),
      field('body', $.value_expression),
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
    _component_property_definition: $ => choice(
      seq(
        field('name', alias($._component_field_name, $.markup_identifier)),
        ':',
        field('type', $.type),
        optional(seq(
          '=',
          field('default', $.rhs_expression),
        )),
      ),
      seq(
        field('modifier', alias($._component_field_name, $.markup_identifier)),
        field('name', alias($._component_field_name, $.markup_identifier)),
        ':',
        field('type', $.type),
        optional(seq(
          '=',
          field('default', $.rhs_expression),
        )),
      ),
    ),

    _component_field_name: $ => choice(
      $.identifier,
      $.markup_identifier,
    ),

    property_definition: $ => choice(
      seq(
        field('name', $.markup_identifier),
        ':',
        field('type', $.type),
        optional(seq(
          '=',
          field('default', $.rhs_expression),
        )),
      ),
      seq(
        field('modifier', $.markup_identifier),
        field('name', $.markup_identifier),
        ':',
        field('type', $.type),
        optional(seq(
          '=',
          field('default', $.rhs_expression),
        )),
      ),
    ),

    // ===== Expressions =====
    rhs_expression: $ => choice(
      $.element,
      $.literal,
      $.values_braced_expression,
    ),

    values_braced_expression: $ => seq(
      '{',
      choice(
        prec.dynamic(2, $.value_expression),
        prec.dynamic(1, $._value_list_expression),
      ),
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
      $.literal,
      $.identifier_expression,
      $.unit_literal,
      $.parenthesized_expression,
    ),

    value_expression: $ => choice(
      $.value_list_item_expression,
      $.conditional_expression,
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

    conditional_expression: $ => prec.right(20, seq(
      field('condition', $.value_expression),
      '?',
      field('consequent', $.value_expression),
      ':',
      field('alternative', $.value_expression),
    )),

    binary_expression: $ => {
      const operators = [
        [prec.left, 120, '*'],
        [prec.left, 120, '/'],
        [prec.left, 120, '%'],
        [prec.left, 110, '+'],
        [prec.left, 110, '-'],
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
        $.value_expression,
        repeat(seq(',', $.value_expression)),
      )),
      ')',
    )),

    member_access_expression: $ => prec.left(140, seq(
      field('target', $.value_expression),
      '.',
      field('member', $.identifier),
    )),

    // ===== Literals =====
    literal: $ => choice(
      $.string_literal,
      $.int_literal,
      $.real_literal,
      $.hex_literal,
      $.bool_literal,
      $.null_literal,
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
    null_literal: $ => 'null',

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
              field('content', $.mixed_content),
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
    pattern: $ => choice(
      $.literal,
      $.qualified_name,
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
