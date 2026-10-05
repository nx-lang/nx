import { fileURLToPath } from "node:url";

/** The package's root directory. Tests run from `dist/test`, two levels below it. */
export const packageRoot = fileURLToPath(new URL("../../", import.meta.url));

/** The repository's root directory. */
export const repositoryRoot = fileURLToPath(new URL("../../../../", import.meta.url));
