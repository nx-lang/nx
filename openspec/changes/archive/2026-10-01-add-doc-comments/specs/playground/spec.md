## MODIFIED Requirements

### Requirement: Diagnostics are reported against the visitor's source
The playground SHALL report every diagnostic from compiling the visitor's source, positioned at the
line and column the visitor sees, including the warnings of source that compiles, alongside its
value or its runtime error.

#### Scenario: Diagnostics are shown in the source pane
- **WHEN** compilation reports a diagnostic with a source position
- **THEN** the source pane SHALL mark the reported span, and the message SHALL be readable by the
  visitor

#### Scenario: A position with no width is still a position
- **WHEN** a diagnostic names a point rather than a range, as a missing token's insertion point does
- **THEN** the source pane SHALL mark it visibly

#### Scenario: Compilation errors do not break the session
- **WHEN** compilation fails for any reason, including a compiler that crashed or did not load
- **THEN** the playground SHALL report the failure and remain editable

#### Scenario: A warning in source that compiles is shown
- **WHEN** the visitor's source compiles and evaluates, and compiling reported a warning
- **THEN** the source pane SHALL mark the warning's span as a warning while the output pane shows
  the value

## ADDED Requirements

### Requirement: Source and output use readable themes
The playground SHALL color its source and output with GitHub's current light and dark themes,
`github-light-default` and `github-dark-default`, whose comment colors meet a 4.5:1 contrast ratio
against their backgrounds, and its panes SHALL use those themes' background and foreground colors.

#### Scenario: Comments are readable in the dark theme
- **WHEN** the playground is shown in its dark theme
- **THEN** a comment in the source SHALL be drawn with a contrast ratio of at least 4.5:1 against
  the editor background

### Requirement: Completion documentation is visible from the first completion
The source pane SHALL open the completion list's details, with the item's documentation, the first
time a completion list appears whose focused item has a detail or documentation to show. After
that the details SHALL be the visitor's to open and close, and SHALL stay as the visitor left them.

#### Scenario: The first completion shows its documentation
- **WHEN** the visitor requests completions for the first time at a property slot whose property is
  documented
- **THEN** the completion list SHALL show that documentation beside it without a further gesture

#### Scenario: Closed details stay closed
- **WHEN** the visitor closes the details and requests completions again
- **THEN** the details SHALL stay closed
