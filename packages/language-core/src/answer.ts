/**
 * Answering one language query against a snapshot, with the prelude arithmetic that keeps every
 * position in the document's own coordinates.
 *
 * <para>This is the half of NX language serving that has nothing to do with a transport. The HTTP
 * handler calls it after parsing a request; the in-process service calls it directly. Neither can
 * disagree with the other about a line number, because there is only one copy of the shift.</para>
 */
import type {
  CompletionList,
  DiagnosticReport,
  DocumentDiagnostics,
  DocumentSymbol,
  EditorDiagnostic,
  Hover,
  LanguageDocument,
  LanguageQueryName,
  RelatedLocation,
  TextPosition,
  WorkspaceDiagnostic
} from "@nx-lang/language-protocol";

import type { SnapshotLike } from "./cache.js";
import { shiftPositionIn, shiftRangeOut, withPrelude, type PreludeOffsets } from "./prelude.js";

/** The label identity a diagnostic carries when it lies inside the prelude rather than the document. */
export const PRELUDE_ORIGIN = "prelude";

/** What one query needs to be answered: the set it is asked over, and where in it. */
export interface AnswerRequest {
  documents: LanguageDocument[];
  uri: string;
  position: TextPosition | undefined;
}

/** Answers one validated query, applying the prelude around the snapshot when there is one. */
export function answerQuery(
  query: LanguageQueryName,
  request: AnswerRequest,
  prelude: PreludeOffsets | undefined,
  snapshotFor: (documents: LanguageDocument[]) => SnapshotLike
): unknown {
  const documents =
    prelude === undefined
      ? request.documents
      : request.documents.map((document) =>
          document.uri === request.uri ? { ...document, source: withPrelude(prelude, document.source) } : document,
        );
  const position =
    request.position === undefined || prelude === undefined
      ? request.position
      : shiftPositionIn(request.position, prelude);
  const snapshot = snapshotFor(documents);
  // The snapshot may have been built by an earlier request over the same text; its answers carry
  // that request's versions. The client discards answers by version, so it gets this request's.
  const versions = new Map(request.documents.map((document) => [document.uri, document.version ?? null]));
  const withVersion = <T extends { uri: string; version: number | null }>(answer: T): T => ({
    ...answer,
    version: versions.get(answer.uri) ?? null,
  });

  switch (query) {
    case "hover": {
      const hover = snapshot.hover(request.uri, position as TextPosition) as Hover | null;
      if (hover === null) {
        return null;
      }
      if (prelude === undefined) {
        return withVersion(hover);
      }
      const range = shiftRangeOut(hover.range, prelude);
      return range === null ? null : withVersion({ ...hover, range });
    }
    case "completions":
      return withVersion(snapshot.completions(request.uri, position as TextPosition) as CompletionList);
    case "diagnostics": {
      const report = snapshot.diagnostics() as DiagnosticReport;
      const shifted = prelude === undefined ? report : shiftReport(report, request.uri, prelude);
      return { ...shifted, documents: shifted.documents.map(withVersion) };
    }
    case "documentSymbols": {
      const symbols = snapshot.documentSymbols(request.uri) as DocumentSymbol[];
      if (prelude === undefined) {
        return symbols;
      }
      return symbols.flatMap((symbol) => {
        const range = shiftRangeOut(symbol.range, prelude);
        const selectionRange = shiftRangeOut(symbol.selectionRange, prelude);
        return range === null || selectionRange === null ? [] : [{ ...symbol, range, selectionRange }];
      });
    }
  }
}

/**
 * Moves the queried document's diagnostics back into its own coordinates.
 *
 * A diagnostic inside the prelude is the host's fault, not the author's: it is reported as a
 * workspace diagnostic labelled with the prelude origin and no range, rather than positioned on a
 * line the author cannot see. A related location that points into the queried document is shifted
 * wherever it appears, since a sibling's diagnostic names the same combined text.
 */
export function shiftReport(report: DiagnosticReport, uri: string, prelude: PreludeOffsets): DiagnosticReport {
  const workspace: WorkspaceDiagnostic[] = [...report.workspace];
  const documents: DocumentDiagnostics[] = report.documents.map((document) => {
    if (document.uri !== uri) {
      return {
        ...document,
        diagnostics: document.diagnostics.map((diagnostic) => ({
          ...diagnostic,
          related: shiftRelated(diagnostic.related, uri, prelude),
        })),
      };
    }
    const diagnostics: EditorDiagnostic[] = [];
    for (const diagnostic of document.diagnostics) {
      const range = shiftRangeOut(diagnostic.range, prelude);
      if (range === null) {
        workspace.push({
          severity: diagnostic.severity,
          code: diagnostic.code,
          message: diagnostic.message,
          labels: [{ identity: PRELUDE_ORIGIN, message: null }],
        });
        continue;
      }
      diagnostics.push({ ...diagnostic, range, related: shiftRelated(diagnostic.related, uri, prelude) });
    }
    return { ...document, diagnostics };
  });
  return { documents, workspace };
}

/** Related locations with those in the queried document shifted; one inside the prelude is dropped. */
function shiftRelated(related: readonly RelatedLocation[], uri: string, prelude: PreludeOffsets): RelatedLocation[] {
  const shifted: RelatedLocation[] = [];
  for (const location of related) {
    if (location.uri !== uri) {
      shifted.push(location);
      continue;
    }
    const range = shiftRangeOut(location.range, prelude);
    if (range !== null) {
      shifted.push({ ...location, range });
    }
  }
  return shifted;
}
