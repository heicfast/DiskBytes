/**
 * Sidebar §2 (spec §6.4): the last 2 scanned paths in localStorage,
 * one click navigates into the current tree (no rescan) when the path
 * is inside it — a fresh scan only when it isn't (senior-UX: recents
 * are destinations first, scan targets second).
 */
import { useEffect, useState } from "react";
import { Clock3Icon } from "../components/Icon";
import { SectionCaption } from "../components/buttons";
import { invoke } from "../lib/ipc";
import { useExploreStore } from "../state/explore";
import { useScanStore } from "../state/scan";
import { useViewStore } from "../state/view";

const KEY = "diskbytes.recent";
const MAX = 2;
/** RecentSection listens for this after every push so the list updates
 * live in the same page (no focus/reload needed). */
const RECENTS_EVENT = "diskbytes.recents-changed";

export function pushRecent(path: string): void {
  try {
    const list: string[] = JSON.parse(window.localStorage.getItem(KEY) ?? "[]");
    const next = [path, ...list.filter((p) => p !== path)].slice(0, MAX);
    window.localStorage.setItem(KEY, JSON.stringify(next));
    window.dispatchEvent(new CustomEvent(RECENTS_EVENT));
  } catch {
    /* storage unavailable — Recent simply stays empty */
  }
}

export function RecentSection() {
  const [recent, setRecent] = useState<string[]>([]);
  const startScan = useScanStore((s) => s.startScan);
  const status = useScanStore((s) => s.status);
  const generation = useScanStore((s) => s.generation);
  const openFolder = useExploreStore((s) => s.openFolder);
  const setTab = useViewStore((s) => s.setTab);

  useEffect(() => {
    const read = () => {
      try {
        setRecent(JSON.parse(window.localStorage.getItem(KEY) ?? "[]"));
      } catch {
        setRecent([]);
      }
    };
    read();
    const onFocus = () => read();
    const onRecents = () => read();
    window.addEventListener("focus", onFocus);
    window.addEventListener(RECENTS_EVENT, onRecents);
    return () => {
      window.removeEventListener("focus", onFocus);
      window.removeEventListener(RECENTS_EVENT, onRecents);
    };
  }, []);

  if (recent.length === 0) return null;

  /** Navigate into the current tree when possible; scan otherwise. */
  const open = async (p: string) => {
    setTab("explore");
    if (status === "done") {
      try {
        const id = await invoke<number | null>("resolve_path", { generation, path: p });
        if (id != null) {
          openFolder(id);
          return;
        }
      } catch {
        /* fall through to a fresh scan */
      }
    }
    void startScan(p);
  };

  return (
    <>
      <SectionCaption>Recent</SectionCaption>
      <div className="db-recent-list">
        {recent.map((p) => (
          <button
            key={p}
            type="button"
            className="db-recent"
            title={status === "done" ? `Go to ${p}` : `Scan ${p}`}
            onClick={() => void open(p)}
          >
            <Clock3Icon size={13} />
            <span>{p}</span>
          </button>
        ))}
      </div>
    </>
  );
}
