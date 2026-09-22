/**
 * Cleanup Queue popover (spec §9): 460×520 portal, opaque background,
 * click-outside/Esc; header (staged total, Clear, red Move-to-Recycle-
 * Bin), rows with reason + ✕, tray empty state, confirmation dialog
 * ("Items go to the Recycle Bin. Space is only freed when you empty
 * it." + Open Recycle Bin link) and a per-item failure alert.
 */
import { useEffect, useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { CheckIcon, Trash2Icon, XIcon } from "./Icon";
import { useCleanupStore } from "../state/cleanup";
import { useLicenseStore } from "../state/license";
import { bytes } from "../lib/format";
import { BIN_NAME, IS_MAC } from "../lib/platform";
import { invoke } from "../lib/ipc";

export interface CommitFailure {
  path: string;
  reason: string;
}

export function CleanupQueuePopover({
  open,
  onClose,
  anchor,
}: {
  open: boolean;
  onClose: () => void;
  anchor: "topbar";
}) {
  const items = useCleanupStore((s) => s.items);
  const remove = useCleanupStore((s) => s.remove);
  const clear = useCleanupStore((s) => s.clear);
  const commit = useCleanupStore((s) => s.commitToRecycleBin);
  const license = useLicenseStore((s) => s.status);
  const [confirming, setConfirming] = useState(false);
  const [committing, setCommitting] = useState(false);
  const [failure, setFailure] = useState<{ count: number; failed: CommitFailure[]; error: string | null } | null>(null);
  const popRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) {
      setConfirming(false);
      setFailure(null);
      return;
    }
    const onDown = (e: PointerEvent) => {
      if (popRef.current && !popRef.current.contains(e.target as Node)) onClose();
    };
    const esc = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (confirming) setConfirming(false);
        else onClose();
      }
    };
    window.addEventListener("pointerdown", onDown, true);
    window.addEventListener("keydown", esc);
    return () => {
      window.removeEventListener("pointerdown", onDown, true);
      window.removeEventListener("keydown", esc);
    };
  }, [open, onClose, confirming]);

  if (!open) return null;

  const total = items.reduce((a, i) => a + i.size, 0);
  const freeCap = license?.freeCommitCap ?? 0;
  const overFreeCap = !license?.isPro && freeCap > 0 && total > freeCap;

  const doCommit = async () => {
    setCommitting(true);
    setFailure(null);
    try {
      const result = await commit();
      const failed = result.failed;
      if (failed.length > 0) {
        setFailure({ count: failed.length, failed, error: null });
      } else {
        setConfirming(false);
        onClose();
      }
    } catch (e) {
      setFailure({ count: 0, failed: [], error: String(e) });
    } finally {
      setCommitting(false);
    }
  };

  return (
    <>
      <AnimatePresence>
        <motion.div
          ref={popRef}
          className="db-pop"
          style={anchor === "topbar" ? { right: 18, top: 96 } : undefined}
          role="dialog"
          aria-label="Cleanup Queue"
        >
          <div className="db-pop-head">
            <div>
              <h3>Cleanup Queue</h3>
              <span className="db-pop-total tnum">{items.length > 0 ? `${bytes(total)} staged` : "Nothing staged"}</span>
            </div>
            <button type="button" className="db-pop-close" onClick={onClose} aria-label="Close">
              <XIcon size={15} />
            </button>
          </div>
          <div className="db-pop-actions">
            <button type="button" className="db-btn-clear" disabled={items.length === 0} onClick={clear}>
              Clear
            </button>
            <button
              type="button"
              className="db-btn-commit"
              disabled={items.length === 0 || committing || overFreeCap}
              title={overFreeCap ? `Free tier caps cleanup at ${bytes(freeCap)} — activate DiskBytes Pro to clean more` : undefined}
              onClick={() => setConfirming(true)}
            >
              <Trash2Icon size={14} />
              {committing ? "Moving…" : `Move to ${BIN_NAME}…`}
            </button>
          </div>
          {failure && (
            <div className="db-pop-failed" role="alert">
              <strong>{failure.error ? "Commit failed" : `Couldn’t recycle ${failure.count} item(s)`}</strong>
              {failure.error ? (
                <p style={{ margin: 0, fontSize: 11 }}>{failure.error}</p>
              ) : (
                <ul>
                  {failure.failed.slice(0, 8).map((f) => (
                    <li key={f.path} title={f.path}>
                      {f.reason} — {f.path}
                    </li>
                  ))}
                </ul>
              )}
              <button
                type="button"
                className="db-outline compact"
                style={{ marginTop: 8, width: "auto", padding: "0 12px" }}
                onClick={() => setFailure(null)}
              >
                Dismiss
              </button>
            </div>
          )}
          {items.length === 0 ? (
            <div className="db-pop-empty">
              <span className="db-pop-empty-icon">
                <Trash2Icon size={24} />
              </span>
              <p>Nothing staged yet — pick folders or files you want gone, then commit them in one move.</p>
            </div>
          ) : (
            <div className="db-pop-list db-scroll">
              {items.map((i) => (
                <div key={`${i.id}:${i.path}`} className="db-pop-row">
                  <Trash2Icon size={14} />
                  <div className="db-pop-item">
                    <strong title={i.path}>{i.path}</strong>
                    <small>{i.reason}</small>
                  </div>
                  <b className="tnum">{bytes(i.size)}</b>
                  <button
                    type="button"
                    className="db-pop-remove"
                    aria-label={`Remove ${i.path}`}
                    onClick={() => remove(i.id)}
                  >
                    <XIcon size={13} />
                  </button>
                </div>
              ))}
            </div>
          )}
        </motion.div>
      </AnimatePresence>

      {confirming && (
        <div className="db-scrim" role="dialog" aria-modal="true">
          <div className="db-dialog">
            <h3>Move {items.length.toLocaleString()} item{items.length === 1 ? "" : "s"} to the {BIN_NAME}?</h3>
            <p>
              {items.length.toLocaleString()} item{items.length === 1 ? "" : "s"} · {bytes(total)} of data.{" "}
              {IS_MAC
                ? `Items go to the Trash. Space is only freed when you empty it.`
                : `Items go to the Recycle Bin. Space is only freed when you empty it.`}
            </p>
            <button
              type="button"
              className="db-dialog-link"
              onClick={() => void invoke("open_recycle_bin").catch(() => undefined)}
            >
              <CheckIcon size={12} /> Open {BIN_NAME}
            </button>
            <div className="db-dialog-actions">
              <button type="button" className="db-outline" style={{ width: "auto", padding: "0 16px" }} onClick={() => setConfirming(false)}>
                Cancel
              </button>
              <button
                type="button"
                className="db-ink-button"
                style={{ width: "auto", padding: "0 18px", background: "var(--used)" }}
                disabled={committing}
                onClick={() => void doCommit()}
              >
                <Trash2Icon size={14} />
                {committing ? "Moving…" : `Move to ${BIN_NAME}`}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
