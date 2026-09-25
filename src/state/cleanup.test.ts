/**
 * Cleanup store tests (spec §9 sync half). M1 needs the badge contract:
 * stage/unstage/clear must update the count IMMEDIATELY (the spec warns
 * stale copies were a real bug in the previous version of this app).
 */
import { beforeEach, describe, expect, it } from "vitest";

import { useCleanupStore, type QueueItem } from "./cleanup";

const item = (id: number, size = 100): QueueItem => ({ id, path: `C:\\x\\${id}`, size, reason: "Duplicate" });

describe("useCleanupStore (spec §9)", () => {
  beforeEach(() => useCleanupStore.getState().clear());

  it("stages items and reports membership", () => {
    const { stage, contains } = useCleanupStore.getState();
    stage(item(1));
    stage(item(2, 2048));
    expect(contains(1)).toBe(true);
    expect(contains(2)).toBe(true);
    expect(contains(3)).toBe(false);
  });

  it("stage is idempotent per node id", () => {
    useCleanupStore.getState().stage(item(1));
    useCleanupStore.getState().stage(item(1));
    expect(useCleanupStore.getState().items).toHaveLength(1);
  });

  it("stageMany skips already-staged ids", () => {
    useCleanupStore.getState().stage(item(1));
    useCleanupStore.getState().stageMany([item(1), item(2), item(3)]);
    expect(useCleanupStore.getState().items.map((i) => i.id)).toEqual([1, 2, 3]);
  });

  it("badge count updates instantly on unstage/remove/clear (spec pitfall)", () => {
    const s = useCleanupStore.getState();
    s.stageMany([item(1), item(2), item(3)]);
    expect(useCleanupStore.getState().items).toHaveLength(3);
    s.unstage(2);
    expect(useCleanupStore.getState().items.map((i) => i.id)).toEqual([1, 3]);
    s.remove(1);
    expect(useCleanupStore.getState().items.map((i) => i.id)).toEqual([3]);
    s.clear();
    expect(useCleanupStore.getState().items).toHaveLength(0);
  });

  it("totalSize sums staged sizes", () => {
    useCleanupStore.getState().stageMany([item(1, 500), item(2, 700)]);
    expect(useCleanupStore.getState().totalSize()).toBe(1200);
  });

  // ── Duplicates staging (synthetic id 0 = path-only items) ──────────
  // THE BUG: dedupe keyed by id alone meant only ONE duplicate could
  // ever be staged (every Duplicates row stages with id 0), and
  // remove(0) removed them all at once.
  const dupe = (path: string, size = 50): QueueItem => ({ id: 0, path, size, reason: "Duplicate" });

  it("stages multiple synthetic id-0 items with different paths", () => {
    const s = useCleanupStore.getState();
    s.stageMany([dupe("C:\\a\\1.txt"), dupe("C:\\a\\2.txt"), dupe("C:\\a\\3.txt")]);
    expect(useCleanupStore.getState().items).toHaveLength(3);
  });

  it("stageMany across batches stages NEW paths under id 0", () => {
    const s = useCleanupStore.getState();
    s.stageMany([dupe("C:\\a\\1.txt")]);
    s.stageMany([dupe("C:\\a\\2.txt")]);
    expect(useCleanupStore.getState().items).toHaveLength(2);
  });

  it("the same duplicate path is idempotent across batches", () => {
    const s = useCleanupStore.getState();
    s.stageMany([dupe("C:\\a\\1.txt")]);
    s.stageMany([dupe("C:\\a\\1.txt")]);
    expect(useCleanupStore.getState().items).toHaveLength(1);
  });

  it("stageMany dedupes within one batch too", () => {
    const s = useCleanupStore.getState();
    s.stageMany([dupe("C:\\a\\1.txt"), dupe("C:\\a\\1.txt"), dupe("C:\\a\\2.txt")]);
    expect(useCleanupStore.getState().items).toHaveLength(2);
  });

  it("remove(id, path) removes exactly one synthetic row", () => {
    const s = useCleanupStore.getState();
    s.stageMany([dupe("C:\\a\\1.txt"), dupe("C:\\a\\2.txt"), dupe("C:\\a\\3.txt")]);
    s.remove(0, "C:\\a\\2.txt");
    const left = useCleanupStore.getState().items.map((i) => i.path);
    expect(left).toEqual(["C:\\a\\1.txt", "C:\\a\\3.txt"]);
  });

  it("real ids still remove by id alone", () => {
    const s = useCleanupStore.getState();
    s.stageMany([item(7), dupe("C:\\a\\1.txt")]);
    s.remove(7);
    expect(useCleanupStore.getState().items.map((i) => i.id)).toEqual([0]);
  });
});
