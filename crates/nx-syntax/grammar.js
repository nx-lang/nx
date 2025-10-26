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
    $.entity,
    $.escaped_lbrace,
    $.escaped_rbrace,
  ],

  conflicts: $ => [
    // Disambiguate contexts where 'if' can start different expression types
    [$.value_if_expression, $.elements_if_expression],
    [$.property_list_if_expression],
    // Elements vs mixed content - disambiguated by presence of text
    [$.elements_expression, $.mixed_content],
    // Value expression can contain element
    [$.value_expression, $.elements_expression],
    // Recursive content expressions
    [$.elements_expression],
    [$.mixed_content],
    [$.text_run],
    [$.property_list],
    // Property list if arms vs value if arms
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
    // Elements if arms
    [$.elements_if_condition_arm],
    [$.elements_if_match_arm],
    [$.elements_if_simple_expression],
    [$.elements_if_match_expression],
    [$.elements_if_condition_list_expression],
    // Identifier as expression vs property name
    [$.identifier_expression, $.qualified_markup_name],
    [$.identifier_expression, $.qualified_name],
  ],

  word: $ => $.identifier,

  rules: {
    // ===== Module Definition =====
    module_definition: $ => seq(
      repeat($.import_statement),
      choice(
        seq(
          repeat($.type_definition),
          repeat($.function_definition),
        ),
        $.element,
      ),
    ),

    // ===== Imports =====
    import_statement: $ => seq(
      'import',
      $.qualified_name,
    ),

    // ===== Type Definitions =====
    type_definition: $ => seq(
      'type',
      field('name', $.identifier),
      '=',
      field('type', $.type),
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
      'int',
      'long',
      'float',
      'double',
      'boolean',
      'void',
      'object',
    ),

    user_defined_type: $ => $.qualified_name,

    // ===== Function Definitions =====
    function_definition: $ => seq(
      'let',
      '<',
      field('name', $.element_name),
      repeat($.property_definition),
      '/',
      '>',
      '=',
      field('body', $.rhs_expression),
    ),

    property_definition: $ => seq(
      field('name', $.markup_identifier),
      ':',
      field('type', $.type),
      optional(seq(
        '=',
        field('default', $.rhs_expression),
      )),
    ),

    // ===== Expressions =====
    rhs_expression: $ => choice(
      $.element,
      $.literal,
      $.interpolation_expression,
    ),

    interpolation_expression: $ => seq(
      '{',
      $.value_expression,
      '}',
    ),

    value_expression: $ => choice(
      $.element,
      $.value_if_expression,
      $.value_for_expression,
      $.value_expr,
    ),

    // Pratt-parsed expressions
    value_expr: $ => choice(
      // Primary expressions
      $.literal,
      $.identifier_expression,
      $.unit_literal,
      $.parenthesized_expression,

      // Binary expressions
      $.conditional_expression,
      $.binary_expression,
      $.prefix_unary_expression,

      // Postfix expressions
      $.call_expression,
      $.member_access_expression,
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
      field('operator', '-'),
      field('operand', $.value_expression),
    )),

    call_expression: $ => prec.left(140, seq(
      field('callee', $.value_expression),
      '(',
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
      '{',
      field('then', $.value_expression),
      '}',
      optional(seq(
        'else',
        '{',
        field('else', $.value_expression),
        '}',
      )),
    ),

    value_if_match_expression: $ => seq(
      'if',
      optional(field('scrutinee', $.value_expression)),
      'is',
      '{',
      repeat1($.value_if_match_arm),
      optional(seq(
        'else',
        ':',
        field('else', $.value_expression),
      )),
      '}',
    ),

    value_if_match_arm: $ => seq(
      $.pattern,
      repeat(seq(',', $.pattern)),
      ':',
      $.value_expression,
    ),

    value_if_condition_list_expression: $ => seq(
      'if',
      optional(field('scrutinee', $.value_expression)),
      '{',
      repeat1($.value_if_condition_arm),
      optional(seq(
        'else',
        ':',
        field('else', $.value_expression),
      )),
      '}',
    ),

    value_if_condition_arm: $ => seq(
      field('condition', $.value_expression),
      ':',
      field('body', $.value_expression),
    ),

    // ===== Value For Expressions =====
    value_for_expression: $ => seq(
      'for',
      field('item', $.identifier),
      optional(seq(',', field('index', $.identifier))),
      'in',
      field('iterable', $.value_expression),
      '{',
      field('body', $.value_expression),
      '}',
    ),

    // ===== Elements Expression =====
    elements_expression: $ => repeat1(choice(
      $.element,
      $.elements_if_expression,
      $.elements_for_expression,
    )),

    elements_if_expression: $ => choice(
      $.elements_if_simple_expression,
      $.elements_if_match_expression,
      $.elements_if_condition_list_expression,
    ),

    elements_if_simple_expression: $ => seq(
      'if',
      field('condition', $.value_expression),
      '{',
      field('then', $.elements_expression),
      '}',
      optional(seq(
        'else',
        '{',
        field('else', $.elements_expression),
        '}',
      )),
    ),

    elements_if_match_expression: $ => seq(
      'if',
      optional(field('scrutinee', $.value_expression)),
      'is',
      '{',
      repeat1($.elements_if_match_arm),
      optional(seq(
        'else',
        ':',
        field('else', $.elements_expression),
      )),
      '}',
    ),

    elements_if_match_arm: $ => seq(
      $.pattern,
      repeat(seq(',', $.pattern)),
      ':',
      $.elements_expression,
    ),

    elements_if_condition_list_expression: $ => seq(
      'if',
      optional(field('scrutinee', $.value_expression)),
      '{',
      repeat1($.elements_if_condition_arm),
      optional(seq(
        'else',
        ':',
        field('else', $.elements_expression),
      )),
      '}',
    ),

    elements_if_condition_arm: $ => seq(
      field('condition', $.value_expression),
      ':',
      field('body', $.elements_expression),
    ),

    // ===== Elements For Expression =====
    elements_for_expression: $ => seq(
      'for',
      field('item', $.identifier),
      optional(seq(',', field('index', $.identifier))),
      'in',
      field('iterable', $.value_expression),
      '{',
      field('body', $.elements_expression),
      '}',
    ),

    // ===== Elements (Markup) =====
    element: $ => seq(
      '<',
      field('name', $.element_name),
      choice(
        // Regular element
        seq(
          field('properties', optional($.property_list)),
          choice(
            seq('/', '>'),  // self-closing
            seq(
              '>',
              field('content', $.content),
              '<',
              '/',
              field('close_name', $.element_name),
              '>',
            ),
          ),
        ),
        // Embed element
        seq(
          ':',
          field('text_type', $.identifier),
          optional('raw'),
          field('properties', optional($.property_list)),
          '>',
          field('content', $.embed_content),
          '<',
          '/',
          field('close_name', $.element_name),
          '>',
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
      optional(field('scrutinee', $.value_expression)),
      'is',
      '{',
      repeat1($.property_list_if_match_arm),
      optional(seq(
        'else',
        ':',
        field('else', optional($.property_list)),
      )),
      '}',
    ),

    property_list_if_match_arm: $ => seq(
      $.pattern,
      repeat(seq(',', $.pattern)),
      ':',
      optional($.property_list),
    ),

    property_list_if_condition_list_expression: $ => seq(
      'if',
      optional(field('scrutinee', $.value_expression)),
      '{',
      repeat1($.property_list_if_condition_arm),
      optional(seq(
        'else',
        ':',
        field('else', optional($.property_list)),
      )),
      '}',
    ),

    property_list_if_condition_arm: $ => seq(
      field('condition', $.value_expression),
      ':',
      field('body', optional($.property_list)),
    ),

    // ===== Content =====
    content: $ => choice(
      $.elements_expression,
      $.mixed_content,
    ),

    mixed_content: $ => repeat1(choice(
      $.text_part,
      $.element,
      $.interpolation_expression,
    )),

    text_part: $ => $.text_run,

    text_run: $ => repeat1(choice(
      $.text_chunk,
      $.entity,
      $.escaped_lbrace,
      $.escaped_rbrace,
    )),

    // ===== Embed Content =====
    embed_content: $ => repeat1(choice(
      $.text_run,
      $.interpolation_expression,
    )),

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
