## Why

`support-rust-hosts` gave a Rust host what it needs to run NX: the three runtime crates, an instance
tree for components below the root, and a stack budget for a 1 MiB WebAssembly stack. None of it has
a real consumer yet. A spike (`~/src/drawnui-nx-spike`) drew five controls and one event with
DrawnUI for Rust; the fiddle's NX catalog declares 35 controls, 371 properties and 32 events.

The crates are not on crates.io yet, and a version published there is permanent. The DrawnUI fiddle
(`~/src/DrawnUi.FiddleEngine`) is the natural first consumer, and running it against this checkout
before the first publish is the cheapest moment to find what the runtime API gets wrong.

The fiddle has a second reason of its own. Its Rust language compiles every edit on a server and
takes 10 to 16 seconds. An NX snippet needs no Rust build: the spike showed that a prebuilt player
(DrawnUI for Rust plus the NX runtime; 4.78 MB in the spike, 6.4 MB with every control, 1.8 MB
compressed) takes a compiled NX image and redraws in about 10 ms. So NX is the one way to edit a DrawnUI for Rust drawing live.

## What Changes

- **A second NX language in the fiddle, `nx-rust` ("NX (Rust)").** It takes the same source as `nx`,
  with the same compiler, catalog, presets, diagnostics, hover and completion. It differs in what
  draws: DrawnUI for Rust in a frame of its own, where `nx` draws with DrawnUi.React.
- **A player crate in the fiddle repository** (`nx/rust/`), built once into a WebAssembly page the
  host serves. The page stays loaded across edits; each run hands it a new compiled image.
- **A control table generated from the catalog and from DrawnUI for Rust's own property
  declarations**, not written by hand. Most of the catalog's 371 properties have a DrawnUI for Rust
  property of the same name today; the generator lists the rest as unsupported, control by control
  with the reason, and a snippet that sets one is told once in the console.
- **The whole catalog, not five controls:** properties applied, patched in place and reset when a
  snippet stops setting them; events with their payloads; templated lists; authored components with
  their own state through the runtime's `InstanceTree`.
- **Errors and the console reach the editor**, shares store the compiled image, and the `/p/`
  player draws a share without the NX compiler.
- **Built against this checkout until the crates are published.** The player depends on
  `nx-ir-runtime` and `nx-value` by path while the work is under way. Releasing the crates is the
  last group of tasks, after the fiddle draws every NX preset through them, and the player then
  moves to the published versions.
- **Fixes in this repository for what the integration finds.** One gap is known already: no test
  covers the way a host holds components drawn inside a templated control's cells, which DrawnUI
  binds during layout, outside any pass over the instance tree.

Not in this change: a project export or publishing for `nx-rust`; a by-name property API inside
DrawnUI for Rust itself (the generator works from its source instead); anything in the private
drawfiddle.com host beyond the two registration lines its maintainer adds.

## Capabilities

### New Capabilities

- `fiddle-nx-rust-language`: NX drawn with DrawnUI for Rust as a language of the fiddle engine: the
  language entry, the player that stays loaded, how the catalog's controls, properties, events and
  templates reach DrawnUI for Rust, what a snippet is told about what cannot be drawn, shares and the
  player, and how the player is built against an NX checkout or published crates.

### Modified Capabilities

- `component-instance-tree`: a requirement is added. The tree bounds how deeply component
  instances nest, by `RuntimeOptions::max_component_depth` (100 by default), so a component that
  renders itself ends in a diagnostic in every Rust host. What the player does with cells needed
  no requirement: the tree already allowed it, and it gets a test and a README section.
- `rust-ir-runtime`: the requirement that names the limits a diagnostic carries gains
  `maxComponentDepth`, and `maxInputSize`, which the runtime already reported and the sentence
  left out.

## Impact

- **DrawnUi.FiddleEngine** (a branch there, a pull request to the DrawnUi organization):
  `nx/rust/` (the player crate, the control-table generator, its tests), `NxRustLanguage.cs`,
  `fiddle-nx-rust.js`, a build script next to `dev/build-nx-runtime.mjs`, presets shared with `nx`,
  `docs/ADDING-A-LANGUAGE.md`, `README-NX.md`, `AGENTS.md`, and one function on the React surface
  (`fiddleReactCompile`) so a language can compile with another's compiler without drawing.
- **This repository:** a test for cells held outside a pass in
  `crates/nx-codegen/src/ir_instance_tree_tests.rs`, whatever fixes in `crates/nx-ir-runtime` that
  and the integration call for, one new public option (`RuntimeOptions::max_component_depth`, with
  `NX_DEFAULT_MAX_COMPONENT_DEPTH`) checked by `InstanceTree::visit`, the runtime README,
  and the first crates.io release (the release itself is the user's: a tag, then publishing the
  draft).
- **Toolchains:** building the player needs Rust 1.98.1 with `wasm32-unknown-emscripten` and
  Emscripten 6. A fiddle host that does not offer `nx-rust` needs neither.
- **Dependencies:** DrawnUI for Rust `0.1.0-preview.6` from crates.io; `nx-ir-runtime` and
  `nx-value` by path, then by version.
- **The spike** (`~/src/drawnui-nx-spike`) is superseded once the player draws what it drew.
