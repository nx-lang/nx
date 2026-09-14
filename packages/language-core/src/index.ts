/**
 * The transport-free half of NX language serving: prelude coordinates, the snapshot cache, the
 * query dispatcher, and an in-process `NxLanguageService` over any snapshot factory.
 *
 * `@nx-lang/language-http` adds a Fetch-shaped handler and a Node listener on top of this;
 * `@nx-lang/sdk-wasm` binds it to snapshots from a WebAssembly host. Because both go through the
 * same dispatcher, a hover answered in a browser and the same hover answered by a server agree on
 * every line and column.
 */
export { PRELUDE_ORIGIN, answerQuery, shiftReport, type AnswerRequest } from "./answer.js";
export { SnapshotCache, documentSetKey, type SnapshotLike } from "./cache.js";
export {
  preludeOffsets,
  shiftPositionIn,
  shiftRangeOut,
  withPrelude,
  type PreludeOffsets
} from "./prelude.js";
export {
  DEFAULT_CACHE_SIZE,
  createSnapshotLanguageService,
  type SnapshotLanguageService,
  type SnapshotLanguageServiceOptions
} from "./service.js";
