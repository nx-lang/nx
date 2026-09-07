import { createHash } from "node:crypto";
import type { LanguageDocument } from "@nx-lang/language-protocol";

/** The subset of `NxLanguageSnapshot` the handler uses, so a test can inject a fake. */
export interface SnapshotLike {
  hover(uri: string, position: { line: number; character: number }): unknown;
  completions(uri: string, position: { line: number; character: number }): unknown;
  diagnostics(): unknown;
  documentSymbols(uri: string): unknown;
  dispose(): void;
}

/**
 * A stable key for a document set: any change to any document's text, URI or identity changes it.
 *
 * A document's `version` is left out on purpose. It is the editor's counter, not content — a revert
 * or a host that bumps it on every keystroke sends the same text under a new number, and that text
 * has the same analysis. The handler puts the current request's versions back onto the answer.
 */
export function documentSetKey(documents: readonly LanguageDocument[]): string {
  const hash = createHash("sha256");
  for (const document of documents) {
    for (const part of [document.uri, document.identity ?? "", document.source]) {
      hash.update(String(Buffer.byteLength(part, "utf8")));
      hash.update("\0");
      hash.update(part);
    }
    hash.update("\n");
  }
  return hash.digest("hex");
}

/**
 * The most recently used snapshots, by document-set key.
 *
 * A hover storm sends the same text many times a second; the snapshot analyzes once and answers
 * each later query from that analysis, so keeping the last few sets is what makes the handler cheap
 * on a single-threaded server. The bound keeps memory flat, and an evicted snapshot is disposed.
 */
export class SnapshotCache<T extends SnapshotLike> {
  private readonly entries = new Map<string, T>();

  constructor(private readonly capacity: number) {
    if (!Number.isInteger(capacity) || capacity < 1) {
      throw new RangeError(`cache size must be a positive integer, got ${capacity}`);
    }
  }

  /** The snapshot for `key`, creating it with `create` on a miss. */
  getOrCreate(key: string, create: () => T): T {
    const existing = this.entries.get(key);
    if (existing !== undefined) {
      // Re-insert so the map's iteration order is least-recently-used first.
      this.entries.delete(key);
      this.entries.set(key, existing);
      return existing;
    }
    const created = create();
    this.entries.set(key, created);
    while (this.entries.size > this.capacity) {
      const oldest = this.entries.keys().next().value as string;
      const evicted = this.entries.get(oldest);
      this.entries.delete(oldest);
      evicted?.dispose();
    }
    return created;
  }

  /** How many snapshots are held. */
  get size(): number {
    return this.entries.size;
  }

  /** Disposes every held snapshot. */
  clear(): void {
    for (const snapshot of this.entries.values()) {
      snapshot.dispose();
    }
    this.entries.clear();
  }
}
