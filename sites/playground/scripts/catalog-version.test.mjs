/**
 * The catalog is generated from the `drawnui-react` package the site pins, and it records that
 * package's version. When the pin moves and nobody regenerates, the catalog still describes the old
 * DrawnUI while the page draws with the new one. Nothing else would notice until a property the
 * catalog offers was missing from the control, so this test catches it first.
 */
import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import { test } from "node:test";

const read = (path) => JSON.parse(readFileSync(new URL(path, import.meta.url), "utf8"));

test("the catalog was generated from the drawnui-react version the site pins", () => {
  const pinned = read("../package.json").dependencies["drawnui-react"];
  const recorded = read("../catalog/catalog-meta.json").version;
  assert.match(pinned, /^\d+\.\d+\.\d+(-[\w.]+)?$/, `drawnui-react is pinned as "${pinned}"; pin it to one exact version`);
  assert.equal(
    recorded,
    pinned,
    `catalog/catalog-meta.json records drawnui-react ${recorded} but package.json pins ${pinned}: run \`pnpm run generate-catalog\``,
  );
});
