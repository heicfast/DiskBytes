/**
 * Cleanup Queue store (BuildPrompt §9) — its OWN Zustand store, exactly
 * as the spec demands (never copy queue items into other stores or
 * component state; the badge and popover subscribe via selectors).
 *
 * `commitToRecycleBin` (M5) invokes the Rust commit (IFileOperation +
 * pre-flight refusals + tree surgery); recycled items leave the queue,
 * failed items STAY with their reasons surfaced to the caller (the
 * popover shows the failure alert).
 */
import { create } from "zustand";
import { invoke } from "../lib/ipc";
import { EVENTS, track } from "../lib/analytics";

/** One staged item (spec §9: id, path, size, reason). */
export interface QueueItem {
  /** Real node id, or the synthetic id of a regroup group. */
  id: number;
  /** Display path of the staged item. */
  path: string;
  /** Size on disk in bytes. */
  size: number;
  /** Why it was staged ("Duplicate", "Leftovers: …", "Large media", …). */
  reason: string;
}

/** The Rust commit response (cleanup-committed shape). */
export interface CommitResult {
  generation: number;
  trashed: { path: string; alreadyGone: boolean; nested: boolean }[];
  failed: { path: string; reason: string }[];
  stats: [number, number, number, number] | null;
  currentFolder: number;
  selectedNode: number | null;
}

interface CleanupState {
  items: QueueItem[];
  stage: (item: QueueItem) => void;
  stageMany: (items: QueueItem[]) => void;
  unstage: (id: number, path?: string) => void;
  remove: (id: number, path?: string) => void;
  clear: () => void;
  contains: (id: number, path?: string) => boolean;
  totalSize: () => number;
  /** Commit to the Recycle Bin after explicit confirmation (M5).
   *  Rejects on stale generation / COM failure — the queue stays intact
   *  (safe default) and the popover surfaces the error. */
  commitToRecycleBin: () => Promise<CommitResult>;
}

/** Stable identity: real node ids dedupe by id, but synthetic items
 * (Duplicates stage with id 0 — path-only; the Rust commit path treats
 * id 0 as path-only) must dedupe by PATH. Keying everything by id alone
 * meant only ONE duplicate could ever be staged, and remove(0) nuked
 * every duplicate row at once. */
const keyOf = (i: Pick<QueueItem, "id" | "path">): string =>
  i.id === 0 ? `p:${i.path}` : `i:${i.id}`;

/** The staged queue. Popover + badge subscribe via selectors (spec §9). */
export const useCleanupStore = create<CleanupState>((set, get) => ({
  items: [],

  stage: (item) =>
    set((s) => {
      if (s.items.some((i) => keyOf(i) === keyOf(item))) return s; // idempotent
      return { items: [...s.items, item] };
    }),

  stageMany: (items) => {
    if (items.length > 0) {
      const source = items[0].reason.split(":")[0]?.split(" — ")[0] ?? "manual";
      track(EVENTS.cleanupStaged, {
        source,
        items: items.length,
        bytes: items.reduce((a, b) => a + b.size, 0),
      });
    }
    set((s) => {
      // Dedupe against the queue AND within the batch (same keyOf).
      const seen = new Set(s.items.map((i) => keyOf(i)));
      const add = items.filter((i) => {
        const k = keyOf(i);
        if (seen.has(k)) return false;
        seen.add(k);
        return true;
      });
      return add.length ? { items: [...s.items, ...add] } : s;
    });
  },

  /** Remove by id — with a path, removes exactly one synthetic item
   * (Duplicates id=0 rows); without, removes every item with that id
   * (real node ids are unique in the queue). */
  unstage: (id, path) =>
    set((s) => ({
      items: s.items.filter((i) => (path === undefined ? i.id !== id : !(i.id === id && i.path === path))),
    })),

  remove: (id, path) =>
    set((s) => ({
      items: s.items.filter((i) => (path === undefined ? i.id !== id : !(i.id === id && i.path === path))),
    })),

  clear: () => set({ items: [] }),

  contains: (id, path) =>
    get().items.some((i) => (path === undefined ? i.id === id : i.id === id && i.path === path)),

  totalSize: () => get().items.reduce((acc, i) => acc + i.size, 0),

  commitToRecycleBin: async () => {
    const status = await invoke<{ generation: number }>("get_status");
    const items = get().items.map((i) => ({
      id: i.id,
      path: i.path,
      size: i.size,
      reason: i.reason,
    }));
    try {
      const result = await invoke<CommitResult>("commit_cleanup", {
        generation: status.generation,
        items,
      });
      // Recycled (incl. already-gone + nested-with-parent) leave the queue.
      const recycled = new Set(result.trashed.map((t) => t.path));
      set((s) => ({ items: s.items.filter((i) => !recycled.has(i.path)) }));
      return result;
    } catch (e) {
      // Stale generation or COM failure: the queue stays intact (safe
      // default); the popover surfaces the error.
      throw e;
    }
  },
}));
