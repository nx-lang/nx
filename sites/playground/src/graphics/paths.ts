import type { Color, GraphicsNode, PathElement } from "./protocol.ts";
export function pathData(path: readonly PathElement[]): string {
  const point = (p: { x: number; y: number }) => `${p.x} ${p.y}`;
  return path
    .map((e) =>
      typeof e === "string"
        ? "Z"
        : "MoveTo" in e
          ? `M${point(e.MoveTo)}`
          : "LineTo" in e
            ? `L${point(e.LineTo)}`
            : "QuadTo" in e
              ? `Q${e.QuadTo.map(point).join(" ")}`
              : `C${e.CurveTo.map(point).join(" ")}`,
    )
    .join(" ");
}
export function color(c: Color | null): string {
  return c === null
    ? "none"
    : `rgba(${c[0] * 255},${c[1] * 255},${c[2] * 255},${c[3]})`;
}
export function flattenNodes(node: GraphicsNode): GraphicsNode[] {
  return [node, ...node.children.flatMap(flattenNodes)];
}
