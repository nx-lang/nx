import { defineConfig } from "vitest/config";

// `node:wasi` is marked experimental and warns on every import. The loader uses nothing but `WASI`
// with `version: "preview1"`, so the warning is noise in every run; silencing it here reaches the
// worker processes vitest spawns, which is where the tests import it.
process.env["NODE_OPTIONS"] = `${process.env["NODE_OPTIONS"] ?? ""} --no-warnings`.trim();

export default defineConfig({
  test: {
    environment: "node",
    include: ["test/**/*.test.ts"],
    // The module compiles the catalog and the source together; a cold build is slower than the
    // default per-test budget on a loaded machine.
    testTimeout: 30_000
  }
});
