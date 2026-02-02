# NX Markup Language Planning Document

High-level product vision, syntax overviews, and core language concepts now live in the documentation:

- `docs/src/content/docs/overview/what-is-nx.md`
- `docs/src/content/docs/overview/design-goals.md`
- `docs/src/content/docs/overview/comparison.md`
- `docs/src/content/docs/reference/syntax/modules.md`
- `docs/src/content/docs/reference/syntax/functions.md`
- `docs/src/content/docs/reference/syntax/expressions.md`
- `docs/src/content/docs/reference/syntax/types.md`
- `docs/src/content/docs/reference/syntax/elements.md`
- `docs/src/content/docs/reference/concepts/sequences-and-objects.md`

## Implementation Strategy

### Phase 1: Lexer and Parser
**Core Infrastructure**
- Implement tokenizer with position tracking
- Build recursive descent parser with error recovery
- Define comprehensive AST node hierarchy
- Add syntax error reporting with suggestions
- Create basic REPL for testing

**Simplified Feature Set:**
- Basic element syntax (self-closing and container)
- Function definitions with typed parameters
- Simple expressions (literals, identifiers, member access)
- Basic control flow (if/else, for loops)
- Type declarations (primitives, arrays, function types)

**Key Deliverables:**
- Complete lexical analyzer
- AST generation for core syntax elements
- Basic error reporting
- Simple test suite

### Phase 2: Type System
**Type Infrastructure**
- Implement basic type checking visitor
- Add support for primitive types and sequences
- Implement function type checking
- Add nullable type support
- Create meaningful type error messages

**Type System:**
- Primitive types: string, int, float, bool, void, object
- Sequence types: T[] (sequences are the primary collection type)
- Function types: (T1, T2) => T3
- Nullable types: T?
- Object types: type <Name field:Type/>
- User-defined types and type aliases
- **Unified syntax**: Objects use element-like syntax for perfect duality with components

**Key Deliverables:**
- Working type checker for core features
- Clear type error messages

### Phase 3: Expression Trees & Runtime
**Execution Engine**
- Convert AST to .NET Expression Trees
- Implement standard library functions
- Add debugging hooks and breakpoint support
- Create memory-efficient evaluation
- Build error handling and recovery

**Key Deliverables:**
- Working interpreter
- Standard library
- Error handling system
- Performance benchmarks

### Phase 4: Tooling & IDE Support
**Development Experience**
- Language Server Protocol implementation
- VS Code extension with full IntelliSense
- Visual Studio integration
- Syntax highlighting for multiple editors
- Auto-formatting and code completion

**Key Deliverables:**
- LSP server
- IDE extensions
- Rich debugging experience
- Code formatting tools

## Technical Architecture

### Core Components

#### Lexer (`NX.Lexer`)
```csharp
public class NXLexer
{
    public IEnumerable<Token> Tokenize(string source);
    public TokenStream CreateStream(string source);
}

public enum TokenType
{
    // Literals
    StringLiteral, IntegerLiteral, BooleanLiteral,

    // Identifiers & Keywords
    Identifier, Let, If, Else, For, In, Switch, Import, From, Type,

    // Operators
    Arrow, FatArrow, Question, Colon, Semicolon, Comma, Dot,
    Plus, Minus, Star, Slash, Equals, NotEquals, LessThan, GreaterThan,

    // Delimiters
    LeftBrace, RightBrace, LeftParen, RightParen, LeftAngle, RightAngle,
    LeftBracket, RightBracket,

    // Special
    EndOfFile, Invalid
}
```

#### Parser (`NX.Parser`)
```csharp
public class NXParser
{
    public ModuleNode ParseModule(TokenStream tokens);
    public ExpressionNode ParseExpression(TokenStream tokens);
    public TypeNode ParseType(TokenStream tokens);
    public ObjectCreationNode ParseObjectCreation(TokenStream tokens);
}

// AST Node Hierarchy
public abstract class AstNode
{
    public SourceLocation Location { get; set; }
}

public class ModuleNode : AstNode
{
    public List<ImportNode> Imports { get; set; }
    public List<TypeDefinitionNode> Types { get; set; }
    public List<FunctionDefinitionNode> Functions { get; set; }
    public ElementNode? MainElement { get; set; }
}

public class ObjectCreationNode : ExpressionNode
{
    public string TypeName { get; set; }
    public List<AttributeNode> Attributes { get; set; }
}

public class ObjectTypeDefinitionNode : TypeDefinitionNode
{
    public string TypeName { get; set; }
    public List<TypeParameterNode> Parameters { get; set; }
}
```

#### Type System (`NX.TypeSystem`)
```csharp
public abstract class NXType
{
    public abstract bool IsAssignableFrom(NXType other);
    public abstract NXType Substitute(Dictionary<TypeVariable, NXType> substitutions);
}

public class TypeChecker
{
    public TypeCheckResult CheckModule(ModuleNode module);
    public NXType InferType(ExpressionNode expression, TypeEnvironment env);
}

public class TypeInference
{
    public UnificationResult Unify(NXType type1, NXType type2);
    public NXType Instantiate(NXType type, TypeEnvironment env);
}
```

#### Runtime (`NX.Runtime`)
```csharp
public class NXInterpreter
{
    public object Evaluate(ExpressionNode expression, RuntimeEnvironment env);
    public RuntimeResult EvaluateWithErrorHandling(ExpressionNode expression, RuntimeEnvironment env);
}

public class ExpressionTreeBuilder
{
    public Expression<Func<RuntimeEnvironment, object>> Build(ExpressionNode node);
}
```

### Error Handling Strategy

#### Compile-time Error Categories
1. **Syntax Errors**: Malformed expressions, missing tokens, invalid structure
2. **Type Errors**: Type mismatches, undefined identifiers
3. **Semantic Errors**: Unreachable code, invalid operations
4. **Import Errors**: Missing modules, circular dependencies, access violations

#### Runtime Error Handling
```nx
// Simple error handling pattern
let <SafeOperation/> = {
  let result = tryRiskyOperation()
  if result.success =>
    <SuccessView data={result.value}/>
  else
    <ErrorView error={result.error}/>
}

// Error boundaries
<ErrorBoundary onError={(error) => logError(error)}>
  <SomeComponent/>
</ErrorBoundary>
```

### Testing Strategy

#### Unit Testing Structure
```
test/
├── Lexer.Tests/
│   ├── TokenizerTests.cs
│   ├── ErrorRecoveryTests.cs
│   └── PerformanceTests.cs
├── Parser.Tests/
│   ├── ExpressionParsingTests.cs
│   ├── TypeParsingTests.cs
│   └── ModuleParsingTests.cs
├── TypeSystem.Tests/
│   ├── TypeInferenceTests.cs
│   └── UnificationTests.cs
└── Runtime.Tests/
    ├── EvaluationTests.cs
    └── PerformanceTests.cs
```

#### Integration Testing
- End-to-end compilation and execution
- Complex component composition scenarios
- Real-world application examples
- Cross-platform compatibility testing
- Memory usage and performance profiling

### Performance Considerations

#### Compilation Optimizations
- **Constant Folding**: Evaluate constant expressions at compile time
- **Dead Code Elimination**: Remove unreachable code paths
- **Inline Expansion**: Inline simple function calls

#### Runtime Optimizations
- **Expression Caching**: Cache compiled expressions for reuse
- **Lazy Evaluation**: Defer computation until needed
- **Memory Pooling**: Reuse objects to reduce GC pressure

### Future Roadmap

This planning document focuses on the core NX v1.0 implementation. Advanced features such as union types, generics, async support, destructuring, and advanced pattern matching are documented separately to maintain focus on the achievable core functionality.

This focused approach ensures NX v1.0 provides a complete, usable markup language while establishing the foundation for future enhancements.
