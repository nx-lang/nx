// Cargo rejects normal dependency cycles; this covers test-only upward edges and the
// independent program/runtime boundary, including optional and target-specific dependencies.
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

export function layeringViolations(metadata) {
  const packages = new Map(metadata.packages.map(pkg => [pkg.name, pkg]));
  const belowFacade = new Set();
  const visit = name => {
    if (belowFacade.has(name)) return;
    belowFacade.add(name);
    for (const dependency of packages.get(name)?.dependencies ?? []) {
      if (dependency.kind !== "dev" && dependency.name !== "nx-api") visit(dependency.name);
    }
  };
  for (const dependency of packages.get("nx-api").dependencies) {
    if (dependency.kind !== "dev") visit(dependency.name);
  }
  const errors = [];
  for (const pkg of metadata.packages) {
    for (const dependency of pkg.dependencies) {
      const upward = belowFacade.has(pkg.name) && dependency.name === "nx-api";
      const runtimeBoundary = (pkg.name === "nx-program" && dependency.name === "nx-ir-runtime")
        || (pkg.name === "nx-ir-runtime" && dependency.name === "nx-program");
      if (upward || runtimeBoundary) {
        errors.push(`${pkg.name} -> ${dependency.name} (${dependency.kind ?? "normal"} dependency${
          dependency.target ? `, target ${dependency.target}` : ""})`);
      }
    }
  }
  return errors;
}

export function workspaceMetadata() {
  return JSON.parse(execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1", "--locked"], {
    cwd: path.resolve(path.dirname(fileURLToPath(import.meta.url)), ".."),
    encoding: "utf8"
  }));
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const errors = layeringViolations(workspaceMetadata());
  if (errors.length) {
    console.error(`Forbidden crate dependencies:\n${errors.map(error => `  ${error}`).join("\n")}`);
    process.exitCode = 1;
  } else {
    console.log("Crate layering verified.");
  }
}
