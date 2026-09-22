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
  /** Name filter for Folders / Top Sizes / List (spec M4.15). */
  nameFilter: string;
  setTab: (tab: TabId) => void;
  toggleInspector: () => void;
  setInspectorVisible: (visible: boolean) => void;
  setNameFilter: (filter: string) => void;
}

export const useViewStore = create<ViewStore>((set) => ({
  tab: "explore",
  inspectorVisible: true,
  nameFilter: "",
  setTab: (tab) => set({ tab }),
  toggleInspector: () => set((s) => ({ inspectorVisible: !s.inspectorVisible })),
  setInspectorVisible: (inspectorVisible) => set({ inspectorVisible }),
  setNameFilter: (nameFilter) => set({ nameFilter }),
}));
