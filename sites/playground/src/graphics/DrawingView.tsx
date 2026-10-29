import { useEffect, useMemo, useRef, useState } from "react";
import { FONT_LICENSE_URL, loadGraphicsFont } from "./browser-font.ts";
import { hitTest } from "./hit-test.ts";
import { renderCanvas } from "./canvas.ts";
import { flattenNodes } from "./paths.ts";
import {
  authoredSite,
  selectedNodes,
  type DrawingSelection,
} from "./selection.ts";
import type { GraphicsNode, GraphicsScene, SourceSite } from "./protocol.ts";
import type { DrawingResult } from "./protocol.ts";
export function DrawingView({
  drawing,
  source,
  sourceSelection,
  onNavigate,
}: {
  drawing: DrawingResult & {
    readonly pending: boolean;
    readonly failure?: string | null;
    retry(): void;
  };
  source: string | null;
  sourceSelection: DrawingSelection | null;
  onNavigate: (site: SourceSite) => void;
}) {
  const [selected, setSelected] = useState<{
    revision: string;
    id: string;
  } | null>(null);
  const errors = drawing.diagnostics.filter((d) => d.severity === "error");
  const current = drawing.source === source;
  const scene =
    current && !drawing.pending && !drawing.failure ? drawing.scene : null;
  const nodes = useMemo(
    () => (scene === null ? [] : flattenNodes(scene.root)),
    [scene],
  );
  const selectedIds = useMemo(
    () =>
      selectedNodes(
        nodes,
        sourceSelection?.source === source ? sourceSelection : null,
      ),
    [
      nodes,
      source,
      sourceSelection?.source,
      sourceSelection?.start,
      sourceSelection?.end,
    ],
  );
  const selectedId =
    selected !== null &&
    selected.revision === scene?.revision &&
    selectedIds.includes(selected.id)
      ? selected.id
      : selectedIds[0];
  const chosen = nodes.find((n) => n.id === selectedId) ?? scene?.root;
  function select(node: GraphicsNode) {
    if (scene === null) {
      return;
    }
    setSelected({ revision: scene.revision, id: node.id });
    const site = authoredSite(node);
    if (site !== undefined) {
      onNavigate(site);
    }
  }
  return (
    <div className="drawing-view">
      {!drawing.failure && (!current || drawing.pending) && (
        <p className="output-message quiet" role="status">
          Preparing drawing…
        </p>
      )}
      {(drawing.failure || (current && errors.length > 0)) && (
        <div className="output-message error" role="alert">
          <p className="output-heading">Drawing could not be rendered</p>
          {drawing.failure && <p>{drawing.failure}</p>}
          <ul>
            {errors.map((d, i) => (
              <li key={i}>{d.message}</li>
            ))}
          </ul>
          <button type="button" onClick={drawing.retry}>
            Retry drawing
          </button>
        </div>
      )}
      {scene !== null && (
        <>
          <div className="drawing-surface">
            <SceneCanvas
              key={scene.revision}
              scene={scene}
              selected={selectedIds}
              select={select}
              retry={drawing.retry}
            />
          </div>
          <div className="drawing-info">
            <label>
              Inspect element{" "}
              <select
                aria-label="Drawing element"
                value={chosen?.id ?? ""}
                onChange={(event) => {
                  const node = nodes.find((n) => n.id === event.target.value);
                  if (node) {
                    select(node);
                  }
                }}
              >
                {nodes.map((n) => (
                  <option value={n.id} key={n.id}>
                    {n.identity.declaration} · {n.id}
                  </option>
                ))}
              </select>
            </label>
            <details>
              <summary>Bounds and text metrics</summary>
              <pre>
                {chosen === undefined
                  ? ""
                  : JSON.stringify(
                      {
                        identity: chosen.identity,
                        geometry: chosen.geometry,
                        painted: chosen.painted,
                        visible: chosen.visible,
                        text: chosen.text?.run,
                        provenance: chosen.provenance,
                      },
                      null,
                      2,
                    )}
              </pre>
            </details>
            <p className="quiet">
              Canvas 2D preview ·{" "}
              <span title={scene.revision}>
                Revision {scene.revision.slice(0, 12)}
              </span>{" "}
              ·{" "}
              <a href={FONT_LICENSE_URL} target="_blank" rel="noreferrer">
                Font license
              </a>
            </p>
          </div>
        </>
      )}
    </div>
  );
}
function SceneCanvas({
  scene,
  selected,
  select,
  retry,
}: {
  scene: GraphicsScene;
  selected: readonly string[];
  select: (node: GraphicsNode) => void;
  retry: () => void;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [status, setStatus] = useState<{
    revision: string;
    error: string | null;
  } | null>(null);
  useEffect(() => {
    let current = true;
    void loadGraphicsFont()
      .then(() => {
        if (!current || canvas.current === null) {
          return;
        }
        const frame = renderCanvas(scene, selected);
        try {
          const target = canvas.current.getContext("2d");
          if (target === null) {
            throw new Error("This browser does not support Canvas 2D drawing.");
          }
          target.clearRect(0, 0, target.canvas.width, target.canvas.height);
          target.drawImage(frame, 0, 0);
          setStatus({ revision: scene.revision, error: null });
        } finally {
          frame.width = frame.height = 0;
        }
      })
      .catch((error) => {
        if (current) {
          setStatus({
            revision: scene.revision,
            error: error instanceof Error ? error.message : String(error),
          });
        }
      });
    return () => {
      current = false;
    };
  }, [scene, selected]);
  const ready = status?.revision === scene.revision && status.error === null;
  return (
    <>
      {status?.error != null && (
        <div className="output-message error" role="alert">
          {status.error}
          <button type="button" onClick={retry}>
            Retry drawing
          </button>
        </div>
      )}
      {status === null && (
        <p className="output-message quiet" role="status">
          Painting drawing…
        </p>
      )}
      <canvas
        ref={canvas}
        className="drawing-canvas"
        data-revision={ready ? scene.revision : undefined}
        data-selected-ids={JSON.stringify(selected)}
        role="img"
        aria-label={scene.root.accessibility?.label ?? "NX drawing"}
        aria-description={scene.root.accessibility?.description}
        style={{
          display: ready ? undefined : "none",
          width: scene.config.width,
          height: "auto",
        }}
        width={Math.ceil(scene.config.width * scene.config.scale)}
        height={Math.ceil(scene.config.height * scene.config.scale)}
        onClick={(event) => {
          if (!ready) {
            return;
          }
          const bounds = event.currentTarget.getBoundingClientRect();
          const x =
            ((event.clientX - bounds.left) * scene.config.width) / bounds.width;
          const y =
            ((event.clientY - bounds.top) * scene.config.height) /
            bounds.height;
          const hit = hitTest(scene, x, y);
          if (hit) {
            select(hit);
          }
        }}
      >
        NX drawing. Use the element inspector for geometry and text metrics.
      </canvas>
    </>
  );
}
