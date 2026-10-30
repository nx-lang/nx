import { canvasPaint } from "./paint.ts";
import { configureText, TEXT_REFERENCE_SIZE } from "./browser-font.ts";
import { flattenNodes, pathData } from "./paths.ts";
import type { GraphicsNode, GraphicsScene } from "./protocol.ts";
/** Paint only prepared data. The caller publishes the completed frame, never a partial canvas. */
export function renderCanvas(
  scene: GraphicsScene,
  selected: readonly string[] = [],
): HTMLCanvasElement {
  if (scene.profile !== "graphics-core-v3") {
    throw new Error("Unsupported graphics profile.");
  }
  const frame = document.createElement("canvas");
  frame.width = Math.ceil(scene.config.width * scene.config.scale);
  frame.height = Math.ceil(scene.config.height * scene.config.scale);
  const context = frame.getContext("2d");
  if (context === null) {
    throw new Error("This browser does not support Canvas 2D drawing.");
  }
  function contents(ctx: CanvasRenderingContext2D, node: GraphicsNode): void {
    if (node.path !== null) {
      const path = new Path2D(pathData(node.path));
      if (node.fill !== null) {
        ctx.fillStyle = canvasPaint(ctx, node.fill);
        ctx.fill(path, node.evenodd ? "evenodd" : "nonzero");
      }
      if (node.stroke !== null && node.stroke.width > 0) {
        const stroke = node.stroke;
        ctx.strokeStyle = canvasPaint(ctx, stroke.paint);
        ctx.lineWidth = stroke.width;
        ctx.lineCap = stroke.cap;
        ctx.lineJoin = stroke.join;
        ctx.miterLimit = stroke.miter;
        ctx.setLineDash([...stroke.dashes]);
        ctx.lineDashOffset = stroke.offset;
        ctx.stroke(path);
      }
    }
    if (node.text !== null && node.fill !== null) {
      const text = node.text;
      configureText(ctx, text.request);
      const ratio = text.request.size / TEXT_REFERENCE_SIZE;
      const advance = ctx.measureText(text.request.text).width * ratio;
      if (Math.abs(advance - text.run.advance) > 1e-6 * Math.max(1, advance)) {
        throw new Error(
          "Drawing font metrics changed between preparation and painting. Retry drawing.",
        );
      }
      ctx.save();
      ctx.translate(text.x, text.y);
      ctx.scale(ratio, ratio);
      // Paint coordinates remain element-local despite reference-font normalization.
      ctx.fillStyle = canvasPaint(ctx, node.fill, text.x, text.y, ratio);
      ctx.fillText(text.request.text, 0, 0);
      ctx.restore();
    }
    for (const child of node.children) {
      paint(ctx, child);
    }
  }
  function paint(ctx: CanvasRenderingContext2D, node: GraphicsNode): void {
    if (node.opacity === 0) {
      return;
    }
    ctx.save();
    try {
      ctx.transform(
        ...(node.transform as [number, number, number, number, number, number]),
      );
      if (node.clip !== null) {
        ctx.clip(
          new Path2D(pathData(node.clip)),
          node.clip_evenodd ? "evenodd" : "nonzero",
        );
      }
      if (node.opacity === 1) {
        contents(ctx, node);
        return;
      }
      // Canvas globalAlpha alone would apply opacity independently to overlapping children.
      // Paint the whole subtree first, then composite once through the parent's/local clip.
      const layer = document.createElement("canvas");
      layer.width = frame.width;
      layer.height = frame.height;
      try {
        const isolated = layer.getContext("2d");
        if (isolated === null) {
          throw new Error(
            "The browser could not allocate a drawing opacity layer.",
          );
        }
        isolated.setTransform(ctx.getTransform());
        contents(isolated, node);
        ctx.resetTransform();
        ctx.globalAlpha = node.opacity;
        ctx.drawImage(layer, 0, 0);
      } finally {
        layer.width = layer.height = 0;
      }
    } finally {
      ctx.restore();
    }
  }
  try {
    context.scale(scene.config.scale, scene.config.scale);
    paint(context, scene.root);
    const selectedIds = new Set(selected);
    context.strokeStyle = "#b45309";
    context.lineWidth = 1;
    context.setLineDash([4, 2]);
    for (const node of flattenNodes(scene.root)) {
      const visible = node.visible.rect;
      if (selectedIds.has(node.id) && visible != null) {
        context.strokeRect(
          visible.x0,
          visible.y0,
          visible.x1 - visible.x0,
          visible.y1 - visible.y0,
        );
      }
    }
    return frame;
  } catch (error) {
    frame.width = frame.height = 0;
    throw error;
  }
}
