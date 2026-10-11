## 1. Package

- [x] 1.1 Create `packages/previewer` (`@nx-lang/previewer`) on the pattern of `packages/value-view`, depending on `@nx-lang/ir-runtime` alone, with `programFromImages` and `createPreviewSession`; verify with the scenarios of `A session runs one component of a prepared program`
- [x] 1.2 Add test fixtures that build the question-flow conformance program's images and read its `program.json` and `expected/` files, shared by the package's tests; verify the tests load them
- [x] 1.3 Implement the tick tree, `path`, going back and branching; verify with the scenarios of `Each tick keeps what made it and what it rendered` and `Going back runs nothing, and dispatching from the past branches`
- [x] 1.4 Implement dispatch by token and by handler pointer, with failures adding no tick; verify with the scenarios of `A host can dispatch by handler` and `A failure adds no tick`
- [x] 1.5 Implement new props on the current tick; verify with `A new respondent`

## 2. Scenarios and reload

- [x] 2.1 Implement scenario export and replay, with the stopping report; verify with the scenarios of `A run saves and replays as a scenario`
- [x] 2.2 Implement reload: keep the state, else replay, else keep the old program; verify with the scenarios of `Hot reload keeps the reviewer where they were`
- [x] 2.3 Keep origin entries on every tick and add the lookup by pointer; verify with the scenarios of `Each tick knows where its records came from`

## 3. Docs and release

- [x] 3.1 Document the package in its README with an example that runs the question flow, branches and replays a scenario; verify the example runs
- [x] 3.2 List `@nx-lang/previewer` in `docs/deployment.md` and `docs/deployment-setup.md` beside the other npm packages; verify the package builds and packs as the others do

## 4. Specs

- [x] 4.1 Run `openspec validate add-preview-session --strict`; verify it passes
