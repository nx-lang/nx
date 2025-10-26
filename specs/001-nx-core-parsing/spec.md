# Feature Specification: Core NX Parsing and Validation

**Feature Branch**: `001-nx-core-parsing`
**Created**: 2025-10-25
**Updated**: 2025-10-26
**Status**: Draft
**Input**: User description: "Core support, implemented in Rust, to parse and validate NX files, including checking the types and semantics"

## Clarifications

### Session 2025-10-26

- Q: Should the NX type checker support type inference, or require explicit type annotations everywhere? → A: Local type inference including function return types - Infer types for local variables, expressions, and function return types; require explicit parameter types
- Q: Should the library API support concurrent parsing of multiple files, and should it be thread-safe? → A: Thread-safe with concurrent parsing - Library can safely parse multiple files concurrently on different threads
- Q: How deeply should the parser attempt to recover from syntax errors before giving up? → A: Best-effort within current construct - Attempt to recover and continue within current scope/block, skip malformed constructs
- Q: What file encoding(s) should the parser support? → A: UTF-8 only - All .nx files must be UTF-8 encoded, reject or report errors for other encodings
- Q: What are the acceptable memory usage constraints for parsing and type checking? → A: Proportional to file size, target <100MB for 10,000-line files

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Parse and Validate NX Files (Priority: P1)

Developers receive immediate feedback when their NX code contains syntax errors, with clear error messages pointing to the problem location.

**Why this priority**: Error detection is essential for productive development. Developers need to know when they've made syntax mistakes before attempting to compile or run code. This enables rapid feedback loops.

**Independent Test**: Can be tested by running a CLI command or library API to parse .nx files and verify that valid files parse successfully while invalid files report specific errors with line/column numbers. Delivers value by catching syntax errors early.

**Acceptance Scenarios**:

1. **Given** a syntactically valid .nx file, **When** parsed by the NX parser, **Then** parsing succeeds and produces a concrete syntax tree
2. **Given** a .nx file with mismatched element tags, **When** parsed, **Then** an error reports the location of the unclosed or mismatched tag
3. **Given** a .nx file with invalid property syntax, **When** parsed, **Then** an error indicates the specific property with incorrect syntax
4. **Given** a parser processes a directory of .nx files, **When** multiple .nx files exist, **Then** all files are validated and errors are reported for each problematic file
5. **Given** multiple .nx files are parsed concurrently on different threads, **When** parsing completes, **Then** all files are correctly parsed without data races or corruption

---

### User Story 2 - Check Types and Semantics (Priority: P1)

Developers receive compile-time type checking that catches type errors, undefined identifiers, and semantic issues before runtime.

**Why this priority**: Type checking prevents entire classes of runtime errors and improves code quality. It's crucial for a statically-typed language like NX and works alongside the parser to provide comprehensive validation.

**Independent Test**: Can be tested by writing .nx files with deliberate type errors and verifying that the type checker reports them with clear messages via CLI or library API. Delivers value by catching bugs at compile time.

**Acceptance Scenarios**:

1. **Given** a function expects an integer parameter, **When** called with a string argument, **Then** the type checker reports a type mismatch error
2. **Given** an undefined identifier is referenced, **When** the code is analyzed, **Then** an error indicates the identifier is not defined in scope
3. **Given** a nullable type is used without null checking, **When** the value is dereferenced, **Then** a warning suggests adding a null check
4. **Given** a local variable is assigned a value without explicit type annotation, **When** type checking runs, **Then** the variable's type is correctly inferred from the assigned value
5. **Given** all types are correct in a .nx file, **When** type checking runs, **Then** no type errors are reported

---

### Edge Cases

- What happens when a .nx file contains mixed valid and invalid syntax? System should use best-effort recovery within each scope/block, skipping malformed constructs and reporting all errors found while continuing to parse remaining valid code.
- How does the system handle very large .nx files (10,000+ lines)? Parser should maintain reasonable performance (<1 second for parsing, <2 seconds for type checking) and memory usage (<100MB for 10,000 lines).
- What happens when type definitions are circular or mutually recursive? Type checker should detect cycles and report clear errors without infinite loops.
- How does the parser handle incomplete or truncated files? Parser should gracefully handle EOF errors, report missing elements, and provide partial results for valid portions.
- What happens when a .nx file uses undefined types? Type checker should report undefined type errors with helpful suggestions if similar types exist.
- What happens when a file is not valid UTF-8? Parser should detect invalid UTF-8 encoding early and report a clear error indicating the encoding issue and byte position.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST parse valid NX syntax including elements, properties, expressions, control flow, functions, and type definitions
- **FR-001a**: System MUST validate that input files are UTF-8 encoded and report clear errors for invalid UTF-8 or other encodings
- **FR-002**: System MUST report syntax errors with precise line and column numbers and helpful error messages
- **FR-003**: System MUST implement type checking for NX's type system including primitives, sequences, functions, nullable types, and user-defined types
- **FR-003a**: System MUST support local type inference for variables, expressions, and function return types while requiring explicit function parameter types
- **FR-004**: System MUST detect undefined identifiers and report them as errors
- **FR-005**: System MUST generate a concrete syntax tree (CST) from parsed .nx code
- **FR-006**: System MUST provide formatted error output with source code context and suggestions
- **FR-007**: System MUST handle parser errors gracefully and continue analyzing the rest of the file (error recovery)
- **FR-007a**: Parser MUST attempt best-effort recovery within the current scope/block, skipping malformed constructs while continuing to parse remaining valid code
- **FR-008**: System MUST provide a library API for parsing and type checking .nx files
- **FR-008a**: Library API MUST be thread-safe and support concurrent parsing of multiple files on different threads
- **FR-009**: System MUST validate type compatibility in function calls, assignments, and operations
- **FR-010**: System MUST support semantic analysis including scope resolution and identifier binding
- **FR-011**: System MUST detect type mismatches and report them with expected vs actual type information
- **FR-012**: System MUST provide diagnostic messages with severity levels (error, warning, info)
- **FR-013**: System MUST support batch processing of multiple .nx files
- **FR-014**: System MUST maintain performance under 1 second for parsing files up to 10,000 lines
- **FR-014a**: System MUST maintain memory usage proportional to file size, targeting less than 100MB for 10,000-line files
- **FR-015**: System MUST detect circular or recursive type definitions and report appropriate errors

### Key Entities

- **Syntax Tree (CST)**: Concrete representation of parsed NX code preserving all tokens including whitespace and comments for tooling purposes
- **Abstract Syntax Tree (AST)**: Simplified representation for semantic analysis and type checking, discarding formatting details
- **Type**: Representation of NX types including primitives (string, int, float, boolean, void), sequences (T[]), functions ((T1, T2) => T3), nullable (T?), and user-defined types
- **Diagnostic**: Error or warning message with source location (line, column, span), severity level, message text, and optional suggested fixes
- **Symbol**: Named entity in NX code (function, type, variable) with location, type information, and scope
- **Scope**: Context for identifier resolution, containing bindings and parent scope reference for nested scopes

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Parser successfully parses all existing example .nx files without errors
- **SC-002**: Type checker identifies 100% of type mismatches in test suite
- **SC-003**: Parser processes files at minimum 10,000 lines per second
- **SC-004**: Type checking completes within 2 seconds for files up to 10,000 lines
- **SC-004a**: Parser and type checker use less than 100MB of memory when processing 10,000-line files
- **SC-005**: Parser reports all syntax errors found in a file, not just the first error
- **SC-006**: Error messages include source code context showing the problematic line
- **SC-007**: System correctly detects and reports circular type dependencies
- **SC-008**: Undefined identifier errors include line and column number information
- **SC-009**: Type mismatch errors show both expected and actual types
- **SC-010**: Parser gracefully handles incomplete files and reports meaningful EOF errors

## Assumptions

- NX grammar is already defined and stable (tree-sitter grammar.js exists and is complete)
- Rust is the implementation language for the parser and type checker
- tree-sitter will be used for parsing
- Existing crates (nx-diagnostics, nx-syntax) provide foundation for error reporting and parsing
- Type system semantics follow the design documented in project planning documents
- Performance targets are based on typical developer hardware (not low-end devices)
- This is a library/core implementation; CLI and IDE integration are separate features
- The parser will be used as a foundation for future IDE tooling but does not include IDE integration itself
- All .nx source files are UTF-8 encoded; other encodings are not supported

## Dependencies

- tree-sitter library and infrastructure (already integrated in nx-syntax crate)
- Rust toolchain 1.75+ with cargo, rustfmt, clippy
- Ariadne library for formatted diagnostic output (already integrated in nx-diagnostics)
- Sample .nx files for testing and validation
