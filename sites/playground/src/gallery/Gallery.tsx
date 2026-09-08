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

function Preview({ example, compile }: { example: Example; compile: Compile }) {
  const drawing = useNxDrawing(example.source, compile, 0);
  return (
    <div className="preview">
      {/* Software rendering: a dozen previews on one page would exhaust the browser's WebGL contexts. */}
      <Canvas BackgroundColor="#212529" RenderingMode="Default" Gestures="Disabled">
        {drawing.node}
      </Canvas>
      {drawing.node === null && (
        <div className="preview-status">
          {drawing.compiling
            ? "Drawing…"
            : (drawing.failure ?? drawing.diagnostics[0]?.message ?? "Did not draw")}
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
 * The heading names the site and the subtitle names what it draws with today, so that when a
 * second target joins, the subtitle and the cards change and the identity does not.
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
        <p className="subtitle">Drawing with DrawnUI today.</p>
        <p>
          Try NX in the browser. Every card below is one of the DrawnUI demo pages ported to NX,
          compiled to NX IR and drawn live by DrawnUI. Edit any of them and watch the drawing follow.
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
