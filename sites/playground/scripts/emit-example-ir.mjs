/**
 * Emits each example's NX IR, explained, into a directory, for comparing one edit of the corpus
 * against another.
 *
 * Each image is written as the text `nxlang ir explain` prints rather than as bytes, so a diff
 * between two baselines names what changed about the program. The module fingerprints are stripped
 * from the text: a fingerprint moves whenever the source text does, and that movement is correct
 * rather than a difference in meaning. A compile emits the example's own module without a debug
 * section, so what is left is what must not change.
 *
 * Usage: node scripts/emit-example-ir.mjs <out-dir>
 */
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { compile, explain } from "./compile-example.mjs";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const outDir = resolve(process.argv[2] ?? "ir-baseline");

function stripProvenance(text) {
  return text.replace(/ fingerprint \d+/g, "");
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
  writeFileSync(join(outDir, `${example.file}.nxir.txt`), stripProvenance(explain(ir)));
}

console.log(`wrote IR for ${examples.length} examples to ${outDir}`);
