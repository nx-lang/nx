import type { LanguageDocument } from "@nx-lang/language-protocol";

/** The subset of a language snapshot the query dispatcher uses, so a test can inject a fake. */
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
 * <para>The key is the document set's own content, each part preceded by its length so no two sets
 * can join into the same string. The cache holds a handful of entries of a few tens of kilobytes,
 * so a digest would buy nothing but a dependency on a hash the browser and Node spell
 * differently.</para>
 *
 * <para>A document's `version` is left out on purpose. It is the editor's counter, not content — a
 * revert or a host that bumps it on every keystroke sends the same text under a new number, and
 * that text has the same analysis. The caller puts the current request's versions back onto the
 * answer.</para>
 */
export function documentSetKey(documents: readonly LanguageDocument[]): string {
  const parts: string[] = [];
  for (const document of documents) {
    for (const part of [document.uri, document.identity ?? "", document.source]) {
      parts.push(String(part.length), part);
    }
  }
  return parts.join("\u0000");
}

/**
 * The most recently used snapshots, by document-set key.
 *
 * A hover storm sends the same text many times a second; the snapshot analyzes once and answers
 * each later query from that analysis, so keeping the last few sets is what makes answering cheap
 * on one thread, whether that thread is a server's or a worker's. The bound keeps memory flat, and
 * an evicted snapshot is disposed.
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
