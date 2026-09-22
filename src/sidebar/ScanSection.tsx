/**
 * Sidebar §1 (spec §6.1–6.3): the big scan ink button, Home / Folder…
 * outline pair, fixed-drive chips. Every action switches to Explore
 * (spec: "Sidebar actions always switch to the Explore tab").
 */
import { useEffect, useState } from "react";
import { FolderIcon, HardDriveIcon, HomeIcon, ScanLineIcon } from "../components/Icon";
import { OutlineButton } from "../components/buttons";
import { SCAN_THIS_PC } from "../lib/platform";
import { invoke } from "../lib/ipc";
import { useScanStore } from "../state/scan";
import { useViewStore } from "../state/view";
import { EVENTS, track } from "../lib/analytics";

interface DriveChip {
  letter: string;
  target: string;
}

export function ScanSection() {
  const startScan = useScanStore((s) => s.startScan);
  const status = useScanStore((s) => s.status);
  const setTab = useViewStore((s) => s.setTab);
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

  const pickFolder = async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const picked = await open({ directory: true, multiple: false, title: "Scan" });
      if (typeof picked === "string" && picked.length > 0) {
        scan(picked);
      }
    } catch {
      // Dialog unavailable (plain browser mock): fall back to Home.
      if (home) scan(home);
    }
  };

  const busy = status === "scanning";

  return (
    <section aria-label="Scan targets">
      <button type="button" className="db-ink-button" disabled={busy} onClick={() => scan("ThisPC")}>
        <ScanLineIcon size={17} />
        {SCAN_THIS_PC}
      </button>
      <div className="db-sidebar-actions">
        <OutlineButton onClick={() => home && scan(home)} disabled={busy || !home}>
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
