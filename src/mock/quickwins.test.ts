/**
 * Quick Wins mock↔engine parity tests (core quickwins.rs):
 * the old mock fabricated rows (fake counts, kebab ids) and
 * quick_win_items only matched node_modules/vm-disks — "Add all N"
 * staged NOTHING for every other category, which masked two
 * production bugs (missing ICONS entries, "1 items" pluralization).
 */
import { describe, expect, it } from "vitest";
import { resolveQuickWins, stripItems, quickWinItems } from "./commands";

describe("mock quick_wins engine port", () => {
  const cats = resolveQuickWins();
  const rows = cats.map(stripItems);

  it("uses production snake_case category ids", () => {
    const ids = rows.map((r) => r.id);
    // The production vocabulary (core quickwins.rs resolve()).
    for (const id of [
      "downloads",
      "temp_caches",
      "dev_caches",
      "node_modules",
      "build_artifacts",
      "large_media",
      "vm_disks",
    ]) {
      expect(ids).toContain(id);
    }
    // The old mock's kebab ids must be gone.
    for (const bad of ["large-media", "node-modules", "build-artifacts", "dev-caches", "vm-disks"]) {
      expect(ids).not.toContain(bad);
    }
  });

  it("every rendered row has at least one stageable item", () => {
    // THE original bug: rows existed whose add-all staged zero items.
    expect(rows.length).toBeGreaterThan(0);
    for (const r of rows) {
      expect((r.count as number) >= 1).toBe(true);
    }
  });

  it("count equals the item count, not a fabricated number", () => {
    for (const c of cats) {
      const row = rows.find((r) => r.id === c.id);
      expect(row?.count).toBe(c.items.length);
    }
  });

  it("rows are sorted by size descending", () => {
    const sizes = rows.map((r) => r.size as number);
    const sorted = [...sizes].sort((a, b) => b - a);
    expect(sizes).toEqual(sorted);
  });

  it("vm_disks is review-only; downloads is not", () => {
    const vm = rows.find((r) => r.id === "vm_disks");
    expect(vm?.reviewOnly).toBe(true);
    const dl = rows.find((r) => r.id === "downloads");
    expect(dl?.reviewOnly).toBe(false);
  });

  it("production icon tags are used (frontend ICONS map keys)", () => {
    const icons = rows.map((r) => r.icon);
    expect(icons).toContain("download"); // downloads
    expect(icons).toContain("video"); // large media
    expect(icons).toContain("server"); // vm disks
  });

  it("quickWinItems returns items for EVERY category (the staging bug)", () => {
    for (const r of rows) {
      const items = quickWinItems(String(r.id));
      expect(items.length).toBe(r.count);
      expect(items.length).toBeGreaterThan(0);
      for (const it of items) {
        expect(it.path.length).toBeGreaterThan(0);
        expect(it.size).toBeGreaterThanOrEqual(0);
      }
    }
  });

  it("no category stages a parent together with its descendant", () => {
    for (const r of rows) {
      const items = quickWinItems(String(r.id));
      for (let i = 0; i < items.length; i++) {
        for (let j = 0; j < items.length; j++) {
          if (i === j) continue;
          const a = items[i].path.toLowerCase();
          const b = items[j].path.toLowerCase();
          const prefix = a.endsWith("\\\\") ? a : a + "\\\\";
          expect(b.startsWith(prefix)).toBe(false);
        }
      }
    }
  });
});
