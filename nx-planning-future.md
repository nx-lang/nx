# NX Markup Language - Future Features

This document outlines features that are planned for future versions of NX, beyond the core v1.0 implementation. These features represent the long-term vision for NX while keeping the initial version focused and achievable.

## Version 1.1: Enhanced Type System

### Union Types
Union types allow a value to be one of several specific types or values, providing compile-time safety for common patterns.

#### String Literal Union Types
```nx
type Color = "red" | "green" | "blue" | "transparent"
type ButtonVariant = "primary" | "secondary" | "danger" | "outline"
type UserRole = "admin" | "editor" | "viewer" | "guest"

// Usage with compile-time safety
let <ColoredBox color:Color/> =
  <div style={{ backgroundColor: color }}>
    Content
  </div>

// Valid usage
<ColoredBox color="red"/>      // ✅ Valid
<ColoredBox color="blue"/>     // ✅ Valid

// Invalid usage (compile errors)
<ColoredBox color="purple"/>   // ❌ Error: "purple" is not assignable to Color
```

#### Mixed Type Unions
```nx
// Union of different types
type StringOrNumber = string | int
type OptionalUser = User | void

// API response patterns
type ApiResult<T> = 
  | { Success: T }
  | { Error: string }
  | { Loading: void }
```

#### Pattern Matching with Union Types
```nx
let <StatusIcon<T> status:ApiResult<T>/> =
  {match status {
    { Success: data } => <CheckIcon color="green"/>
    { Error: message } => <XIcon color="red" title={message}/>
    { Loading: _ } => <SpinnerIcon/>
  }}

let <RoleIcon role:UserRole/> =
  {match role {
    "admin" => <AdminIcon/>
    "editor" => <EditIcon/>  
    "viewer" => <ViewIcon/>
    "guest" => <GuestIcon/>
  }}
```

## Version 1.2: Generic Type System

### Basic Generics
Generic types allow for reusable components and data structures that work with multiple types while maintaining type safety.

#### Generic Object Types
```nx
// Generic container types
type <Container<T> value:T metadata:string created:DateTime/>
type <Result<T, E> success:T? error:E?/>
type <List<T> items:T[] count:int/>

// Usage
let stringContainer = <Container<string> 
  value="hello world" 
  metadata="text data"
  created={DateTime.Now}
/>

let userResult = <Result<User, string> 
  success={currentUser}
  error=void
/>
```

#### Generic Components
```nx
// Generic component definitions
let <DataGrid<T> 
  data:T[] 
  columns:ColumnDefinition<T>[] 
  onRowClick:(T) => void
  className:string?
/> =
  <table className={className ?? "data-grid"}>
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

// Generic list component
let <GenericList<T> items:T[] renderer:(T) => Element/> =
  <ul>
    {for item in items => <li>{renderer(item)}</li>}
  </ul>

// Usage
<GenericList<User> items={users} renderer={user => <UserCard user={user}/>}/>
<GenericList<string> items={names} renderer={name => <span>{name}</span>}/>
```

#### Type Constraints
```nx
// Constraints on generic types (future consideration)
type <Comparable<T> where T : IComparable value:T/>
type <Displayable<T> where T : { Name: string } item:T/>

// Generic functions with constraints
let <SortableList<T> where T : IComparable items:T[]/> =
  <ul>
    {for item in items.Sort() => <li>{item}</li>}
  </ul>
```

### Advanced Generic Patterns

#### Higher-Order Components with Generics
```nx
// HOC that adds loading state
let withLoading<T>(Component:<T/>) =
  let <WithLoadingWrapper props:T isLoading:bool/> =
    {if isLoading => <LoadingSpinner/> else <Component {...props}/>}
  WithLoadingWrapper

// Usage
let LoadingUserCard = withLoading<{user:User}>(UserCard)
<LoadingUserCard user={user} isLoading={fetchingUser}/>
```

#### Generic Data Fetching
```nx
// Generic data fetching component
let <AsyncData<T> 
  fetcher:() => Promise<T>
  fallback:Element
  children:(T) => Element
/> = {
  let result = await fetcher()
  children(result)
}

// Usage
<AsyncData<User[]> 
  fetcher={() => fetchUsers()}
  fallback={<LoadingSpinner/>}
  children={users => 
    <ul>
      {for user in users => <UserCard user={user}/>}
    </ul>
  }
/>
```

## Version 1.3: Async Support

### Implicit Async Evaluation
Rather than explicit async/await syntax, NX may handle asynchronous operations transparently at the runtime level.

#### Conceptual Approach
```nx
// Functions may be evaluated asynchronously behind the scenes
// without requiring explicit async syntax from the developer
let <UserProfile userId:string/> = {
  let user = fetchUser(userId)  // May be async internally
  <div>
    <img src={user.avatarUrl}/>
    <h2>{user.name}</h2>
    <span>{user.email}</span>
  </div>
}

// The runtime could handle the async nature transparently
<UserProfile userId="123"/>
```

#### Error and Loading Boundaries
```nx
// Error boundaries for handling async failures and loading states
<AsyncBoundary 
  fallback={<LoadingSpinner/>}
  errorFallback={(error) => <ErrorDisplay error={error}/>}
>
  <UserProfile userId="123"/>
  <UserPosts userId="123"/>
  <UserStats userId="123"/>
</AsyncBoundary>

// Suspense-like behavior without explicit async syntax
<SuspenseBoundary fallback={<LoadingPlaceholder/>}>
  <DataDependentComponent/>
</SuspenseBoundary>
```

#### Potential Data Fetching Patterns
```nx
// Parallel data loading (handled implicitly by runtime)
let <DashboardView userId:string/> = {
  let user = fetchUser(userId)        // These could run in parallel
  let posts = fetchUserPosts(userId)  // automatically by the runtime
  let stats = fetchUserStats(userId)
  
  <div>
    <UserHeader user={user}/>
    <PostsList posts={posts}/>
    <StatsWidget stats={stats}/>
  </div>
}
```

### Research Areas for Async Implementation
- **Automatic dependency analysis** to determine what can run in parallel
- **Caching strategies** for data fetching operations
- **Error propagation** through component hierarchies
- **Loading state management** without explicit state variables
- **Integration with existing async patterns** in C# and web frameworks

## Version 1.4: Advanced Pattern Matching

### Destructuring Assignment
Extracting values from objects and arrays into individual variables.

#### Object Destructuring
```nx
// Destructuring in function parameters
let <UserDisplay {name, email, avatarUrl}:User/> =
  <div>
    <img src={avatarUrl}/>
    <h3>{name}</h3>
    <span>{email}</span>
  </div>

// Destructuring in variable assignments
let {name, email} = user
let {x, y} = coordinates
```

#### Array Destructuring
```nx
// Array destructuring
let [first, second, ...rest] = items
let [x, y] = coordinates
let [head, ...tail] = list

// Usage in components
<div>
  <span>First: {first}</span>
  <span>Second: {second}</span>
  <span>Rest: {rest.length} items</span>
</div>
```

#### Advanced Pattern Matching
```nx
// Pattern matching with destructuring
{match result {
  { Success: { user: {name, email}, metadata } } => 
    <SuccessView userName={name} userEmail={email} meta={metadata}/>
  { Error: message } => 
    <ErrorView message={message}/>
  { Loading: _ } => 
    <LoadingView/>
}}

// Nested destructuring
let <ComplexComponent {user: {profile: {name, bio}}, settings: {theme}}:ComplexData/> =
  <div className={`theme-${theme}`}>
    <h1>{name}</h1>
    <p>{bio}</p>
  </div>
```

## Version 1.5: Tuple Types

### Basic Tuple Support
Tuples provide a way to group multiple values of different types together.

#### Tuple Type Definitions
```nx
// Tuple types
type Point = (int, int)
type NamedPoint = (string, int, int)
type ColorRGB = (int, int, int)
type KeyValuePair<K, V> = (K, V)

// Usage
let origin = (0, 0)
let corner = (100, 50)
let red = (255, 0, 0)
```

#### Tuple Destructuring
```nx
// Tuple destructuring
let (x, y) = coordinates
let (r, g, b) = color
let (key, value) = pair

// Function returning tuples
let <CoordinateDisplay point:(int, int)/> = {
  let (x, y) = point
  <div>Point: ({x}, {y})</div>
}
```

#### Named Tuples
```nx
// Named tuple fields (future consideration)
type Point = (x: int, y: int)
type Color = (red: int, green: int, blue: int)

// Access by name or position
let point = (x: 10, y: 20)
let x = point.x        // Named access
let y = point.Item2    // Positional access
```

## Version 2.0: Advanced Language Features

### Macro System
Compile-time code generation and transformation capabilities.

```nx
// Hypothetical macro syntax
macro generateCrud(entityType) = {
  // Generate CRUD operations for entity type
  let <{entityType}List items:{entityType}[]/> = ...
  let <{entityType}Form item:{entityType}/> = ...
  let <{entityType}Detail item:{entityType}/> = ...
}

// Usage
generateCrud(User)  // Creates UserList, UserForm, UserDetail components
```

### Advanced Type System Features
- **Intersection types**: `A & B` for combining object types
- **Mapped types**: Transform existing types programmatically
- **Conditional types**: Types that depend on type conditions
- **Template literal types**: String manipulation at the type level

### Module System Enhancements
- **Module federation**: Dynamic module loading and composition
- **Version compatibility**: Handle different versions of the same module
- **Tree shaking**: Eliminate unused code from final output

### Performance Optimizations
- **Static analysis**: Compile-time optimizations based on usage patterns
- **Incremental compilation**: Only recompile changed parts
- **Hot reloading**: Update running applications without full restart
- **Bundle optimization**: Minimize output size for web deployments

## Implementation Strategy for Future Features

### Feature Gating
Each version's features will be implemented behind feature flags, allowing:
- **Gradual rollout** of new functionality
- **Experimental features** that can be toggled on/off
- **Backward compatibility** with previous versions
- **Community feedback** before finalizing features

### Community Involvement
- **RFC process** for major language changes
- **Beta releases** for community testing
- **Feedback incorporation** before final release
- **Migration tools** for upgrading between versions

### Tooling Evolution
- **Enhanced IDE support** for new language features
- **Better error messages** as the language grows more complex
- **Debugging improvements** for advanced patterns
- **Performance profiling** tools for optimization

This roadmap ensures NX can evolve into a powerful, full-featured language while maintaining the simplicity and elegance of its core design principles.
