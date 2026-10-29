/** Unstable, backend-neutral graphics preparation contract; mirrors nx-graphics, not NX IR. */
export type NxGraphicsValue =
  | null
  | boolean
  | number
  | string
  | readonly NxGraphicsValue[]
  | { readonly [key: string]: NxGraphicsValue };
export interface NxGraphicsSource {
  readonly module: string;
  readonly start: number;
  readonly end: number;
}
export interface NxGraphicsOrigin extends NxGraphicsSource {
  readonly path: string;
  readonly callSites?: readonly NxGraphicsSource[];
}
export interface NxGraphicsProvenance extends NxGraphicsSource {
  readonly callSites?: readonly NxGraphicsSource[];
}
export interface NxGraphicsPoint {
  readonly x: number;
  readonly y: number;
}
export interface NxGraphicsRect {
  readonly x0: number;
  readonly y0: number;
  readonly x1: number;
  readonly y1: number;
}
export interface NxGraphicsBounds {
  readonly rect: NxGraphicsRect | null;
  readonly conservative: boolean;
}
export type NxGraphicsPathElement =
  | { readonly MoveTo: NxGraphicsPoint }
  | { readonly LineTo: NxGraphicsPoint }
  | { readonly QuadTo: readonly [NxGraphicsPoint, NxGraphicsPoint] }
  | {
      readonly CurveTo: readonly [
        NxGraphicsPoint,
        NxGraphicsPoint,
        NxGraphicsPoint,
      ];
    }
  | "ClosePath";
export type NxGraphicsColor = readonly [number, number, number, number];
export interface NxGraphicsTextRequest {
  readonly text: string;
  readonly family: string;
  readonly size: number;
  readonly weight: number;
  readonly italic: boolean;
}
export interface NxGraphicsTextRun {
  readonly font: string;
  readonly advance: number;
  readonly ink: NxGraphicsBounds;
  readonly ascent: number;
  readonly descent: number;
  readonly baseline: number;
}
export interface NxGraphicsMeasurement {
  readonly request: NxGraphicsTextRequest;
  readonly run: NxGraphicsTextRun;
}
export interface NxGraphicsConfig {
  readonly width: number;
  readonly height: number;
  readonly scale: number;
  /** Rasterization uncertainty in physical pixels (default zero). */
  readonly text_ink_padding?: number;
  /** Replaced by the artifact's actual linked versions during preparation. */
  readonly program_revision: string;
  readonly graphics_version: string;
  readonly renderer: string;
  readonly fonts: readonly string[];
  readonly limits: {
    readonly nodes: number;
    readonly depth: number;
    readonly path_commands: number;
    readonly coordinate: number;
    readonly surface_pixels: number;
    readonly layer_pixels: number;
  };
}
export interface NxGraphicsNode {
  readonly id: string;
  readonly parent: string | null;
  readonly order: number;
  readonly identity: {
    readonly module: string;
    readonly version: string;
    readonly declaration: string;
  };
  readonly provenance: NxGraphicsProvenance | null;
  readonly accessibility: {
    readonly label?: string;
    readonly description?: string;
  } | null;
  readonly transform: readonly number[];
  readonly opacity: number;
  readonly clip: readonly NxGraphicsPathElement[] | null;
  readonly clip_evenodd: boolean;
  readonly path: readonly NxGraphicsPathElement[] | null;
  readonly evenodd: boolean;
  readonly fill: NxGraphicsColor | null;
  readonly stroke: {
    readonly color: NxGraphicsColor;
    readonly width: number;
    readonly cap: "butt" | "round" | "square";
    readonly join: "miter" | "round" | "bevel";
    readonly miter: number;
    readonly dashes: readonly number[];
    readonly offset: number;
  } | null;
  readonly text: {
    readonly request: NxGraphicsTextRequest;
    readonly run: NxGraphicsTextRun;
    readonly x: number;
    readonly y: number;
  } | null;
  readonly geometry: NxGraphicsBounds;
  readonly painted: NxGraphicsBounds;
  readonly visible: NxGraphicsBounds;
  readonly children: readonly NxGraphicsNode[];
}
export interface NxGraphicsScene {
  readonly revision: string;
  readonly profile: string;
  readonly config: NxGraphicsConfig;
  readonly root: NxGraphicsNode;
}
export interface NxGraphicsDiagnostic {
  readonly code: string;
  readonly message: string;
  readonly element: string;
  readonly provenance?: NxGraphicsProvenance | null;
}
export interface NxGraphicsPreparationRequest {
  readonly value: NxGraphicsValue;
  readonly origins: readonly NxGraphicsOrigin[];
  readonly measurements?: readonly NxGraphicsMeasurement[];
  readonly config: NxGraphicsConfig;
  /** Validate identities, geometry and limits, collecting text requests without measuring or publishing a scene. */
  readonly preflight?: boolean;
}
export interface NxGraphicsTextPreparation {
  readonly request: NxGraphicsTextRequest;
  /** First requesting element's scene-local ID and construction/call provenance. */
  readonly element: string;
  readonly provenance: NxGraphicsProvenance | null;
}
export interface NxGraphicsPreparation {
  readonly scene: NxGraphicsScene | null;
  readonly diagnostics: readonly NxGraphicsDiagnostic[];
  /** Present only on a successful preflight; requests come from validated graphics Text declarations. */
  readonly textRequests?: readonly NxGraphicsTextPreparation[];
}
