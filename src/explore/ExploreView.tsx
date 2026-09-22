/**
 * Explore view (spec §7): idle / scanning / done / error states, the
 * big header (folder name + stats + reveal), the unreadable notice,
 * the mode toolbar, and the stage hosting one of the 9 modes. Wires the
 * shared interaction set (select, dblclick-open, context menu, hover
 * chip, preview) into every mode.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { motion, useAnimate } from "framer-motion";
import { ExternalLinkIcon, FolderIcon, ScanLineIcon } from "../components/Icon";
import { EmptyState } from "../components/buttons";
import { UnreadableNotice } from "../sidebar";
import { ExploreHeader } from "./ExploreHeader";
import { FoldersMode } from "./modes/FoldersMode";
import { CanvasMode } from "./modes/CanvasMode";
import { TopSizesMode } from "./modes/TopSizesMode";
import { AgeMapMode } from "./modes/AgeMapMode";
import { ListMode } from "./modes/ListMode";
import { HoverChip, type HoverChipHandle } from "../components/HoverChip";
import { ItemContextMenu, type ItemMenuState } from "../components/ItemContextMenu";
import { invoke } from "../lib/ipc";
import { bytes } from "../lib/format";
import { useExploreStore } from "../state/explore";
import { useScanStore } from "../state/scan";
import { useViewStore } from "../state/view";
import { useVizUiStore, type Mode } from "../state/vizUi";
import { useCleanupStore } from "../state/cleanup";
import { CANVAS_MODES } from "../state/vizUi";
import { getHoverDetails } from "../viz/layoutIpc";
import { getNodeDetails, type NodeDetailsData } from "../viz/exploreIpc";
import { EVENTS, track } from "../lib/analytics";

export function ExploreView({ onPreview }: { onPreview: (id: number) => void }) {
  const status = useScanStore((s) => s.status);
  const progress = useScanStore((s) => s.progress);
  const error = useScanStore((s) => s.error);
  const scanTarget = useScanStore((s) => s.scanTarget);
  const generation = useScanStore((s) => s.generation);
  const currentFolder = useExploreStore((s) => s.currentFolder);
  const selectedNode = useExploreStore((s) => s.selectedNode);
  const openFolder = useExploreStore((s) => s.openFolder);
  const select = useExploreStore((s) => s.select);
  const nameFilter = useViewStore((s) => s.nameFilter);
  const mode = useVizUiStore((s) => s.mode);
  const stage = useCleanupStore((s) => s.stage);

  const chip = useRef<HoverChipHandle>(null);
  const [menu, setMenu] = useState<ItemMenuState | null>(null);
  const [folderView, setFolderView] = useState<NodeDetailsData | null>(null);
  const [counterValue, setCounterValue] = useState("0");
  const [scope, animValue] = useAnimate();

  // Live "N files · X GB" numeric text transition (spec §7 scanning state)
  useEffect(() => {
    if (status !== "scanning" || !progress) return;
    setCounterValue(`${progress.files.toLocaleString()} files · ${bytes(progress.bytes)}`);
  }, [status, progress]);

  useEffect(() => {
    if (status !== "scanning") return;
    const el = scope.current;
    if (!el) return; // ref attaches after paint — null-guard (weak-map crash)
    void animValue(el, { scale: [1, 1.04, 1] }, { duration: 0.5, repeat: Infinity });
    return () => void animValue(el, { scale: 1 }, { duration: 0.1 });
  }, [status, scope, animValue]);

  // Folder header info (name + stats for the current folder)
  useEffect(() => {
    if (status !== "done") {
      setFolderView(null);
      return;
    }
    let disposed = false;
    void (async () => {
      const d = await getNodeDetails(generation, currentFolder).catch(() => null);
      if (!disposed) setFolderView(d);
    })();
    return () => {
      disposed = true;
    };
  }, [status, generation, currentFolder]);

  // ── Hover chip controller (refs; never re-renders on pointer moves) ─
  // A view change orphans a pinned chip: programmatic navigation (mode
  // switch, folder open, new generation) never fires pointerleave, so
  // the chip kept floating over the NEW view (CI tour frames showed it
  // stuck over treemap/top-sizes). Hide on every transition.
  useEffect(() => {
    chip.current?.hide();
  }, [mode, currentFolder, status, generation, nameFilter]);

  const hoverFetch = useCallback(
    (id: number | null, x: number, y: number) => {
      if (id == null || id < 0) {
        chip.current?.hide();
        return;
      }
      chip.current?.move(x, y);
      void (async () => {
        const d = await getHoverDetails(generation, id).catch(() => null);
        if (d) chip.current?.show(
          {
            name: d.name,
            size: d.size,
            shareOfScan: d.shareOfScan,
            fileCount: d.fileCount,
            isDir: d.isDir,
            category: d.category,
            categoryColor: d.categoryColor,
            isCloud: d.isCloud,
            isProtected: d.isProtected,
          },
          x,
          y,
        );
      })();
    },
    [generation],
  );

  const menuResolver = useCallback(
    async (id: number) => {
      const d = await getHoverDetails(generation, id).catch(() => null);
      if (!d) return null;
      const details = await getNodeDetails(generation, id).catch(() => null);
      return {
        id,
        name: details?.name ?? "",
        isDir: d.isDir,
        isProtected: d.isProtected,
        isCloud: d.isCloud,
      };
    },
    [generation],
  );

  // ── Shared actions ──────────────────────────────────────────────────
  const actions = useMemo(
    () => ({
      open: (id: number) => {
        track(EVENTS.searchUsed, { mode });
        openFolder(id);
      },
      preview: (id: number) => onPreview(id),
      reveal: (id: number) => void invoke("reveal_in_explorer", { generation, id }).catch(() => undefined),
      copyPath: (id: number) => {
        void invoke("copy_path", { generation, id }).catch(() => undefined);
      },
      stage: (id: number) => {
        void (async () => {
          const d = await getNodeDetails(generation, id).catch(() => null);
          if (d) {
            stage({ id, path: d.path, size: d.size, reason: "Manual" });
          }
        })();
      },
    }),
    [generation, mode, onPreview, openFolder, stage],
  );

  // Dev-hook auto-start (spec §15 DISKBYTES_SCAN / --scan)
  useEffect(() => {
    if (status !== "idle") return;
    void (async () => {
      const hooks = await invoke<{ scan: string | null; mode: string | null }>("get_dev_hooks").catch(() => null);
      if (hooks?.scan) {
        const startScan = useScanStore.getState().startScan;
        void startScan(hooks.scan);
      }
      if (hooks?.mode) {
        useVizUiStore.getState().setMode(hooks.mode as Mode);
      }
    })();
    // Only on mount — the hook fires once per process.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const revealCurrent = () => {
    void invoke("reveal_in_explorer", { generation, id: currentFolder }).catch(() => undefined);
  };

  const stageNode = () => {
    const id = selectedNode ?? currentFolder;
    if (id == null) return;
    void (async () => {
      const d = await getNodeDetails(generation, id).catch(() => null);
      if (d) {
        stage({ id, path: d.path, size: d.size, reason: "Manual" });
      }
    })();
  };
  void stageNode;

  // ── Render by state ────────────────────────────────────────────────
  if (status === "idle") {
    return (
      <div className="db-main">
        <div className="db-state">
          <span className="db-idle-art">
            <FolderIcon size={34} />
          </span>
          <h2>Map every byte on your PC</h2>
          <p>Scan your whole PC, your Home folder, or any folder — DiskBytes builds a complete tree and shows you exactly where the space went.</p>
          <div className="db-state-actions">
            <button
              type="button"
              className="db-ink-button"
              style={{ width: "auto", padding: "0 20px" }}
              onClick={() => {
                track(EVENTS.scanStarted, { target: "ThisPC" });
                void useScanStore.getState().startScan("ThisPC");
              }}
            >
              <ScanLineIcon size={16} /> Scan This PC
            </button>
            <button
              type="button"
              className="db-outline"
              style={{ width: "auto", padding: "0 18px" }}
              onClick={async () => {
                try {
                  const { open } = await import("@tauri-apps/plugin-dialog");
                  const picked = await open({ directory: true, multiple: false, title: "Scan" });
                  if (typeof picked === "string" && picked.length > 0) {
                    track(EVENTS.scanStarted, { target: "folder" });
                    void useScanStore.getState().startScan(picked);
                  }
                } catch {
                  const home = await invoke<string>("get_home_path").catch(() => null);
                  if (home) void useScanStore.getState().startScan(home);
                }
              }}
            >
              <ExternalLinkIcon size={14} /> Choose Folder…
            </button>
          </div>
        </div>
      </div>
    );
  }

  if (status === "scanning") {
    return (
      <div className="db-main">
        <div className="db-state">
          <span className="db-spinner" />
          <h2>Scanning…</h2>
          <motion.span ref={scope} className="db-live-counter tnum" aria-live="polite">
            {counterValue}
          </motion.span>
          <span className="db-current-path" title={progress?.currentPath ?? ""}>
            {progress?.currentPath ?? ""}
          </span>
        </div>
      </div>
    );
  }

  if (status === "error") {
    const elevation = error?.includes("ELEVATION_REQUIRED");
    return (
      <div className="db-main">
        <div className="db-state db-error">
          <h2>{elevation ? "Administrator rights needed" : "Scan failed"}</h2>
          <p>{elevation ? "This scan target needs elevation. Restart as administrator and it re-runs automatically." : (error ?? "Something went wrong.")}</p>
          <div className="db-state-actions">
            <button
              type="button"
              className="db-ink-button"
              style={{ width: "auto", padding: "0 18px" }}
              onClick={() => {
                if (elevation) {
                  void invoke("restart_as_admin", { scanTarget, turbo: true }).catch(() => undefined);
                } else {
                  void useScanStore.getState().startScan("ThisPC");
                }
              }}
            >
              {elevation ? "Restart as administrator" : "Try again"}
            </button>
          </div>
        </div>
      </div>
    );
  }

  // status === "done"
  const st = folderView;
  return (
    <div className="db-main">
      <div className="db-content-head">
        <div className="db-title-row">
          <div>
            <h1>{st?.name ?? "Scan"}</h1>
            <span className="db-stat size tnum">{st ? bytes(st.size) : ""}</span>
            <i className="db-dot-sep" />
            <span className="db-stat tnum">{(st?.files ?? 0).toLocaleString()} files</span>
            <i className="db-dot-sep" />
            <span className="db-stat tnum">{(st?.folders ?? 0).toLocaleString()} folders</span>
          </div>
          <div className="db-title-actions">
            <button
              type="button"
              className="db-icon-button"
              onClick={revealCurrent}
              aria-label="Show in Explorer"
              title="Show in Explorer"
            >
              <ExternalLinkIcon size={15} />
            </button>
          </div>
        </div>
        <ExploreHeader />
        <UnreadableNotice />
      </div>
      <section className="db-visual-stage db-scroll" aria-label={`${mode} visualization`}>
        {mode === "Folders" && (
          <FoldersMode
            generation={generation}
            folder={currentFolder}
            filter={nameFilter}
            selectedId={selectedNode}
            onSelect={select}
            onOpen={actions.open}
            onPreview={onPreview}
            onContextMenu={(id, x, y) => setMenu({ id, x, y })}
            onHover={hoverFetch}
          />
        )}
        {CANVAS_MODES.has(mode) && (
          <CanvasMode
            generation={generation}
            folder={currentFolder}
            mode={mode}
            selectedId={selectedNode}
            onSelect={select}
            onOpen={actions.open}
            onContextMenu={(id, x, y) => setMenu({ id, x, y })}
            onHover={hoverFetch}
          />
        )}
        {mode === "Top Sizes" && (
          <TopSizesMode
            generation={generation}
            folder={currentFolder}
            filter={nameFilter}
            selectedId={selectedNode}
            onSelect={select}
            onOpen={actions.open}
            onContextMenu={(id, x, y) => setMenu({ id, x, y })}
            onHover={hoverFetch}
          />
        )}
        {mode === "Age Map" && (
          <AgeMapMode
            generation={generation}
            folder={currentFolder}
            onSelect={select}
            onHover={hoverFetch}
            onContextMenu={(id, x, y) => setMenu({ id, x, y })}
          />
        )}
        {mode === "List" && (
          <ListMode
            generation={generation}
            folder={currentFolder}
            filter={nameFilter}
            selectedId={selectedNode}
            onSelect={select}
            onOpen={actions.open}
            onContextMenu={(id, x, y) => setMenu({ id, x, y })}
            onHover={hoverFetch}
          />
        )}
      </section>

      <HoverChip ref={chip} sizeFmt={bytes} />
      <ItemContextMenu target={menu} actions={actions} resolver={menuResolver} onClose={() => setMenu(null)} />
    </div>
  );
}

export { EmptyState };
