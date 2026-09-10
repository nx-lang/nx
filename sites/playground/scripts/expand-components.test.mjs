/**
 * Pins the component expansion `check-examples` relies on: a runtime failure inside a component
 * body must surface, and it only does if the walk expands the body rather than stopping at the
 * descriptor `root` evaluates to. The failing body is FINDINGS F22 — `+` on a record field lowers
 * to a numeric add — which is exactly the failure the plain evaluation of `root` let through.
 */
import { evaluateFunction, prepareNxIrProgram } from "@nx-lang/ir-runtime";
import { strict as assert } from "node:assert";
import { test } from "node:test";
import { compile } from "../server/compile.mjs";
import { expandComponents } from "./expand-components.mjs";

function evaluateRoot(source) {
  const result = compile(source);
  assert.deepEqual(result.diagnostics, []);
  const program = prepareNxIrProgram(result.ir);
  return { program, root: evaluateFunction(program, "root") };
}

const FAILING_BODY = `type Item = { Title: string }
component <Row extends DrawnNode Item:Item /> = {
  <SkiaLabel Text={"Reorder " + Item.Title} />
}
<SkiaStack>
  <Row Item=<Item Title="English" /> />
</SkiaStack>
`;

test("evaluating root alone does not run a component body", () => {
  const { root } = evaluateRoot(FAILING_BODY);
  // The use is a descriptor; nothing has failed yet, which is why the expansion exists.
  assert.equal(root.Children[0].$type, "Row");
});

test("expanding the components runs the body and surfaces its failure", () => {
  const { program, root } = evaluateRoot(FAILING_BODY);
  assert.throws(() => expandComponents(program, root), /Operator 'add'/);
});

test("a body that evaluates expands silently, nested uses included", () => {
  const { program, root } = evaluateRoot(`component <Chip extends DrawnNode Text:string /> = {
  <SkiaLabel Text={Text + "!"} />
}
component <Card extends DrawnNode content Children:DrawnNode[] /> = {
  <SkiaStack>{Children}</SkiaStack>
}
<Card><Chip Text="a" /><Chip Text="b" /></Card>
`);
  assert.doesNotThrow(() => expandComponents(program, root));
});
