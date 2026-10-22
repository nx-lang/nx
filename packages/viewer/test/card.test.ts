import assert from "node:assert/strict";
import { after, describe, it } from "node:test";

import type { SourceNode } from "@nx-lang/language-protocol";

import { cardOf } from "../src/card.js";
import { indexTree } from "../src/tree.js";
import { fixture, nxHost } from "./fixtures.js";

after(async () => (await nxHost()).dispose());

/** The card of the first node of the `hover` fixture that `test` picks. */
async function cardWhere(test: (node: SourceNode) => boolean): Promise<ReturnType<typeof cardOf>> {
  const { tree, text } = await fixture("hover");
  const at = tree.nodes.findIndex(test);
  assert.ok(at >= 0, "the fixture has the node the test looks for");
  return cardOf(indexTree(tree, text), at);
}

const inRoot = (node: SourceNode): boolean => node.key.startsWith("root.");

describe("hover cards", () => {
  it("says what an element is: its declaration's kind, name, base and doc", async () => {
    assert.deepEqual(await cardWhere((node) => inRoot(node) && node.role === "element" && node.name === "Shape"), {
      signature: "component Shape extends Node",
      doc: "A filled shape.",
    });
    assert.deepEqual(await cardWhere((node) => inRoot(node) && node.role === "element" && node.name === "Glow"), {
      signature: "let <Glow />",
      doc: "A glowing shape.",
    });
  });

  it("spells each kind of declaration as the editor's hover begins it", async () => {
    const declared = async (name: string) => cardWhere((node) => node.role === "declaration" && node.name === name);
    assert.deepEqual(await declared("Node"), { signature: "abstract component Node", doc: "A node in the tree." });
    assert.deepEqual(await declared("ShapeType"), { signature: "type ShapeType" });
    assert.deepEqual(await declared("Task"), { signature: "type Task", doc: "A task to do." });
    assert.deepEqual(await declared("Submitted"), { signature: "action Submitted", doc: "Something was sent." });
    assert.deepEqual(await declared("Place"), { signature: "type Place = string", doc: "Where it lives." });
    assert.deepEqual(await declared("twice"), { signature: "let twice(n:int): int", doc: "Doubles." });
    assert.deepEqual(await declared("size"), { signature: "let size: int", doc: "How big." });
    assert.deepEqual(await declared("Glow"), { signature: "let <Glow />", doc: "A glowing shape." });
  });

  it("gives a property's type, whether it is optional, its default and its doc, inherited ones too", async () => {
    const property = async (name: string) =>
      cardWhere((node) => inRoot(node) && node.role === "attribute" && node.name === name);
    assert.deepEqual(await property("Type"), {
      signature: "(property) Type: ShapeType = Rectangle",
      doc: "What the shape draws.",
    });
    assert.deepEqual(await property("label"), { signature: "(property) label?: string", doc: "Words under it." });
    assert.deepEqual(await property("visible"), {
      signature: "(property) visible: boolean = true",
      doc: "Whether it shows.",
    }, "a property of the base it extends");
  });

  it("names a union case by its union, bare or qualified, with the case's doc", async () => {
    assert.deepEqual(await cardWhere((node) => inRoot(node) && node.role === "case" && node.name === "Circle"), {
      signature: "(case) ShapeType.Circle",
      doc: "Round.",
    });
    assert.deepEqual(await cardWhere((node) => inRoot(node) && node.role === "case" && node.name === "Rectangle"), {
      signature: "(case) ShapeType.Rectangle",
      doc: "Square corners.",
    });
  });

  it("gives a reference's or a member's type, and a call its function's signature", async () => {
    assert.deepEqual(await cardWhere((node) => inRoot(node) && node.role === "reference" && node.name === "size"), {
      signature: "size: int",
      doc: "How big.",
    });
    assert.deepEqual(await cardWhere((node) => inRoot(node) && node.role === "member"), { signature: "title: string" });
    const call = { signature: "let twice(n:int): int", doc: "Doubles." };
    assert.deepEqual(await cardWhere((node) => inRoot(node) && node.role === "call"), call);
    assert.deepEqual(await cardWhere((node) => inRoot(node) && node.role === "reference" && node.name === "twice"), call);
  });

  it("says nothing about a literal or a comment", async () => {
    assert.equal(await cardWhere((node) => inRoot(node) && node.role === "literal" && node.value === '"hi"'), undefined);
    assert.equal(await cardWhere((node) => inRoot(node) && node.role === "comment"), undefined);
  });
});
