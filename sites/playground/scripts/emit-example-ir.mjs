/**
 * Emits each example's NX IR into a directory, for comparing one edit of the corpus against another.
 *
 * Source provenance is stripped: spans, the retained source text, and the fingerprint over them all
 * move when a literal changes width in the file, and that movement is correct rather than a
 * difference in meaning. What is left is what must not change.
 *
 * Usage: node scripts/emit-example-ir.mjs <out-dir>
 */
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { compile } from "../server/compile.mjs";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const outDir = resolve(process.argv[2] ?? "ir-baseline");

function stripProvenance(value) {
  if (Array.isArray(value)) {
    return value.map(stripProvenance);
  }
  if (value && typeof value === "object") {
    const stripped = {};
    for (const [key, nested] of Object.entries(value)) {
      if (key === "span" || key === "source" || key === "sources" || key === "programFingerprint") {
        continue;
      }
      stripped[key] = stripProvenance(nested);
    }
    return stripped;
  }
  return value;
}

mkdirSync(outDir, { recursive: true });
const examples = JSON.parse(readFileSync(join(appRoot, "src/examples/examples.json"), "utf8"));

for (const example of examples) {
  const source = readFileSync(join(appRoot, "src/examples/nx", example.file), "utf8");
  const { ir, diagnostics } = compile(source);
  if (ir === null) {
    console.error(`${example.id}: did not compile`);
    for (const diagnostic of diagnostics) {
      console.error(`  ${diagnostic.code}: ${diagnostic.message}`);
    }
    process.exitCode = 1;
    continue;
  }
  writeFileSync(join(outDir, `${example.file}.json`), JSON.stringify(stripProvenance(ir), null, 2));
}

console.log(`wrote IR for ${examples.length} examples to ${outDir}`);
