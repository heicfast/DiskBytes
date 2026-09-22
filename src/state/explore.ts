/**
 * Explore navigation state (spec §7): the current folder, the selected
 * node, and the back-stack. Folder navigation resets the canvas caches
 * per node automatically (layouts are node-keyed on the Rust side).
 */
import { create } from "zustand";

export interface ExploreStore {
  /** The folder the Explore tab is currently showing (scan root = 0). */
  currentFolder: number;
  /** The selected node (single click; inspector + hover chip track it). */
  selectedNode: number | null;
  /** Back-stack of previously visited folders (for the top-bar back chevron). */
  folderStack: number[];
  openFolder: (id: number) => void;
  goBack: () => void;
  select: (id: number | null) => void;
  /** Reset navigation on a new scan (scan-done swap). */
  resetNavigation: () => void;
}

export const useExploreStore = create<ExploreStore>((set, get) => ({
  currentFolder: 0,
  selectedNode: null,
  folderStack: [],

  openFolder: (id) => {
    if (id === get().currentFolder) return;
    set((s) => ({
      folderStack: [...s.folderStack, s.currentFolder],
      currentFolder: id,
      selectedNode: null,
    }));
  },

  goBack: () => {
    const stack = get().folderStack;
    if (stack.length === 0) return;
    const prev = stack[stack.length - 1];
    set((s) => ({
      folderStack: s.folderStack.slice(0, -1),
      currentFolder: prev,
      selectedNode: null,
    }));
  },

  select: (id) => set({ selectedNode: id }),

  resetNavigation: () => set({ currentFolder: 0, selectedNode: null, folderStack: [] }),
}));
