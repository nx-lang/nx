/**
 * Builds every NX logo file from the numbers below, then copies the ones the website and the
 * VS Code extension use into place:
 *
 *   pnpm install
 *   pnpm --filter @nx-lang/branding generate
 *
 * The SVGs are drawn from polygons, so they need no fonts and look the same everywhere. PNGs and
 * the social card are rendered with resvg; the social card's text is set in Geist.
 *
 * To change the logo, change the numbers here and run the script again. Never edit the generated
 * files by hand: the next run overwrites them.
 */
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { Resvg } from "@resvg/resvg-js";

const brandingRoot = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = join(brandingRoot, "..");

/**
 * The shape, in design units. The N and both brackets share one 84-unit cap height (y 8 to 92);
 * everything else is measured against it.
 */
const geometry = {
  stroke: 13, // Stroke width of the N's stems and of each bracket arm.
  gap: 4, // Space between the tips of the two brackets.
  top: 8,
  bottom: 92,
  middle: 50,
  margin: 4, // Space around the drawing inside the design grid (cropped away in the exports).
  nWidth: 66, // Outside width of the N.
  nToBracket: 16, // Space between the N and the closing bracket.
  bracketWidth: 42, // Outside width of each bracket, from the flat ends to the tip.
  iconFill: 0.62, // Share of the icon tile's width the brackets take up.
  iconRadius: 0.22, // Icon tile corner radius, as a share of the tile's width.
  // Share of the avatar's width the brackets take up. Larger than the icon's, since avatars are
  // shown small and rounded by the site; above 0.72 a circular crop cuts off the X's corners.
  avatarFill: 0.7
};

const colors = {
  ink: "#16171C",
  paper: "#F5F4EF",
  white: "#FFFFFF",
  black: "#000000",
  blue: "#1F4FE0", // Closing bracket `>`, on light backgrounds.
  gold: "#C98500", // Opening bracket `<`, on light backgrounds.
  blueOnDark: "#4D78FF",
  goldOnDark: "#FFB020"
};

const round = (value) => Number(value.toFixed(2));
const points = (list) => list.map(([x, y]) => `${round(x)},${round(y)}`).join(" ");

/**
 * Solves for the horizontal thickness of a slanted stroke: a stroke `stroke` wide that crosses
 * `rise` units vertically while its centerline covers `span - thickness` units horizontally.
 */
function slantedThickness(span, rise) {
  let thickness = geometry.stroke;
  for (let step = 0; step < 32; step++) {
    thickness = (geometry.stroke * Math.hypot(span - thickness, rise)) / rise;
  }
  return thickness;
}

/** The logo's polygons, with or without the N, plus the horizontal extent they cover. */
function shapes(withN) {
  const { top, bottom, middle, margin, nWidth, nToBracket, bracketWidth, gap, stroke } = geometry;
  const nLeft = margin;
  const nRight = margin + nWidth;
  const closeLeft = withN ? nRight + nToBracket : margin;
  const closeTip = closeLeft + bracketWidth;
  const openTip = closeTip + gap;
  const openRight = openTip + bracketWidth;

  const result = { left: withN ? nLeft : closeLeft, right: openRight, n: [] };

  if (withN) {
    const diagonal = slantedThickness(nWidth, bottom - top);
    result.n = [
      points([[nLeft, top], [nLeft + stroke, top], [nLeft + stroke, bottom], [nLeft, bottom]]),
      points([[nRight - stroke, top], [nRight, top], [nRight, bottom], [nRight - stroke, bottom]]),
      points([[nLeft, top], [nLeft + diagonal, top], [nRight, bottom], [nRight - diagonal, bottom]])
    ];
  }

  // Each bracket is two arms with flat, horizontal ends meeting in a mitered tip.
  const arm = slantedThickness(bracketWidth, middle - top);
  result.close = points([
    [closeLeft, top], [closeLeft + arm, top], [closeTip, middle],
    [closeLeft + arm, bottom], [closeLeft, bottom], [closeTip - arm, middle]
  ]);
  result.open = points([
    [openRight, top], [openRight - arm, top], [openTip, middle],
    [openRight - arm, bottom], [openRight, bottom], [openTip + arm, middle]
  ]);
  return result;
}

function svg(viewBox, body, width, height) {
  const size = width ? ` width="${width}" height="${height}"` : "";
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${viewBox}"${size} role="img" aria-label="NX">` +
    `<title>NX</title>${body}</svg>\n`
  );
}

/** The full logo, cropped tight to the drawing. */
function logo({ n, close, open }) {
  const s = shapes(true);
  const body =
    s.n.map((p) => `<polygon points="${p}" fill="${n}"/>`).join("") +
    `<polygon points="${s.close}" fill="${close}"/><polygon points="${s.open}" fill="${open}"/>`;
  const height = geometry.bottom - geometry.top;
  return svg(`${round(s.left)} ${geometry.top} ${round(s.right - s.left)} ${height}`, body);
}

/**
 * The brackets alone: cropped tight, or centered on a square tile. The tile defaults to the app
 * icon's fill and corner radius.
 */
function mark({ close, open }, tile, { fill = geometry.iconFill, radius = geometry.iconRadius } = {}) {
  const s = shapes(false);
  const width = s.right - s.left;
  const height = geometry.bottom - geometry.top;
  const brackets =
    `<polygon points="${s.close}" fill="${close}"/><polygon points="${s.open}" fill="${open}"/>`;
  if (!tile) {
    return svg(`${round(s.left)} ${geometry.top} ${round(width)} ${height}`, brackets);
  }
  const side = round(Math.max(width, height) / fill);
  const dx = round((side - width) / 2 - s.left);
  const dy = round((side - height) / 2 - geometry.top);
  const body =
    `<rect width="${side}" height="${side}" rx="${round(side * radius)}" fill="${tile}"/>` +
    `<g transform="translate(${dx} ${dy})">${brackets}</g>`;
  return svg(`0 0 ${side} ${side}`, body);
}

/** The tagline, as it appears with the logo. See "Tagline" in README.md for how to set it. */
const tagline = {
  line1: "A modern language for executable markup.",
  line2: "Built for dynamic, AI-native experiences."
};

/**
 * The card shown when a link is shared: the logo, then the tagline in the dark-theme style (line 2
 * all gold), then the site's address. The website's card is 1200×630; GitHub's social preview is
 * 1280×640. Both keep the content at the same place and only the ink background grows.
 */
function socialCard(width, height) {
  const s = shapes(true);
  const logoHeight = 120;
  const scale = logoHeight / (geometry.bottom - geometry.top);
  const x = 96 - s.left * scale;
  const y = 96 - geometry.top * scale;
  const drawing =
    s.n.map((p) => `<polygon points="${p}" fill="${colors.paper}"/>`).join("") +
    `<polygon points="${s.close}" fill="${colors.blueOnDark}"/>` +
    `<polygon points="${s.open}" fill="${colors.goldOnDark}"/>`;
  const body =
    `<rect width="${width}" height="${height}" fill="${colors.ink}"/>` +
    `<g transform="translate(${round(x)} ${round(y)}) scale(${round(scale * 1000) / 1000})">${drawing}</g>` +
    `<text x="96" y="318" font-family="Geist" font-weight="600" font-size="40" letter-spacing="-0.4" ` +
    `fill="${colors.paper}">${tagline.line1}</text>` +
    `<text x="96" y="376" font-family="Geist" font-weight="400" font-size="30" ` +
    `fill="${colors.goldOnDark}">${tagline.line2}</text>` +
    `<text x="96" y="532" font-family="Geist Mono" font-size="22" fill="#8C8D95">nxlang.org</text>`;
  return svg(`0 0 ${width} ${height}`, body, width, height);
}

// Geist, by Vercel, under the SIL Open Font License (fonts/OFL.txt).
const fontsRoot = join(brandingRoot, "fonts");
const fontFiles = [
  join(fontsRoot, "Geist-Regular.ttf"),
  join(fontsRoot, "Geist-SemiBold.ttf"),
  join(fontsRoot, "GeistMono-Medium.ttf")
];

function png(svgText, width) {
  const renderer = new Resvg(svgText, {
    fitTo: { mode: "width", value: width },
    font: { fontFiles, loadSystemFonts: false, defaultFontFamily: "Geist" }
  });
  return renderer.render().asPng();
}

function write(relativePath, contents) {
  const path = join(repositoryRoot, relativePath);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, contents);
  console.log(`  ${relativePath}`);
}

const light = { n: colors.ink, close: colors.blue, open: colors.gold };
const dark = { n: colors.paper, close: colors.blueOnDark, open: colors.goldOnDark };

const files = {
  "nx-logo.svg": logo(light),
  "nx-logo-on-dark.svg": logo(dark),
  "nx-logo-mono-black.svg": logo({ n: colors.black, close: colors.black, open: colors.black }),
  "nx-logo-mono-white.svg": logo({ n: colors.white, close: colors.white, open: colors.white }),
  "nx-mark.svg": mark(light),
  "nx-mark-on-dark.svg": mark(dark),
  "nx-icon.svg": mark(dark, colors.ink),
  "nx-icon-light.svg": mark(light, colors.white),
  // Square corners, for places that round or crop the image themselves, such as GitHub avatars.
  "nx-avatar.svg": mark(dark, colors.ink, { fill: geometry.avatarFill, radius: 0 })
};

console.log("branding/");
for (const [name, text] of Object.entries(files)) {
  write(join("branding", name), text);
}
for (const size of [512, 256, 180, 128, 32, 16]) {
  write(join("branding", "png", `nx-icon-${size}.png`), png(files["nx-icon.svg"], size));
}
write(join("branding", "png", "nx-logo-1200.png"), png(files["nx-logo.svg"], 1200));
write(join("branding", "png", "nx-logo-on-dark-1200.png"), png(files["nx-logo-on-dark.svg"], 1200));
write(join("branding", "png", "nx-avatar-512.png"), png(files["nx-avatar.svg"], 512));
const card = png(socialCard(1200, 630), 1200);
write(join("branding", "png", "nx-social-card.png"), card);
write(join("branding", "png", "nx-github-social-preview.png"), png(socialCard(1280, 640), 1280));

// The copies other parts of the repository serve. They can't reference branding/ directly: the
// website serves its own public/ folder and the VS Code extension packages its own images/.
console.log("copies:");
write("sites/website/src/assets/logo-light.svg", files["nx-logo.svg"]);
write("sites/website/src/assets/logo-dark.svg", files["nx-logo-on-dark.svg"]);
write("sites/website/public/favicon.svg", files["nx-icon.svg"]);
write("sites/website/public/og.png", card);
write("src/vscode/images/icon.png", readFileSync(join(brandingRoot, "png", "nx-icon-128.png")));
