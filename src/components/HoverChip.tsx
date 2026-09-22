/**
 * Hover chip (spec §7): bottom-left over any mode — icon, name, size,
 * % of scan, file-count pill. Driven by DIRECT DOM WRITES through an
 * imperative handle (hover NEVER re-renders React — spec §9 pitfall).
 */
import { createElement, forwardRef, useImperativeHandle, useRef } from "react";
import { createRoot } from "react-dom/client";
import { categoryIcon, FolderIcon } from "./Icon";

export interface HoverChipHandle {
  show: (data: HoverChipData, x: number, y: number) => void;
  move: (x: number, y: number) => void;
  hide: () => void;
}

export interface HoverChipData {
  name: string;
  size: number;
  shareOfScan: number;
  fileCount: number;
  isDir: boolean;
  category: string;
  categoryColor: number;
  isCloud: boolean;
  isProtected: boolean;
}

export function formatSizeLocal(bytes: number): string {
  // Local twin to avoid importing the store in this leaf component.
  const units = ["B", "KB", "MB", "GB", "TB"];
  let v = bytes;
  let u = 0;
  while (v >= 1024 && u < units.length - 1) {
    v /= 1024;
    u++;
  }
  return `${v >= 100 || u === 0 ? Math.round(v) : v.toFixed(1)} ${units[u]}`;
}

export const HoverChip = forwardRef<HoverChipHandle, { sizeFmt: (b: number) => string }>(function HoverChip(
  { sizeFmt },
  ref,
) {
  const root = useRef<HTMLDivElement>(null);
  const visible = useRef(false);

  useImperativeHandle(ref, () => ({
    show(data, x, y) {
      const el = root.current;
      if (!el) return;
      visible.current = true;
      const Icon = data.isDir ? FolderIcon : categoryIcon(data.category);
      const icon = el.querySelector<HTMLElement>(".db-hc-icon");
      if (icon) {
        icon.style.background = `#${(data.categoryColor || 0xcbd5e1).toString(16).padStart(6, "0")}dd`;
        // Render the glyph into a detached root, then move the node.
        const host = document.createElement("span");
        const root = createRoot(host);
        root.render(createElement(Icon, { size: 15 }));
        icon.replaceChildren(host);
      }
      const strong = el.querySelector<HTMLElement>(".db-hc-body > strong");
      if (strong) strong.textContent = data.name;
      const small = el.querySelector<HTMLElement>(".db-hc-body > small");
      if (small) {
        small.textContent =
          `${sizeFmt(data.size)} · ${(data.shareOfScan * 100).toFixed(1)}% of scan` +
          (data.isProtected ? " · managed by Windows" : "") +
          (data.isCloud ? " · in the cloud" : "");
      }
      const pill = el.querySelector<HTMLElement>(".db-hc-pill");
      if (pill) {
        pill.textContent = data.isDir
          ? `${Math.round(data.fileCount).toLocaleString()} files`
          : data.category;
        pill.style.display = "";
      }
      el.style.opacity = "1";
      el.style.transform = "translateY(0)";
      move(x, y);
    },
    move(x, y) {
      const el = root.current;
      if (!el || !visible.current) return;
      const W = 380;
      const left = Math.min(Math.max(12, x + 16), window.innerWidth - W - 12);
      const top = Math.min(window.innerHeight - 70, y + 18);
      el.style.left = `${left}px`;
      el.style.top = `${top}px`;
    },
    hide() {
      const el = root.current;
      if (!el) return;
      visible.current = false;
      el.style.opacity = "0";
      el.style.transform = "translateY(4px)";
    },
  }));

  function move(x: number, y: number): void {
    const el = root.current;
    if (!el) return;
    el.style.left = `${x}px`;
    el.style.top = `${y}px`;
  }

  return (
    <div ref={root} className="db-hover-chip" style={{ opacity: 0, pointerEvents: "none" }} aria-hidden>
      <span className="db-hc-icon" />
      <span className="db-hc-body">
        <strong>Name</strong>
        <small>—</small>
      </span>
      <span className="db-hc-pill">—</span>
    </div>
  );
});
