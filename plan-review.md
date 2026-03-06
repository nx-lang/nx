# Module Loader Plan Review

The `ModuleLoadError` layering issue is now resolved in the OpenSpec artifacts. These plan issues still need explicit decisions before implementation starts.

## 1. Binding imported names into scope is not enough for the current interpreter

The plan still says imported names will be "bound into the importing module's scope" (`openspec/changes/module-loader/specs/module-imports/spec.md:4`, `openspec/changes/module-loader/tasks.md:40`). That does not match the current runtime model. Function calls, element resolution, and enum member lookup all resolve through `module.find_item(...)`, not through runtime variables (`crates/nx-interpreter/src/interpreter.rs:466`, `crates/nx-interpreter/src/interpreter.rs:570`, `crates/nx-interpreter/src/interpreter.rs:690`; `crates/nx-hir/src/lib.rs:305`).

Recommendation: pick an explicit import-execution model in the plan. The realistic options are:
- synthesize a merged module that contains imported items before execution, or
- add an import-aware item-resolution layer instead of relying on scope injection.

## 2. The owner of multi-file orchestration is still undefined

The design says the implementation "may live partly in `nx-api` and partly in `nx-interpreter`" (`openspec/changes/module-loader/design.md:128`). The tasks are also split across `nx-api` and `nx-interpreter` without naming a single owner for module graph traversal, caching, and import evaluation (`openspec/changes/module-loader/tasks.md:26`, `openspec/changes/module-loader/tasks.md:35`). In the current code, `nx-api` is still a single-module path (`crates/nx-api/src/eval.rs:36`) and `nx-cli` still runs its own parse/lower/execute flow directly (`crates/nx-cli/src/main.rs:127`).

Recommendation: decide whether:
- `nx-api` owns multi-file orchestration and `nx-interpreter` stays a single-module executor, or
- a new driver/session layer owns module traversal and both `nx-api` and `nx-cli` call into it.

## 3. Entry-source origin is still underspecified

The new loader API correctly needs importer context (`openspec/changes/module-loader/design.md:88`), but the public entry point is still modeled as `eval_source(source, file_name, loaders)` (`openspec/changes/module-loader/design.md:115`). `ModuleSource` also still carries only `source` and `file_name` (`openspec/changes/module-loader/design.md:75`, `openspec/changes/module-loader/specs/module-loader/spec.md:40`). In the current code, `file_name` is explicitly display-only in `eval_source` (`crates/nx-api/src/eval.rs:27`), while `parse_str` hashes that same string into `SourceId` (`crates/nx-syntax/src/lib.rs:193`).

Recommendation: separate display name from true origin. The plan should introduce an entry path/origin concept, or a `ModuleOrigin`-style type, so relative imports, source identity, and diagnostics all use the same underlying origin instead of overloading `file_name`.

## 4. Export and visibility semantics are still missing

The spec still says wildcard imports expose "all public definitions" and selective imports pull named public items (`openspec/changes/module-loader/specs/module-imports/spec.md:20`, `openspec/changes/module-loader/specs/module-imports/spec.md:28`; `openspec/changes/module-loader/tasks.md:40`). But HIR items do not carry visibility or export metadata at all (`crates/nx-hir/src/lib.rs:213`), so there is nothing in the current model that distinguishes public from private.

Recommendation: decide one of these for v1:
- all top-level items are importable, with no export control yet, or
- export/visibility becomes part of this change and gets modeled in HIR.

## 5. Namespace imports are still not executable under the current runtime model

The spec still includes namespace imports such as `import "./ui" as UI` (`openspec/changes/module-loader/specs/module-imports/spec.md:10`), and the task list still expects namespace-import tests (`openspec/changes/module-loader/tasks.md:42`). But the interpreter only allows identifier callees for function calls (`crates/nx-interpreter/src/interpreter.rs:449`), and member access currently supports runtime records and enum members, not module namespaces (`crates/nx-interpreter/src/interpreter.rs:677`).

Recommendation: either cut namespace imports from v1, or define module-namespace semantics explicitly:
- what runtime value represents a module namespace,
- how `UI.Button(...)` is resolved,
- and whether namespace members participate in the same lookup rules as local items.

## 6. Circular imports are still specified as an infinite loop

The design still documents circular imports as a known limitation that will "infinite-loop" (`openspec/changes/module-loader/design.md:148`). The task list includes caching (`openspec/changes/module-loader/tasks.md:39`) but does not require any in-progress state or deterministic cycle failure.

Recommendation: even if full cycle handling stays out of scope, v1 should still fail deterministically instead of looping forever. The plan should require tracking modules in a loading state and producing a clear cycle diagnostic when re-entering one.
