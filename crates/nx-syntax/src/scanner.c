/**
 * External scanner for NX language text content tokens.
 *
 * This scanner handles text content inside markup elements, including:
 * - TEXT_CHUNK: sequences of text characters
 * - ENTITY: HTML/XML entities like &amp; &#10;
 * - ESCAPED_LBRACE: \{
 * - ESCAPED_RBRACE: \}
 *
 * Only backslash-brace pairs are treated as escapes; any other backslash
 * sequence is left as literal text.
 *
 * Context Detection Strategy:
 * - The scanner doesn't maintain state between calls
 * - Instead, it relies on the valid_symbols array from the parser
 * - When TEXT_CHUNK is valid, we're in element content context
 * - This stateless approach is simpler and avoids serialization complexity
 */

#include <tree_sitter/parser.h>
#include <wctype.h>
#include <string.h>
#include <stdbool.h>

enum TokenType {
  TEXT_CHUNK,
  ENTITY,
  ESCAPED_LBRACE,
  ESCAPED_RBRACE,
};

void *tree_sitter_nx_external_scanner_create() {
  return NULL;
}

void tree_sitter_nx_external_scanner_destroy(void *payload) {
}

unsigned tree_sitter_nx_external_scanner_serialize(void *payload, char *buffer) {
  return 0;
}

void tree_sitter_nx_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {
}

/**
 * Check if we're at the start of an HTML/XML entity.
 * Entities: &name; or &#digits; or &#xhex;
 */
static bool is_entity_start(TSLexer *lexer) {
  if (lexer->lookahead != '&') return false;

  // Peek ahead to see if it looks like an entity
  lexer->advance(lexer, false);

  if (lexer->lookahead == '#') {
    lexer->advance(lexer, false);
    // Numeric entity: &#10; or &#x0A;
    if (lexer->lookahead == 'x' || lexer->lookahead == 'X') {
      lexer->advance(lexer, false);
      return (lexer->lookahead >= '0' && lexer->lookahead <= '9') ||
             (lexer->lookahead >= 'a' && lexer->lookahead <= 'f') ||
             (lexer->lookahead >= 'A' && lexer->lookahead <= 'F');
    }
    return lexer->lookahead >= '0' && lexer->lookahead <= '9';
  }

  // Named entity: &amp; &lt; etc.
  return (lexer->lookahead >= 'a' && lexer->lookahead <= 'z') ||
         (lexer->lookahead >= 'A' && lexer->lookahead <= 'Z');
}

/**
 * Scan an HTML/XML entity.
 * Returns true if a complete entity was scanned.
 */
static bool scan_entity(TSLexer *lexer) {
  if (lexer->lookahead != '&') return false;

  lexer->advance(lexer, false);

  if (lexer->lookahead == '#') {
    // Numeric entity
    lexer->advance(lexer, false);

    if (lexer->lookahead == 'x' || lexer->lookahead == 'X') {
      // Hex entity: &#xHHHH;
      lexer->advance(lexer, false);
      if (!((lexer->lookahead >= '0' && lexer->lookahead <= '9') ||
            (lexer->lookahead >= 'a' && lexer->lookahead <= 'f') ||
            (lexer->lookahead >= 'A' && lexer->lookahead <= 'F'))) {
        return false;
      }
      while ((lexer->lookahead >= '0' && lexer->lookahead <= '9') ||
             (lexer->lookahead >= 'a' && lexer->lookahead <= 'f') ||
             (lexer->lookahead >= 'A' && lexer->lookahead <= 'F')) {
        lexer->advance(lexer, false);
      }
    } else {
      // Decimal entity: &#DDDD;
      if (!(lexer->lookahead >= '0' && lexer->lookahead <= '9')) {
        return false;
      }
      while (lexer->lookahead >= '0' && lexer->lookahead <= '9') {
        lexer->advance(lexer, false);
      }
    }
  } else {
    // Named entity: &name;
    if (!((lexer->lookahead >= 'a' && lexer->lookahead <= 'z') ||
          (lexer->lookahead >= 'A' && lexer->lookahead <= 'Z'))) {
      return false;
    }
    while ((lexer->lookahead >= 'a' && lexer->lookahead <= 'z') ||
           (lexer->lookahead >= 'A' && lexer->lookahead <= 'Z') ||
           (lexer->lookahead >= '0' && lexer->lookahead <= '9')) {
      lexer->advance(lexer, false);
    }
  }

  // Must end with semicolon
  if (lexer->lookahead == ';') {
    lexer->advance(lexer, false);
    return true;
  }

  return false;
}

/**
 * Scanner for NX text content tokens.
 *
 * The scanner is invoked when the parser encounters content inside elements.
 * It handles four token types that represent different forms of text content.
 *
 * Token Priority (highest to lowest):
 * 1. ESCAPED_LBRACE: \{
 * 2. ESCAPED_RBRACE: \}
 * 3. ENTITY: &name; &#10; &#x0A;
 * 4. TEXT_CHUNK: any other text
 *
 * The scanner stops at:
 * - '<' (start of element or close tag)
 * - '{' (start of interpolation expression)
 * - '&' (might be entity or part of text)
 * - '\' followed by '{' or '}' (escaped brace)
 */
bool tree_sitter_nx_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid_symbols) {
  // Note: We do NOT skip leading whitespace here because whitespace is significant
  // in text content. The grammar handles whitespace in the "extras" section.

  // Check for escaped braces first (highest priority)
  if (lexer->lookahead == '\\') {
    lexer->mark_end(lexer);
    lexer->advance(lexer, false);

    if (lexer->lookahead == '{' && valid_symbols[ESCAPED_LBRACE]) {
      lexer->advance(lexer, false);
      lexer->result_symbol = ESCAPED_LBRACE;
      return true;
    }

    if (lexer->lookahead == '}' && valid_symbols[ESCAPED_RBRACE]) {
      lexer->advance(lexer, false);
      lexer->result_symbol = ESCAPED_RBRACE;
      return true;
    }

    // Not an escaped brace, backtrack and handle as text
    // (Note: tree-sitter doesn't support backtracking, so we'll
    // include the backslash in the text chunk below)
  }

  // Check for entities
  if (lexer->lookahead == '&' && valid_symbols[ENTITY]) {
    // Save position in case entity scan fails
    if (scan_entity(lexer)) {
      lexer->result_symbol = ENTITY;
      return true;
    }
    // If entity scan failed, fall through to text chunk
  }

  // Scan text chunk
  if (valid_symbols[TEXT_CHUNK]) {
    bool has_content = false;

    // Consume characters until we hit a delimiter
    while (lexer->lookahead != 0) {
      // Stop at element delimiters
      if (lexer->lookahead == '<' || lexer->lookahead == '>') {
        break;
      }

      // Stop at interpolation start (but not escaped brace)
      if (lexer->lookahead == '{') {
        break;
      }

      // Stop at closing brace (might be end of interpolation)
      if (lexer->lookahead == '}') {
        break;
      }

      // Stop at backslash if next char is brace (let escaped brace scanner handle it)
      if (lexer->lookahead == '\\') {
        lexer->mark_end(lexer);
        lexer->advance(lexer, false);
        if (lexer->lookahead == '{' || lexer->lookahead == '}') {
          // Don't include the backslash, let escaped brace scanner handle it
          if (has_content) {
            lexer->result_symbol = TEXT_CHUNK;
            return true;
          }
          return false;
        }
        // Include the backslash in text (it's not escaping a brace)
        has_content = true;
        continue;
      }

      // Stop at entity start (let entity scanner handle it)
      if (lexer->lookahead == '&') {
        // Check if it looks like an entity
        TSLexer saved = *lexer;
        if (is_entity_start(lexer)) {
          // Restore lexer position
          *lexer = saved;
          if (has_content) {
            lexer->result_symbol = TEXT_CHUNK;
            return true;
          }
          return false;
        }
        // Not an entity, restore and include in text
        *lexer = saved;
      }

      // Include this character in text chunk
      lexer->advance(lexer, false);
      has_content = true;
      lexer->mark_end(lexer);
    }

    if (has_content) {
      lexer->result_symbol = TEXT_CHUNK;
      return true;
    }
  }

  return false;
}
