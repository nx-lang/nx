/**
 * Coercing evaluated NX values into DrawnUI props. The runtime leaves an empty field out of the
 * element it evaluates, so these cases hand the renderer the empty value written out, which is what
 * it has to survive if a later runtime writes it.
 */
import { strict as assert } from "node:assert";
import { test } from "node:test";
import { childrenOf, coerce, coerceProps } from "./values.ts";

test("the empty value, an empty sequence, is dropped from props, a record and content like a null", () => {
  const props = coerceProps({
    $type: "SkiaLabel",
    Text: "hi",
    FontSize: [],
    Padding: { $type: "Thickness", Left: 4, Top: [] },
  });
  assert.deepEqual(Object.keys(props), ["Text", "Padding"]);
  assert.deepEqual([props.Padding.Left, props.Padding.Top], [4, 0]);
  assert.deepEqual(coerce({ $type: "SkiaGradient", Colors: [], Angle: 45 }), { Angle: 45 });
  assert.deepEqual(childrenOf({ $type: "SkiaStack", Children: [] }), []);
});
