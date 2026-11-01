## REMOVED Requirements

### Requirement: The interpreter checks constraints only at compile time
**Reason**: The interpreter is deleted. The IR runtimes' constraint checks and numeric match
patterns are already specified by "Any other value is checked where it enters a constrained site"
and "A match arm may use a numeric value pattern".
**Migration**: None. Programs with constrained types run through an IR runtime.
