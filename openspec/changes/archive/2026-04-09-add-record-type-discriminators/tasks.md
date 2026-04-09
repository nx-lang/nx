## 1. Exported Record Model

- [x] 1.1 Extend the exported code generation model with abstract-record metadata and descendant
      lookup helpers needed by the emitters.
- [x] 1.2 Add focused unit coverage for exported abstract record families in the shared code
      generation model.

## 2. TypeScript Generation

- [x] 2.1 Update the TypeScript emitter to generate `NxRecord`, abstract base contracts, concrete
      literal `$type` fields, and action-record discriminators for single-file output.
- [x] 2.2 Update library TypeScript generation to emit any needed cross-module `import type`
      statements for abstract-family unions and preserve stable module exports.

## 3. C# Generation

- [x] 3.1 Update the C# emitter to honor abstract records, keep abstract bases inheritable, and
      initialize concrete discriminator members to the concrete generated record name.
- [x] 3.2 Preserve the MessagePack `$type` mapping without colliding with user-defined generated
      members.

## 4. Verification

- [x] 4.1 Add or update TypeScript code generation tests for concrete records, abstract record
      families, cross-module library output, and action records.
- [x] 4.2 Add or update C# code generation tests for concrete records and abstract-base inheritance
      discriminator behavior.
