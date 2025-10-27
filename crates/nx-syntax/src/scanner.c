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
 */

#include <tree_sitter/parser.h>
#include <wctype.h>
#include <string.h>

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
 * Scanner for NX text content tokens.
 *
 * This is a simplified implementation that handles basic text content.
 * A full implementation would need to track context (inside element vs not)
 * and handle all the text token types properly.
 */
bool tree_sitter_nx_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid_symbols) {
  // For now, we'll implement a minimal scanner that doesn't emit these tokens
  // This allows the grammar to compile and be tested with non-text content
  // TODO: Implement full text content scanning with proper context tracking

  return false;
}
