# NX Markup Language Planning Document

## Vision

NX is a next-generation functional markup language that can be seen as functional XML or JSX without JavaScript.
It unifies markup and programmatic constructs into a single, strongly-typed language where both markup elements
and programming logic are first-class citizens. Unlike traditional template languages that awkwardly combine
two separate languages (like HTML + JavaScript in JSX), NX provides a clean, cohesive syntax throughout.

While NX is a general purpose language, it is especially targeted for UI development (think better JSX and XAML) and configuration (think better JSON, XML, and YAML).

NX is runtime agnostic and can work on top of .NET, JavaScript, and other runtimes.

## Core Design Goals

### 1. **Unified Language Design**
- Single language incorporating both markup syntax and programmatic constructs
- No awkward transitions between "template" and "logic" modes
- Properties can contain arbitrary markup, not just strings
- Everything is an expression that can be composed and transformed

### 2. **Strong Type System**
- Pure functional language with strong typing
- Type inference where possible, explicit types where needed
- Compile-time safety for markup structure and data flow

### 3. **Familiar Yet Improved Syntax**
- Based on XML and JSX/TSX for familiarity
- Cleaner syntax that eliminates common pain points
- Better support for composition and abstraction
- Native support for modern programming patterns

### 4. **Performance & Tooling**
- Initial C# implementation with expression tree interpreter (JIT compiled on most platforms)
- Later support for transpilers, generating C# code or other targets
- Rich tooling support (LSP, debugging, etc.)
- Cross-platform compatibility

## Core Syntax Elements

### Source File (Module) Syntax

#### Module Definition Structure
```ebnf
ModuleDefinition ::=
    {ImportStatement} {TypeDefinition} {FunctionDefinition} [MainElement]

ImportStatement ::=
    "import" (ImportAll | ImportSpecific) "from" StringLiteral

ImportAll ::= "*" ["as" Identifier]

ImportSpecific ::= "{" ImportItem {"," ImportItem} "}"

ImportItem ::= Identifier ["as" Identifier]

TypeDefinition ::=
    "type" Identifier "=" TypeDeclaration
    | "type" "<" Identifier {TypeParameter} "/>"

TypeParameter ::= Identifier ":" TypeDeclaration ["=" DefaultValue]
```

#### Examples
```nx
// Import examples
import * from "./components"
import { Button, Input } from "./ui/controls"
import { List } from "./collections" as Collections

// Simple module with main element
private let WelcomeMessage = <span>Hello World</span>

<MainApp>
  <WelcomeMessage/>
</MainApp>
```

### Function Definition Syntax

#### Design Philosophy: Definition Mirrors Invocation

A core principle of NX is that function definitions should mirror their invocation syntax. This creates a consistent, intuitive experience where the way you define a component looks exactly like the way you use it.

In traditional languages, there's often a disconnect between definition and usage:
```jsx
// JavaScript: Definition looks nothing like invocation
function UserCard(props) { ... }  // Definition
<UserCard user={user}/>           // Invocation (in JSX)
```

In NX, the definition structure mirrors the invocation structure:
```nx
// Definition - looks like an element with type annotations
let <UserCard user:User className:string = "card"/> =
  <div>...</div>

// Invocation - identical structure, just with values instead of types
<UserCard user={currentUser} className="featured"/>
```

This principle extends to container elements with children:
```nx
// Definition - the closing tag shows it accepts children
let <Layout title:string> content:Element </Layout> =
  <html>...</html>

// Invocation - visually identical pattern
<Layout title="Home">
  <div>My content</div>
</Layout>
```

This approach has several benefits:
1. **Intuitive Learning**: Once you know how to use a component, you know how to define one
2. **Self-Documenting**: The definition visually shows exactly how the component should be used
3. **Consistency**: Everything in NX follows XML-like syntax, including function definitions
4. **Copy-Paste Friendly**: You can often start defining a function by copying its usage and adding types

#### Self-Closing Function Definition
```nx
let <UserCard user:User className:string = "card"/> =
  <div className={className}>
    <img src={user.avatarUrl} alt="User avatar"/>
    <h3>{user.name}</h3>
    <p>{user.email}</p>
  </div>
```

#### Container Function Definition
```nx
let <Layout title:string> content:Element </Layout> =
  <html>
    <head><title>{title}</title></head>
    <body>
      <header><h1>{title}</h1></header>
      <main>{content}</main>
    </body>
  </html>
```

#### Advanced Parameter Types
```nx
let <DataGrid
  data:object...
  columns:object...
  onRowClick:(object) => void = _ => {}
  className:string? /> =
  <table className={if className => className else "data-grid"}>
    <thead>
      <tr>
        {for column in columns => <th>{column.Header}</th>}
      </tr>
    </thead>
    <tbody>
      {for item in data =>
        <tr onClick={() => onRowClick(item)}>
          {for column in columns => <td>{column.Render(item)}</td>}
        </tr>
      }
    </tbody>
  </table>

// Simple user component
let <UserDisplay user:User /> =
  <div>
    <img src={user.avatarUrl}/>
    <h3>{user.name}</h3>
    <span>{user.email}</span>
  </div>
```

### Expression Syntax

#### Core Expression Types
```ebnf
Expression ::=
    Literal
    | Identifier
    | BraceExpression
    | Element
    | ObjectCreation
    | ConditionalExpression
    | IterationExpression
    | PatternMatchExpression
    | LambdaExpression
    | ArithmeticExpression
    | ComparisonExpression
    | LogicalExpression
    | MemberAccess
    | FunctionCall
    | "(" Expression ")"

BraceExpression ::= "{" Expression "}"

ObjectCreation ::= "<" TypeIdentifier {AttributeAssignment} "/>"

ConditionalExpression ::=
    "if" Expression "=>" Expression ["else" Expression]

IterationExpression ::=
    "for" Identifier "in" Expression "=>" Expression
    | "for" Identifier "," Identifier "in" Expression "=>" Expression  // With index

PatternMatchExpression ::=
    "match" Expression "{" {Pattern "=>" Expression} ["_" "=>" Expression] "}"

ListPattern ::=
    "[]"  // Empty list
    | "[" Pattern {"," Pattern} "]"  // Exact match
    | "[" Pattern "," "..." Identifier "]"  // Head and tail

LambdaExpression ::=
    "(" [ParameterList] ")" "=>" Expression
    | Identifier "=>" Expression  // Single parameter shorthand
```

### Pattern Matching
```nx
// Simple pattern matching on primitive values
{match user.role {
  "admin" => <AdminPanel/>
  "user" => <UserDashboard/>
  "guest" => <PublicContent/>
  _ => <AccessDenied/>
}}

// Pattern matching with strings
{match status {
  "loading" => <LoadingSpinner/>
  "success" => <SuccessMessage/>
  "error" => <ErrorMessage/>
  _ => <UnknownStatus/>
}}

// Pattern matching in function definitions
let <StatusDisplay status:string/> =
  {match status {
    "pending" => <PendingIcon/>
    "complete" => <CheckIcon/>
    "failed" => <XIcon/>
    _ => <QuestionIcon/>
  }}
```

#### Advanced Expression Examples
```nx
// Conditional rendering
{if user.isAuthenticated => <WelcomeUser user={user}/> else <LoginForm/>}

// Ternary operator
<div className={isActive ? "active" : "inactive"}>Content</div>

// List transformation with index
{for item, index in items =>
  <div key={index} className={index % 2 == 0 ? "even" : "odd"}>
    {item.name}
  </div>
}

// Basic pattern matching (simple values only)
{match user.role {
  "admin" => <AdminPanel/>
  "user" => <UserDashboard/>
  "guest" => <PublicContent/>
  _ => <AccessDenied/>
}}

// Lambda expressions in attributes
<Button onClick={(e) => handleClick(e.target)}>Click Me</Button>
<List items={data} transform={item => item.displayName}/>

// List literals and operations
let numbers = [1, 2, 3, 4, 5]  // List literal
let empty = []  // Empty list
let strings = ["red", "green", "blue"]

// List comprehensions
let doubled = [for x in numbers => x * 2]
let filtered = [for x in numbers if x > 0 => x]

// Object creation expressions
let user = <User id="123" name="John" email="john@example.com"/>
let position = <Point x={mouseX} y={mouseY}/>

// Objects in component attributes
<UserCard user={<User id="456" name="Jane" email="jane@example.com"/>}/>
<DrawingCanvas points={[<Point x=10 y=20/>, <Point x=30 y=40/>]}/>
```

### Type System

#### Type Declaration Syntax
```ebnf
TypeDeclaration ::=
    PrimitiveType [TypeModifier]
    | UserDefinedType [TypeModifier]
    | FunctionType
    | ListType

TypeModifier ::= "?" | "..."

PrimitiveType ::= "string" | "int" | "float" | "boolean" | "void" | "object"

UserDefinedType ::= TypeIdentifier

FunctionType ::= "(" [TypeDeclaration {"," TypeDeclaration}] ")" "=>" TypeDeclaration

ListType ::= TypeDeclaration "..."

TypeDefinition ::=
    "type" Identifier "=" TypeDeclaration                    // Type alias
    | "type" "<" Identifier {TypeParameter} "/>"             // Object type

TypeParameter ::= Identifier ":" TypeDeclaration ["=" DefaultValue]

ListLiteral ::= "[" [Expression {"," Expression}] "]"
```

#### Type Definition Examples
```nx
// Type aliases for primitives
type UserId = string
type EventHandler = (string) => void

// Object types using element syntax
type <User id:UserId name:string email:string avatarUrl:string?/>
type <Point x:int y:int/>
type <ComponentProps data:object className:string? children:Element?/>

// Nested object types
type <Address street:string city:string state:string zip:string/>
type <Person name:string email:string address:Address/>

// Object creation using element syntax
let user = <User
  id="123"
  name="John Doe"
  email="john@example.com"
  avatarUrl="/avatars/john.jpg"
/>

let origin = <Point x=0 y=0/>
let userAddress = <Address
  street="123 Main St"
  city="Springfield"
  state="IL"
  zip="62701"
/>

// Simple function definitions
let <SimpleList items:string... renderer:(string) => Element/> =
  <ul>
    {for item in items => <li>{renderer(item)}</li>}
  </ul>
```

### Element Syntax

#### Enhanced Element Features
```ebnf
Element ::=
    "<" ElementName {Attribute} "/>"
    | "<" ElementName {Attribute} ">" {Child} "</" ElementName ">"
    | "<" ElementName {Attribute} ">" SingleChild "</" ElementName ">"

ElementName ::= Identifier | NamespacedIdentifier

NamespacedIdentifier ::= Identifier "." Identifier {"." Identifier}

Attribute ::=
    AttributeName "=" AttributeValue
    | "{..." Expression "}"  // Spread attributes

AttributeName ::= Identifier | "..." Identifier  // Rest attributes

AttributeValue ::=
    StringLiteral
    | BraceExpression
    | Element  // Nested markup in attributes!

Child ::= Element | BraceExpression | StringLiteral
```

#### Advanced Element Examples
```nx
// Nested markup in attributes
<Tooltip content={<div><strong>Bold</strong> and <em>italic</em> text</div>}>
  Hover me
</Tooltip>

// Spread attributes
let commonProps = { className: "btn", disabled: false }
<Button {...commonProps} onClick={handleClick}>Submit</Button>

// Namespaced components
<UI.Controls.Button variant="primary">Click</UI.Controls.Button>

// Complex attribute expressions with lists
<Form
  onSubmit={(data) => validateAndSubmit(data)}
  validationRules={[
    { field: "email", validator: isValidEmail },
    { field: "age", validator: (val) => val >= 18 }
  ]}
  className={`form ${if isLoading => "loading" else ""} ${if hasErrors => "error" else ""}`}
>
  Form content
</Form>

// List pattern matching
{match items {
  [] => <EmptyList/>
  [single] => <SingleItem item={single}/>
  [first, ...rest] => <ItemList first={first} rest={rest}/>
}}
```

## Core Features

### Lists as Primary Collection Type

NX uses lists as its fundamental collection type, with the postfix `...` syntax for type declarations:

```nx
// List type declarations
let numbers: int... = [1, 2, 3, 4, 5]
let names: string... = ["Alice", "Bob", "Carol"]
let users: User... = [user1, user2, user3]

// Empty list
let empty: string... = []

// Lists in function parameters
let <Gallery images:Image.../> =
  <div class="gallery">
    {for img in images => <img src={img.url} alt={img.title}/>}
  </div>

// List comprehensions
let squares = [for n in numbers => n * n]
let evens = [for n in numbers if n % 2 == 0 => n]

// Pattern matching on lists
{match items {
  [] => <p>No items</p>
  [single] => <p>One item: {single}</p>
  [first, second] => <p>Two items: {first} and {second}</p>
  [first, ...rest] => <p>First: {first}, and {rest.length} more</p>
}}

// Nested lists
let matrix: (int...)... = [[1, 2], [3, 4], [5, 6]]  // List of lists
let grouped: (string, User...)... = [
  ("admins", [admin1, admin2]),
  ("users", [user1, user2, user3])
]
```

### Unified Object and Component Syntax
```nx
// Object types mirror component syntax perfectly
type <User id:string name:string email:string avatarUrl:string?/>
type <Point x:int y:int/>
type <Color r:int g:int b:int a:float = 1.0/>

// Object creation uses same syntax as component instantiation
let user = <User
  id="123"
  name="John Doe"
  email="john@example.com"
  avatarUrl="/avatars/john.jpg"
/>

let origin = <Point x=0 y=0/>
let red = <Color r=255 g=0 b=0/>
let transparentBlue = <Color r=0 g=0 b=255 a=0.5/>

// Components and objects compose naturally
let <UserProfile userId:string/> = {
  let user = <User
    id={userId}
    name="John Doe"
    email="john@example.com"
  />

  <div>
    <img src={if user.avatarUrl => user.avatarUrl else "/default-avatar.jpg"}/>
    <h2>{user.name}</h2>
    <span>{user.email}</span>
  </div>
}

// Objects can be passed inline to components
<UserCard user={<User id="456" name="Jane" email="jane@example.com"/>}/>

// Lists of objects use familiar syntax
let users = [
  <User id="1" name="Alice" email="alice@example.com"/>,
  <User id="2" name="Bob" email="bob@example.com"/>,
  <User id="3" name="Carol" email="carol@example.com"/>
]

// Basic container types
type <StringContainer value:string metadata:string created:string/>

let stringContainer = <StringContainer
  value="hello world"
  metadata="text data"
  created="2023-01-01"
/>
```

### Style Integration
```nx
// CSS-in-NX with basic styling
let <StyledButton variant:string = "primary">children:Element</StyledButton> =
  <button style={{
    backgroundColor: if variant == "primary" => "#007bff" else "#6c757d",
    color: "white",
    border: "none",
    padding: "8px 16px",
    borderRadius: "4px",
    cursor: "pointer"
  }}>
    {children}
  </button>

// Responsive design with breakpoints
let <ResponsiveGrid/> =
  <div style={{
    display: "grid",
    gridTemplateColumns: "repeat(auto-fit, minmax(250px, 1fr))",
    gap: "1rem"
  }}>
    Grid content
  </div>
```

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
- Add support for primitive types and lists
- Implement function type checking
- Add nullable type support
- Create meaningful type error messages

**Type System:**
- Primitive types: string, int, float, boolean, void, object
- List types: T... (lists are the primary collection type)
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
    Identifier, Let, If, Else, For, In, Match, Import, From, Type,

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
