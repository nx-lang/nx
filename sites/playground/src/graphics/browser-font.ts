import { glyphCoverage } from "./cmap.ts";
import type { Measurement, TextRequest } from "./protocol.ts";
export const TEXT_REFERENCE_SIZE = 1000;
export const FONT_FAMILY = "NXGraphicsFixtureV1";
export const FONT_URL = new URL(
  "../../../../crates/nx-graphics-skia/fixtures/fonts/DejaVuSans.ttf",
  import.meta.url,
).href;
export const FONT_LICENSE_URL = new URL(
  "../../../../crates/nx-graphics-skia/fixtures/fonts/LICENSE.txt?no-inline",
  import.meta.url,
).href;
let measurementContext: OffscreenCanvasRenderingContext2D | null | undefined;
let loaded:
  | Promise<{
      identity: string;
      covers: (code: number) => boolean;
    }>
  | undefined;
/** Each realm loads the same bundled face explicitly before measuring or displaying text. */
export function loadGraphicsFont() {
  return (loaded ??= (async () => {
    const response = await fetch(FONT_URL);
    if (!response.ok) {
      throw new Error("The bundled drawing font could not be loaded.");
    }
    const bytes = new Uint8Array(await response.arrayBuffer());
    const covers = glyphCoverage(bytes);
    const face = await new FontFace(FONT_FAMILY, bytes.slice().buffer, {
      weight: "400",
      style: "normal",
    }).load();
    const realm = globalThis as unknown as {
      document?: Document;
      fonts?: FontFaceSet;
    };
    const fonts = realm.document?.fonts ?? realm.fonts;
    if (fonts === undefined) {
      throw new Error(
        "This browser does not support supplied fonts in the drawing worker.",
      );
    }
    fonts.add(face);
    const hash = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
    return {
      identity: `sha256:${Array.from(hash, (b) => b.toString(16).padStart(2, "0")).join("")}`,
      covers,
    };
  })().catch((error) => {
    loaded = undefined;
    throw error;
  }));
}
export async function measureBrowserText(
  request: TextRequest,
): Promise<Measurement> {
  if (
    request.family !== "Fixture" ||
    request.weight !== 400 ||
    request.italic
  ) {
    throw new Error(
      "The browser preview supplies Fixture at weight 400, normal style. Select that font for this drawing.",
    );
  }
  if (
    !Number.isFinite(request.size) ||
    request.size <= 0 ||
    request.size > 1e6 ||
    request.text.length > 100000 ||
    /[\n\r\u2028\u2029]/u.test(request.text)
  ) {
    throw new Error(
      "Drawing text must be a bounded single line with a positive finite font size.",
    );
  }
  const font = await loadGraphicsFont();
  for (const ch of request.text) {
    const code = ch.codePointAt(0)!;
    if (!font.covers(code) && ![0x200c, 0x200d, 0xfe0f].includes(code)) {
      throw new Error(
        `Fixture is missing glyph U+${code.toString(16).toUpperCase().padStart(4, "0")}.`,
      );
    }
  }
  const ctx = (measurementContext ??= new OffscreenCanvas(1, 1).getContext(
    "2d",
  ));
  if (ctx === null) {
    throw new Error("This browser cannot measure drawing text.");
  }
  configureText(ctx, request);
  const m = ctx.measureText(request.text);
  const ratio = request.size / TEXT_REFERENCE_SIZE;
  const rect =
    request.text.length === 0 ||
    (m.actualBoundingBoxLeft + m.actualBoundingBoxRight === 0 &&
      m.actualBoundingBoxAscent + m.actualBoundingBoxDescent === 0)
      ? null
      : {
          x0: (-Math.ceil(m.actualBoundingBoxLeft) - 1) * ratio,
          y0: (-Math.ceil(m.actualBoundingBoxAscent) - 1) * ratio,
          x1: (Math.ceil(m.actualBoundingBoxRight) + 1) * ratio,
          y1: (Math.ceil(m.actualBoundingBoxDescent) + 1) * ratio,
        };
  const run = {
    font: font.identity,
    advance: m.width * ratio,
    ink: { rect, conservative: true },
    ascent: (m.fontBoundingBoxAscent ?? m.actualBoundingBoxAscent) * ratio,
    descent: (m.fontBoundingBoxDescent ?? m.actualBoundingBoxDescent) * ratio,
    baseline: 0,
  };
  for (const value of [
    run.advance,
    run.ascent,
    run.descent,
    ...(rect === null ? [] : Object.values(rect)),
  ]) {
    if (!Number.isFinite(value)) {
      throw new Error("The browser returned non-finite text metrics.");
    }
  }
  return { request, run };
}
/** Identical settings in the worker's measurement context and the page's painting context. */
export function configureText(
  ctx: CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D,
  request: TextRequest,
): void {
  ctx.font = `400 ${TEXT_REFERENCE_SIZE}px ${FONT_FAMILY}`;
  if (!ctx.font.includes(FONT_FAMILY)) {
    throw new Error("The browser rejected the requested drawing font size.");
  }
  ctx.textBaseline = "alphabetic";
  ctx.textAlign = "start";
  ctx.direction = "ltr";
  ctx.fontKerning = "normal";
  ctx.letterSpacing = "0px";
  ctx.wordSpacing = "0px";
}
