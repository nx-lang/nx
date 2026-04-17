## 1. Update C# discriminator generation

- [x] 1.1 Remove synthetic `$type`/`__NxType` member emission from generated C# records and actions.
- [x] 1.2 Keep abstract-root JSON and MessagePack polymorphism metadata intact for record and action hierarchies.

## 2. Refresh verification coverage

- [x] 2.1 Update C# code generation unit tests to assert discriminator-free concrete and abstract output.
- [x] 2.2 Update any CLI integration assertions that still assume generated `__NxType` output.
- [x] 2.3 Run focused generator tests to verify the new contract.
