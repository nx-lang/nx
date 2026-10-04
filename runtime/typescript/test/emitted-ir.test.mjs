import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import {
  NxIrRuntimeError,
  applyComponentStatePatch,
  callFunction,
  constructComponentDescriptor,
  dispatchComponentActions,
  evaluateComponent,
  evaluateFunction,
  initializeComponent,
  measureInputSize,
  normalizeComponentState,
  prepareNxIrProgram,
} from "../dist/src/index.js";

const testRoot = fileURLToPath(new URL(".", import.meta.url));
const repoRoot = resolve(testRoot, "../../..");

function assertEqual(actual, expected) {
  const actualJson = stableJson(actual);
  const expectedJson = stableJson(expected);
  if (actualJson !== expectedJson) {
    throw new Error(`Expected ${expectedJson}, got ${actualJson}`);
  }
}

function stableJson(value) {
  if (Array.isArray(value)) {
    return `[${value.map(stableJson).join(",")}]`;
  }
  if (value !== null && typeof value === "object") {
    return `{${Object.entries(value)
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([key, item]) => `${JSON.stringify(key)}:${stableJson(item)}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    encoding: "utf8",
    ...options,
  });
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(" ")} failed\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`,
    );
  }
  return result.stdout;
}

function runNxCli(args) {
  return run("cargo", ["run", "-q", "-p", "nx-cli", "--", ...args]);
}

function withSource(source, runTest) {
  const dir = mkdtempSync(join(tmpdir(), "nx-ir-runtime-"));
  try {
    const sourcePath = join(dir, "test.nx");
    writeFileSync(sourcePath, source);
    runTest(dir, sourcePath);
  } finally {
    rmSync(dir, { force: true, recursive: true });
  }
}

// Only the program's own image is read back. A source that reaches a prelude declaration also emits
// `prelude.nxir` beside it, and deliberately nothing here loads it: `prepareNxIrProgram` links with
// no resolver, so the runtime's built-in prelude supplies that slot — which is how a host runs one
// of these images too. No case below reaches the prelude today; one that does gets the fallback for
// free, so do not "fix" this by resolving `prelude.nxir` from disk.
function emitIr(dir, sourcePath) {
  const outputPath = join(dir, "ir");
  runNxCli(["codegen", sourcePath, "--target", "nx-ir", "--output", outputPath]);
  return new Uint8Array(readFileSync(join(outputPath, "test.nxir")));
}

function requiredFeaturesOf(image) {
  return prepareNxIrProgram(image).entry.module.artifact.requiredFeatures;
}

function nativeJson(sourcePath) {
  return JSON.parse(runNxCli(["run", sourcePath, "--format", "json"]));
}

function nativeFailure(sourcePath) {
  return spawnSync(
    "cargo",
    ["run", "-q", "-p", "nx-cli", "--", "run", sourcePath, "--format", "json"],
    {
      cwd: repoRoot,
      encoding: "utf8",
    },
  );
}

function generatedJsJson(outputPath, script) {
  writeFileSync(join(outputPath, "package.json"), "{ \"type\": \"module\" }\n");
  return JSON.parse(run("node", ["--input-type=module", "--eval", script], { cwd: outputPath }));
}

withSource(
  `
let answer(): int = { 41 }
let root(): int = { answer() + 1 + (7 / 2) + (7 % 2) }
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    assertEqual(evaluateFunction(prepared, "root"), nativeJson(sourcePath));
    console.log("ok - emitted function IR matches native interpreter output");
  },
);

withSource(
  `
let root(): int = { 1 / 0 }
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    let runtimeFailed = false;
    try {
      evaluateFunction(prepared, "root");
    } catch (error) {
      runtimeFailed = String(error).includes("Division by zero");
    }
    if (!runtimeFailed) {
      throw new Error("Expected TypeScript IR runtime to fail with Division by zero");
    }

    const native = nativeFailure(sourcePath);
    if (native.status === 0 || !`${native.stdout}\n${native.stderr}`.includes("Division by zero")) {
      throw new Error(
        `Expected native interpreter to fail with Division by zero\nstdout:\n${native.stdout}\nstderr:\n${native.stderr}`,
      );
    }
    console.log("ok - emitted division-by-zero IR matches native interpreter failure");
  },
);

withSource(
  `
external component <Item label:string />
external component <Stack content Children:Item+ />
let root() = { <Stack><Item label="only" /></Stack> }
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    assertEqual(evaluateFunction(prepared, "root"), nativeJson(sourcePath));
    console.log("ok - a single child of a list-typed content property matches the native interpreter");
  },
);

withSource(
  `
type Shadow = { Y:float64 = 0.0 }
external component <Shape shadows?:Shadow+ sizes?:float64+ />
let root() = { <Shape shadows={ <Shadow Y=6.0 /> } sizes={3.0} /> }
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    assertEqual(evaluateFunction(prepared, "root"), nativeJson(sourcePath));
    console.log("ok - a single value at a list-typed property matches the native interpreter");
  },
);

withSource(
  `
type User = { name:string = "anon" email?:string }
let root(): User.Update = { <User.Update email={} /> }
`,
  (dir, sourcePath) => {
    const ir = emitIr(dir, sourcePath);
    if (!requiredFeaturesOf(ir).includes("update-records-v1")) {
      throw new Error(`Expected the update-record feature, got ${JSON.stringify(requiredFeaturesOf(ir))}`);
    }
    const prepared = prepareNxIrProgram(ir);
    const expected = { $type: "User.Update", email: null };
    assertEqual(evaluateFunction(prepared, "root"), expected);
    assertEqual(nativeJson(sourcePath), expected);
    console.log("ok - an emitted update record keeps absent fields absent like the native interpreter");
  },
);

withSource(
  `
abstract type Base = { name:string = "anon" }
type User extends Base = { role:string }
let root() = { <User role="admin" /> }
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    assertEqual(evaluateFunction(prepared, "root"), nativeJson(sourcePath));
    console.log("ok - an inherited record field and its default match the native interpreter");
  },
);

withSource(
  `
abstract type EventBase = { source:string = "app" }
type UiEvent extends EventBase =
  | clicked { x:int }
  | dismissed
let root(): UiEvent = { <UiEvent.clicked x=3 /> }
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    assertEqual(evaluateFunction(prepared, "root"), nativeJson(sourcePath));
    console.log("ok - a union case carries its base's fields like the native interpreter");
  },
);

withSource(
  `
abstract type Base = { name:string }
type User extends Base = { role:string }
external component <Card owner:Base />
let root() = { <Card owner={<User name="Ada" role="admin" />} /> }
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    assertEqual(evaluateFunction(prepared, "root"), nativeJson(sourcePath));
    console.log("ok - a derived record at a base-typed field matches the native interpreter");
  },
);

withSource(
  `
abstract type Shape = { name:string = "anon" }
type Figure extends Shape =
  | circle { r:int }
  | square { s:int }
external component <Frame held:Shape />
let root() = { <Frame held={<Figure.circle r=2 />} /> }
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    assertEqual(evaluateFunction(prepared, "root"), nativeJson(sourcePath));
    console.log("ok - a union case at a base-typed field matches the native interpreter");
  },
);

withSource(
  `
type Ints = int+
type AlsoInts = Ints
abstract external component <Item />
external component <Leaf extends Item />
type Items = Item+
external component <Box xs?:AlsoInts content items?:Items />
let root() = { <Box xs={3}><Leaf /></Box> }
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    assertEqual(evaluateFunction(prepared, "root"), nativeJson(sourcePath));
    console.log("ok - a list spelled through aliases coerces like the native interpreter");
  },
);

withSource(
  `
type Thickness = { Left:float64 = 0.0  Top:float64 = 0.0 }
abstract external component <Control Padding:Thickness = {<Thickness />} content Children?:Control+ />
external component <Panel extends Control />
let root() = { <Panel Padding={<Thickness Left=4.0 />}><Panel /></Panel> }
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    assertEqual(evaluateFunction(prepared, "root"), nativeJson(sourcePath));
    console.log("ok - a record-typed property matches the native interpreter");
  },
);

withSource(
  `
external component <TextInput value:string />
component <SearchBox placeholder:string = "Find docs" /> = {
  state { query:string = { placeholder } }
  <TextInput value={query} />
}
let root() = { <SearchBox /> }
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    const generatedPath = join(dir, "js");
    runNxCli(["codegen", sourcePath, "--target", "javascript", "--output", generatedPath]);
    const indexUrl = pathToFileURL(join(generatedPath, "index.js")).href;
    const generated = generatedJsJson(
      generatedPath,
      `
import { root, SearchBoxSchema } from ${JSON.stringify(indexUrl)};
console.log(JSON.stringify({
  descriptor: root(),
  init: SearchBoxSchema.initializeJson({}),
  evaluated: SearchBoxSchema.evaluateJson({}, { query: "docs" })
}));
`,
    );

    const { rendered, state } = initializeComponent(prepared, "SearchBox");
    assertEqual(
      {
        descriptor: evaluateFunction(prepared, "root"),
        init: { rendered, state },
        evaluated: evaluateComponent(prepared, "SearchBox", {}, { query: "docs" }).rendered,
      },
      generated,
    );
    console.log("ok - emitted component IR matches generated JavaScript behavior");
  },
);

withSource(
  `
type Range = { T:type start:T end:T }
let ints(): <Range T=int/> = { <Range T=int start={1} end={5} /> }
let first(): int = { ints().start }
let moved(): <Range T=int/> = { apply(ints(), <Range.Update T=int end={9} />) }
let root() = { moved() }
`,
  (dir, sourcePath) => {
    // A type argument leaves no trace below the checker, so all three backends see one `Range`
    // with the fields `start` and `end` and agree on every value it takes part in.
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    const generatedPath = join(dir, "js");
    runNxCli(["codegen", sourcePath, "--target", "javascript", "--output", generatedPath]);
    const indexUrl = pathToFileURL(join(generatedPath, "index.js")).href;
    const generated = generatedJsJson(
      generatedPath,
      `
import { ints, first, moved } from ${JSON.stringify(indexUrl)};
console.log(JSON.stringify({ ints: ints(), first: first(), moved: moved() }));
`,
    );

    const fromIr = {
      ints: evaluateFunction(prepared, "ints"),
      first: evaluateFunction(prepared, "first"),
      moved: evaluateFunction(prepared, "moved"),
    };
    assertEqual(fromIr, generated);
    assertEqual(fromIr.ints, { $type: "Range", start: 1, end: 5 });
    assertEqual(fromIr.moved, nativeJson(sourcePath));
    console.log("ok - a generic record agrees across the interpreter, generated code and the IR runtime");
  },
);

withSource(
  `
type User = { name:string email?:string age?:int }
let key(): User.Property = { User.Property.email }
let root(): User = { apply(<User name="Ada" email="x@y" />, <User.Update email={} />) }
let keys(): User.Property* = { changed(<User.Update age={} name="Ada" />) }
`,
  (dir, sourcePath) => {
    const ir = emitIr(dir, sourcePath);
    for (const feature of ["property-unions-v1", "update-intrinsics-v1"]) {
      if (!requiredFeaturesOf(ir).includes(feature)) {
        throw new Error(`Expected the ${feature} feature, got ${JSON.stringify(requiredFeaturesOf(ir))}`);
      }
    }
    const prepared = prepareNxIrProgram(ir);
    assertEqual(evaluateFunction(prepared, "key"), "email");
    assertEqual(evaluateFunction(prepared, "root"), nativeJson(sourcePath));
    assertEqual(evaluateFunction(prepared, "keys"), ["name", "age"]);
    console.log("ok - an emitted property reference and apply match the native interpreter");
  },
);

withSource(
  `
external component <Button label:string emits { Tapped { } } />
component <Counter step:int = 1 /> = {
  state { count:int = 0 }
  <Button label="Add" onTapped=<Update count={count + step} /> />
}
`,
  (dir, sourcePath) => {
    const ir = emitIr(dir, sourcePath);
    if (!requiredFeaturesOf(ir).includes("action-handlers-v1")) {
      throw new Error(`Expected the action-handler feature, got ${JSON.stringify(requiredFeaturesOf(ir))}`);
    }
    const prepared = prepareNxIrProgram(ir);
    const initialized = initializeComponent(prepared, "Counter", { step: 3 });
    assertEqual(initialized.rendered, {
      $type: "Button",
      label: "Add",
      onTapped: { $type: "ActionHandler", action: "Button.Tapped", token: "h1-1" },
    });
    const dispatched = dispatchComponentActions(prepared, initialized.instance, [
      { $type: "ActionHandlerInvocation", token: "h1-1", action: { $type: "Button.Tapped" } },
    ]);
    assertEqual(dispatched.state, { count: 3 });
    assertEqual(evaluateComponent(prepared, "Counter", {}, { count: 1 }).rendered.onTapped, {
      $type: "ActionHandler",
      action: "Button.Tapped",
    });
    console.log("ok - emitted component IR carries an action handler the runtime initializes and dispatches");
  },
);

withSource(
  `
type Contact = { name:string }
let <ContactRow Item:Contact Index:int />: string = {Item.name + "#" + Index}
let <Compact Item:Contact />: string = {Item.name}
external component <List TItem:type ItemsSource?:TItem+ ItemTemplate?:<function Item:TItem Index:int />: string />
component <Section Item:Contact Row:<function Item:Contact Index:int />: string /> = { <Row Item={Item} Index=2 /> }
let root() = <List TItem=Contact ItemsSource={ <Contact name="Ada" /> } ItemTemplate={ContactRow} />
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    if (!requiredFeaturesOf(emitIr(dir, sourcePath)).includes("function-values-v1")) {
      throw new Error("Expected the module to require function-values-v1");
    }
    const identity = prepared.entry.module.identity;
    const root = evaluateFunction(prepared, "root");
    assertEqual(root, nativeJson(sourcePath));
    assertEqual(root.ItemTemplate, { $type: "Function", module: identity, name: "ContactRow" });
    assertEqual(callFunction(prepared, root.ItemTemplate, { Item: { $type: "Contact", name: "Kai" }, Index: 5, Extra: true }), "Kai#5");
    const compact = { $type: "Function", module: identity, name: "Compact" };
    assertEqual(callFunction(prepared, compact, { Item: { $type: "Contact", name: "Kai" }, Index: 5 }), "Kai");
    assertEqual(initializeComponent(prepared, "Section", { Item: { $type: "Contact", name: "Ada" }, Row: compact }).rendered, "Ada");
    assertEqual(initializeComponent(prepared, "Section", { Item: { $type: "Contact", name: "Ada" }, Row: root.ItemTemplate }).rendered, "Ada#2");
    console.log("ok - function values render as Function records and call by name in both runtimes");
  },
);

withSource(
  `
external component <Box Label?:string Same?:boolean Other?:boolean />
let <Wrap Item:object />: string = "w"
let <Plain Item:object />: string = "p"
let F: <function Item:object />: string = {Wrap}
let G: <function Item:object />: string = {Wrap}
let H: <function Item:object />: string = {Plain}
let root() = <Box Label=<F Item="x" /> Same={F == G} Other={F == H} />
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    const root = evaluateFunction(prepared, "root");
    assertEqual(root, nativeJson(sourcePath));
    assertEqual(root, { $type: "Box", Label: "w", Same: true, Other: false });
    console.log("ok - a top-level let of function type is callable and compares by declaration in both runtimes");
  },
);

withSource(
  `
type Item = { n:int }
external component <Stack content Children:Item+ />
let xs = { 1 2 3 }
let <Items content Items:Item+ />: Item+ = {Items}
let <Shift Items:Item+ By:int />: Item+ = { for i in Items { <Item n={i.n + By} /> } }
let seed = { for x in xs { <Item n={x} /> } }
let viaComponent() = <Stack> for x in xs { <Item n={x} /> } for x in xs { <Item n={x + 10} /> } </Stack>
let viaFunction() = <Items> <Shift Items={seed} By=0 /> <Shift Items={seed} By=10 /> </Items>
let root() = { viaComponent() viaFunction() }
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    const root = evaluateFunction(prepared, "root");
    assertEqual(root, nativeJson(sourcePath));
    // Two `for` loops side by side, and two list-returning calls, each read as one list of children.
    assertEqual(root[0].Children.length, 6);
    // The braced value at the top splices too, so the six items of `viaFunction()` sit beside the
    // one element of `viaComponent()` rather than nested inside a list of their own.
    assertEqual(root.length, 7);
    console.log("ok - list-valued content children are spliced as the native interpreter splices them");
  },
);

withSource(
  `
let xs:string+ = {"a" "b"}
let ys:string+ = {"c"}
let root(): string+ = { xs ys }
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    const root = evaluateFunction(prepared, "root");
    assertEqual(root, nativeJson(sourcePath));
    assertEqual(root, ["a", "b", "c"]);
    console.log("ok - two sequences in a braced value concatenate as the native interpreter concatenates them");
  },
);

withSource(
  `
type Row = { cells:int+ }
let rows:Row+ = { <Row cells={1 2}/> <Row cells={3 4}/> }
let flat(): int+ = { for r in rows { r.cells } }
let evens(): int* = { for n in 1..=4 { if (n % 2 == 0) { n } } }
let root() = { flat() evens() }
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    const root = evaluateFunction(prepared, "root");
    assertEqual(root, nativeJson(sourcePath));
    // A list-yielding body concatenates, and an iteration whose conditional is not taken
    // contributes nothing.
    assertEqual(root, [1, 2, 3, 4, 2, 4]);
    console.log("ok - a list-yielding and a conditional `for` body match the native interpreter");
  },
);

withSource(
  `
type A = { n:int = 1 }
type Box = { content items:A+ }
let c = false
let root(): Box = { <Box><A/>{if c { <A/> }}</Box> }
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    const root = evaluateFunction(prepared, "root");
    assertEqual(root, nativeJson(sourcePath));
    // A conditional child that does not fire contributes no items, and never a null.
    assertEqual(root.items, [{ $type: "A", n: 1 }]);
    console.log("ok - an untaken conditional child contributes no items, as in the native interpreter");
  },
);

// The flat sequence model, checked in all three engines at once: a spliced braced value, a
// list-yielding `for` and an untaken conditional child have to produce one value, with no nested
// list and no null item, in the interpreter, the IR runtime and generated JavaScript alike.
withSource(
  `
type Badge = { n:int = 1 }
type Row = { cells:int+ }
type Box = { content items:Badge+ }
type Result = { spliced:Badge+ values:string+ flat:int+ boxed:Box }
let some:Badge+ = { <Badge/> <Badge/> }
let xs:string+ = {"a" "b"}
let rows:Row+ = { <Row cells={1 2}/> <Row cells={3 4}/> }
let c = false
let root(): Result = <Result
  spliced={some <Badge/>}
  values={ xs "c" }
  flat={for r in rows { r.cells }}
  boxed={<Box><Badge/>{if c { <Badge/> }}</Box>}
/>
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    const generatedPath = join(dir, "js");
    runNxCli(["codegen", sourcePath, "--target", "javascript", "--output", generatedPath]);
    const indexUrl = pathToFileURL(join(generatedPath, "index.js")).href;
    const generated = generatedJsJson(
      generatedPath,
      `
import { root } from ${JSON.stringify(indexUrl)};
console.log(JSON.stringify(root()));
`,
    );

    const viaIr = evaluateFunction(prepared, "root");
    assertEqual(viaIr, nativeJson(sourcePath));
    assertEqual(viaIr, generated);
    assertEqual(viaIr.spliced.length, 3);
    assertEqual(viaIr.values, ["a", "b", "c"]);
    assertEqual(viaIr.flat, [1, 2, 3, 4]);
    // The conditional child did not fire, so it contributed no items and no null.
    assertEqual(viaIr.boxed.items.length, 1);
    console.log("ok - splicing and untaken conditionals agree in the interpreter, the IR runtime and generated JavaScript");
  },
);

// The contribution rule applies again to whatever a taken branch is, so a conditional nested in a
// conditional contributes an item or nothing -- never the `null` its value form would have. Each
// engine resolves the branch and then asks the same question of it, so this is where the three
// would drift apart if one of them evaluated the item whole instead.
withSource(
  `
type Badge = { n:int = 1 }
type Box = { content items:Badge+ }
type Loose = { content items?:Badge+ }
type Nested = { openInOpen:Box takenInner:Box closedOverOpen:Loose }
let yes = true
let no = false
let root(): Nested = <Nested
  openInOpen={<Box><Badge/>{if yes { if no { <Badge n=2 /> } }}</Box>}
  takenInner={<Box><Badge/>{if yes { if yes { <Badge n=5 /> } }}</Box>}
  closedOverOpen={<Loose><Badge/><Badge n=6 />{if yes { if no { <Badge n=3 /> } } else { <Badge n=4 /> }}</Loose>}
/>
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    const generatedPath = join(dir, "js");
    runNxCli(["codegen", sourcePath, "--target", "javascript", "--output", generatedPath]);
    const indexUrl = pathToFileURL(join(generatedPath, "index.js")).href;
    const generated = generatedJsJson(
      generatedPath,
      `
import { root } from ${JSON.stringify(indexUrl)};
console.log(JSON.stringify(root()));
`,
    );

    const viaIr = evaluateFunction(prepared, "root");
    assertEqual(viaIr, nativeJson(sourcePath));
    assertEqual(viaIr, generated);
    // The inner conditional was not taken, so the outer one contributed nothing at all.
    assertEqual(viaIr.openInOpen.items.length, 1);
    // Both taken: the inner conditional's item arrives through the outer one.
    assertEqual(viaIr.takenInner.items.length, 2);
    assertEqual(viaIr.takenInner.items[1].n, 5);
    // An `else` on the outer conditional does not make the untaken inner one contribute an empty
    // item: the branch that was taken is itself the thing that contributes nothing.
    assertEqual(viaIr.closedOverOpen.items.length, 2);
    console.log("ok - a conditional nested in a conditional contributes nothing in all three engines");
  },
);

// A body that was written and produced nothing is not the same as no body at all. The first binds
// the empty value; only the second leaves the content property to its declared default. A body that
// may produce nothing is admitted only at an optional content property, which has no default, so
// the two are told apart across two types: `Open` binds the empty value — an omitted key — however
// the body came to produce nothing, and `Box` takes its default only when no body was written.
withSource(
  `
type Badge = { n:int = 1 }
type Box = { content items:Badge+ = { <Badge n=9 /> } }
type Open = { content items?:Badge+ }
type Defaults = { untaken:Open empty:Open noIterations:Open absent:Box }
let c = false
let none:Badge* = { }
let root(): Defaults = <Defaults
  untaken={<Open>{if c { <Badge n=2 /> }}</Open>}
  empty={<Open>{}</Open>}
  noIterations={<Open>{for b in none { b }}</Open>}
  absent={<Box/>}
/>
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    const generatedPath = join(dir, "js");
    runNxCli(["codegen", sourcePath, "--target", "javascript", "--output", generatedPath]);
    const indexUrl = pathToFileURL(join(generatedPath, "index.js")).href;
    const generated = generatedJsJson(
      generatedPath,
      `
import { root } from ${JSON.stringify(indexUrl)};
console.log(JSON.stringify(root()));
`,
    );

    const viaIr = evaluateFunction(prepared, "root");
    assertEqual(viaIr, nativeJson(sourcePath));
    assertEqual(viaIr, generated);
    // A body that produced nothing binds the empty value, however it came to produce nothing, and
    // an empty optional field is an omitted key.
    for (const field of ["untaken", "empty", "noIterations"]) {
      assertEqual("items" in viaIr[field], false);
      assertEqual("items" in generated[field], false);
    }
    // No body at all is the one case the declared default is for.
    assertEqual(viaIr.absent.items.length, 1);
    assertEqual(viaIr.absent.items[0].n, 9);
    console.log("ok - a body that produced nothing binds the empty value rather than the declared default");
  },
);

// A conditional with no `else` carries an implicit `else { }`, so it admits zero in its own right
// and needs no rule of its own in any engine: an empty arm written out and a missing one agree, and
// a conditional alone in its braces behaves as it does beside other items.
withSource(
  `
type Badge = { n:int = 1 }
type Box = { content items:Badge+ }
type Implicit = { emptyArm:Box missingArm:Box alone?:int beside:int+ taken?:int }
let c = false
let t = true
let root(): Implicit = <Implicit
  emptyArm={<Box><Badge/>{if c { <Badge n=2 /> } else { }}</Box>}
  missingArm={<Box><Badge/>{if c { <Badge n=2 /> }}</Box>}
  alone={if c { 1 }}
  beside={if c { 1 } 2}
  taken={if t { 1 }}
/>
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    const generatedPath = join(dir, "js");
    runNxCli(["codegen", sourcePath, "--target", "javascript", "--output", generatedPath]);
    const indexUrl = pathToFileURL(join(generatedPath, "index.js")).href;
    const generated = generatedJsJson(
      generatedPath,
      `
import { root } from ${JSON.stringify(indexUrl)};
console.log(JSON.stringify(root()));
`,
    );

    const viaIr = evaluateFunction(prepared, "root");
    assertEqual(viaIr, nativeJson(sourcePath));
    // The written empty arm and the missing one are the same thing.
    assertEqual(viaIr.emptyArm, viaIr.missingArm);
    assertEqual(viaIr.emptyArm.items.length, 1);
    // Alone in its braces an untaken conditional is the empty value, an omitted key at an optional
    // field; beside another item it contributes nothing.
    assertEqual("alone" in viaIr, false);
    assertEqual(viaIr.beside, [2]);
    // A taken conditional is its branch: a `?` value that holds an item is the item itself.
    assertEqual(viaIr.taken, 1);
    assertEqual(generated, viaIr);
    console.log("ok - a conditional with no else admits zero in all three engines, alone or beside other items");
  },
);

// A taken conditional's value has to be what its type claims, or code that consumes it breaks
// differently in each engine. Iterating it is the sharpest test: a `for` over a `?` value that
// holds an item runs once over that item — before this was pinned, the interpreter raised, the IR
// runtime threw, and generated JavaScript iterated a string's characters, so `"new"` became
// `"n!" "e!" "w!"`.
withSource(
  `
type Out = { counted?:int tagged?:string mixed:int+ }
let c = true
let v = { if c { 1 } }
let tags:string? = { if c { "new" } }
let xs:int+ = {5 6}
let either = { if c { 1 } else { xs } }
let root(): Out = <Out
  counted={for x in v { x * 10 }}
  tagged={for t in tags { t + "!" }}
  mixed={for x in either { x }}
/>
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    const generatedPath = join(dir, "js");
    runNxCli(["codegen", sourcePath, "--target", "javascript", "--output", generatedPath]);
    const indexUrl = pathToFileURL(join(generatedPath, "index.js")).href;
    const generated = generatedJsJson(
      generatedPath,
      `
import { root } from ${JSON.stringify(indexUrl)};
console.log(JSON.stringify(root()));
`,
    );

    const viaIr = evaluateFunction(prepared, "root");
    assertEqual(viaIr, nativeJson(sourcePath));
    assertEqual(viaIr, generated);
    // A `for` over a `?` value yields a `?` value, so the one item is the value itself.
    assertEqual(viaIr.counted, 10);
    assertEqual(viaIr.tagged, "new!");
    assertEqual(viaIr.mixed, [1]);
    console.log("ok - a taken conditional iterates as the value its type says, in all three engines");
  },
);

// A conditional whose branches join to a sequence that admits zero has to work beside a sequence as
// well as beside a scalar: an untaken branch is the empty value, never `[[]]` or a null item, and an
// empty optional field is an omitted key. `xs` holds two items on purpose: a one-item `{"a"}` at the
// `let xs:string+` annotation meets the separate single-item-lift gap in generated code, which this
// case is not about.
withSource(
  `
type Out = { absent?:string+ present?:string+ reversed?:string+ }
let no = false
let yes = true
let xs:string+ = {"a" "b"}
let none:string* = {}
let root(): Out = <Out
  absent={if no { xs }}
  present={if yes { xs }}
  reversed={if yes { none } else { xs }}
/>
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    const generatedPath = join(dir, "js");
    runNxCli(["codegen", sourcePath, "--target", "javascript", "--output", generatedPath]);
    const indexUrl = pathToFileURL(join(generatedPath, "index.js")).href;
    const generated = generatedJsJson(
      generatedPath,
      `
import { root } from ${JSON.stringify(indexUrl)};
console.log(JSON.stringify(root()));
`,
    );

    const viaIr = evaluateFunction(prepared, "root");
    assertEqual(viaIr, nativeJson(sourcePath));
    assertEqual(viaIr, generated);
    assertEqual(viaIr.present, ["a", "b"]);
    // An empty optional field is an omitted key in canonical output, never `[]` or `[null]`.
    assertEqual("absent" in viaIr, false);
    assertEqual("reversed" in viaIr, false);
    console.log("ok - an untaken branch beside a sequence is an empty optional sequence in all three engines");
  },
);

// A lone content child binds as the child's value lifted once to the property's declared type,
// the one lone-child rule every engine shares. So an empty sequence alone in a body binds the empty
// value at an optional content property — an omitted key — in the interpreter, generated JavaScript
// and the IR runtime alike; the IR runtime once spliced a lone child at any list-typed property and
// then rejected the null it found, which is why it was left out of this comparison before.
withSource(
  `
type A = { n:int = 1 }
type Box = { content items?:A+ }
let c = false
let as2:A+ = { <A/> <A n=2 /> }
let root() = { <Box>{if c { as2 }}</Box> }
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    const generatedPath = join(dir, "js");
    runNxCli(["codegen", sourcePath, "--target", "javascript", "--output", generatedPath]);
    const indexUrl = pathToFileURL(join(generatedPath, "index.js")).href;
    const generated = generatedJsJson(
      generatedPath,
      `
import { root } from ${JSON.stringify(indexUrl)};
console.log(JSON.stringify(root()));
`,
    );
    const native = nativeJson(sourcePath);
    assertEqual(generated, native);
    assertEqual(evaluateFunction(prepared, "root"), native);
    assertEqual("items" in native, false);
    console.log("ok - a lone empty sequence binds an empty optional content property in all three engines");
  },
);

// The same lone-child rule at an optional `+` content property, whose read type is `*`: one child
// binds a one-item array, as it does at a required `+` property, in every engine.
withSource(
  `
type A = { n:int = 1 }
type Box = { content items?:A+ }
let root() = { <Box><A/></Box> }
`,
  (dir, sourcePath) => {
    const prepared = prepareNxIrProgram(emitIr(dir, sourcePath));
    const generatedPath = join(dir, "js");
    runNxCli(["codegen", sourcePath, "--target", "javascript", "--output", generatedPath]);
    const indexUrl = pathToFileURL(join(generatedPath, "index.js")).href;
    const generated = generatedJsJson(
      generatedPath,
      `
import { root } from ${JSON.stringify(indexUrl)};
console.log(JSON.stringify(root()));
`,
    );
    const native = nativeJson(sourcePath);
    assertEqual(native.items, [{ $type: "A", n: 1 }]);
    assertEqual(evaluateFunction(prepared, "root"), native);
    assertEqual(generated, native);
    console.log("ok - a lone child at an optional plus content property binds a one-item array in all three engines");
  },
);

// ------------------------------------------------------------------------------------------------
// The operation budget and the limits a diagnostic names
// ------------------------------------------------------------------------------------------------

/** The error `run` throws, which must be the runtime's own. */
function runtimeFailure(run) {
  try {
    run();
  } catch (error) {
    if (!(error instanceof NxIrRuntimeError)) {
      throw new Error(`Expected an NxIrRuntimeError, got ${error?.constructor?.name}: ${error instanceof Error ? error.message : String(error)}`);
    }
    return error;
  }
  throw new Error("Expected the evaluation to fail");
}

/** The limit a resource-limit failure names. */
function limitOf(run) {
  const diagnostic = runtimeFailure(run).diagnostics[0];
  if (diagnostic.code !== "nx-ir-resource-limit") {
    throw new Error(`Expected nx-ir-resource-limit, got ${diagnostic.code}: ${diagnostic.message}`);
  }
  return diagnostic.limit;
}

/** A budget no evaluation of these tests reaches: what a host sets to read a count and limit nothing. */
const ample = 2 ** 40;

/** The operations a call reports it used under an ample budget, whether it returns or throws. */
function reportedOperations(run) {
  const usage = {};
  try {
    run({ maxOperations: ample, usage });
  } catch (error) {
    if (!(error instanceof NxIrRuntimeError)) {
      throw error;
    }
  }
  return usage.operations;
}

/**
 * What an evaluation costs: the least budget it succeeds under, found by doubling and bisecting.
 * The usage report gives the same number in one evaluation, which is checked for every count
 * found here.
 */
function cost(run) {
  const succeeds = (maxOperations) => {
    try {
      run({ maxOperations });
      return true;
    } catch (error) {
      if (error instanceof NxIrRuntimeError && error.diagnostics[0].limit?.name === "maxOperations") {
        return false;
      }
      throw error;
    }
  };
  let high = 1;
  while (!succeeds(high)) {
    high *= 2;
  }
  let low = 0;
  while (low < high) {
    const middle = Math.floor((low + high) / 2);
    if (succeeds(middle)) {
      high = middle;
    } else {
      low = middle + 1;
    }
  }
  assertEqual(reportedOperations(run), high);
  return high;
}

/** Asserts that an evaluation costs exactly `operations`: it succeeds under that budget and fails on the budget under one less. */
function assertCosts(operations, run) {
  run({ maxOperations: operations });
  assertEqual(limitOf(() => run({ maxOperations: operations - 1 })), { name: "maxOperations", value: operations - 1 });
  assertEqual(reportedOperations(run), operations);
}

withSource(
  `
external component <Button emits { Tapped { } } />
action Ping = { n:int }
type Shape = | dot | ring { radius:int } | box { side:int }
type Bag = { items?:int+ }

let add(a:int, b:int) = { a + b }
let squares() = { for i in 0..4 { i * i } }
let pick(flag:boolean) = { if flag { 1 } else { for i in 0..1000 { i } } }
let one() = { 1 }
let callOne() = { one() }
let twice(xs:int+) = { xs xs }
let same(xs:int+) = { xs }
let join(a:string, b:string) = { a + b }
let sameText(a:string, b:string) = { a == b }
let corners(shape:Shape) = { if shape is { dot, ring => 0  box => 4 } }
let triple(n:int) = { n * 3 }
let through(f: <function n:int />: int) = <f n={2} />
let nested() = { for a in 0..1000 { for b in 0..1000 { for c in 0..1000 { a + b + c } } } }
let wide() = { for i in 0..1000000 { i } }
let tooWide() = { for i in 0..2000000 { i } }
let growList(n:int, xs:int+): int+ = { if n == 0 { xs } else { growList(n - 1, { xs xs }) } }
let growText(n:int, s:string): string = { if n == 0 { s } else { growText(n - 1, s + s) } }
let spin(n:int): int = { spin(n + 1) }
let ratio(n:int): int = { n / 0 }
let longContent() = <list>{for i in 0..200000 { i }}</list>

component <Busy /> = {
  state { items?:int+ }
  <Button onTapped=<Update items={for i in 0..300 { i }} /> />
}
component <Eager /> = {
  state { bag:Bag = <Bag items={for i in 0..100000 { i }} /> }
  <Button />
}
type Heavy = { bag:Bag = <Bag items={for i in 0..100000 { i }} /> }
component <Costly heavy:Heavy = <Heavy /> /> = {
  state { held?:Heavy }
  <Button />
}
type F = { v:float64 }
type Opt = { a:int = 1 b?:int c?:string }
let selfEq(f:F): boolean = { f == f }
let sameObject(o:object): boolean = { o == o }
let selfEqElement(v:float64): boolean = { sameObject(<box v={v} />) }
let ignore(content c:object): int = { 1 }
let many(xs:object, count:int) = { for i in 0..count { ignore(xs) } }
let repeated(o:object, n:int) = { for i in 0..n { o } }
let passObject(o:object): object = { o }
let patchOnly() = { <Opt.Update b={5} /> }
let cmpMany(a:object, b:object, count:int) = { for i in 0..count { a == b } }
let bindObject(xs:object): int = { ignore(xs) }
let wideOther(): object = { 1152921504606846977 }
let wideSame(): object = { 1152921504606846976 }
let patWide(): int = { if wideOther() is { 1152921504606846976 => 1  else => 0 } }
let patWideSame(): int = { if wideSame() is { 1152921504606846976 => 1  else => 0 } }
let eqWide(o:object): boolean = { o == wideSame() }
let patHost(o:object): int = { if o is { 1152921504606846976 => 1  else => 0 } }
let button(a:int, b:int) = <Button onTapped=<Ping n={a + b} /> />
let sameButton(a:int, b:int, c:int, d:int): boolean = { button(a, b) == button(c, d) }
type Tree = { kids?:Tree+ }
let growTree(n:int, t:Tree): Tree = { if n == 0 { t } else { growTree(n - 1, <Tree kids={ t t } />) } }
let held(n:int): int = { if growTree(n, <Tree />) == <Tree /> { 1 } else { 0 } }
let build(n:int): Tree = { growTree(n, <Tree />) }
let growPair(n:int, t:object): object = { if n == 0 { t } else { growPair(n - 1, <pair a={t} b={t} />) } }
let samePair(n:int): boolean = { growPair(n, <leaf />) == growPair(n, <leaf />) }
let buildPair(n:int): object = { growPair(n, <leaf />) }
let keepPair(n:int): int = { if growPair(n, <leaf />) is { {} => 0  else => 1 } }
let repeat(s:string, count:int) = { for i in 0..count { s } }
let fan(n:int, count:int) = { repeat(growText(n, "x"), count) }
component <Holder /> = {
  state { view?:object }
  <Button onTapped=<Update view={growPair(40, <leaf />)} /> />
}
let pings(): Ping+ = { <Ping n={0} /> for i in 1..200000 { <Ping n={i} /> } }
component <Burst emits { Fired { } } /> = { <Button /> }
component <Relay emits { Ping } /> = {
  <Burst onFired={pings()} />
}
`,
  (dir, sourcePath) => {
    const program = prepareNxIrProgram(emitIr(dir, sourcePath));
    const source = readFileSync(sourcePath, "utf8");
    /** The source text a diagnostic's span covers. The source is ASCII, so offsets are indices. */
    const spanned = (diagnostic) => source.slice(diagnostic.source.start, diagnostic.source.end);
    const evaluate = (name, args = []) => (options) => evaluateFunction(program, name, args, options);
    const ints = (count) => Array.from({ length: count }, (_, index) => index);

    // The worked counts of docs/nx-ir-format.md, which the Rust runtime is held to as well: what is
    // evaluated, what is checked against a type on the way in, what is placed, and what is written
    // for the host on the way out.
    assertCosts(6, evaluate("add", [1, 2])); // two arguments checked, three nodes, one number written
    assertCosts(29, evaluate("squares")); // the loop's 21, three `Range` fields checked, the list and four numbers written
    assertCosts(8, evaluate("pick", [true])); // its result is the one-item list `{ 1 }`: the list and the number
    assertCosts(4, evaluate("callOne"));
    assertCosts(2504, evaluate("twice", [ints(500)])); // 500 checked, 1,003 to build, the list and 1,000 items written
    assertCosts(20002, evaluate("same", [ints(10000)])); // a host's list is checked and written item by item
    // A concatenation pays for the UTF-16 code units of its result when it is built and again when
    // it is written: `é` is one unit and two bytes.
    assertCosts(68, evaluate("join", ["a".repeat(1000), "b".repeat(1000)]));
    assertCosts(68, evaluate("join", ["é".repeat(1000), "é".repeat(1000)]));
    assertCosts(6, evaluate("join", ["a".repeat(31), "b".repeat(32)]));
    assertCosts(8, evaluate("join", ["a".repeat(32), "b".repeat(32)]));
    // An equality of two strings reads them, so it pays for the code units of the shorter.
    assertCosts(22, evaluate("sameText", ["a".repeat(1000), "a".repeat(1000)]));
    assertCosts(22, evaluate("sameText", ["a".repeat(1000), "b".repeat(5000)]));
    assertCosts(7, evaluate("sameText", ["a".repeat(1000), "a".repeat(63)]));
    // A pattern costs one whether it is evaluated (the constant `dot`, which is then compared with
    // the scrutinee for one more) or read in place and matched by type (`ring`, `box`).
    assertCosts(7, evaluate("corners", ["dot"]));
    assertCosts(9, evaluate("corners", [{ $type: "Shape.ring", radius: 1 }]));
    assertCosts(10, evaluate("corners", [{ $type: "Shape.box", side: 1 }]));
    const identity = program.entry.module.identity;
    const triple = { $type: "Function", module: identity, name: "triple" };
    assertCosts(9, evaluate("through", [triple]));
    assertCosts(29, (options) => callFunction(program, { $type: "Function", module: identity, name: "squares" }, {}, options));
    console.log("ok - an evaluation costs what it evaluates, checks, places and writes, as the Rust runtime counts them");

    const nested = runtimeFailure(() => evaluateFunction(program, "nested", [], { maxOperations: 100000 })).diagnostics[0];
    assertEqual(nested.limit, { name: "maxOperations", value: 100000 });
    assertEqual(nested.declaration, `${identity}::nested`);
    assertEqual(evaluateFunction(program, "wide").length, 1000000);
    console.log("ok - nested loops end at the budget, and an absent budget is unlimited");

    // A list is paid for where it is checked as well as where it is built: `growList` checks its
    // argument on the way in and its declared result on the way out of every call, so twelve
    // doublings fit in 100,000 and thirteen do not, and the budget runs out in `growList` at a
    // check of the list or at the placing of its items. A string of 2^j code units costs 2^j / 64
    // to build, and writing the result costs as much again, so the twenty-second doubling is the
    // one that does not fit, refused at the `concat` that would build it.
    //
    // What this cannot show is that a charge comes before the value is built: a charge made after
    // would fail at the same node with the same count, and nothing here observes what was
    // allocated. In this runtime that order is two adjacent lines, in `evalItemInto` and in the
    // `concat` case of `evalBinary`; the Rust runtime's tests measure it.
    const limited = { maxOperations: 100000 };
    assertEqual(evaluateFunction(program, "growList", [12, [1]], limited).length, 2 ** 12);
    assertEqual(evaluateFunction(program, "growText", [21, "x"], limited).length, 2 ** 21);
    for (const [name, seed, doublings, node] of [
      ["growList", [1], 13, undefined],
      ["growList", [1], 60, undefined],
      ["growText", "x", 22, "s + s"],
      ["growText", "x", 60, "s + s"],
    ]) {
      const diagnostic = runtimeFailure(() => evaluateFunction(program, name, [doublings, seed], limited)).diagnostics[0];
      assertEqual(diagnostic.limit, { name: "maxOperations", value: 100000 });
      assertEqual(diagnostic.declaration, `${identity}::${name}`);
      if (node !== undefined) {
        assertEqual(spanned(diagnostic), node);
      }
    }
    // And the count is exact: one operation fewer than ten doublings cost stops them.
    assertCosts(cost(evaluate("growList", [10, [1]])), evaluate("growList", [10, [1]]));
    assertCosts(cost(evaluate("growText", [10, "x"])), evaluate("growText", [10, "x"]));
    console.log("ok - a list or a string that doubles on each call stops at the doubling the budget cannot pay for");

    // A count per call: a batch, a state default, and two calls with one options object.
    const busy = initializeComponent(program, "Busy").instance;
    const tap = { $type: "ActionHandlerInvocation", token: "h1-1", action: { $type: "Button.Tapped" } };
    const single = cost((options) => dispatchComponentActions(program, busy, [tap], options));
    assertEqual(limitOf(() => dispatchComponentActions(program, busy, [tap, tap, tap], { maxOperations: single * 2 })), { name: "maxOperations", value: single * 2 });
    dispatchComponentActions(program, busy, [tap], { maxOperations: single * 2 });
    assertEqual(limitOf(() => initializeComponent(program, "Eager", {}, { maxOperations: 1000 })), { name: "maxOperations", value: 1000 });
    const shared = { maxOperations: Math.floor(29 * 1.5) };
    evaluateFunction(program, "squares", [], shared);
    evaluateFunction(program, "squares", [], shared);
    console.log("ok - one budget covers one call: a whole batch, a state default, and each of two calls afresh");

    // `Heavy` has a field whose default is a loop of 100,000. It runs when a `Heavy` is
    // constructed, whether as the default of the `heavy` prop or from a host's `{}`, and each API
    // that constructs one is stopped by a budget of 1,000 and succeeds without one.
    for (const [name, run] of [
      ["constructComponentDescriptor", (options) => constructComponentDescriptor(program, "Costly", {}, [], options)],
      ["evaluateComponent", (options) => evaluateComponent(program, "Costly", {}, {}, options)],
      ["normalizeComponentState", (options) => normalizeComponentState(program, "Costly", { held: {} }, options)],
      ["applyComponentStatePatch", (options) => applyComponentStatePatch(program, "Costly", {}, { held: {} }, options)],
    ]) {
      try {
        assertEqual(limitOf(() => run({ maxOperations: 1000 })), { name: "maxOperations", value: 1000 });
        run({});
      } catch (error) {
        throw new Error(`${name}: ${error instanceof Error ? error.message : String(error)}`);
      }
    }
    console.log("ok - the descriptor, evaluation, state and patch APIs are each under the budget");

    // A value that holds another value twice is one object reached by two paths: forty levels of
    // that are 2^40 values to anything that walks them as a tree, built for a few hundred
    // operations. Every walk the runtime makes is paid for by the budget, so each of these ends
    // in a diagnostic at once; without the charges none of them ends at all.
    const exhausted = { name: "maxOperations", value: 100000 };
    // Checked against a type at each call, compared pair by pair, and written for the host.
    for (const name of ["held", "build", "samePair"]) {
      assertEqual(limitOf(() => evaluateFunction(program, name, [40], limited)), exhausted);
    }
    const unwritten = runtimeFailure(() => evaluateFunction(program, "buildPair", [40], limited)).diagnostics[0];
    assertEqual(unwritten.limit, exhausted);
    assertEqual(unwritten.declaration, `${identity}::buildPair`);
    assertEqual(unwritten.source, undefined);
    // Held and never walked, it costs what building it costs and nothing more.
    if (cost(evaluate("keepPair", [40])) >= 1000) {
      throw new Error("A value nobody walks should not be paid for by its size");
    }
    // Stored in state, it is refused when the state is written for the host.
    const holder = initializeComponent(program, "Holder").instance;
    assertEqual(limitOf(() => dispatchComponentActions(program, holder, [tap], limited)), exhausted);
    // One string of 2^14 code units held 100 times is 100 strings to the host: each costs one and
    // 256 for its length, where building it once cost 255.
    if (cost(evaluate("fan", [14, 100])) <= 100 * 257) {
      throw new Error("A string held many times over should be paid for each time it is written");
    }
    assertEqual(limitOf(() => evaluateFunction(program, "fan", [14, 100], { maxOperations: 1000 })), { name: "maxOperations", value: 1000 });
    console.log("ok - a value shared many times over is paid for wherever it is walked");

    // A value that holds a NaN is not equal to itself, under any budget and under none.
    for (const options of [{}, limited]) {
      assertEqual(evaluateFunction(program, "selfEq", [{ v: Number.NaN }], options), false);
      assertEqual(evaluateFunction(program, "selfEqElement", [Number.NaN], options), false);
      assertEqual(evaluateFunction(program, "selfEqElement", [1.5], options), true);
    }
    console.log("ok - an equality gives one result with and without a budget");

    // A list bound to a content parameter is built anew for each call, so each call places its
    // items: two arguments checked, eight for the loop and its `Range`, and for each of ten calls
    // the call, its callee and its argument, 100 items bound, the list and the result checked, the
    // body and the item the loop places, then the list and its ten numbers written.
    assertCosts(2 + 8 + 10 * (3 + 100 + 2 + 1 + 1) + 11, evaluate("many", [ints(100), 10]));
    const unbound = runtimeFailure(() => evaluateFunction(program, "many", [ints(20000), 16000], limited)).diagnostics[0];
    assertEqual(unbound.limit, exhausted);
    assertEqual(unbound.declaration, `${identity}::ignore`);
    assertEqual(unbound.source, undefined);
    console.log("ok - a list bound to a content parameter is paid for at every call");

    // A name a host supplied is text like any other: a megabyte key, or a megabyte `$type`, costs
    // 16,384 each time the object is written, and when it is compared, the key for each of the two
    // records that hold it and the type name once.
    const long = "k".repeat(2 ** 20);
    for (const [object, names] of [[{ [long]: 1 }, 2 * 2 ** 14], [{ $type: long, k: 1 }, 2 ** 14]]) {
      if (cost(evaluate("repeated", [object, 20])) <= 20 * 2 ** 14) {
        throw new Error("A long name should be paid for each time it is written");
      }
      assertEqual(limitOf(() => evaluateFunction(program, "repeated", [object, 20], limited)), exhausted);
      const compared = cost(evaluate("sameObject", [object]));
      if (compared < names || compared >= names + 100) {
        throw new Error(`A long name should be paid for once when it is compared, got ${compared}`);
      }
    }
    // A host can spell the forms the runtime gives a meaning to, and what it puts in them is text
    // like any other: the digits of a wide integer, and whatever an object holds that carries the
    // tag or the type name of a handler or a function. None is written or compared for the price
    // of one value.
    const spelled = [
      { $type: "nx.int", value: long },
      { $type: "nx.int", value: "1", extra: long },
      { $nxKind: "actionHandler", text: long },
      { $nxKind: "functionReference", text: long },
      { $type: "ActionHandler", action: long },
      { $type: "Function", module: long },
    ];
    for (const object of spelled) {
      if (cost(evaluate("repeated", [object, 20])) <= 20 * 2 ** 14) {
        throw new Error(`Text in ${JSON.stringify(object).slice(0, 40)} should be paid for each time it is written`);
      }
      assertEqual(limitOf(() => evaluateFunction(program, "repeated", [object, 20], limited)), exhausted);
      if (cost(evaluate("sameObject", [object])) < 2 ** 14) {
        throw new Error(`Text in ${JSON.stringify(object).slice(0, 40)} should be paid for when it is compared`);
      }
    }
    console.log("ok - text in a form the runtime gives a meaning to is paid for by its length");


    // A `null` in an opaque host value is the empty value, and every value written is one: the
    // record, the empty value under `a`, the list under `b` and its two, and the list under `c`.
    assertCosts(3 + 6, evaluate("passObject", [{ a: null, b: [null, null], c: [] }]));
    // A field of an update record is checked for the update record, which the third charge names.
    assertCosts(5, evaluate("patchOnly"));
    const patch = runtimeFailure(() => evaluateFunction(program, "patchOnly", [], { maxOperations: 2 })).diagnostics[0];
    assertEqual(patch.declaration, `${identity}::Opt.Update`);
    assertEqual(patch.source, undefined);
    console.log("ok - names, nulls and update records cost the same as in the Rust runtime");

    // Lining two records up reads the names of both, so a comparison that fails on the names is
    // paid for by the names. Around each comparison: three arguments checked, eight for the loop
    // and its `Range`, the `binary` and its two slots, the item placed, and the list and its
    // boolean written.
    assertCosts(17 + 4, evaluate("cmpMany", [{ a: 1, b: 2 }, { a: 1, c: 2 }, 1])); // the pair, `a`, and `b` and `c` that one holds
    assertCosts(17 + 3, evaluate("cmpMany", [{ a: 1 }, { a: 1, b: 2 }, 1]));
    assertCosts(17 + 3, evaluate("cmpMany", [{ a: 1, b: 2 }, { a: 1 }, 1]));
    assertCosts(17 + 1, evaluate("cmpMany", [{ $type: "A", a: 1, b: 2 }, { $type: "B", a: 1 }, 1])); // unlike types: the pair alone
    const wide = Object.fromEntries(Array.from({ length: 8000 }, (_, key) => [`k${key}`, key]));
    for (const [left, right] of [[wide, { x: 1 }], [{ x: 1 }, wide]]) {
      if (cost(evaluate("cmpMany", [left, right, 1])) <= 8000) {
        throw new Error("A comparison with a wide record should be paid for by its names");
      }
      assertEqual(limitOf(() => evaluateFunction(program, "cmpMany", [left, right, 16000], limited)), exhausted);
    }
    // A wide object that has a wide integer's type name is told from a record of another type by
    // the names alone, for the pair: its keys are not listed to see whether it is an integer, so
    // comparing it many times lists them no more often than comparing it once. Compared with a
    // record of its own type name it is lined up, and paid for by its names.
    let listed = 0;
    const spelledWide = new Proxy({ $type: "nx.int", value: "1", ...wide }, {
      ownKeys(target) {
        listed += 1;
        return Reflect.ownKeys(target);
      },
    });
    const listedBy = (comparisons, other) => {
      listed = 0;
      evaluateFunction(program, "cmpMany", [spelledWide, other, comparisons], limited);
      return listed;
    };
    for (const other of [{ $type: "other" }, { x: 1 }, 7, "text"]) {
      assertCosts(17 + 1, evaluate("cmpMany", [spelledWide, other, 1]));
      assertEqual(listedBy(2000, other), listedBy(1, other));
    }
    if (cost(evaluate("cmpMany", [spelledWide, { $type: "nx.int", value: "1" }, 1])) <= 8000) {
      throw new Error("A wide record with a wide integer's type name should be paid for by its names when it is lined up");
    }
    console.log("ok - a record with a wide integer's type name is not listed to be told from another type");

    // A JavaScript number cannot hold an integer outside the safe range, so this runtime refuses
    // one where it reads it from the image, naming the function and the literal: whatever would
    // have used it never runs. The Rust runtime, which has 64-bit integers, computes with it.
    for (const [name, args] of [["wideSame", []], ["patWide", []], ["patWideSame", []], ["eqWide", [1]], ["patHost", [1]]]) {
      const refused = runtimeFailure(() => evaluateFunction(program, name, args)).diagnostics[0];
      assertEqual(refused.code, "nx-ir-number");
      if (!refused.declaration?.startsWith(`${identity}::`) || refused.source === undefined) {
        throw new Error(`A refused integer should name its declaration and its span, got ${JSON.stringify(refused)}`);
      }
    }
    // Refusing it costs what reaching it costs: the node is charged, and nothing after it.
    assertEqual(limitOf(() => evaluateFunction(program, "wideSame", [], { maxOperations: 0 })), { name: "maxOperations", value: 0 });
    assertEqual(runtimeFailure(() => evaluateFunction(program, "wideSame", [], { maxOperations: 100 })).diagnostics[0].code, "nx-ir-number");
    // The record canonical JSON spells such an integer with is, at `object`, a record like any
    // other, as it is in the Rust runtime: returned unchanged, equal to itself, and paid for as
    // a record and its string, where a number is one value. At a parameter typed `int` it is
    // refused, as it always was.
    const spelledInteger = { $type: "nx.int", value: "1152921504606846976" };
    assertEqual(evaluateFunction(program, "passObject", [spelledInteger]), spelledInteger);
    assertCosts(5, evaluate("passObject", [spelledInteger]));
    assertCosts(cost(evaluate("passObject", [{ a: "b" }])), evaluate("passObject", [spelledInteger]));
    assertEqual(evaluateFunction(program, "sameObject", [spelledInteger], { maxOperations: 100000 }), true);
    assertEqual(runtimeFailure(() => evaluateFunction(program, "twice", [[spelledInteger]])).diagnostics[0].code, "nx-ir-boundary-type");
    console.log("ok - an integer outside the safe range is refused where it is read, and its JSON form at object is a record");
    // Every item bound to a content parameter costs one, empty or not, and a list among them one
    // for each item it contributes or one when it contributes none; nine is the call around it.
    for (const [list, items] of [[[null], 1], [[1, null], 2], [[null, [null], 1], 3], [[[null, null], 1], 3], [[[], [], [1, 2, 3]], 5]]) {
      assertCosts(9 + items, evaluate("bindObject", [list]));
    }
    console.log("ok - records are lined up by name at a cost, and every item bound costs one");

    // Two lists are compared in order up to the first pair that differs: the pair of lists and
    // each pair of items compared. Two records inside them are compared to the end.
    assertCosts(17 + 4, evaluate("cmpMany", [[1, 2, 3], [1, 2, 3], 1]));
    assertCosts(17 + 4, evaluate("cmpMany", [[1, 2, 3], [1, 2, 9], 1]));
    assertCosts(17 + 2, evaluate("cmpMany", [[0, 1, 2], [9, 1, 2], 1]));
    assertCosts(17 + 1, evaluate("cmpMany", [[1, 2], [1, 2, 3], 1])); // unlike lengths compare no items
    assertCosts(17 + 4, evaluate("cmpMany", [[[1, 2], [3, 4]], [[1, 9], [3, 4]], 1]));
    assertCosts(17 + 4, evaluate("cmpMany", [[{ a: 1, b: 2 }, { a: 5 }], [{ a: 9, b: 9 }, { a: 5 }], 1]));
    const longList = (first) => [first, ...Array.from({ length: 99999 }, (_, index) => index + 1)];
    if (cost(evaluate("cmpMany", [longList(0), longList(-1), 200])) >= 2000) {
      throw new Error("Two long lists that differ in their first item should cost two operations to compare");
    }
    console.log("ok - two lists are compared in order up to the first pair that differs");

    // Each button holds a handler made by one node, which captured the two parameters of
    // `button`. Twenty-three surrounds the comparison in `sameButton`; the comparison is the pair
    // of buttons, the pair of handlers, and each pair of captured values up to the first that
    // differs: a walk to the end would cost one more where both differ.
    assertCosts(23 + 4, evaluate("sameButton", [1, 2, 1, 2]));
    assertCosts(23 + 3, evaluate("sameButton", [9, 2, 1, 2]));
    assertCosts(23 + 4, evaluate("sameButton", [1, 9, 1, 2]));
    assertCosts(23 + 3, evaluate("sameButton", [9, 9, 1, 2]));
    for (const [args, equal] of [[[1, 2, 1, 2], true], [[9, 9, 1, 2], false], [[1, 9, 1, 2], false]]) {
      assertEqual(evaluateFunction(program, "sameButton", args), equal);
      assertEqual(evaluateFunction(program, "sameButton", args, { maxOperations: 100000 }), equal);
    }
    console.log("ok - two handlers are compared up to the first captured value that differs");

    // An empty value takes a place in a list like any other, so a value made of them costs its
    // length to write and to bind: 20,000 in a list, as `null` or as `[]`, or under 8,000 keys.
    const made = (item) => ({ a: Array.from({ length: 20000 }, () => item) });
    const keys = Object.fromEntries(Array.from({ length: 8000 }, (_, key) => [`k${key}`, null]));
    for (const [object, values] of [[made(null), 20000], [made([]), 20000], [keys, 8000]]) {
      if (cost(evaluate("repeated", [object, 2])) <= 2 * values) {
        throw new Error("A value made of empty values should cost its length each time it is written");
      }
      assertEqual(limitOf(() => evaluateFunction(program, "repeated", [object, 500], limited)), exhausted);
    }
    const emptyBound = runtimeFailure(() => evaluateFunction(program, "many", [Array.from({ length: 20000 }, () => null), 10000], limited)).diagnostics[0];
    assertEqual(emptyBound.limit, exhausted);
    assertEqual(emptyBound.declaration, `${identity}::ignore`);
    console.log("ok - a value made of empty values costs its length to write and to bind");

    for (const maxOperations of [Number.NaN, -1, 1.5, 2 ** 60]) {
      const diagnostic = runtimeFailure(() => evaluateFunction(program, "ratio", [1], { maxOperations })).diagnostics[0];
      // A division by zero would mean the function ran; the option is refused before anything does.
      assertEqual(diagnostic.code, "nx-ir-options");
      if (!diagnostic.message.includes("maxOperations") || diagnostic.limit !== undefined) {
        throw new Error(`Unexpected diagnostic for maxOperations ${maxOperations}: ${JSON.stringify(diagnostic)}`);
      }
    }
    console.log("ok - a budget that is not a count is refused before anything is evaluated");

    assertEqual(limitOf(() => evaluateFunction(program, "spin", [0])), { name: "maxCallDepth", value: 100 });
    assertEqual(limitOf(() => evaluateFunction(program, "tooWide")), { name: "maxRangeLength", value: 1000000 });
    const deep = limitOf(() => evaluateFunction(program, "spin", [0], { maxCallDepth: 1000000 }));
    if (!(deep.name === "maxExpressionNesting" && deep.value === 1000) && deep.name !== "engine") {
      throw new Error(`Unexpected limit for runaway recursion: ${JSON.stringify(deep)}`);
    }
    assertEqual(runtimeFailure(() => evaluateFunction(program, "ratio", [1])).diagnostics[0].limit, undefined);
    console.log("ok - every resource-limit diagnostic names its limit, and no other diagnostic names one");

    // Past the engine's own limits: a string longer than it holds is a diagnostic, not a RangeError,
    // and long lists are spliced without the engine's limit on the arguments of a spread.
    assertEqual(limitOf(() => evaluateFunction(program, "growText", [40, "x"])), { name: "engine" });
    assertEqual(evaluateFunction(program, "longContent").content.length, 200000);
    const relay = initializeComponent(program, "Relay");
    const { $type: _burst, ...burstProps } = relay.rendered;
    const burst = initializeComponent(program, "Burst", burstProps, { parent: relay.instance });
    assertEqual(dispatchComponentActions(program, burst.instance, [{ $type: "Burst.Fired" }]).effects.length, 200000);
    console.log("ok - engine limits are diagnostics, and long lists splice and dispatch");

    // Only a RangeError is converted: any other exception raised during evaluation is a fault, not
    // a limit, and reaches the host as itself.
    const throwing = {
      get n() {
        throw new TypeError("deliberate");
      },
    };
    let propagated = false;
    try {
      callFunction(program, triple, throwing);
    } catch (error) {
      propagated = error instanceof TypeError && error.message === "deliberate";
    }
    if (!propagated) {
      throw new Error("Expected a TypeError raised during evaluation to propagate unchanged");
    }
    console.log("ok - an exception other than a RangeError propagates unchanged");
  },
);

// ------------------------------------------------------------------------------------------------
// The input limit, the usage report and the exported measure
// ------------------------------------------------------------------------------------------------

/** Asserts that a failure is the input limit's, set to `size`, and names no declaration and no span. */
function assertInputRefused(run, size) {
  const diagnostic = runtimeFailure(run).diagnostics[0];
  assertEqual(diagnostic.code, "nx-ir-resource-limit");
  assertEqual(diagnostic.limit, { name: "maxInputSize", value: size });
  assertEqual([diagnostic.declaration, diagnostic.source], [undefined, undefined]);
}

/**
 * Asserts that the input of a call has exactly the size `size`: the call proceeds under that limit
 * and reports that size, and is refused under one less.
 */
function assertInputSize(size, run) {
  const usage = {};
  run({ maxInputSize: size, usage });
  assertEqual(usage, { inputSize: size });
  assertInputRefused(() => run({ maxInputSize: size - 1, usage }), size - 1);
  assertEqual(usage, {});
}

/** The input size a call reports, whether it then returns or throws. */
function inputSizeOf(run) {
  const usage = {};
  try {
    run({ maxInputSize: ample, usage });
  } catch (error) {
    if (!(error instanceof NxIrRuntimeError)) {
      throw error;
    }
  }
  return usage.inputSize;
}

withSource(
  `
external component <Button label?:string emits { Tapped { } } />
external component <Text value:string />
let pass(o:object): object = { o }
let ints(xs:int+) = { xs }
let add(a:int, b:int): int = { a + b }
let join(a:string, b:string) = { a + b }
let squares() = { for i in 0..4 { i * i } }
let ratio(n:int): int = { n / 0 }
component <Card title:string = "none" count:int = 0 content body?:object+ /> = {
  <Text value={title} />
}
component <Counter /> = {
  state { count:int = 0 note?:string data?:object }
  <Button onTapped=<Update count={count + 1} /> />
}
component <Keeper /> = {
  state { data?:object n:int = {0} }
  <panel>
    <Button onTapped=<Update data=<bag>{for i in 0..50000 { <leaf /> }}</bag> /> />
    <Button onTapped=<Update data={data} n={n + 1} /> />
  </panel>
}
`,
  (dir, sourcePath) => {
    const program = prepareNxIrProgram(emitIr(dir, sourcePath));
    const identity = program.entry.module.identity;
    const ints = (count) => Array.from({ length: count }, (_, index) => index);
    const pass = (value) => (options) => evaluateFunction(program, "pass", [value], options);
    const tap = (token) => ({ $type: "ActionHandlerInvocation", token, action: { $type: "Button.Tapped" } });
    const megabyte = 2 ** 20;

    // Input over the limit is refused before anything runs: a budget of nothing beside the limit
    // would be exhausted by the first node, and it is the input that is reported, with no
    // operation used. With no limit a list of a million integers is input like any other.
    {
      const usage = {};
      assertInputRefused(() => evaluateFunction(program, "ints", [ints(20000)], { maxInputSize: 1000, maxOperations: 0, usage }), 1000);
      assertEqual(usage, { operations: 0 });
      assertEqual(evaluateFunction(program, "ints", [ints(1000000)]).length, 1000000);
      // The limit and the budget have one code and are told apart by the limit's name.
      const forBudget = runtimeFailure(() => evaluateFunction(program, "ints", [ints(20000)], { maxOperations: 1000 })).diagnostics[0];
      assertEqual([forBudget.code, forBudget.limit.name], ["nx-ir-resource-limit", "maxOperations"]);
    }
    console.log("ok - input over the limit is refused before anything runs, and an absent limit is unlimited");

    for (const maxInputSize of [Number.NaN, -1, 1.5, 2 ** 60]) {
      // A division by zero would mean the function ran; the option is refused before anything does.
      const diagnostic = runtimeFailure(() => evaluateFunction(program, "ratio", [1], { maxInputSize })).diagnostics[0];
      assertEqual(diagnostic.code, "nx-ir-options");
      if (!diagnostic.message.includes("maxInputSize") || diagnostic.limit !== undefined) {
        throw new Error(`Unexpected diagnostic for maxInputSize ${maxInputSize}: ${JSON.stringify(diagnostic)}`);
      }
    }
    console.log("ok - an input limit that is not a size is refused before anything is measured");

    // The worked sizes of docs/nx-ir-format.md, the same numbers the Rust runtime's tests assert,
    // and one call of each function over its limit.
    //
    // `evaluateFunction`: each positional argument. A list is its items and itself.
    assertInputSize(1001, (options) => evaluateFunction(program, "ints", [ints(1000)], options));
    assertInputSize(2, (options) => evaluateFunction(program, "add", [1, 2], options));
    // Text costs its length in UTF-16 code units, and a name its own.
    assertInputSize(101, pass("a".repeat(6400)));
    assertInputSize(101, pass("é".repeat(6400)));
    assertInputSize(16386, pass({ ["k".repeat(megabyte)]: 1 }));
    // The JSON form of a wide integer is the record it is, whatever its string holds.
    const wide = { $type: "nx.int", value: "9007199254740993" };
    assertInputSize(2, pass(wide));
    assertInputSize(5, pass([wide, wide]));
    assertInputSize(16386, pass({ $type: "nx.int", value: "9".repeat(megabyte) }));
    // An object read from JSON that carries the marking the runtime gives a handler is a plain
    // object: the object, its tag, and the string with its 16,384.
    const marked = JSON.parse(JSON.stringify({ $nxKind: "actionHandler", text: "t".repeat(megabyte) }));
    assertInputSize(16387, pass(marked));
    assertInputRefused(() => pass(marked)({ maxInputSize: 1000 }), 1000);
    // `callFunction`: the function record, and the arguments by name as one record.
    const add = { $type: "Function", module: identity, name: "add" };
    assertInputSize(6, (options) => callFunction(program, add, { a: 1, b: 2 }, options));
    // `constructComponentDescriptor`: the props as one record, and each content item.
    const props = { title: "Home", count: 3 };
    assertInputSize(6, (options) => constructComponentDescriptor(program, "Card", props, [1, 2, 3], options));
    // `initializeComponent`: the props, and a state when one is passed. Props left out are an
    // empty record, as `{}` is; a state not passed is not input.
    assertInputSize(3, (options) => initializeComponent(program, "Card", props, options));
    assertInputSize(1, (options) => initializeComponent(program, "Card", undefined, options));
    assertInputSize(1, (options) => initializeComponent(program, "Card", {}, options));
    const state = { count: 3, note: "x" };
    assertInputSize(4, (options) => initializeComponent(program, "Counter", {}, { ...options, state }));
    // `evaluateComponent`: the props and the state.
    assertInputSize(4, (options) => evaluateComponent(program, "Counter", {}, state, options));
    // `dispatchComponentActions`: each entry of the batch. An invocation is its record, its token
    // and its action.
    const counter = initializeComponent(program, "Counter").instance;
    assertInputSize(6, (options) => dispatchComponentActions(program, counter, [tap("h1-1"), tap("h1-1")], options));
    // `normalizeComponentState`: the state. `applyComponentStatePatch`: the state and the patch.
    assertInputSize(3, (options) => normalizeComponentState(program, "Counter", state, options));
    assertInputSize(5, (options) => applyComponentStatePatch(program, "Counter", state, { count: 4 }, options));
    // Arguments by name left out are an empty record beside the function record.
    assertEqual(inputSizeOf((options) => callFunction(program, add, undefined, options)), 4);
    // A type name among props is a type name: the record and the title.
    assertEqual(measureInputSize({ $type: "Card", title: "Home" }), 2);
    assertEqual(inputSizeOf((options) => initializeComponent(program, "Card", { $type: "Card", title: "Home" }, options)), 2);
    assertInputRefused(() => initializeComponent(program, "Card", { $type: "Card", title: "Home" }, { maxInputSize: 1 }), 1);
    // An element of the positional arguments that is undefined, or a hole, is the empty value.
    assertEqual(inputSizeOf((options) => evaluateFunction(program, "add", [undefined, 2], options)), 2);
    // eslint-disable-next-line no-sparse-arrays
    assertEqual(inputSizeOf((options) => evaluateFunction(program, "add", [, 2], options)), 2);
    assertEqual(inputSizeOf((options) => evaluateFunction(program, "add", [1], options)), 1);
    console.log("ok - every function measures what the host passes it, at the sizes the Rust runtime measures");

    // Oversized input is reported before any other fault of the call.
    {
      const large = ints(20000);
      const limited = { maxInputSize: 1000 };
      assertInputRefused(() => evaluateFunction(program, "noSuchFunction", [large], limited), 1000);
      assertEqual(runtimeFailure(() => evaluateFunction(program, "noSuchFunction", [ints(10)], limited)).diagnostics[0].code, "nx-ir-missing-entrypoint");
      assertInputRefused(() => callFunction(program, 1, { a: large }, limited), 1000);
      assertInputRefused(() => normalizeComponentState(program, "NoSuchComponent", { a: large }, limited), 1000);
      // Props of the wrong type beside a state larger than the limit.
      assertInputRefused(() => evaluateComponent(program, "Card", { title: 5 }, { data: large }, limited), 1000);
      assertEqual(runtimeFailure(() => evaluateComponent(program, "Card", { title: 5 }, {}, limited)).diagnostics[0].code, "nx-ir-boundary-type");
      assertInputRefused(() => initializeComponent(program, "Card", { title: 5 }, { ...limited, state: { data: large } }), 1000);
      // Props, content, and a patch.
      assertInputRefused(() => initializeComponent(program, "Card", { title: "t", body: large }, limited), 1000);
      assertInputRefused(() => constructComponentDescriptor(program, "Card", {}, [large], limited), 1000);
      assertInputRefused(() => applyComponentStatePatch(program, "Counter", {}, { data: large }, limited), 1000);
      // An unlinked module, which no evaluation runs, is still told about its input first.
      assertInputRefused(() => evaluateFunction(program.entry.module, "ints", [large], limited), 1000);
    }
    console.log("ok - oversized input is reported before any other fault of the call");

    // A batch is measured whole before any of it runs: a first entry that would run, and exhaust
    // a budget of nothing at its first node, and a second that is too large.
    {
      const usage = {};
      assertInputRefused(
        () => dispatchComponentActions(program, counter, [tap("h1-1"), { data: ints(20000) }], { maxInputSize: 100, maxOperations: 0, usage }),
        100,
      );
      assertEqual(usage, { operations: 0 });
      // The instance given is untouched, and dispatches a batch the limit covers.
      assertEqual(dispatchComponentActions(program, counter, [tap("h1-1")], { maxInputSize: 100 }).state.count, 1);
    }
    console.log("ok - a batch is measured whole before any of it runs, and a refused dispatch leaves the instance usable");

    // An instance is not input: the first button of `Keeper` stores 50,000 values in its state,
    // and a batch of one small entry against that instance is three, whatever the instance holds.
    // Nor is the parent instance a child is initialized against.
    {
      const keeper = initializeComponent(program, "Keeper").instance;
      const full = dispatchComponentActions(program, keeper, [tap("h1-1")]);
      assertEqual(full.state.data.content.length, 50000);
      assertInputSize(3, (options) => dispatchComponentActions(program, full.instance, [tap("h2-2")], options));
      dispatchComponentActions(program, full.instance, [tap("h2-2")], { maxInputSize: 100 });
      assertInputSize(1, (options) => initializeComponent(program, "Card", {}, { ...options, parent: full.instance }));
    }
    console.log("ok - an instance is not measured");

    // The walk keeps its own stack and takes every reference as a new value, so a value that
    // holds itself and one that nests deeply are refused by the limit and not by the engine.
    {
      const itself = { name: "loop" };
      itself.self = itself;
      assertInputRefused(() => pass(itself)({ maxInputSize: 1000 }), 1000);
      let deep = 1;
      for (let level = 0; level < 100000; level += 1) {
        deep = [deep];
      }
      assertInputRefused(() => pass(deep)({ maxInputSize: 1000 }), 1000);
      assertEqual(measureInputSize(deep), 100001);

      // It stops at the limit: of a million items under a limit of ten, eleven are read.
      let read = 0;
      const counted = new Proxy(ints(1000000), {
        get(target, key, receiver) {
          if (typeof key === "string" && /^\d+$/.test(key)) {
            read += 1;
          }
          return Reflect.get(target, key, receiver);
        },
      });
      assertInputRefused(() => pass(counted)({ maxInputSize: 10 }), 10);
      if (read > 12) {
        throw new Error(`Refusing a million items under a limit of ten read ${read} of them`);
      }

      // A value that is no canonical value is one value and is not entered: a `Date`, a `Map`, a
      // function, a big integer, an instance of a class, and a handler the runtime made, which
      // holds the linked program.
      class Held {
        constructor() {
          this.text = "t".repeat(megabyte);
        }
      }
      const handler = counter.handlers.get("h1-1");
      assertEqual(handler.$nxKind, "actionHandler");
      for (const value of [new Date(), new Map([[1, "t".repeat(megabyte)]]), () => 1, 10n, new Held(), handler, undefined, Symbol("s")]) {
        assertEqual(measureInputSize(value), 1);
        assertEqual(measureInputSize([value, value]), 3);
      }
      // One object of two million names is refused for its size after its names are listed once,
      // which is the one cost the limit does not bound: 440 to 670 ms in Node 24 on the laptop
      // this was written on, against under a millisecond for a list as long.
      const wideObject = {};
      for (let key = 0; key < 2000000; key += 1) {
        wideObject[`k${key}`] = key;
      }
      let listed = 0;
      const listing = new Proxy(wideObject, {
        ownKeys(target) {
          listed += 1;
          return Reflect.ownKeys(target);
        },
      });
      assertInputRefused(() => pass(listing)({ maxInputSize: 1000 }), 1000);
      assertEqual(listed, 1);

      // That listing is once in a call, however objects are nested: an object's members are
      // counted against the limit when its names are listed, so a wide object that holds itself,
      // or a chain of wide objects, is refused at the listing that cannot fit and is not listed
      // once for every value the limit allows. `names` counts every name the walk was given.
      let names = 0;
      const counting = (target) =>
        new Proxy(target, {
          ownKeys(inner) {
            const keys = Reflect.ownKeys(inner);
            listed += 1;
            names += keys.length;
            return keys;
          },
        });
      const wideAndItself = (width, typed) => {
        const target = typed ? { $type: "T", self: undefined } : { self: undefined };
        for (let key = 0; key < width; key += 1) {
          target[`k${key}`] = key;
        }
        const object = counting(target);
        target.self = object;
        return object;
      };
      for (const [width, typed, listings] of [[100000, false, 1], [400, false, 3], [1, false, 1000], [1, true, 500]]) {
        listed = 0;
        names = 0;
        const object = wideAndItself(width, typed);
        assertInputRefused(() => pass(object)({ maxInputSize: 1000 }), 1000);
        if (!(measureInputSize(object, 1000) > 1000)) {
          throw new Error("A value that holds itself is larger than any limit");
        }
        // Two walks, each within the bound: twice the limit, and the one listing that is refused.
        // An object of 100,000 names is refused at its first listing; one of 400 is entered twice
        // and refused at the third; one of a single name beside itself is entered once for each
        // unit of the limit, two names at a time; and the same with a type name, which nothing
        // is reserved for, half as often and three names at a time.
        assertEqual(listed, 2 * listings);
        if (names > 2 * (2 * 1000 + width + 2)) {
          throw new Error(`Refusing an object of ${width} names that holds itself listed ${names} names`);
        }
      }
      // A chain of 300 objects, each 20,000 names wide and holding the next under its first name.
      listed = 0;
      names = 0;
      let chain = 1;
      for (let link = 0; link < 300; link += 1) {
        const target = { next: chain };
        for (let key = 0; key < 20000; key += 1) {
          target[`k${key}`] = key;
        }
        chain = counting(target);
      }
      assertInputRefused(() => pass(chain)({ maxInputSize: 100 }), 100);
      assertEqual([listed, names], [1, 20001]);
      // And objects the limit has room for are measured to the end, at the size they have.
      listed = 0;
      const nested = counting({ a: counting({ b: counting({ c: 1, d: [counting({ e: "x" })] }) }) });
      assertEqual(measureInputSize(nested, 7), 7);
      assertEqual(measureInputSize(nested), 7);
      assertEqual(measureInputSize(nested, 6) > 6, true);
      assertInputSize(7, pass(nested));
    }
    console.log("ok - a value that holds itself, nests deeply or is very long is refused by the limit, and what is no canonical value is one");

    // With no limit nothing is measured: a value is read exactly as often as the evaluation reads
    // it, and once more when the input is measured. And a limit the input fits costs no operations.
    {
      let reads = 0;
      const counting = {
        get n() {
          reads += 1;
          return 1;
        },
      };
      const readsUnder = (options) => {
        reads = 0;
        evaluateFunction(program, "pass", [counting], options);
        return reads;
      };
      const unmeasured = readsUnder({});
      assertEqual(readsUnder({ usage: {} }), unmeasured);
      assertEqual(readsUnder({ maxInputSize: 100 }), unmeasured + 1);
      const args = [ints(1000)];
      const unlimited = cost((options) => evaluateFunction(program, "ints", args, options));
      assertEqual(cost((options) => evaluateFunction(program, "ints", args, { ...options, maxInputSize: 1001 })), unlimited);
      // A budget of nothing does not keep the input from being measured in full.
      const usage = {};
      assertEqual(limitOf(() => evaluateFunction(program, "ints", args, { maxInputSize: 1001, maxOperations: 0, usage })), { name: "maxOperations", value: 0 });
      assertEqual(usage, { operations: 0, inputSize: 1001 });
    }
    console.log("ok - an absent limit measures nothing, and a limit costs no operations");

    // The usage report.
    {
      // A call that succeeds reports its count, the least budget it succeeds under.
      const usage = {};
      evaluateFunction(program, "squares", [], { maxOperations: 100000, usage });
      assertEqual(usage, { operations: 29 });
      // One that fails for its budget reports what it used, which is no more than the budget.
      assertEqual(limitOf(() => evaluateFunction(program, "squares", [], { maxOperations: 10, usage })), { name: "maxOperations", value: 10 });
      if (!(usage.operations <= 10)) {
        throw new Error(`A failed call should report no more than its budget, got ${usage.operations}`);
      }
      // A charge the budget refused is not among them: `join` of 2,000 code units checks its two
      // arguments and evaluates its node and two slots, which is five, and its sixth charge is 31
      // for the length the concatenation would build.
      assertEqual(limitOf(() => evaluateFunction(program, "join", ["a".repeat(1000), "b".repeat(1000)], { maxOperations: 10, usage })), { name: "maxOperations", value: 10 });
      assertEqual(usage, { operations: 5 });
      // A failure that is no limit's reports what ran before it.
      assertEqual(runtimeFailure(() => evaluateFunction(program, "ratio", [1], { maxOperations: 10, usage })).diagnostics[0].code, "nx-ir-division-by-zero");
      assertEqual(usage, { operations: 4 });

      // The input size is reported when it is measured, and a report does not carry over.
      const args = [ints(1000)];
      evaluateFunction(program, "ints", args, { maxInputSize: 5000, maxOperations: 100000, usage });
      assertEqual(usage, { operations: 2002, inputSize: 1001 });
      evaluateFunction(program, "ints", args, { maxInputSize: 5000, usage });
      assertEqual(usage, { inputSize: 1001 });
      evaluateFunction(program, "ints", args, { usage });
      assertEqual(usage, {});
      if ("operations" in usage || "inputSize" in usage) {
        throw new Error("A report should not carry over to a call with no budget and no limit");
      }
      // Nor to a call that is refused for another of its options before anything runs.
      for (const refused of [{ maxOperations: Number.NaN }, { maxInputSize: -1 }]) {
        evaluateFunction(program, "ints", args, { maxInputSize: 5000, maxOperations: 100000, usage });
        assertEqual(usage, { operations: 2002, inputSize: 1001 });
        assertEqual(runtimeFailure(() => evaluateFunction(program, "ints", args, { ...refused, usage })).diagnostics[0].code, "nx-ir-options");
        assertEqual(usage, {});
      }

      // A dispatch reports one number for the batch: the render and the state written are paid
      // once, and each handler once for each entry.
      const dispatch = (entries) => (options) => dispatchComponentActions(program, counter, Array.from({ length: entries }, () => tap("h1-1")), options);
      const [none, one, three] = [cost(dispatch(0)), cost(dispatch(1)), cost(dispatch(3))];
      assertEqual(three - none, 3 * (one - none));
      dispatch(3)({ maxOperations: 100000, maxInputSize: 100, usage });
      assertEqual(usage, { operations: three, inputSize: 9 });

      // A sink the runtime cannot write to is refused before anything runs: a division by zero
      // would mean the function ran.
      const throwing = new Proxy({}, {
        set() {
          throw new Error("deliberate");
        },
      });
      for (const sink of [Object.freeze({}), Object.preventExtensions({}), Object.seal({}), 5, "usage", null, throwing]) {
        const diagnostic = runtimeFailure(() => evaluateFunction(program, "ratio", [1], { usage: sink })).diagnostics[0];
        assertEqual(diagnostic.code, "nx-ir-options");
        if (!diagnostic.message.includes("usage")) {
          throw new Error(`Unexpected diagnostic for an unwritable usage: ${JSON.stringify(diagnostic)}`);
        }
      }
      // The object is not measured as input, whatever it holds.
      const heavy = { extra: ints(100000) };
      evaluateFunction(program, "add", [1, 2], { maxInputSize: 2, usage: heavy });
      assertEqual(heavy.inputSize, 2);

      // Writing the report never replaces the outcome: a sink whose writes begin to throw once the
      // call has started leaves a failing call its diagnostic and a returning call its value.
      const failingLater = () => {
        let writes = 0;
        return new Proxy({}, {
          set(target, key, value) {
            writes += 1;
            if (writes > 2) {
              throw new Error("deliberate");
            }
            target[key] = value;
            return true;
          },
        });
      };
      assertEqual(
        runtimeFailure(() => evaluateFunction(program, "ratio", [1], { maxOperations: 100, usage: failingLater() })).diagnostics[0].code,
        "nx-ir-division-by-zero",
      );
      assertEqual(evaluateFunction(program, "add", [1, 2], { maxOperations: 100, usage: failingLater() }), 3);
    }
    console.log("ok - a call reports what it used, to a sink that is validated first and can never replace the outcome");

    // The exported measure: a value measured alone has the size it has in a call.
    {
      const values = [
        null,
        ints(1000),
        "é".repeat(6400),
        { ["k".repeat(megabyte)]: 1 },
        wide,
        { a: [null, [], {}], b: { $type: "T", c: "d" } },
      ];
      for (const value of values) {
        const alone = measureInputSize(value);
        assertEqual(alone, inputSizeOf(pass(value)));
        if (alone > 1 && !(measureInputSize(value, alone - 1) > alone - 1)) {
          throw new Error("A limit under the size should be passed");
        }
        assertEqual(measureInputSize(value, alone), alone);
      }
      // A list of arguments measured as one value is one more than its entries add to a call.
      assertEqual(measureInputSize(values), inputSizeOf((options) => evaluateFunction(program, "pass", values, options)) + 1);
      // An object is measured as the one record props, a state and arguments by name are.
      assertEqual(measureInputSize(props), 3);
      assertEqual(measureInputSize({}), 1);
      // Measuring stops at the limit: of a million integers under a limit of 1,000, 1,001 are read.
      let read = 0;
      const counted = new Proxy(ints(1000000), {
        get(target, key, receiver) {
          if (typeof key === "string" && /^\d+$/.test(key)) {
            read += 1;
          }
          return Reflect.get(target, key, receiver);
        },
      });
      assertEqual(measureInputSize(counted, 1000), 1001);
      assertEqual(read, 1000);
      for (const limit of [Number.NaN, -1, 1.5]) {
        assertEqual(runtimeFailure(() => measureInputSize(1, limit)).diagnostics[0].code, "nx-ir-options");
      }
    }
    console.log("ok - a value measured alone has the size it has in a call, and measuring stops at the limit");
  },
);
