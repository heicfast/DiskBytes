/**
 * Sidebar §3 (spec §6.5): volume label + usage ring (conic gradient,
 * "xx.x% USED" center) + Total / Used (red) / Free (green) from
 * `disk_storage` (GetDiskFreeSpaceExW on the scan root's volume).
 */
import { useEffect, useState } from "react";
import { SectionCaption } from "../components/buttons";
import { invoke } from "../lib/ipc";
import { bytes } from "../lib/format";
import { useScanStore } from "../state/scan";

interface StorageInfo {
  label: string;
  total: number;
  used: number;
  free: number;
  usedPct: number;
}

export function StorageSection() {
  const generation = useScanStore((s) => s.generation);
  const status = useScanStore((s) => s.status);
  const [info, setInfo] = useState<StorageInfo | null>(null);

  useEffect(() => {
    if (status !== "done") return;
    let disposed = false;
    void (async () => {
      const res = await invoke<StorageInfo>("disk_storage").catch(() => null);
      if (!disposed) setInfo(res);
    })();
    return () => {
      disposed = true;
    };
  }, [generation, status]);

  if (!info) return null;
  const pct = Math.round(info.usedPct * 1000) / 10;

  return (
    <>
      <SectionCaption right={info.label}>Disk Storage</SectionCaption>
      <div className="db-storage">
        <div className="db-ring" style={{ ["--pct" as string]: pct }}>
          <strong>{pct}%</strong>
          <span>used</span>
        </div>
        <dl>
          <div>
            <dt>Total</dt>
            <dd className="tnum">{bytes(info.total)}</dd>
          </div>
          <div>
            <dt>Used</dt>
            <dd className="used tnum">{bytes(info.used)}</dd>
          </div>
          <div>
            <dt>Free</dt>
            <dd className="free tnum">{bytes(info.free)}</dd>
          </div>
        </dl>
      </div>
    </>
  );
}
