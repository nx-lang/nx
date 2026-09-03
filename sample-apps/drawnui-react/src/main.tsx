import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { drawnUiReady } from "./drawnui-runtime";

await drawnUiReady;

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
