import { useEffect, useRef, useState } from "react";
import type { Compile } from "../compile";
import { Canvas } from "../drawnui/react/index";
import { EXAMPLES, coverageNote, type Example } from "../examples";
import { useNxDrawing } from "../render/useNxDrawing";

/** The chip that says how completely an example covers its original, or nothing when it is complete. */
export function CoverageChip({ example }: { example: Example }) {
  const note = coverageNote(example);
  if (note === null) {
    return null;
  }
  return <span className={`chip ${example.coverage}`}>{note}</span>;
}

/** How far outside the viewport a card may be and still have its preview mounted. */
const PREVIEW_MARGIN = "300px 0px";

/**
 * One card's drawing, mounted only while the card is near the viewport.
 *
 * Every preview is a WebGL canvas: `RenderingMode="Default"` asks for software rendering, but the
 * engine creates its surface in the constructor, before the wrapper assigns the prop, so the
 * request never lands (FINDINGS.md F24). A browser keeps only about sixteen WebGL contexts alive,
 * and with twenty cards the oldest went blank. Mounting the canvas only near the viewport keeps a
 * handful alive at a time; unmounting disposes the view, which releases its context.
 */
function Preview({ example, compile }: { example: Example; compile: Compile }) {
  const drawing = useNxDrawing(example.source, compile, 0);
  const host = useRef<HTMLDivElement>(null);
  const [near, setNear] = useState(false);
  useEffect(() => {
    const element = host.current;
    if (element === null) {
      return;
    }
    const observer = new IntersectionObserver(
      (entries) => setNear(entries.some((entry) => entry.isIntersecting)),
      { rootMargin: PREVIEW_MARGIN },
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  return (
    <div className="preview" ref={host}>
      {near && (
        <Canvas BackgroundColor="#212529" RenderingMode="Default" Gestures="Disabled">
          {drawing.node}
        </Canvas>
      )}
      {(drawing.node === null || !near) && (
        <div className="preview-status">
          {drawing.compiling
            ? "Drawing…"
            : (drawing.failure ?? drawing.diagnostics[0]?.message ?? (near ? "Did not draw" : ""))}
        </div>
      )}
    </div>
  );
}

export interface GalleryProps {
  readonly compile: Compile;
  readonly onOpen: (example: Example) => void;
}

/**
 * The gallery: the front of the playground, listing the DrawnUI demo site's examples drawn from NX.
 *
 * The heading names the site and the subtitle names what it draws with today and links to the
 * projects the cards are ported from, so that when a second target joins, the subtitle and the
 * cards change and the identity does not.
 *
 * Every card draws through the app's own pipeline rather than rendering the vendored TSX page, so a
 * gap in the catalog or the renderer shows up here as a broken card instead of hiding behind a
 * picture that was always going to look right.
 */
export function Gallery({ compile, onOpen }: GalleryProps) {
  return (
    <div className="gallery">
      <header className="gallery-head">
        <h1>NX Playground</h1>
        <p className="subtitle">
          Drawing with <a href="https://drawnui.net">DrawnUI</a> today, following its{" "}
          <a href="https://helloreact.drawnui.net">React demo site</a>.
        </p>
        <p>
          Try NX in the browser. Every card below is one of the DrawnUI demo pages ported to NX,
          compiled to NX IR and drawn live by DrawnUI React. Edit any of them and the drawing updates
          as you type.
        </p>
      </header>
      <div className="cards">
        {EXAMPLES.map((example) => (
          <article className="card" key={example.id}>
            <div className="card-head">
              <h2>{example.name}</h2>
              <span className="spacer" />
              <button className="link" type="button" onClick={() => onOpen(example)}>
                Edit →
              </button>
            </div>
            <p className="blurb">{example.blurb}</p>
            {/* Below the blurb rather than beside the name: a reduced example's note names what the
                original demonstrates, which is a sentence, not a word. */}
            <CoverageChip example={example} />
            <Preview example={example} compile={compile} />
          </article>
        ))}
      </div>
    </div>
  );
}
