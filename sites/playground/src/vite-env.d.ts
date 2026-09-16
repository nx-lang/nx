/// <reference types="vite/client" />

declare module "*.nx?raw" {
  const source: string;
  export default source;
}

/** The catalog's NX IR image as base64, emitted at build time by the plugin in `vite.config.ts`. */
declare module "virtual:nx-catalog-artifact" {
  const encoded: string;
  export default encoded;
}
