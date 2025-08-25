# AI Agent Guidelines

This file provides guidance to AI agents and LLMs when working with code in this repository to ensure high-quality, consistent contributions.

## Code Style Guidelines

### C# Conventions

#### Formatting & Structure
- **Indentation**: 4 spaces for C# code, 2 spaces for XML/JSON/XAML
- **Braces**: Allman style (opening brace on new line), always use braces for all code blocks (if, for, while, etc.)
- **Line endings**: CRLF on Windows, LF on other platforms
- **Nullable reference types**: Enabled by default (`<Nullable>enable</Nullable>`)
- **Implicit usings**: Enabled by default (except where explicitly disabled)
- **Doc Comments**: Wrap XML documentation comments at 120  characters. If there are multiple paragraphs, use <para> tags for them. Don't use <para> for single paragraphs.

#### Naming Conventions
- **Classes/Methods/Properties**: PascalCase
- **Fields**: camelCase with underscore prefix for private fields (`_fieldName`)
- **Parameters/Local variables**: camelCase
- **Constants**: PascalCase
- **Interfaces**: PascalCase with 'I' prefix (`IServiceName`)
- **Generic type parameters**: Single uppercase letter (`T`, `TKey`, `TValue`)
- **Abbreviations**: Treat "UI" as a word (e.g., `UIComponent`, not `UiComponent`)

#### Code Organization
- **Using directives**: Outside namespace, System directives first
- **File structure**: One primary type per file
- **Namespace**: Match folder structure
- **Access modifiers**: Always explicit, prefer most restrictive appropriate level
- **Control flow**: Always use braces, even for single-line statements
- **Variable declarations**: Avoid `var` unless the type is obvious from the right-hand side (e.g., `new SomeType()`, LINQ queries with obvious types)
- **JSON RPC Methods**: All JSON RPC methods must have the `JsonRpcMethod` attribute on both the interface method and implementation, and they must match

```csharp
// Preferred - always use braces
if (condition)
{
    DoSomething();
}

// Avoid - single-line without braces
if (condition)
    DoSomething();
```
## Development Guidelines

### Refactoring
- There's no need for backward compatibility yet. Prefer simpler new code instead.
- Update related tests when modifying code
- Follow the existing architectural patterns
- Consider cross-platform compatibility

### Documentation
- Update relevant documentation when making changes
- Include code examples in documentation
- Use clear, concise language
- Follow the established documentation structure
