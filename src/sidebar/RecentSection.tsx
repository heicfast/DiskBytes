/**
 * Sidebar §2 (spec §6.4): the last 5 scanned paths in localStorage,
 * one click rescans.
 */
import { useEffect, useState } from "react";
import { Clock3Icon } from "../components/Icon";
import { SectionCaption } from "../components/buttons";
import { useScanStore } from "../state/scan";
import { useViewStore } from "../state/view";

const KEY = "diskbytes.recent";
const MAX = 5;

export function pushRecent(path: string): void {
  try {
    const list: string[] = JSON.parse(window.localStorage.getItem(KEY) ?? "[]");
    const next = [path, ...list.filter((p) => p !== path)].slice(0, MAX);
    window.localStorage.setItem(KEY, JSON.stringify(next));
  } catch {
    /* storage unavailable — Recent simply stays empty */
  }
}

export function RecentSection() {
  const [recent, setRecent] = useState<string[]>([]);
  const startScan = useScanStore((s) => s.startScan);
  const setTab = useViewStore((s) => s.setTab);

  useEffect(() => {
    try {
      setRecent(JSON.parse(window.localStorage.getItem(KEY) ?? "[]"));
    } catch {
      setRecent([]);
    }
    const onFocus = () => {
      try {
        setRecent(JSON.parse(window.localStorage.getItem(KEY) ?? "[]"));
      } catch {
        /* ignore */
      }
    };
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, []);

  if (recent.length === 0) return null;

  return (
    <>
      <SectionCaption>Recent</SectionCaption>
      <div className="db-recent-list">
        {recent.map((p) => (
          <button
            key={p}
            type="button"
            className="db-recent"
            title={`Rescan ${p}`}
            onClick={() => {
              setTab("explore");
              void startScan(p);
            }}
          >
            <Clock3Icon size={13} />
            <span>{p}</span>
          </button>
        ))}
      </div>
    </>
  );
}
