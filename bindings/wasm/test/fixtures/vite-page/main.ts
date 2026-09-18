// The smallest page that proves the package's default (browser) entry point bundles: it loads the
// module as an asset, creates a host and compiles one source.
import { compileNxModule, createNxHost } from "@nx-lang/sdk-wasm";

import nxModuleUrl from "@nx-lang/sdk-wasm/nx.wasm?url";

const module = await compileNxModule(fetch(nxModuleUrl));
const host = createNxHost(module);
const artifact = host.buildProgramArtifact("let root() = { 42 }", { fileName: "input.nx" });

document.body.textContent = artifact.generateNxIr()[0]!.metadata.fingerprint;
artifact.dispose();
host.dispose();
