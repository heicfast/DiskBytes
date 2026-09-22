import { describe, expect, it } from "vitest";
import { fitPath } from "./fitPath";

/** Deterministic measure: ~6px per char (jsdom has no canvas). */
const FONT = "10px mono";

describe("fitPath middle-ellipsis", () => {
  it("returns short paths untouched", () => {
    expect(fitPath("C:\\Users", 500, FONT)).toBe("C:\\Users");
  });

  it("keeps the head and the tail with a middle ellipsis", () => {
    const out = fitPath(
      "C:\\Users\\dev\\Documents\\Visual Studio 2022\\Projects\\x\\index.d.ts",
      200, // ~32 chars
      FONT,
    );
    expect(out).toContain("…");
    // The tail (what the user scanned for) must survive.
    expect(out.endsWith("index.d.ts")).toBe(true);
    expect(out.startsWith("C:\\")).toBe(true);
  });

  it("never returns a string wider than the budget (char model)", () => {
    const path = "C:\\a\\b\\c\\d\\e\\f\\g\\h\\i\\j\\k\\l\\m\\n\\o\\p\\very-long-file-name.txt";
    for (const budget of [40, 60, 120, 240]) {
      const out = fitPath(path, budget, FONT);
      expect(out.length * 6.2).toBeLessThanOrEqual(budget + 8); // ellipsis tolerance
      expect(out).toContain("…");
    }
  });

  it("degrades to a leading ellipsis when nothing else fits", () => {
    const out = fitPath("C:\\very\\long\\path\\that\\keeps\\going\\on\\and\\on\\forever.txt", 24, FONT);
    expect(out.startsWith("…")).toBe(true);
    expect(out.length).toBeLessThanOrEqual(5); // ellipsis + ~3 chars at 24px
  });

  it("handles POSIX separators", () => {
    const out = fitPath("/Users/dev/Library/Application Support/Code/User/settings.json", 160, FONT);
    expect(out).toContain("…");
    expect(out.endsWith("settings.json")).toBe(true);
  });

  it("letter-spacing widens the measurement (never overflows the box)", () => {
    const path = "C:\\Users\\dev\\Documents\\Visual Studio 2022\\Projects\\index.d.ts";
    const tight = 200;
    const noLs = fitPath(path, tight, FONT);
    const withLs = fitPath(path, tight, FONT, { letterSpacing: 1.5 });
    // Tracking effectively shrinks the budget → the result is at least
    // as aggressive at eliding.
    expect(withLs.length).toBeLessThanOrEqual(noLs.length);
    expect(withLs).toContain("…");
    // Measured width incl. tracking must respect the budget.
    const px = (s: string) => s.length * 6.2 + Math.max(0, s.length - 1) * 1.5;
    expect(px(withLs)).toBeLessThanOrEqual(tight + 8);
  });

  it("pad reserves a safety margin", () => {
    const path = "C:\\Users\\dev\\Documents\\Visual Studio 2022\\Projects\\index.d.ts";
    const out = fitPath(path, 200, FONT, { pad: 12 });
    expect(out.length * 6.2).toBeLessThanOrEqual(200 - 12 + 8);
  });
});
