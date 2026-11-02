## REMOVED Requirements

### Requirement: Retirement preserves pending construction regression expectations
**Reason**: This follow-up completes conditional-property execution and intentionally replaces
positional record construction with a static language diagnostic. The temporary exception has
served its purpose; positional success is no longer a promised source behavior.
**Migration**: Activate the five conditional/example success tests. Replace the one ignored
positional constructor success test with active rejection plus equivalent markup success/default
coverage. Remove the six follow-up ignores and temporary limitation wording while keeping the
archived retirement inventories as historical records.

## ADDED Requirements

### Requirement: Rust evaluates selected property spreads
The runtime SHALL prepare `conditional-properties-v1` modules and evaluate property entries in
order, spreading only selected binding bags before content/default normalization. It SHALL reject
reserved spread entries without the required feature and unsupported features by name.

#### Scenario: Selected binding bags preserve presence
- **WHEN** a selected bag omits a field or explicitly supplies an empty value
- **THEN** omission SHALL remain distinct from the supplied empty value until normalization
- **AND** an inactive branch SHALL not evaluate its value expressions
