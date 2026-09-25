/**
 * Pins the component expansion `check-examples` relies on: a runtime failure inside a component
 * body must surface, and it only does if the walk expands the body rather than stopping at the
 * descriptor `root` evaluates to. The failing body divides by a prop that is zero, which compiles
 * clean and fails only when the body runs. The failure this was written for, a `+` on a record
 * field lowering to a numeric add, was a compiler defect and is fixed, so it no longer serves as one.
 */
import { evaluateFunction } from "@nx-lang/ir-runtime";
import { strict as assert } from "node:assert";
import { test } from "node:test";
import { catalogModule, compile } from "./compile-example.mjs";
import { expandComponents } from "./expand-components.mjs";
import { prepare } from "../src/render/evaluate.ts";

function evaluateRoot(source) {
  const result = compile(source);
  assert.deepEqual(result.diagnostics, []);
  const program = prepare(result.ir, catalogModule);
  return { program, root: evaluateFunction(program, "root") };
}

const FAILING_BODY = `type Item = { Title: string  Columns: int }
component <Row extends DrawnNode Item:Item /> = {
  <SkiaLabel Text={Item.Title} FontSize={120 / Item.Columns} />
}
<SkiaStack>
  <Row Item=<Item Title="English" Columns=0 /> />
</SkiaStack>
`;

test("evaluating root alone does not run a component body", () => {
  const { root } = evaluateRoot(FAILING_BODY);
  // The use is a descriptor; nothing has failed yet, which is why the expansion exists.
  assert.equal(root.Children[0].$type, "Row");
});

test("expanding the components runs the body and surfaces its failure", () => {
  const { program, root } = evaluateRoot(FAILING_BODY);
  assert.throws(() => expandComponents(program, root), /Division by zero/);
});

test("a string joined to a record field in a component body evaluates", () => {
  // This `+` used to lower to a numeric add and fail only here, in the body.
  const { program, root } = evaluateRoot(`type Item = { Title: string }
component <Row extends DrawnNode Item:Item /> = {
  <SkiaLabel Text={"Reorder " + Item.Title} />
}
<SkiaStack>
  <Row Item=<Item Title="English" /> />
</SkiaStack>
`);
  assert.deepEqual(expandComponents(program, root), { inert: [] });
});

test("a child is initialized under its parent, so a handler the parent bound resolves", () => {
  const { program, root } = evaluateRoot(`component <Child extends DrawnNode emits { Chosen { } } /> = {
  <SkiaButton Text="Pick" onTapped=<Child.Chosen /> />
}
component <Page /> = {
  state { picked:boolean = false }
  <Child onChosen=<Update picked=true /> />
}
<Page />
`);
  assert.deepEqual(expandComponents(program, root), { inert: [] });
});

test("a handler bound outside any component is reported rather than expanded", () => {
  const { program, root } = evaluateRoot(`action Log = { }
component <Card extends DrawnNode content Children:DrawnNode+ /> = { <SkiaStack>{Children}</SkiaStack> }
<Card><SkiaButton Text="Log" onTapped=<Log /> /></Card>
`);
  assert.deepEqual(expandComponents(program, root), { inert: ["SkiaButton.onTapped"] });
});

test("a handler bound on a control outside any component is reported, as the renderer reports it", () => {
  const { program, root } = evaluateRoot(`action Log = { }
<SkiaStack><SkiaButton Text="Log" onTapped=<Log /> /></SkiaStack>
`);
  assert.deepEqual(expandComponents(program, root), { inert: ["SkiaButton.onTapped"] });
});

test("a body that evaluates expands silently, nested uses included", () => {
  const { program, root } = evaluateRoot(`component <Chip extends DrawnNode Text:string /> = {
  <SkiaLabel Text={Text + "!"} />
}
component <Card extends DrawnNode content Children:DrawnNode+ /> = {
  <SkiaStack>{Children}</SkiaStack>
}
<Card><Chip Text="a" /><Chip Text="b" /></Card>
`);
  assert.doesNotThrow(() => expandComponents(program, root));
});
