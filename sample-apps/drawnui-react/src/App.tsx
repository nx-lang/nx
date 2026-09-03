import { useEffect, useState } from "react";
import "./app.css";
import { compileOverHttp } from "./compile";
import { EXAMPLES, coverageNote, exampleById } from "./examples";
import { Fiddle } from "./fiddle/Fiddle";
import { CoverageChip, Gallery } from "./gallery/Gallery";
import { useRoute } from "./router";

export function App() {
  const [route, navigate] = useRoute();
  // Edits live for as long as the session, per example, and never reach the gallery's own copy.
  const [edits, setEdits] = useState<Record<string, string>>({});

  useEffect(() => {
    document.title = route.kind === "gallery" ? "NX × DrawnUI" : `${exampleById(route.id)?.name ?? "Fiddle"} — NX × DrawnUI`;
  }, [route]);

  if (route.kind === "gallery") {
    return <Gallery compile={compileOverHttp} onOpen={(example) => navigate({ kind: "fiddle", id: example.id })} />;
  }

  const example = exampleById(route.id) ?? EXAMPLES[0]!;
  const note = coverageNote(example);
  return (
    <Fiddle
      title={example.name}
      source={edits[example.id] ?? example.source}
      onSourceChange={(source) => setEdits((previous) => ({ ...previous, [example.id]: source }))}
      compile={compileOverHttp}
      coverage={note === null ? undefined : <CoverageChip example={example} />}
      onBack={() => navigate({ kind: "gallery" })}
    />
  );
}
