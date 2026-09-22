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
});
