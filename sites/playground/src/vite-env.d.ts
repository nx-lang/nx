/// <reference types="vite/client" />

declare module "*.nx?raw" {
  const source: string;
  export default source;
}
