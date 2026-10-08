/**
 * Checks that a Rust project outside this repository can build the runtime from its packaged
 * crates alone.
 *
 * `node scripts/verify-crates.mjs` packs `nx-ir`, `nx-value` and `nx-ir-runtime` at a test version,
 * unpacks the three packages into a scratch directory, and builds a host project there that
 * depends on `nx-ir-runtime` at exactly that version, with the three names patched to the unpacked
 * packages in place of the registry. It then reads the project's dependency tree and fails when a
 * crate of the NX compiler is in it: the reason the runtime is split from the compiler is that a
 * host compiles neither the parser nor the type checker.
 *
 * Packing is itself the check that each crate builds from its packaged contents; this adds the
 * consumer's view, with the exact pins resolved the way a registry would resolve them.
 */
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { packCrates, packedCrateProblems, runtimeCrates } from "./runtime-crates.mjs";

const repositoryRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
const version = "0.0.0-verify.1";
const scratch = mkdtempSync(join(tmpdir(), "nx-verify-crates-"));

try {
  const packed = await packCrates(repositoryRoot, version, join(scratch, "crates"));
  const problems = packed.flatMap(({ name, file, path }) =>
    packedCrateProblems(path, name, version).map((problem) => `${file}: ${problem}`),
  );
  if (problems.length > 0) {
    throw new Error(problems.join("\n"));
  }
  for (const { path } of packed) {
    execFileSync("tar", ["-xzf", path, "-C", scratch]);
  }

  const host = join(scratch, "host");
  mkdirSync(join(host, "src"), { recursive: true });
  writeFileSync(
    join(host, "Cargo.toml"),
    [
      "[package]",
      'name = "nx-host-check"',
      'version = "0.0.0"',
      'edition = "2021"',
      "publish = false",
      "",
      "[dependencies]",
      `nx-ir-runtime = "=${version}"`,
      `nx-value = "=${version}"`,
      "",
      "[patch.crates-io]",
      ...runtimeCrates.map((name) => `${name} = { path = "../${name}-${version}" }`),
      "",
      "[workspace]",
      "",
    ].join("\n"),
  );
  // Names what a host uses, so the check fails if the package lost any of it.
  writeFileSync(
    join(host, "src", "main.rs"),
    [
      "use nx_ir_runtime::{InstanceTree, LinkOptions, PreparedModule, Program, RuntimeOptions};",
      "use nx_value::NxValue;",
      "",
      "fn main() -> nx_ir_runtime::Result<()> {",
      "    let image = std::fs::read(std::env::args().nth(1).unwrap_or_default()).unwrap_or_default();",
      "    let module = PreparedModule::prepare(image)?;",
      "    let program = Program::link(&module, |_| None, &LinkOptions::default())?;",
      "    let options = RuntimeOptions { max_stack_bytes: 256 << 10, ..RuntimeOptions::default() };",
      "    let root = program.evaluate_function(\"root\", &[], &options)?;",
      "    let mut tree = InstanceTree::new(program);",
      "    tree.begin();",
      "    let rendered = tree.visit(\"root\", &root, None, &options)?;",
      "    tree.finish();",
      "    println!(\"{}\", rendered.to_json_string().unwrap_or_default());",
      "    let _ = NxValue::empty();",
      "    Ok(())",
      "}",
      "",
    ].join("\n"),
  );

  execFileSync("cargo", ["check"], { cwd: host, stdio: "inherit" });
  const tree = execFileSync("cargo", ["tree", "--edges", "normal,build", "--prefix", "none", "--format", "{p}"], {
    cwd: host,
    encoding: "utf8",
  });
  const crates = new Set(tree.split("\n").map((line) => line.trim().split(" ")[0]).filter(Boolean));
  const ours = [...crates].filter((name) => name.startsWith("nx-") && name !== "nx-host-check");
  const unexpected = ours.filter((name) => !runtimeCrates.includes(name));
  const missing = runtimeCrates.filter((name) => !crates.has(name));
  if (unexpected.length > 0 || missing.length > 0 || crates.has("tree-sitter")) {
    throw new Error(
      `a host of nx-ir-runtime must build ${runtimeCrates.join(", ")} and no other NX crate; ` +
        `its tree has ${ours.join(", ")}${crates.has("tree-sitter") ? " and tree-sitter" : ""}`,
    );
  }
  console.log(`verify-crates: a host builds ${[...ours].sort().join(", ")} from the packages, with ${crates.size - ours.length - 1} other crate(s)`);
} catch (error) {
  console.error(`verify-crates: ${error.message}`);
  process.exitCode = 1;
} finally {
  rmSync(scratch, { recursive: true, force: true });
}
