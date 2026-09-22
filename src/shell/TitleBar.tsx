/**
 * Windows caption buttons (minimize / maximize-restore / close) that mount
 * into the top bar's right end (Windows 11 app convention — Files,
 * Terminal, PowerToys). macOS renders nothing: `titleBarStyle: Overlay`
 * draws the native traffic lights over the top bar's left reserve.
 *
 * Glyphs use the exact Windows 11 caption geometry (thin 1.7 stroke,
 * square corners, L-clipped restore square) from Icon.tsx.
 */
import { useEffect, useState } from "react";
import {
  CaptionCloseIcon, CaptionMaximizeIcon, CaptionMinimizeIcon, CaptionRestoreIcon,
} from "../components/Icon";
import { IS_MAC } from "../lib/platform";

interface WindowApi {
  minimize: () => Promise<void>;
  toggleMaximize: () => Promise<void>;
  close: () => Promise<void>;
  isMaximized: () => Promise<boolean>;
  onResized: (cb: () => void) => Promise<() => void>;
}

function useWindowApi(): WindowApi | null {
  const [api, setApi] = useState<WindowApi | null>(null);
  useEffect(() => {
    let disposed = false;
    import("@tauri-apps/api/window")
      .then(({ getCurrentWindow }) => {
        const w = getCurrentWindow();
        setApi({
          minimize: () => w.minimize(),
          toggleMaximize: () => w.toggleMaximize(),
          close: () => w.close(),
          isMaximized: () => w.isMaximized(),
          onResized: (cb) => w.onResized(cb),
        });
        return undefined;
      })
      .catch(() => {
        if (!disposed) setApi(null); // plain browser (mock) — no chrome
      });
    return () => {
      disposed = true;
    };
  }, []);
  return api;
}

/** Shared window API for drag regions (double-click maximize etc.). */
export function useWindowControls() {
  return useWindowApi();
}

export function CaptionButtons() {
  const api = useWindowApi();
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    if (!api) return;
    let disposed = false;
    const refresh = () => {
      api.isMaximized().then((m) => {
        if (!disposed) setMaximized(m);
      }).catch(() => undefined);
    };
    refresh();
    void api.onResized(refresh).then((un) => {
      if (disposed) {
        un();
      } else {
        unlistenRef.current = un;
      }
    });
    const unlistenRef = { current: null as null | (() => void) };
    return () => {
      disposed = true;
      unlistenRef.current?.();
    };
  }, [api]);

  if (IS_MAC || !api) return null;

  return (
    <div className="db-caption" role="group" aria-label="Window controls">
      <button aria-label="Minimize" title="Minimize" onClick={() => void api.minimize()}>
        <CaptionMinimizeIcon size={15} />
      </button>
      <button
        aria-label={maximized ? "Restore" : "Maximize"}
        title={maximized ? "Restore" : "Maximize"}
        onClick={() => void api.toggleMaximize()}
      >
        {maximized ? <CaptionRestoreIcon size={15} /> : <CaptionMaximizeIcon size={15} />}
      </button>
      <button className="db-close" aria-label="Close" title="Close" onClick={() => void api.close()}>
        <CaptionCloseIcon size={15} />
      </button>
    </div>
  );
}
