/**
 * Shared button vocabulary (BuildPrompt §14.3 / doc 05 §1.3) in the
 * prototype's visual language:
 *   InkButton      — filled accent, the ONE primary action per surface
 *   OutlineButton  — card fill + border; optional transient confirm text
 *   SquareIconButton — 32×32 icon hit target
 *   SectionCaption — uppercase, tracked, tertiary label
 * All of them are `.pressable` (scale 0.985 / opacity 0.75 on press).
 */
import { useCallback, useState, type ButtonHTMLAttributes, type ReactNode } from "react";
import { CheckIcon } from "./Icon";

export interface InkButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  children: ReactNode;
}

export function InkButton({ children, className, ...rest }: InkButtonProps) {
  return (
    <button type="button" className={`db-ink-button ${className ?? ""}`} {...rest}>
      {children}
    </button>
  );
}

export interface OutlineButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  children: ReactNode;
  /** When set, a successful click swaps the label for this text for ~1s. */
  confirmText?: string;
  /** Fires together with the transient confirm. */
  onConfirm?: () => void;
}

export function OutlineButton({ children, confirmText, onConfirm, className, onClick, ...rest }: OutlineButtonProps) {
  const [confirmed, setConfirmed] = useState(false);
  const handleClick = useCallback(
    (e: React.MouseEvent<HTMLButtonElement>) => {
      onClick?.(e);
      if (confirmText && !e.defaultPrevented) {
        onConfirm?.();
        setConfirmed(true);
        window.setTimeout(() => setConfirmed(false), 1000);
      }
    },
    [onClick, confirmText, onConfirm],
  );
  return (
    <button
      type="button"
      className={`db-outline ${className ?? ""}`}
      onClick={handleClick}
      {...rest}
    >
      {confirmText && confirmed ? (
        <>
          <CheckIcon size={14} /> {confirmText}
        </>
      ) : (
        children
      )}
    </button>
  );
}

export interface SquareIconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  children: ReactNode;
  active?: boolean;
}

export function SquareIconButton({ label, children, active, className, ...rest }: SquareIconButtonProps) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      className={`db-icon-button ${active ? "is-active" : ""} ${className ?? ""}`}
      {...rest}
    >
      {children}
    </button>
  );
}

export function SectionCaption({ children, right }: { children: ReactNode; right?: ReactNode }) {
  return (
    <div className="db-section-title">
      <span>{children}</span>
      {right ? <span className="db-section-meta">{right}</span> : null}
    </div>
  );
}

/** Standard row list used across tabs (dupes groups, snapshot rows…). */
export function EmptyState({
  icon,
  title,
  body,
  action,
}: {
  icon: ReactNode;
  title: string;
  body: string;
  action?: ReactNode;
}) {
  return (
    <div className="db-empty">
      <span className="db-empty-icon">{icon}</span>
      <h1>{title}</h1>
      <p>{body}</p>
      {action}
    </div>
  );
}

/** Premium dual-arc loading spinner (replaces the old single-arc
 * border-top spinner — dated and visually noisy). The design language:
 * a faint full-ring track grounds the spinner, the main ~100° arc
 * sweeps with an ease-in-out rotation (the iOS/macOS "organic sweep")
 * while a smaller inner arc counter-rotates for depth. Compositor-only
 * (transform), round linecaps, currentColor. */
export function Spinner({ size = 22, weight = 2.6 }: { size?: number; weight?: number }) {
  return (
    <svg className="db-spinner" width={size} height={size} viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <circle cx="12" cy="12" r="9.4" stroke="currentColor" className="db-spin-track" strokeWidth={Math.max(1.4, weight - 1.1)} />
      <circle cx="12" cy="12" r="9.4" stroke="currentColor" className="db-spin-arc" strokeWidth={weight} strokeLinecap="round" strokeDasharray="16.4 42.7" />
      <circle cx="12" cy="12" r="5.7" stroke="currentColor" className="db-spin-arc-rev" strokeWidth={Math.max(1.6, weight - 0.7)} strokeLinecap="round" strokeDasharray="7.5 28.3" />
    </svg>
  );
}
