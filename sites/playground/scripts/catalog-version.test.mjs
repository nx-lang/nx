/**
 * The catalog is generated from the `drawnui-react` package the site pins, and it records that
 * package's version. When the pin moves and nobody regenerates, the catalog still describes the old
 * DrawnUI while the page draws with the new one. Nothing else would notice until a property the
 * catalog offers was missing from the control, so this test catches it first.
 *
 * The fonts, images and shaders the examples read are copied from the upstream tag of the same
 * release, and `docs/UPSTREAM.md` records which. A pin moved without a sync would draw the new
 * DrawnUI with the old release's assets, so that record is checked against the pin too.
 */
import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import { test } from "node:test";

const read = (path) => JSON.parse(readFileSync(new URL(path, import.meta.url), "utf8"));
const pin = () => read("../package.json").dependencies["drawnui-react"];

test("the catalog was generated from the drawnui-react version the site pins", () => {
  const pinned = pin();
  const recorded = read("../catalog/catalog-meta.json").version;
  assert.match(pinned, /^\d+\.\d+\.\d+(-[\w.]+)?$/, `drawnui-react is pinned as "${pinned}"; pin it to one exact version`);
  assert.equal(
    recorded,
    pinned,
    `catalog/catalog-meta.json records drawnui-react ${recorded} but package.json pins ${pinned}: run \`pnpm run generate-catalog\``,
  );
});

test("the DrawnUI assets were copied from the release the site pins", () => {
  const pinned = pin();
  const upstream = readFileSync(new URL("../docs/UPSTREAM.md", import.meta.url), "utf8");
  const recorded = upstream.match(/^\| Package \| `drawnui-react` (\S+) \|$/m)?.[1];
  assert.ok(recorded, "docs/UPSTREAM.md has no `| Package | drawnui-react <version> |` row: run `pnpm run sync-drawnui`");
  assert.equal(
    recorded,
    pinned,
    `docs/UPSTREAM.md records assets from drawnui-react ${recorded} but package.json pins ${pinned}: run \`pnpm run sync-drawnui\``,
  );
});
