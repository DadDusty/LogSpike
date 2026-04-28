// LRU window cache for line ranges.
//
// The viewport asks for ~50 visible lines at a time. Naively re-fetching on
// every scroll tick wastes IPC round-trips. We cache fetched chunks keyed by
// chunk index (a chunk is `CHUNK_SIZE` consecutive lines) and reuse them on
// subsequent renders. Eviction keeps memory bounded for very large files.

import type { LogLine } from "./types";

export const CHUNK_SIZE = 256;
const MAX_CHUNKS = 256; // ~64K cached lines per file

interface CacheEntry {
  chunkId: number;
  lines: LogLine[];
}

export class LineCache {
  private chunks = new Map<number, CacheEntry>();

  /** Returns the cached lines for chunk `chunkId`, or `undefined`. */
  get(chunkId: number): LogLine[] | undefined {
    const entry = this.chunks.get(chunkId);
    if (!entry) return undefined;
    // Touch for LRU.
    this.chunks.delete(chunkId);
    this.chunks.set(chunkId, entry);
    return entry.lines;
  }

  put(chunkId: number, lines: LogLine[]): void {
    this.chunks.set(chunkId, { chunkId, lines });
    if (this.chunks.size > MAX_CHUNKS) {
      const oldest = this.chunks.keys().next().value;
      if (oldest !== undefined) this.chunks.delete(oldest);
    }
  }

  /** Drop everything (e.g. after rotation or filter change). */
  clear(): void {
    this.chunks.clear();
  }

  /** Compute the chunk ids that overlap a given line range. */
  static chunksFor(start: number, end: number): number[] {
    const first = Math.floor(start / CHUNK_SIZE);
    const last = Math.floor((end - 1) / CHUNK_SIZE);
    const out: number[] = [];
    for (let i = first; i <= last; i++) out.push(i);
    return out;
  }

  /** Stitch cached chunks into a contiguous slice. Missing chunks are gaps. */
  slice(start: number, end: number): { lines: (LogLine | null)[]; missing: number[] } {
    const out: (LogLine | null)[] = new Array(end - start).fill(null);
    const missing: number[] = [];
    for (const chunkId of LineCache.chunksFor(start, end)) {
      const lines = this.get(chunkId);
      if (!lines) {
        missing.push(chunkId);
        continue;
      }
      // Place each line by its absolute number; this is robust to a short
      // final chunk and to any future trimming on the backend.
      for (const line of lines) {
        const idx = line.number - start;
        if (idx >= 0 && idx < out.length) out[idx] = line;
      }
    }
    return { lines: out, missing };
  }
}
