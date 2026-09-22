/**
 * Explore visualization UI state (spec §7 toolbar): current mode, color
 * mode, depth (2–10, default 7), abbreviate toggle. Per-app-run only
 * (spec doc 05 §4.4: remember last mode/depth per run, not persisted).
 * The dev hook DISKBYTES_MODE seeds the initial mode (spec §15).
 */
import { create } from "zustand";

export const MODES = ["Folders", "Treemap", "Sunburst", "Flame", "Bubbles", "Mind Map", "Top Sizes", "Age Map", "List"] as const;
export type Mode = (typeof MODES)[number];

export const CANVAS_MODES = new Set<Mode>(["Treemap", "Sunburst", "Flame", "Bubbles", "Mind Map"]);
/** Modes that show the color-mode segmented control (spec §7). */
export const COLORED_MODES = new Set<Mode>(["Treemap", "Sunburst", "Flame", "Bubbles", "Mind Map"]);
/** Modes that show the depth slider (spec §7). */
export const DEPTH_MODES = new Set<Mode>(["Treemap", "Sunburst", "Flame"]);

export const MODE_CAPTIONS: Record<Mode, string> = {
  Folders: "Browse folder by folder, sized as you go",
  Treemap: "Every file as a rectangle, sized by bytes",
  Sunburst: "Rings radiating out from the scan root",
  Flame: "Depth top to bottom, size left to right",
  Bubbles: "Nested bubbles, one per folder",
  "Mind Map": "Branches from the root, sized by weight",
  "Top Sizes": "The biggest items, ranked",
  "Age Map": "Where your bytes sit on a timeline",
  List: "Every item as an expandable outline",
};

export type ColorMode = "by-folder" | "by-type" | "by-age";

interface VizUiState {
  mode: Mode;
  colorMode: ColorMode;
  depth: number;
  abbreviate: boolean;
  setMode: (mode: Mode) => void;
  setColorMode: (colorMode: ColorMode) => void;
  setDepth: (depth: number) => void;
  setAbbreviate: (on: boolean) => void;
}

/** Dev-hook seeding (§15 DISKBYTES_MODE). */
function seedMode(): Mode {
  const hook = (window as unknown as { __DB_DEV_MODE__?: string }).__DB_DEV_MODE__;
  if (hook && (MODES as readonly string[]).includes(hook)) return hook as Mode;
  return "Folders";
}

export const useVizUiStore = create<VizUiState>((set) => ({
  mode: seedMode(),
  colorMode: "by-folder",
  depth: 7,
  abbreviate: false,
  setMode: (mode) => set({ mode }),
  setColorMode: (colorMode) => set({ colorMode }),
  setDepth: (depth) => set({ depth: Math.min(10, Math.max(2, Math.round(depth))) }),
  setAbbreviate: (abbreviate) => set({ abbreviate }),
}));
