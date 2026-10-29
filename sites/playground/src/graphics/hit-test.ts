import { pathData } from "./paths.ts";
import type { GraphicsNode, GraphicsScene } from "./protocol.ts";

/** Traverse in reverse paint order, testing actual fill/stroke geometry and every ancestor clip. */
export function hitTest(
  scene: GraphicsScene,
  x: number,
  y: number,
): GraphicsNode | undefined {
  const context = new OffscreenCanvas(1, 1).getContext("2d");
  if (context === null) {
    return undefined;
  }
  function visit(node: GraphicsNode): GraphicsNode | undefined {
    const rect = node.visible.rect;
    if (
      node.opacity === 0 ||
      rect === null ||
      x < rect.x0 ||
      x > rect.x1 ||
      y < rect.y0 ||
      y > rect.y1
    ) {
      return undefined;
    }
    context!.save();
    try {
      context!.transform(
        ...(node.transform as [number, number, number, number, number, number]),
      );
      if (
        node.clip !== null &&
        !context!.isPointInPath(
          new Path2D(pathData(node.clip)),
          x,
          y,
          node.clip_evenodd ? "evenodd" : "nonzero",
        )
      ) {
        return undefined;
      }
      for (let index = node.children.length - 1; index >= 0; index--) {
        const hit = visit(node.children[index]!);
        if (hit !== undefined) {
          return hit;
        }
      }
      if (node.path !== null) {
        const path = new Path2D(pathData(node.path));
        if (
          node.fill !== null &&
          node.fill[3] > 0 &&
          context!.isPointInPath(
            path,
            x,
            y,
            node.evenodd ? "evenodd" : "nonzero",
          )
        ) {
          return node;
        }
        const stroke = node.stroke;
        if (stroke !== null && stroke.width > 0 && stroke.color[3] > 0) {
          context!.lineWidth = stroke.width;
          context!.lineCap = stroke.cap;
          context!.lineJoin = stroke.join;
          context!.miterLimit = stroke.miter;
          context!.setLineDash([...stroke.dashes]);
          context!.lineDashOffset = stroke.offset;
          if (context!.isPointInStroke(path, x, y)) {
            return node;
          }
        }
      }
      // Text has no portable glyph-path API in Canvas; use its conservative prepared ink bounds.
      return node.text !== null && node.fill !== null && node.fill[3] > 0
        ? node
        : undefined;
    } finally {
      context!.restore();
    }
  }
  return visit(scene.root);
}
