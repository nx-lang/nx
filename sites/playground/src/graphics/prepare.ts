import {
  NxHostCrashedError,
  type NxProgramArtifact,
  type NxGraphicsConfig,
  type NxGraphicsDiagnostic,
} from "@nx-lang/sdk-wasm";
import {
  type NxCanonicalValue,
  type NxRuntimeOrigins,
} from "@nx-lang/ir-runtime";
import { FILE_NAME, classifyDiagnostic } from "../compile/policy.ts";
import { authoredSites } from "../preview/selection.ts";
import { SourceIndex } from "../preview/spans.ts";
import { measureBrowserText } from "./browser-font.ts";
import {
  BROWSER_RENDERER,
  type DrawingResult,
  type Measurement,
  type TextRequest,
} from "./protocol.ts";
import type { Diagnostic } from "../compile/types.ts";
class GraphicsValueError extends Error {
  constructor(
    message: string,
    readonly path: string,
  ) {
    super(message);
  }
}

/** Reject numbers before JSON could replace NaN/infinity with null; bound the bridge payload. */
export function checkGraphicsValue(value: unknown): void {
  let remaining = 1000000;
  function walk(value: unknown, path: string, depth: number): void {
    if (--remaining < 0 || depth > 110) {
      throw new GraphicsValueError(
        "The drawing value exceeds the browser preview limits.",
        path,
      );
    }
    if (typeof value === "number" && !Number.isFinite(value)) {
      throw new GraphicsValueError(
        `Non-finite graphics value at ${path || "/"}.`,
        path,
      );
    }
    if (typeof value === "string") {
      remaining -= value.length;
      if (remaining < 0) {
        throw new GraphicsValueError(
          "The drawing value exceeds the browser preview limits.",
          path,
        );
      }
    }
    if (Array.isArray(value)) {
      value.forEach((v, i) => walk(v, `${path}/${i}`, depth + 1));
    } else if (value !== null && typeof value === "object") {
      for (const [key, v] of Object.entries(value)) {
        walk(
          v,
          `${path}/${key.replaceAll("~", "~0").replaceAll("/", "~1")}`,
          depth + 1,
        );
      }
    }
  }
  walk(value, "", 0);
}
/** Check scene nesting before JSON serialization, preserving the exact authored node on failure. */
function sceneLimit(
  value: NxCanonicalValue,
  origins: NxRuntimeOrigins,
): NxGraphicsDiagnostic | undefined {
  const pending = [{ value, path: "", depth: 0 }];
  let count = 0;
  let commands = 0;
  while (pending.length > 0) {
    const current = pending.pop()!;
    if (++count > 2000 || current.depth > 48) {
      const origin = origins.entries?.find(
        (entry) => entry.path === current.path,
      );
      return {
        code: "graphics-profile",
        element: `element${current.path}`,
        message: "node or depth limit exceeded",
        provenance: origin,
      };
    }
    if (
      current.value !== null &&
      typeof current.value === "object" &&
      !Array.isArray(current.value)
    ) {
      const points = (
        current.value as Readonly<Record<string, NxCanonicalValue>>
      ).points;
      if (Array.isArray(points)) {
        commands += points.length;
        if (commands > 50000) {
          return {
            code: "graphics-profile",
            element: `element${current.path}`,
            message: "scene path command limit exceeded",
            provenance: origins.entries?.find(
              (entry) => entry.path === current.path,
            ),
          };
        }
      }
      const children = (
        current.value as Readonly<Record<string, NxCanonicalValue>>
      ).children;
      if (Array.isArray(children)) {
        for (let index = children.length - 1; index >= 0; index--) {
          pending.push({
            value: children[index]!,
            path: `${current.path}/children/${index}`,
            depth: current.depth + 1,
          });
        }
      } else if (children !== undefined && children !== null) {
        pending.push({
          value: children,
          path: `${current.path}/children`,
          depth: current.depth + 1,
        });
      }
    }
  }
  return undefined;
}

/** Keep physical surfaces within budget; lowering raster resolution preserves scene semantics. */
function boundedScale(
  requested: number,
  width: number,
  height: number,
  pixels: number,
): number {
  let scale = Math.min(
    Number.isFinite(requested) && requested > 0 ? requested : 1,
    Math.sqrt(pixels / (width * height)),
  );
  while (Math.ceil(width * scale) * Math.ceil(height * scale) > pixels) {
    scale -= 1 / width;
  }
  return scale;
}

/** Prepares the shared evaluation result without compiling or evaluating the document again. */
export async function prepareDrawingValue(
  artifact: NxProgramArtifact,
  source: string,
  value: NxCanonicalValue,
  origins: NxRuntimeOrigins,
  measure: (request: TextRequest) => Promise<Measurement> = measureBrowserText,
  scale = 1,
): Promise<DrawingResult> {
  const diagnostic = (
    code: string,
    message: string,
    module?: string,
    start?: number,
    end?: number,
  ): Diagnostic => ({
    severity: "error",
    code,
    message,
    origin: module === FILE_NAME ? "source" : "program",
    span:
      module === FILE_NAME && start !== undefined && end !== undefined
        ? new SourceIndex(source).span(start, end)
        : null,
  });
  try {
    let config: NxGraphicsConfig = {
      width: 640,
      height: 480,
      scale: boundedScale(scale, 640, 480, 4194304),
      text_ink_padding: 1.5,
      program_revision: "",
      graphics_version: "",
      renderer: BROWSER_RENDERER,
      fonts: [],
      limits: {
        nodes: 2000,
        depth: 48,
        path_commands: 50000,
        coordinate: 1e7,
        surface_pixels: 4194304,
        gradient_stops: 100000,
        layer_pixels: 16777216,
      },
    };
    const request = { value, origins: origins.entries ?? [], config };
    const mapDiagnostic = (d: NxGraphicsDiagnostic) => {
      const site = authoredSites(d.provenance, FILE_NAME)[0];
      return diagnostic(
        d.code,
        `${d.message}${d.element ? ` (${d.element})` : ""}`,
        site?.module,
        site?.start,
        site?.end,
      );
    };
    const limit = sceneLimit(value, origins);
    if (limit !== undefined) {
      return { source, scene: null, diagnostics: [mapDiagnostic(limit)] };
    }
    checkGraphicsValue(value);
    let preflight = artifact.prepareGraphics({
      ...request,
      config,
      preflight: true,
    });
    while (
      config.scale > 1 &&
      preflight.diagnostics.some((d) =>
        /surface limit|concurrent layer area limit/.test(d.message),
      )
    ) {
      config = { ...config, scale: Math.max(1, config.scale * 0.75) };
      preflight = artifact.prepareGraphics({
        ...request,
        config,
        preflight: true,
      });
    }
    if (preflight.diagnostics.length > 0) {
      return {
        source,
        scene: null,
        diagnostics: preflight.diagnostics.map(mapDiagnostic),
      };
    }
    const measurements: Measurement[] = [];
    for (const text of preflight.textRequests ?? []) {
      try {
        measurements.push(await measure(text.request));
      } catch (error) {
        if (error instanceof NxHostCrashedError) {
          throw error;
        }
        return {
          source,
          scene: null,
          diagnostics: [
            mapDiagnostic({
              code: "graphics-browser",
              message: error instanceof Error ? error.message : String(error),
              element: text.element,
              provenance: text.provenance,
            }),
          ],
        };
      }
    }
    const prepared = artifact.prepareGraphics({
      ...request,
      measurements,
      config: {
        ...config,
        fonts: [...new Set(measurements.map((m) => m.run.font))],
      },
    });
    return {
      source,
      scene: prepared.scene,
      diagnostics: [
        ...artifact.diagnostics().map(classifyDiagnostic),
        ...prepared.diagnostics.map(mapDiagnostic),
      ],
    };
  } catch (error) {
    if (error instanceof NxHostCrashedError) {
      throw error;
    }
    if (error instanceof GraphicsValueError) {
      const nodePath = /^(?:\/children(?:\/\d+)?)*/.exec(error.path)?.[0] ?? "";
      const origin = origins.entries?.find((entry) => entry.path === nodePath);
      const site = authoredSites(origin, FILE_NAME)[0];
      return {
        source,
        scene: null,
        diagnostics: [
          diagnostic(
            "graphics-browser",
            `${error.message} (element${nodePath})`,
            site?.module,
            site?.start,
            site?.end,
          ),
        ],
      };
    }
    return {
      source,
      scene: null,
      diagnostics: [
        diagnostic(
          "graphics-browser",
          error instanceof Error ? error.message : String(error),
        ),
      ],
    };
  }
}
