/**
 * The instance tree, against the real wasm compiler and the site's own catalog.
 *
 * Each case compiles a snippet the way the editor does, links it against the catalog, and drives
 * the tree the way the drawing does: visit the root's descriptor, visit each authored descriptor
 * in what it rendered under it, and dispatch the tokens the rendered output carries.
 */
import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import { after, test } from "node:test";
import { createNxHost, loadNxModule } from "@nx-lang/sdk-wasm";
import { compileWithCatalog, emitCatalogArtifact } from "../compile/catalog.ts";
import { evaluateRoot, prepare, prepareCatalog } from "./evaluate.ts";
import { InstanceTree, isHandlerRecord, stripInertHandlers } from "./instances.ts";

const catalog = readFileSync(new URL("../../catalog/skia.nx", import.meta.url), "utf8");
const host = createNxHost(await loadNxModule());
const prepared = prepareCatalog(emitCatalogArtifact(host, catalog));
after(() => host.dispose());

/** Compiles a snippet and returns the tree over it with the root's node visited. */
function open(source) {
  const result = compileWithCatalog(host, catalog, source);
  assert.deepEqual(result.diagnostics.map((d) => d.message), [], "the snippet compiles");
  const program = prepare(result.ir, prepared);
  const tree = new InstanceTree(program);
  tree.begin();
  const root = tree.visit("root", evaluateRoot(program), null);
  return { tree, root };
}

/** The first value in a rendered tree, depth first, that the predicate accepts. */
function find(value, predicate, path = "") {
  if (Array.isArray(value)) {
    for (const [index, item] of value.entries()) {
      const found = find(item, predicate, `${path}[${index}]`);
      if (found !== undefined) {
        return found;
      }
    }
    return undefined;
  }
  if (typeof value !== "object" || value === null) {
    return undefined;
  }
  if (predicate(value)) {
    return value;
  }
  for (const [name, item] of Object.entries(value)) {
    if (name !== "$type") {
      const found = find(item, predicate, `${path}.${name}`);
      if (found !== undefined) {
        return found;
      }
    }
  }
  return undefined;
}

const control = (type, text) => (value) => value.$type === type && (text === undefined || value.Text === text);
const tokenOf = (node, type, text) => find(node.rendered, control(type, text)).onTapped.token;
const tapped = { $type: "SkiaControl.Tapped" };

const COUNTER = `
component <Counter /> = {
  state { count:int = 0 }
  <SkiaStack>
    <SkiaLabel Text={if count > 0 { "tapped" } else { "untapped" }} />
    <SkiaButton Text="Tap" onTapped=<Update count={count + 1} /> />
  </SkiaStack>
}
let root() = { <Counter /> }
`;

test("a tap patches the state of the instance whose body bound the handler and re-renders it", () => {
  const { tree, root } = open(COUNTER);
  assert.equal(root.component, "Counter");
  assert.ok(find(root.rendered, control("SkiaLabel", "untapped")));
  const before = root.instance;
  const effects = tree.dispatch(root, tokenOf(root, "SkiaButton"), tapped);
  assert.deepEqual(effects, []);
  assert.deepEqual(root.instance.state, { count: 1 });
  assert.ok(find(root.rendered, control("SkiaLabel", "tapped")));
  assert.notEqual(root.instance, before, "dispatch replaces the instance rather than mutating it");
});

test("two taps read the state live: the second sees what the first left", () => {
  const { tree, root } = open(COUNTER);
  tree.dispatch(root, tokenOf(root, "SkiaButton"), tapped);
  // The redraw handed out new tokens; the second tap reads the current one.
  tree.dispatch(root, tokenOf(root, "SkiaButton"), tapped);
  assert.deepEqual(root.instance.state, { count: 2 });
});

test("a readout joins the state's number, boolean and float to its words, and follows a tap", () => {
  const { tree, root } = open(`
component <Page /> = {
  state { taps:int = 0 open:boolean = false speed:float64 = 1 }
  <SkiaStack>
    <SkiaLabel Text={"Tapped " + taps + "× · IsOpen: " + open + " · " + speed + "x"} />
    <SkiaButton Text="Tap" onTapped=<Update taps={taps + 1} open={!open} speed={speed / 2} /> />
  </SkiaStack>
}
let root() = { <Page /> }
`);
  assert.ok(find(root.rendered, control("SkiaLabel", "Tapped 0× · IsOpen: false · 1x")));
  tree.dispatch(root, tokenOf(root, "SkiaButton"), tapped);
  assert.ok(find(root.rendered, control("SkiaLabel", "Tapped 1× · IsOpen: true · 0.5x")));
});

const CHOSEN = `
component <Child extends DrawnNode emits { Chosen { name:string } } /> = {
  <SkiaButton Text="Pick" onTapped=<Child.Chosen name="a" /> />
}
component <Page /> = {
  state { picked:string = "" }
  <SkiaStack>
    <SkiaLabel Text={picked} />
    <Child onChosen=<Update picked={action.name} /> />
  </SkiaStack>
}
let root() = { <Page /> }
`;

test("an action a child emits reaches the handler its parent bound and patches the parent", () => {
  const { tree, root } = open(CHOSEN);
  const descriptor = find(root.rendered, (value) => value.$type === "Child");
  assert.ok(isHandlerRecord(descriptor.onChosen), "the parent's rendered descriptor carries the handler");
  const child = tree.visit("root.child", descriptor, root);
  const effects = tree.dispatch(child, tokenOf(child, "SkiaButton"), tapped);
  assert.deepEqual(effects, []);
  assert.deepEqual(root.instance.state, { picked: "a" });
  assert.ok(find(root.rendered, control("SkiaLabel", "a")));
});

test("a handler that emits two actions its parent bound patches the parent with both", () => {
  const { tree, root } = open(`
component <Child extends DrawnNode emits { A { } B { } } /> = {
  <SkiaButton Text="Both" onTapped={<Child.A /> <Child.B />} />
}
component <Page /> = {
  state { a:int = 0 b:int = 0 }
  <Child onA=<Update a={a + 1} /> onB=<Update b={b + a} /> />
}
let root() = { <Page /> }
`);
  const child = tree.visit("root.child", root.rendered, root);
  const effects = tree.dispatch(child, tokenOf(child, "SkiaButton"), tapped);
  assert.deepEqual(effects, []);
  // One batch against the parent, so the second handler reads what the first left.
  assert.deepEqual(root.instance.state, { a: 1, b: 1 });
});

test("a chain that fails part way puts back every instance it had replaced", () => {
  const { tree, root } = open(`
component <Child extends DrawnNode emits { A { } } /> = {
  state { c:int = 0 }
  <SkiaButton Text="Go" onTapped={<Update c={c + 1} /> <Child.A />} />
}
component <Page /> = {
  state { n:int = 1 }
  <Child onA=<Update n={n / 0} /> />
}
let root() = { <Page /> }
`);
  const child = tree.visit("root.child", root.rendered, root);
  const token = tokenOf(child, "SkiaButton");
  const instance = child.instance;
  const rendered = child.rendered;
  assert.throws(() => tree.dispatch(child, token, tapped), /zero/i);
  assert.equal(child.instance, instance, "the child's dispatch is undone with the parent's failure");
  assert.equal(child.rendered, rendered);
  assert.deepEqual(child.instance.state, { c: 0 });
  // The token the standing drawing carries still names a handler.
  assert.throws(() => tree.dispatch(child, token, tapped), /zero/i);
});

test("a drawing pass that throws inside a transaction leaves the tree as it was", () => {
  const { tree, root } = open(COUNTER);
  const instance = root.instance;
  assert.throws(() =>
    tree.atomically(() => {
      tree.dispatch(root, tokenOf(root, "SkiaButton"), tapped);
      tree.begin();
      tree.visit("root", root.descriptor, null);
      throw new Error("draw failed");
    }),
  /draw failed/);
  assert.equal(root.instance, instance);
  assert.deepEqual(root.instance.state, { count: 0 });
});

const CONTENT = `
component <Box extends DrawnNode content Children:DrawnNode+ /> = {
  state { opened:int = 0 }
  <SkiaStack>
    <SkiaButton Text="Open" onTapped=<Update opened={opened + 1} /> />
    {Children}
  </SkiaStack>
}
component <Page /> = {
  state { count:int = 0 }
  <Box>
    <SkiaLabel Text={if count > 0 { "counted" } else { "zero" }} />
    <SkiaButton Text="More" onTapped=<Update count={count + 1} /> />
  </Box>
}
let root() = { <Page /> }
`;

test("a handler in a content child patches its owner, and the child keeps its own state", () => {
  const { tree, root } = open(CONTENT);
  const box = tree.visit("root.box", root.rendered, root);
  // The Box's own button first, so it holds state of its own to keep.
  tree.dispatch(box, tokenOf(box, "SkiaButton", "Open"), tapped);
  assert.deepEqual(box.instance.state, { opened: 1 });
  // The Page's button is drawn inside the Box's output, under the Box's token.
  const effects = tree.dispatch(box, tokenOf(box, "SkiaButton", "More"), tapped);
  assert.deepEqual(effects, []);
  assert.deepEqual(root.instance.state, { count: 1 });
  assert.deepEqual(box.instance.state, { opened: 1 });
});

test("a parent redraw re-initializes the child from the new descriptor with the state it held", () => {
  const { tree, root } = open(CONTENT);
  const box = tree.visit("root.box", root.rendered, root);
  tree.dispatch(box, tokenOf(box, "SkiaButton", "Open"), tapped);
  tree.dispatch(box, tokenOf(box, "SkiaButton", "More"), tapped);
  // The Page redrew: its rendered output is a new descriptor, drawn at the same position.
  tree.begin();
  const again = tree.visit("root", root.descriptor, null);
  assert.equal(again, root);
  const redrawn = tree.visit("root.box", root.rendered, root);
  tree.finish();
  assert.equal(redrawn, box, "the node is kept at its position");
  assert.deepEqual(box.instance.state, { opened: 1 });
  assert.ok(find(box.rendered, control("SkiaLabel", "counted")), "the child draws the parent's new props");
  assert.equal(box.instance.generation, 1, "re-initialized, not dispatched");
});

test("a node not visited by a drawing pass is dropped, and a new one starts from its initial state", () => {
  const { tree, root } = open(CONTENT);
  const box = tree.visit("root.box", root.rendered, root);
  tree.dispatch(box, tokenOf(box, "SkiaButton", "Open"), tapped);
  tree.begin();
  tree.visit("root", root.descriptor, null);
  tree.finish();
  tree.begin();
  tree.visit("root", root.descriptor, null);
  const fresh = tree.visit("root.box", root.rendered, root);
  tree.finish();
  assert.notEqual(fresh, box);
  assert.deepEqual(fresh.instance.state, { opened: 0 });
});

test("an emit nobody bound is a host effect naming the instance that produced it", () => {
  const { tree, root } = open(`
action Saved = { }
component <Child extends DrawnNode emits { Saved } /> = {
  <SkiaButton Text="Save" onTapped=<Saved /> />
}
component <Page /> = { <Child /> }
let root() = { <Page /> }
`);
  const child = tree.visit("root.child", root.rendered, root);
  const effects = tree.dispatch(child, tokenOf(child, "SkiaButton"), tapped);
  assert.deepEqual(effects, [{ instance: "Child", action: { $type: "Saved" } }]);
});

test("a failed dispatch throws the runtime's diagnostic and leaves every instance as it was", () => {
  const { tree, root } = open(`
component <Counter /> = {
  state { count:int = 0 }
  <SkiaButton Text="Tap" onTapped=<Update count={count / 0} /> />
}
let root() = { <Counter /> }
`);
  const instance = root.instance;
  const rendered = root.rendered;
  assert.throws(() => tree.dispatch(root, tokenOf(root, "SkiaButton"), tapped), /zero/i);
  assert.equal(root.instance, instance);
  assert.equal(root.rendered, rendered);
  // The same token still dispatches, since nothing about the instance changed.
  assert.throws(() => tree.dispatch(root, tokenOf(root, "SkiaButton"), tapped), /zero/i);
});

test("a handler bound outside any component carries no token, and is stripped and reported as inert", () => {
  const { tree, root } = open(`
action Log = { }
component <Page extends DrawnNode content Children:DrawnNode+ /> = { <SkiaStack>{Children}</SkiaStack> }
let root() = { <Page><SkiaButton Text="Log" onTapped=<Log /> /></Page> }
`);
  const record = find(root.descriptor, control("SkiaButton")).onTapped;
  assert.ok(isHandlerRecord(record));
  assert.equal(record.token, undefined);
  assert.deepEqual(tree.inert, ["SkiaButton.onTapped"]);
  // The instance was initialized without it, and draws the button with no handler.
  assert.equal(find(root.rendered, control("SkiaButton")).onTapped, undefined);
  const { value, inert } = stripInertHandlers(root.descriptor);
  assert.deepEqual(inert, ["SkiaButton.onTapped"]);
  assert.equal(find(value, control("SkiaButton")).onTapped, undefined);
  tree.begin();
  assert.deepEqual(tree.inert, [], "cleared by the next pass");
});
