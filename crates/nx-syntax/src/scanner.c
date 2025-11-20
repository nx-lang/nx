/**
 * External scanner for NX language text content tokens.
 *
 * This scanner handles text content inside markup elements, including:
 * - TEXT_CHUNK: sequences of text characters
 * - EMBED_TEXT_CHUNK: sequences of text characters inside typed text content
 * - ENTITY: HTML/XML entities like &amp; &#10;
 * - ESCAPED_LBRACE: \{
 * - ESCAPED_RBRACE: \}
 * - ESCAPED_AT: \@ (only used inside typed text content)
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
  EMBED_TEXT_CHUNK,
  ENTITY,
  ESCAPED_LBRACE,
  ESCAPED_RBRACE,
  ESCAPED_AT,
};

void *tree_sitter_nx_external_scanner_create() {
  return NULL;
}

void tree_sitter_nx_external_scanner_destroy(void *payload) {
  (void)payload; // unused
}

unsigned tree_sitter_nx_external_scanner_serialize(void *payload, char *buffer) {
  (void)payload; // unused
  (void)buffer;  // unused
  return 0;
}

void tree_sitter_nx_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {
  (void)payload; // unused
  (void)buffer;  // unused
  (void)length;  // unused
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
 * It handles tokens for both plain text content and typed text content.
 *
 * Token Priority (highest to lowest):
 * 1. ESCAPED_LBRACE: \{
 * 2. ESCAPED_RBRACE: \}
 * 3. ESCAPED_AT: \@ (typed text content only)
 * 4. ENTITY: &name; &#10; &#x0A;
 * 5. TEXT_CHUNK / EMBED_TEXT_CHUNK: any other text
 *
 * The scanner stops at:
 * - '<' (start of element or close tag)
 * - '{' / '}' (interpolation delimiters)
 * - '@{' (typed text interpolation delimiter)
 * - '&' (might be entity or part of text)
 * - '\' followed by '{', '}', or '@' (escaped delimiter)
 */
bool tree_sitter_nx_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid_symbols) {
  (void)payload; // unused
  // Note: We do NOT skip leading whitespace here because whitespace is significant
  // in text content. The grammar handles whitespace in the "extras" section.

  // No special handling for raw text here; raw content is handled by a regex token in the grammar.
  const bool allow_text_chunk = valid_symbols[TEXT_CHUNK];
  const bool allow_embed_text_chunk = valid_symbols[EMBED_TEXT_CHUNK];
  const bool allow_entity = valid_symbols[ENTITY];
  const bool allow_escaped_lbrace = valid_symbols[ESCAPED_LBRACE];
  const bool allow_escaped_rbrace = valid_symbols[ESCAPED_RBRACE];
  const bool allow_escaped_at = valid_symbols[ESCAPED_AT];
  const bool allow_any_chunk = allow_text_chunk || allow_embed_text_chunk;
  const enum TokenType chunk_kind =
      allow_embed_text_chunk ? EMBED_TEXT_CHUNK : TEXT_CHUNK;
  const bool embed_mode = chunk_kind == EMBED_TEXT_CHUNK;

  // Check for escapes first (highest priority)
  if (lexer->lookahead == '\\') {
    lexer->advance(lexer, false);

    if (lexer->lookahead == '{') {
      if (allow_escaped_lbrace) {
        lexer->advance(lexer, false);
        lexer->mark_end(lexer);
        lexer->result_symbol = ESCAPED_LBRACE;
        return true;
      }

      return false;
    }

    if (lexer->lookahead == '}') {
      if (allow_escaped_rbrace) {
        lexer->advance(lexer, false);
        lexer->mark_end(lexer);
        lexer->result_symbol = ESCAPED_RBRACE;
        return true;
      }

      return false;
    }

    if (lexer->lookahead == '@' && allow_escaped_at) {
      lexer->advance(lexer, false);
      lexer->mark_end(lexer);
      lexer->result_symbol = ESCAPED_AT;
      return true;
    }

    if (allow_any_chunk) {
      lexer->mark_end(lexer);
      lexer->result_symbol = chunk_kind;
      return true;
    } else {
      return false;
    }
  }

  // Check for entities
  if (lexer->lookahead == '&' && allow_entity) {
    // Save position in case entity scan fails
    if (scan_entity(lexer)) {
      lexer->result_symbol = ENTITY;
      return true;
    }
    // If entity scan failed, fall through to text chunk
  }

  if (!allow_any_chunk) {
    return false;
  }

  // Scan text chunk
  bool has_content = false;

  // Consume characters until we hit a delimiter
  while (lexer->lookahead != 0) {
    // Stop at element start delimiter. Do NOT stop on '>' because it
    // can legitimately appear in text (e.g., in comparisons like `a > b`).
    if (lexer->lookahead == '<') {
      break;
    }

    if (embed_mode && lexer->lookahead == '@') {
      // Check for interpolation opener "@{"
      TSLexer saved = *lexer;
      lexer->advance(lexer, false);
      if (lexer->lookahead == '{') {
        *lexer = saved;
        if (has_content) {
          lexer->result_symbol = chunk_kind;
          return true;
        }
        return false;
      }
      *lexer = saved;
    }

    // Stop at interpolation delimiters
    if (lexer->lookahead == '{' || lexer->lookahead == '}') {
      break;
    }

    // Stop at backslash if next char is a delimiter (let escaped token handle it)
    if (lexer->lookahead == '\\') {
      lexer->mark_end(lexer);
      lexer->advance(lexer, false);
      if (lexer->lookahead == '{' || lexer->lookahead == '}' || (embed_mode && lexer->lookahead == '@')) {
        // Don't include the backslash, let escaped token scanner handle it
        if (has_content) {
          lexer->result_symbol = chunk_kind;
          return true;
        }
        return false;
      }

      has_content = true;
      continue;
    }

    // Stop at entity start (let entity scanner handle it)
    if (lexer->lookahead == '&' && allow_entity) {
      // Check if it looks like an entity
      TSLexer saved = *lexer;
      if (is_entity_start(lexer)) {
        // Restore lexer position
        *lexer = saved;
        if (has_content) {
          lexer->result_symbol = chunk_kind;
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
    lexer->result_symbol = chunk_kind;
    return true;
  }

  return false;
}
