/**
 * Title bar (spec §3): Windows caption buttons (46×32, close hover
 * #E81123, double-click maximize) on a drag region; on macOS the same
 * bar is a plain drag region with traffic-light reserve (the native
 * overlay controls render above it — cross-platform doc §4 Option A).
 */
import { useEffect, useState } from "react";
import { MinusIcon, SquareIcon, XIcon, Maximize2Icon } from "../components/Icon";
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

export function TitleBar() {
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

  return (
    <div className="db-titlebar" data-os={IS_MAC ? "macos" : "windows"} role="banner">
      <div className="db-titlebar-drag" data-tauri-drag-region onDoubleClick={() => void api?.toggleMaximize()}>
        <span className="db-titlebar-title" data-tauri-drag-region>
          DiskBytes
        </span>
      </div>
      {!IS_MAC && (
        <div className="db-caption">
          <button aria-label="Minimize" title="Minimize" onClick={() => void api?.minimize()}>
            <MinusIcon size={15} />
          </button>
          <button aria-label={maximized ? "Restore" : "Maximize"} title={maximized ? "Restore" : "Maximize"} onClick={() => void api?.toggleMaximize()}>
            {maximized ? <SquareIcon size={13} /> : <Maximize2Icon size={13} />}
          </button>
          <button className="db-close" aria-label="Close" title="Close" onClick={() => void api?.close()}>
            <XIcon size={15} />
          </button>
        </div>
      )}
    </div>
  );
}
