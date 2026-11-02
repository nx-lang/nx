## ADDED Requirements

### Requirement: TypeScript evaluates selected property spreads
The runtime SHALL prepare `conditional-properties-v1` modules and evaluate each property entry in
order, spreading only the selected binding bag before content/default normalization. It SHALL
reject reserved spread entries without the required feature and unsupported features by name.

#### Scenario: Omission and explicit empty remain distinct
- **WHEN** a selected bag omits a field or explicitly supplies an empty value
- **THEN** only the omitted binding SHALL be eligible for an omitted-field default

#### Scenario: Inactive branches remain lazy
- **WHEN** an unselected property branch would fail during evaluation
- **THEN** execution SHALL complete without evaluating it
