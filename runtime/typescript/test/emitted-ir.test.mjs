import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import {
  callFunction,
  dispatchComponentActions,
  evaluateComponent,
  evaluateFunction,
  initializeComponent,
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
external component <Stack content Children:Item[] />
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
external component <Shape shadows:Shadow[]? sizes:float64[]? />
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
type User = { name:string = "anon" email:string? }
let root(): User.Update = { <User.Update email={null} /> }
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
type Ints = int[]
type AlsoInts = Ints
abstract external component <Item />
external component <Leaf extends Item />
type Items = Item[]
type MaybeItems = Items?
external component <Box xs:AlsoInts? content items:MaybeItems />
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
abstract external component <Control Padding:Thickness = {<Thickness />} content Children:Control[]? />
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
type User = { name:string email:string? age:int? }
let key(): User.Property = { User.Property.email }
let root(): User = { apply(<User name="Ada" email="x@y" />, <User.Update email={null} />) }
let keys(): User.Property[] = { changed(<User.Update age={null} name="Ada" />) }
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
external component <List TItem:type ItemsSource:TItem[]? ItemTemplate:(<function Item:TItem Index:int />: string)? />
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
external component <Box Label:string? Same:boolean? Other:boolean? />
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
external component <Stack content Children:Item[] />
let xs = { 1 2 3 }
let <Items content Items:Item[] />: Item[] = {Items}
let <Shift Items:Item[] By:int />: Item[] = { for i in Items { <Item n={i.n + By} /> } }
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
let xs:string[] = {"a" "b"}
let ys:string[] = {"c"}
let root(): string[] = { xs ys }
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
type Row = { cells:int[] }
let rows:Row[] = { <Row cells={1 2}/> <Row cells={3 4}/> }
let flat(): int[] = { for r in rows { r.cells } }
let evens(): int[] = { for n in 1..=4 { if (n % 2 == 0) { n } } }
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
type Box = { content items:A[] }
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
type Row = { cells:int[] }
type Box = { content items:Badge[] }
type Result = { spliced:Badge[] values:string[] flat:int[] boxed:Box }
let some:Badge[] = { <Badge/> <Badge/> }
let xs:string[] = {"a" "b"}
let rows:Row[] = { <Row cells={1 2}/> <Row cells={3 4}/> }
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
type Box = { content items:Badge[] }
type Loose = { content items:Badge?[] }
type Nested = { openInOpen:Box takenInner:Box closedOverOpen:Loose }
let yes = true
let no = false
let root(): Nested = <Nested
  openInOpen={<Box><Badge/>{if yes { if no { <Badge n=2 /> } }}</Box>}
  takenInner={<Box><Badge/>{if yes { if yes { <Badge n=5 /> } }}</Box>}
  closedOverOpen={<Loose><Badge/>{if yes { if no { <Badge n=3 /> } } else { <Badge n=4 /> }}</Loose>}
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
    // An `else` on the outer conditional does not make the untaken inner one contribute a null:
    // the branch that was taken is itself the thing that contributes nothing.
    assertEqual(viaIr.closedOverOpen.items.length, 1);
    console.log("ok - a conditional nested in a conditional contributes nothing in all three engines");
  },
);

// A body that was written and produced nothing is not the same as no body at all. The first binds
// the empty list; only the second leaves the content property to its declared default. Every case
// here has a body, so the default must not appear -- and each sits alone, with no sibling to keep
// the list non-empty, which is what makes the distinction observable.
withSource(
  `
type Badge = { n:int = 1 }
type Box = { content items:Badge[] = { <Badge n=9 /> } }
type Defaults = { untaken:Box empty:Box noIterations:Box absent:Box }
let c = false
let none:Badge[] = { }
let root(): Defaults = <Defaults
  untaken={<Box>{if c { <Badge n=2 /> }}</Box>}
  empty={<Box>{}</Box>}
  noIterations={<Box>{for b in none { b }}</Box>}
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
    // A body that produced nothing binds the empty list, however it came to produce nothing.
    for (const field of ["untaken", "empty", "noIterations"]) {
      assertEqual(viaIr[field].items, []);
      assertEqual(generated[field].items, []);
    }
    // No body at all is the one case the declared default is for.
    assertEqual(viaIr.absent.items.length, 1);
    assertEqual(viaIr.absent.items[0].n, 9);
    // `absent` is compared field by field rather than through `assertEqual(viaIr, generated)`,
    // because generated code binds a one-item default as the item instead of a one-item list. That
    // is a gap in this target's coercion, not in the rule under test, and it is tracked separately.
    console.log("ok - a body that produced nothing binds the empty list rather than the declared default");
  },
);

// A conditional with no `else` carries an implicit `else { }`, so it is a sequence in its own right
// and needs no rule of its own in any engine: an empty arm written out and a missing one agree, and
// a conditional alone in its braces behaves as it does beside other items.
withSource(
  `
type Badge = { n:int = 1 }
type Box = { content items:Badge[] }
type Implicit = { emptyArm:Box missingArm:Box alone:int[] beside:int[] taken:int[] }
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
    // Alone in its braces or beside another item, an untaken conditional contributes nothing.
    assertEqual(viaIr.alone, []);
    assertEqual(viaIr.beside, [2]);
    // A taken conditional is its branch as a one-item sequence, which is what its `int[]` type
    // says; the checker lifts the branch at the source, so no engine binds a bare `1` here.
    assertEqual(viaIr.taken, [1]);
    assertEqual(generated, viaIr);
    console.log("ok - a conditional with no else is a sequence in all three engines, alone or beside other items");
  },
);

// A taken conditional's value has to be the sequence its type claims, or code that consumes it as
// one breaks differently in each engine. Iterating it is the sharpest test: before the branch was
// lifted at the source, the interpreter raised, the IR runtime threw, and generated JavaScript
// iterated a string's characters -- `"new"` became `"n!" "e!" "w!"`.
withSource(
  `
type Out = { counted:int[] tagged:string[] mixed:int[] }
let c = true
let v = { if c { 1 } }
let tags:string[] = { if c { "new" } }
let xs:int[] = {5 6}
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
    assertEqual(viaIr.counted, [10]);
    assertEqual(viaIr.tagged, ["new!"]);
    assertEqual(viaIr.mixed, [1]);
    console.log("ok - a taken conditional iterates as the one-item sequence its type says, in all three engines");
  },
);

// `else { null }` is how a nullable value is written now that a missing `else` is `{}`, and it has
// to work beside a sequence as well as beside a scalar. A written null is not an item to lift, so
// the result is a nullable sequence that is null -- never `[null]`, a null item. `xs` holds two
// items on purpose: a one-item `{"a"}` at the `let xs:string[]` annotation meets the separate
// single-item-lift gap in generated code, which this case is not about.
withSource(
  `
type Out = { absent:string[]? present:string[]? reversed:string[]? }
let no = false
let yes = true
let xs:string[] = {"a" "b"}
let root(): Out = <Out
  absent={if no { xs } else { null }}
  present={if yes { xs } else { null }}
  reversed={if yes { null } else { xs }}
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
    // A null-valued field is absent from canonical output, so these compare as missing, not `[null]`.
    assertEqual(viaIr.absent ?? null, null);
    assertEqual(viaIr.reversed ?? null, null);
    console.log("ok - an explicit null else beside a sequence is a nullable sequence in all three engines");
  },
);

// A lone content child binds as itself, then takes the property's type, which is what the
// interpreter does. So an absent nullable sequence alone in a body binds `null` at a nullable-list
// property. Generated code briefly spliced a lone child instead, turning it into `[null]`; with a
// conditional's branch now lifted at the source there is no reason to splice one, and generated code
// is back to what it emitted before. The IR runtime is not compared: it splices a lone child at any
// list-typed property and then rejects the `null` element, which it already did before this change.
withSource(
  `
type A = { n:int = 1 }
type Box = { content items:A[]? }
let c = false
let as2:A[] = { <A/> <A n=2 /> }
let root() = { <Box>{if c { as2 } else { null }}</Box> }
`,
  (dir, sourcePath) => {
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
    assertEqual(native.items ?? null, null);
    console.log("ok - a lone absent nullable sequence binds null in the interpreter and generated JavaScript");
  },
);
