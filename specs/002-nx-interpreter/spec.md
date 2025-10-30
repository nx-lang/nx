# Feature Specification: NX Interpreter

**Feature Branch**: `002-nx-interpreter`  
**Created**: 2025-10-29  
**Status**: Draft  
**Input**: User description: "Create a simple interpreter for NX, interpreting the HIR. It should be able to take an NX function and parameters as input and return the function value as output."

## Clarifications

### Session 2025-10-29

- Q: What should the interpreter return when a function has no return statement? → A: Raise a runtime error with clear message (Note: NX functions always return something, so this should not occur in well-formed code)
- Q: How should the interpreter protect against infinite loops? → A: Set a maximum total operation count per execution (e.g., 1 million operations)
- Q: What should happen when a function is called with the wrong number of parameters? → A: Raise a runtime error (parameter count mismatch)
- Q: How should null/undefined values be handled in arithmetic and logical operations? → A: Raise runtime error for null in arithmetic/logical operations
- Q: What is the exact recursion depth limit? → A: 1000

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Execute Simple NX Functions (Priority: P1) 🎯 MVP

A developer writes a simple NX function with basic arithmetic or string operations and wants to execute it programmatically to verify the logic works correctly.

**Why this priority**: This is the core MVP - the ability to execute any NX function with parameters and get a result. Without this, the interpreter has no value.

**Independent Test**: Write an NX function `let <add a:int b:int /> = { a + b }`, call it with parameters `add(5, 3)`, and verify it returns `8`.

**Acceptance Scenarios**:

1. **Given** an NX function with arithmetic operations, **When** the interpreter executes it with valid parameters, **Then** it returns the correct computed value
2. **Given** an NX function with string concatenation, **When** executed with string parameters, **Then** it returns the concatenated result
3. **Given** an NX function with boolean logic, **When** executed, **Then** it returns the correct boolean value
4. **Given** an NX function with local variables, **When** executed, **Then** variable assignments and references work correctly

---

### User Story 2 - Execute Functions with Conditionals (Priority: P2)

A developer writes an NX function with if/else logic and wants to verify different code paths execute correctly based on input parameters.

**Why this priority**: Conditional logic is fundamental to most real-world functions. This extends the MVP to handle branching logic.

**Independent Test**: Write a function `let <max a:int b:int /> = { if a > b { a } else { b } }`, test with different inputs, and verify correct branch execution.

**Acceptance Scenarios**:

1. **Given** a function with if/else conditional, **When** condition evaluates to true, **Then** the true branch executes and returns its value
2. **Given** a function with if/else conditional, **When** condition evaluates to false, **Then** the else branch executes and returns its value
3. **Given** a function with nested conditionals, **When** executed, **Then** all conditional evaluations work correctly

---

### User Story 3 - Execute Functions with Loops (Priority: P3)

A developer writes an NX function with loop constructs (for/while) and wants to verify iteration logic works correctly.

**Why this priority**: Loops enable more complex algorithms. Less critical than basic execution and conditionals for an MVP interpreter.

**Independent Test**: Write a function that sums numbers from 1 to N using a loop, execute with N=5, and verify it returns 15.

**Acceptance Scenarios**:

1. **Given** a function with a for loop, **When** executed, **Then** the loop iterates the correct number of times and produces the expected result
2. **Given** a function with a while loop, **When** executed, **Then** the loop continues until the condition is false
3. **Given** a function with loop control (break/continue), **When** executed, **Then** control flow statements work correctly

---

### User Story 4 - Handle Runtime Errors Gracefully (Priority: P2)

A developer executes an NX function that encounters a runtime error (division by zero, type mismatch, undefined variable) and wants clear error messages.

**Why this priority**: Essential for developer experience. Must happen before complex features but after basic execution works.

**Independent Test**: Execute a function with `x / 0` and verify it returns a clear error message indicating division by zero at the correct source location.

**Acceptance Scenarios**:

1. **Given** a function with division by zero, **When** executed, **Then** interpreter reports a runtime error with source location
2. **Given** a function referencing an undefined variable, **When** executed, **Then** interpreter reports the error clearly
3. **Given** a function with type mismatch at runtime, **When** executed, **Then** interpreter reports the type error with context

---

### Edge Cases

- **Missing return statement**: NX functions always return a value (enforced by language semantics). If malformed HIR lacks a return, interpreter raises a runtime error.
- **Infinite loop protection**: Interpreter enforces a maximum total operation count per execution (default: 1 million operations). Exceeding this limit raises a runtime error.
- **Wrong parameter count**: Calling a function with incorrect number of parameters raises a runtime error with parameter count mismatch message.
- **Null/undefined in operations**: Using null or undefined values in arithmetic or logical operations raises a runtime error with clear message indicating the null value and operation attempted.
- **Recursive function calls**: Maximum recursion depth is 1000 calls. Exceeding this limit raises a runtime error (stack overflow protection).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST accept NX HIR (High-level Intermediate Representation) as input
- **FR-002**: System MUST accept function parameters as input values
- **FR-003**: System MUST execute HIR instructions following NX semantics
- **FR-004**: System MUST evaluate expressions (arithmetic, logical, comparison, string operations)
- **FR-005**: System MUST handle variable bindings and lookups within function scope
- **FR-006**: System MUST evaluate conditional expressions (if/else)
- **FR-007**: System MUST execute loop constructs (for, while)
- **FR-008**: System MUST return computed function values as output
- **FR-009**: System MUST report runtime errors with clear messages and source locations
- **FR-010**: System MUST handle type-checked operations (assuming type checking has already occurred)
- **FR-011**: System MUST support function calls within expressions
- **FR-012**: System MUST maintain execution context (variable environment, call stack)
- **FR-013**: System MUST provide execution results in a structured format
- **FR-014**: System MUST raise runtime errors when null/undefined values are used in arithmetic or logical operations
- **FR-015**: System MUST enforce recursion depth limit of 1000 calls to prevent stack overflow
- **FR-016**: System MUST enforce maximum operation count per execution to prevent infinite loops (default: 1 million operations)
- **FR-017**: System MUST validate parameter count matches function signature and raise runtime error on mismatch

### Key Entities

- **Interpreter**: The execution engine that processes HIR and evaluates expressions
- **ExecutionContext**: Runtime state including variable bindings and call stack
- **Value**: Runtime representation of NX values (integers, floats, strings, booleans, null, arrays, functions)
- **RuntimeError**: Error information including error type, message, and source location
- **ExecutionResult**: The outcome of interpretation (success with value, or error)

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Interpreter successfully executes all example NX functions from the test suite without errors
- **SC-002**: Interpreter correctly evaluates 100% of arithmetic, logical, and comparison operations
- **SC-003**: Interpreter handles conditionals correctly in 100% of test cases (true/false branches)
- **SC-004**: Interpreter executes loops correctly and produces expected results in 100% of test cases
- **SC-005**: Interpreter detects and reports 100% of runtime errors with clear messages and source locations
- **SC-006**: Interpreter executes functions with up to 1000 operations in under 100 milliseconds
- **SC-007**: Interpreter supports recursive function calls up to depth of 1000 and reports stack overflow errors when exceeded
- **SC-008**: Interpreter provides execution results in a format that can be easily tested and validated
- **SC-009**: Interpreter handles all NX primitive types (int, float, string, boolean, null)
- **SC-010**: Interpreter gracefully handles and reports malformed HIR input

## Assumptions

- HIR (High-level Intermediate Representation) from nx-hir crate is well-formed and type-checked
- Type checking has already been performed by nx-types crate before interpretation
- NX semantics for operations (arithmetic, logical, etc.) are well-defined
- The interpreter will be used in both development/testing and production environments, prioritizing reliability and security over maximum performance
- Performance target is reasonable for interpreted execution (not JIT compiled)
- The interpreter runs in a single-threaded environment
- No external function calls or I/O operations need to be supported in MVP
- Array and object operations follow standard semantics
- String operations include concatenation and comparison at minimum

## Dependencies

- nx-hir crate for HIR data structures and types
- nx-diagnostics crate for error reporting
- nx-types crate for type information (already integrated in HIR)
- Rust standard library for core execution primitives
- Test fixtures from nx-syntax and nx-types for validation
