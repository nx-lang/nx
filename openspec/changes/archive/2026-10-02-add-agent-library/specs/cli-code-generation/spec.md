## ADDED Requirements

### Requirement: `typegen` generates from a standard library
`nxlang typegen` SHALL accept a standard library name of the form `@nx/<name>` in place of a path and
SHALL generate that library's exported type surface as it generates a library directory's: one
generated file per contributing module, an output directory required, and the same declarations,
companions and documentation comments an identical library on disk would produce. The argument SHALL
be read as a standard library name before it is read as a path, and a name under `@nx/` that is not
a standard library SHALL fail with an error listing the standard libraries that exist.

#### Scenario: The agent library generates TypeScript
- **WHEN** the user runs `nxlang typegen @nx/agent --language typescript --output ./generated`
- **THEN** the command SHALL write `agent.ts`, the shared helper module and the index into `./generated`
- **AND** `agent.ts` SHALL declare `Agent`, `Document`, `AgentLimits`, `Tool`, `FunctionTool`, `WebSearchTool`, `ToolContext`, `Connection`, `HttpConnection`, `HttpMethod`, `HttpParam`, `HttpArguments` and `HttpTool`, each with its NX documentation as a doc comment

#### Scenario: The agent library generates C#
- **WHEN** the user runs `nxlang typegen @nx/agent --language csharp --csharp-namespace NxLang.Agent --output ./generated`
- **THEN** the command SHALL write the library's C# contracts into `./generated` in namespace `NxLang.Agent`

#### Scenario: An unknown standard library is an error
- **WHEN** the user runs `nxlang typegen @nx/nope --language typescript --output ./generated`
- **THEN** the command SHALL fail naming `@nx/nope` and listing `@nx/agent`
- **AND** SHALL write nothing

### Requirement: Generated code refers to a standard library's types by a fixed target
When generated TypeScript references an exported type owned by a standard library, the generated
file SHALL emit a type-only import of it from that standard library's published TypeScript package,
which for `@nx/agent` is `@nx-lang/agent`. The target SHALL NOT be derived from a directory name,
SHALL NOT be affected by `--typescript-package-prefix`, and SHALL NOT produce the
assumed-package-target warning. When generated C# references such a type, the generated file SHALL
qualify it with that standard library's fixed namespace, which for `@nx/agent` is `NxLang.Agent`,
unaffected by `--csharp-namespace`, and SHALL NOT produce the assumed-namespace warning. A library
that extends a standard library's abstract record SHALL generate a type that extends the imported
one.

#### Scenario: A host library's TypeScript imports from the agent package
- **WHEN** library `chat-link` contains `import "@nx/agent"`, `export type RecordSearchTool extends Tool = { recordKind:string }` and `export type AssistantConfig = { agent?:Agent }`
- **AND** the user runs `nxlang typegen ./chat-link --language typescript --typescript-package-prefix @org/nx- --output ./generated`
- **THEN** the generated module SHALL include `import type { Agent, Tool } from "@nx-lang/agent";`
- **AND** SHALL declare `RecordSearchTool` as extending `Tool` and `AssistantConfig.agent` as `Agent`
- **AND** the command SHALL emit no warning about an assumed dependency package

#### Scenario: A host library's C# qualifies with the fixed namespace
- **WHEN** the same library is generated with `--language csharp --csharp-namespace Org.ChatLink`
- **THEN** the generated `AssistantConfig` SHALL type `Agent` with the `Agent` type of namespace `NxLang.Agent`
- **AND** the command SHALL emit no warning about an assumed dependency namespace
