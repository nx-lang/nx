## ADDED Requirements

### Requirement: Generated C# DTO properties preserve supported literal defaults
C# type generation SHALL preserve authored NX literal defaults on generated DTO properties when the
literal can be represented as a C# property initializer. Supported literal defaults SHALL include
string, integer, floating-point, boolean, and null literals. When a generated C# property has a
supported literal default, that authored initializer SHALL take precedence over the generator's
non-null reference `default!` initializer. When a C# generated field has a non-literal default
expression, generation SHALL continue and SHALL emit a warning that the default could not be
preserved.

#### Scenario: Record field literal defaults are emitted as C# initializers
- **WHEN** source contains `export type Settings = { enabled:bool = true count:int = 42 title:string = "hello" maybe:string? = null }`
- **THEN** generated C# SHALL include `public bool Enabled { get; set; } = true;`
- **AND** generated C# SHALL include `public long Count { get; set; } = 42;`
- **AND** generated C# SHALL include `public string Title { get; set; } = "hello";`
- **AND** generated C# SHALL include `public string? Maybe { get; set; } = null;`

#### Scenario: Union case field literal defaults are emitted as C# initializers
- **WHEN** source contains `export type LoadState = | failed { retryable:bool = true }`
- **THEN** generated C# SHALL include generated case DTO `LoadStateFailed`
- **AND** generated C# SHALL include `public bool Retryable { get; set; } = true;`

#### Scenario: External component prop literal defaults are emitted as C# initializers
- **WHEN** source contains `export external component <Toggle selected:bool = true label:string = "On" />`
- **THEN** generated C# SHALL include generated component prop DTO `Toggle`
- **AND** generated C# SHALL include `public bool Selected { get; set; } = true;`
- **AND** generated C# SHALL include `public string Label { get; set; } = "On";`

#### Scenario: Unsupported default expressions warn instead of silently changing semantics
- **WHEN** source contains `export type Settings = { enabled:bool = { !false } }`
- **THEN** C# generation SHALL emit a warning that the default for `Settings.enabled` could not be preserved
- **AND** generated C# SHALL omit a property initializer for `Enabled`
