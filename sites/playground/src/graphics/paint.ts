import type { NxGraphicsPaint } from "@nx-lang/sdk-wasm";
import { color } from "./paths.ts";

export function visiblePaint(paint: NxGraphicsPaint): boolean {
  return paint.kind === "color"
    ? paint.color[3] > 0
    : paint.stops.some((s) => s.color[3] > 0);
}

/** Canvas interpolates unpremultiplied colors. Subdivide alpha transitions to bound
 * premultiplied channel error by 1/256, without allocating raster layers or stop arrays.
 * Origin and scale compensate for internal typography transforms, preserving local paint units. */
export function canvasPaint(
  ctx: CanvasRenderingContext2D,
  paint: NxGraphicsPaint,
  originX = 0,
  originY = 0,
  scale = 1,
): string | CanvasGradient {
  if (paint.kind === "color") {
    return color(paint.color);
  }
  const x = (value: number) => (value - originX) / scale;
  const y = (value: number) => (value - originY) / scale;
  const gradient = paint.kind === "linearGradient"
    ? ctx.createLinearGradient(x(paint.x1), y(paint.y1), x(paint.x2), y(paint.y2))
    : ctx.createRadialGradient(
        x(paint.cx), y(paint.cy), 0,
        x(paint.cx), y(paint.cy), paint.radius / scale,
      );
  const stops = paint.stops;
  for (let i = 0; i < stops.length; i++) {
    const stop = stops[i]!;
    const next = stops[i + 1];
    // At zero alpha the limiting RGB is the adjacent visible endpoint's RGB.
    let start = stop.color;
    if (next && stop.offset < next.offset && start[3] === 0 && next.color[3] > 0) {
      start = [next.color[0], next.color[1], next.color[2], 0];
    }
    gradient.addColorStop(stop.offset, color(start));
    if (!next || next.offset === stop.offset || next.color[3] === stop.color[3]) {
      continue;
    }
    const end = next.color[3] === 0
      ? [stop.color[0], stop.color[1], stop.color[2], 0]
      : next.color;
    if ([0, 1, 2].every((k) => start[k] === end[k])) {
      // Equal effective RGB makes ordinary alpha interpolation exact.
      gradient.addColorStop(next.offset, color([end[0]!, end[1]!, end[2]!, end[3]!]));
      continue;
    }
    const count = 64;
    for (let step = 1; step < count; step++) {
      const t = step / count;
      const alpha = stop.color[3] * (1 - t) + next.color[3] * t;
      const rgb = [0, 1, 2].map((k) => alpha === 0 ? 0 :
        (stop.color[k]! * stop.color[3] * (1 - t) +
          next.color[k]! * next.color[3] * t) / alpha);
      gradient.addColorStop(
        stop.offset + (next.offset - stop.offset) * t,
        color([rgb[0]!, rgb[1]!, rgb[2]!, alpha]),
      );
    }
    if (next.color[3] === 0 && stop.color[3] > 0) {
      gradient.addColorStop(
        next.offset,
        color([stop.color[0], stop.color[1], stop.color[2], 0]),
      );
    }
  }
  return gradient;
}
