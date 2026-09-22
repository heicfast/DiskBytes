/**
 * View/tab state (spec §3): the five tabs, the current one, the Explore
 * breadcrumb folder, name filter, and inspector visibility. The active
 * tab capsule slides between tabs via Framer Motion `layoutId`
 * (TopBar.tsx); this store only owns WHICH tab is active.
 */
import { create } from "zustand";

export type TabId = "explore" | "duplicates" | "applications" | "monitor" | "snapshots";

export interface ViewStore {
  tab: TabId;
  inspectorVisible: boolean;
  /** True once the user has explicitly toggled the inspector — the
   * first-scan auto-reveal never overrides an explicit choice. */
  inspectorTouched: boolean;
  /** Name filter for Folders / Top Sizes / List (spec M4.15). */
  nameFilter: string;
  setTab: (tab: TabId) => void;
  toggleInspector: () => void;
  setInspectorVisible: (visible: boolean) => void;
  setNameFilter: (filter: string) => void;
}

export const useViewStore = create<ViewStore>((set) => ({
  tab: "explore",
  // Open by default (product-owner decision): the inspector is part of
  // the app's identity, and its welcome state teaches what it does.
  // Was hidden-until-first-scan in round 12; users read the closed
  // panel as "broken / missing".
  inspectorVisible: true,
  inspectorTouched: false,
  nameFilter: "",
  setTab: (tab) => set({ tab }),
  toggleInspector: () => set((s) => ({ inspectorVisible: !s.inspectorVisible, inspectorTouched: true })),
  setInspectorVisible: (inspectorVisible) => set({ inspectorVisible, inspectorTouched: true }),
  setNameFilter: (nameFilter) => set({ nameFilter }),
}));
