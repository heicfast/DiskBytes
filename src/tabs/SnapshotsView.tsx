/**
 * Snapshots tab (spec §13): Take Snapshot Now (disabled without a scan
 * or while saving), rows with path / "size · date" / Before & After
 * toggles / delete; diff = top-200 |change| with +red / −green and
 * different-roots notice.
 */
import { useEffect, useState } from "react";
import { Clock3Icon, CameraIcon, Trash2Icon } from "../components/Icon";
import { TailPath } from "../components/TailPath";
import { EmptyState } from "../components/buttons";
import { invoke } from "../lib/ipc";
import { bytes } from "../lib/format";
import { useScanStore } from "../state/scan";
import { useExploreStore } from "../state/explore";

interface SnapshotView {
  id: string;
  root: string;
  takenAt: number;
  total: number;
  folders: number;
}

interface DiffView {
  sameRoot: boolean;
  totalBefore: number;
  totalAfter: number;
  changes: { path: string; before: number; after: number; delta: number }[];
}

function fmtDate(unix: number): string {
  return new Date(unix * 1000).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}

export function SnapshotsView() {
  const status = useScanStore((s) => s.status);
  const generation = useScanStore((s) => s.generation);
  const currentFolder = useExploreStore((s) => s.currentFolder);
  const [list, setList] = useState<SnapshotView[] | null>(null);
  const [saving, setSaving] = useState(false);
  const [before, setBefore] = useState<string | null>(null);
  const [after, setAfter] = useState<string | null>(null);
  const [diff, setDiff] = useState<DiffView | null>(null);
  const [diffing, setDiffing] = useState(false);

  const refresh = async () => {
    try {
      setList(await invoke<SnapshotView[]>("list_snapshots"));
    } catch {
      setList([]);
    }
  };

  useEffect(() => {
    void refresh();
  }, []);

  const take = async () => {
    setSaving(true);
    try {
      await invoke("take_snapshot", { generation, node: currentFolder });
      await refresh();
    } catch {
      /* snapshot failed — the button re-enables */
    } finally {
      setSaving(false);
    }
  };

  const runDiff = async (b: string, a: string) => {
    setBefore(b);
    setAfter(a);
    setDiffing(true);
    setDiff(null);
    try {
      setDiff(await invoke<DiffView>("diff_snapshots", { beforeId: b, afterId: a }));
    } catch {
      setDiff(null);
    } finally {
      setDiffing(false);
    }
  };

  const del = async (id: string) => {
    await invoke("delete_snapshot", { id }).catch(() => undefined);
    if (before === id) setBefore(null);
    if (after === id) setAfter(null);
    void refresh();
  };

  if (status === "idle" || status === "scanning") {
    return (
      <div className="db-tab db-scroll">
        <EmptyState icon={<Clock3Icon size={28} />} title="Snapshots" body={status === "scanning" ? "Scan in progress — snapshots capture the tree when it’s done." : "Before/after diffs of any two scans of the same root."} />
      </div>
    );
  }

  return (
    <div className="db-tab db-scroll">
      <div className="db-tab-head">
        <div>
          <h1>Snapshots</h1>
          <span className="db-tab-sub">{list ? `${list.length} saved · pick Before + After to diff` : "…"}</span>
        </div>
        {(list ?? []).length > 0 && (
          <div className="db-tab-head-actions">
            <button
              type="button"
              className="db-ink-button"
              style={{ width: "auto", padding: "0 18px" }}
              disabled={saving || status !== "done"}
              onClick={() => void take()}
            >
              <CameraIcon size={15} />
              {saving ? "Saving…" : "Take Snapshot Now"}
            </button>
          </div>
        )}
      </div>

      {(list ?? []).length === 0 && (
        <EmptyState
          icon={<CameraIcon size={28} />}
          title="No snapshots yet"
          body="Take a snapshot now, run your cleanup, then take another — the diff shows exactly what changed."
          action={
            <button
              type="button"
              className="db-ink-button"
              disabled={saving || status !== "done"}
              onClick={() => void take()}
            >
              <CameraIcon size={15} />
              {saving ? "Saving…" : "Take Snapshot Now"}
            </button>
          }
        />
      )}

      {(list ?? []).map((s) => (
        <div className="db-snap-row" key={s.id}>
          <Clock3Icon size={15} />
          <div>
            <strong>{s.root}</strong>
            <small>
              <b>{bytes(s.total)}</b> · {fmtDate(s.takenAt)} · {s.folders.toLocaleString()} folders
            </small>
          </div>
          <div className="db-snap-toggle" role="group" aria-label="Before or after">
            <button type="button" data-active={before === s.id} onClick={() => before === s.id ? setBefore(null) : before && after && before !== s.id ? void runDiff(s.id, after) : setBefore(s.id)}>
              Before
            </button>
            <button type="button" data-active={after === s.id} onClick={() => after === s.id ? setAfter(null) : before && after !== s.id ? void runDiff(before, s.id) : setAfter(s.id)}>
              After
            </button>
          </div>
          {before && after && (
            <button type="button" className="db-outline compact" style={{ width: "auto" }} onClick={() => void runDiff(before, after)} disabled={diffing}>
              {diffing ? "Diffing…" : "Compare"}
            </button>
          )}
          <button type="button" className="db-icon-button" style={{ width: 30, height: 30 }} aria-label={`Delete snapshot ${s.id}`} onClick={() => void del(s.id)}>
            <Trash2Icon size={13} />
          </button>
        </div>
      ))}

      {before && after && (
        <div>
          <div className="db-tab-head" style={{ marginTop: 22, marginBottom: 6 }}>
            <div>
              <h1 style={{ fontSize: 17 }}>Changes</h1>
              <span className="db-tab-sub">
                {diff ? (
                  <>
                    {bytes(diff.totalBefore)} → <b>{bytes(diff.totalAfter)}</b> · {diff.changes.length} changed folders
                  </>
                ) : (
                  "…"
                )}
              </span>
            </div>
          </div>
          {diff && !diff.sameRoot && (
            <div className="db-pop-failed" style={{ margin: "0 0 10px" }}>
              <strong>Different roots</strong>
              <p style={{ margin: 0, fontSize: 11 }}>These snapshots were taken at different roots — the diff is not meaningful.</p>
            </div>
          )}
          {diffing && (
            <div className="db-loading-block">
              <span className="db-spinner" style={{ width: 16, height: 16, borderWidth: 2 }} />
              <span>Loading both snapshots…</span>
            </div>
          )}
          {diff?.changes.map((c) => {
            const maxAbs = Math.max(
              1,
              ...diff.changes.map((x) => Math.abs(x.delta)),
            );
            const pct = Math.max(2, (Math.abs(c.delta) / maxAbs) * 100);
            return (
              <div className="db-diff-row" key={c.path} title={c.path}>
                <span className="db-diff-path"><TailPath path={c.path} /></span>
                <em className="tnum">{bytes(c.before)}</em>
                {c.delta > 0 ? (
                  <b className="grew tnum">+{bytes(c.delta)}</b>
                ) : (
                  <b className="shrank tnum">−{bytes(Math.abs(c.delta))}</b>
                )}
                <span
                  className="db-diff-bar"
                  aria-hidden
                >
                  <i
                    style={{
                      width: `${pct}%`,
                      background: c.delta > 0 ? "var(--used)" : "var(--free)",
                    }}
                  />
                </span>
              </div>
            );
          })}
          {diff && diff.changes.length === 0 && <div className="db-substate">No folder changed between these snapshots.</div>}
        </div>
      )}
    </div>
  );
}
