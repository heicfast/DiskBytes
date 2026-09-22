/**
 * Sidebar §1 (spec §6.1–6.3): the big scan ink button, Home / Folder…
 * outline pair, fixed-drive chips. Every action switches to Explore
 * (spec: "Sidebar actions always switch to the Explore tab").
 *
 * State-aware behavior (senior-UX rules):
 * - While a scan runs, the primary button morphs into "Stop scan" so
 *   there is ALWAYS a way to cancel (the scan store reverts to the
 *   previous tree optimistically).
 * - Home navigates INTO the current tree when the home path was part
 *   of the last scan (instant, no rescan); it only starts a new scan
 *   when no tree exists or the path is outside it.
 * - Drive chips stay scan actions ("Scan C:") — that is their contract.
 */
import { useEffect, useState } from "react";
import { FolderIcon, HardDriveIcon, HomeIcon, ScanLineIcon, SquareIcon } from "../components/Icon";
import { OutlineButton } from "../components/buttons";
import { SCAN_THIS_PC } from "../lib/platform";
import { invoke } from "../lib/ipc";
import { useExploreStore } from "../state/explore";
import { useScanStore } from "../state/scan";
import { useViewStore } from "../state/view";
import { EVENTS, track } from "../lib/analytics";

interface DriveChip {
  letter: string;
  target: string;
}

export function ScanSection() {
  const startScan = useScanStore((s) => s.startScan);
  const cancelScan = useScanStore((s) => s.cancelScan);
  const status = useScanStore((s) => s.status);
  const generation = useScanStore((s) => s.generation);
  const setTab = useViewStore((s) => s.setTab);
  const openFolder = useExploreStore((s) => s.openFolder);
  const [drives, setDrives] = useState<DriveChip[]>([]);
  const [home, setHome] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;
    void (async () => {
      const chips = await invoke<DriveChip[]>("get_drive_chips").catch(() => null);
      const homePath = await invoke<string>("get_home_path").catch(() => null);
      if (!disposed) {
        setDrives(chips ?? []);
        setHome(homePath);
      }
    })();
    return () => {
      disposed = true;
    };
  }, []);

  const scan = (target: string) => {
    setTab("explore");
    track(EVENTS.scanStarted, { target });
    void startScan(target);
  };

  /** Navigate-first Home: jump to the home folder inside the CURRENT
   * scan when possible; fall back to scanning it. Never wipes a
   * finished view just to move somewhere. */
  const goHome = async () => {
    if (!home) return;
    setTab("explore");
    if (status === "done") {
      try {
        const id = await invoke<number | null>("resolve_path", { generation, path: home });
        if (id != null) {
          openFolder(id);
          return;
        }
      } catch {
        /* resolve failed: fall through to a fresh scan */
      }
    }
    scan(home);
  };

  const pickFolder = async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const picked = await open({ directory: true, multiple: false, title: "Scan" });
      if (typeof picked === "string" && picked.length > 0) {
        scan(picked);
      }
    } catch {
      // Dialog unavailable (plain browser mock): fall back to Home.
      if (home) void goHome();
    }
  };

  const busy = status === "scanning";

  return (
    <section aria-label="Scan targets">
      {busy ? (
        <button type="button" className="db-ink-button db-stop-scan" onClick={() => void cancelScan()}>
          <SquareIcon size={15} />
          Stop scan
        </button>
      ) : (
        <button type="button" className="db-ink-button" onClick={() => scan("ThisPC")}>
          <ScanLineIcon size={17} />
          {SCAN_THIS_PC}
        </button>
      )}
      <div className="db-sidebar-actions">
        <OutlineButton
          onClick={() => void goHome()}
          disabled={busy || !home}
          title={busy ? "A scan is running" : home ? `Go to ${home}` : undefined}
        >
          <HomeIcon size={14} /> Home
        </OutlineButton>
        <OutlineButton onClick={() => void pickFolder()} disabled={busy}>
          <FolderIcon size={14} /> Folder…
        </OutlineButton>
      </div>
      {drives.length > 0 && (
        <div className="db-drives" role="group" aria-label="Drives">
          {drives.map((d) => (
            <button
              key={d.target}
              type="button"
              className="db-chip"
              disabled={busy}
              onClick={() => scan(d.target)}
              title={`Scan ${d.letter}`}
            >
              <HardDriveIcon size={12} />
              {d.letter}
            </button>
          ))}
        </div>
      )}
    </section>
  );
}
