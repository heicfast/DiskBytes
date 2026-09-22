/**
 * DiskBytes app shell (spec §3): title bar → 56px top bar → body
 * (sidebar 340px | main | inspector 382px on Explore only), 1px
 * dividers. Hosts the 5 tabs, the inspector, the preview overlay, the
 * cleanup queue popover, and the license dialog. Also mounts the
 * DISKBYTES_TOUR driver (dev hook §15 — CI screenshot tours).
 */
import { useCallback, useEffect, useState } from "react";

import { TitleBar } from "./shell/TitleBar";
import { TopBar } from "./shell/TopBar";
import { Sidebar } from "./sidebar";
import { ExploreView } from "./explore/ExploreView";
import { DuplicatesView } from "./tabs/DuplicatesView";
import { ApplicationsView } from "./tabs/ApplicationsView";
import { MonitorView } from "./tabs/MonitorView";
import { SnapshotsView } from "./tabs/SnapshotsView";
import { InspectorPanel } from "./inspector/InspectorPanel";
import { PreviewOverlay } from "./components/PreviewOverlay";
import { CleanupQueuePopover } from "./components/CleanupQueuePopover";
import { LicenseDialog } from "./components/LicenseDialog";
import { useViewStore } from "./state/view";
import { useScanStore } from "./state/scan";
import { useExploreStore } from "./state/explore";
import { useLicenseStore, attachLicenseEvents } from "./state/license";
import { getBreadcrumb, type CrumbData } from "./viz/exploreIpc";
import { invoke } from "./lib/ipc";
import { pushRecent } from "./sidebar/RecentSection";
import { TourDriver } from "./shell/TourDriver";
import { AppErrorBoundary } from "./shell/AppErrorBoundary";
import "./theme/tokens.css";
import "./styles/base.css";
import "./styles/shell.css";
import "./styles/sidebar.css";
import "./styles/explore.css";
import "./styles/viz.css";
import "./styles/inspector.css";
import "./styles/tabs.css";
import "./styles/overlays.css";

function AppShell() {
  const tab = useViewStore((s) => s.tab);
  const inspectorVisible = useViewStore((s) => s.inspectorVisible);
  const nameFilter = useViewStore((s) => s.nameFilter);
  const setNameFilter = useViewStore((s) => s.setNameFilter);
  const generation = useScanStore((s) => s.generation);
  const status = useScanStore((s) => s.status);
  const licensePosture = useLicenseStore((s) => s.status?.posture ?? null);
  const currentFolder = useExploreStore((s) => s.currentFolder);
  const openFolder = useExploreStore((s) => s.openFolder);
  const goBack = useExploreStore((s) => s.goBack);
  const canGoBack = useExploreStore((s) => s.folderStack.length > 0);

  const [crumbs, setCrumbs] = useState<CrumbData[]>([]);
  const [previewId, setPreviewId] = useState<number | null>(null);
  const [queueOpen, setQueueOpen] = useState(false);
  const [licenseOpen, setLicenseOpen] = useState(false);

  useEffect(() => {
    attachLicenseEvents();
    void useLicenseStore.getState().load();
    useScanStore.getState().ensureListeners(); // scan-progress / scan-done / cleanup-committed (once)
  }, []);

  // Tour-driver overlay events (CI screenshot tours).
  useEffect(() => {
    const openLicense = () => setLicenseOpen(true);
    const openQueue = () => setQueueOpen(true);
    const closeOverlays = () => {
      setLicenseOpen(false);
      setQueueOpen(false);
    };
    window.addEventListener("db-open-license", openLicense);
    window.addEventListener("db-open-queue", openQueue);
    window.addEventListener("db-tour-step", closeOverlays);
    return () => {
      window.removeEventListener("db-open-license", openLicense);
      window.removeEventListener("db-open-queue", openQueue);
      window.removeEventListener("db-tour-step", closeOverlays);
    };
  }, []);

  // Scan lifecycle housekeeping: remember recent + reset navigation.
  useEffect(() => {
    if (status !== "done") return;
    useExploreStore.getState().resetNavigation();
    void (async () => {
      const st = await invoke<{ progress: { currentPath: string } }>("get_status").catch(() => null);
      void st;
      try {
        const hooks = await invoke<{ scan: string | null }>("get_dev_hooks").catch(() => null);
        if (hooks?.scan) pushRecent(hooks.scan);
      } catch {
        /* ignore */
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [status]);

  // Breadcrumb chain refreshes on navigation + generation changes.
  useEffect(() => {
    if (status !== "done") {
      setCrumbs([]);
      return;
    }
    let disposed = false;
    void (async () => {
      const chain = await getBreadcrumb(generation, currentFolder).catch(() => null);
      if (!disposed && chain) setCrumbs(chain);
    })();
    return () => {
      disposed = true;
    };
  }, [generation, currentFolder, status]);

  const openPreview = useCallback((id: number) => setPreviewId(id), []);

  return (
    <div className="db-app">
      <TitleBar />
      {licensePosture === "degraded" && (
        <div className="db-degrade-banner" role="alert">
          License couldn’t be validated for over 14 days — scanning works, cleanup is read-only until you reconnect (License in the top bar).
        </div>
      )}
      <TopBar
        crumbs={crumbs}
        onNavigateCrumb={(id) => openFolder(id)}
        onFolderBack={goBack}
        canGoBack={canGoBack}
        query={nameFilter}
        onQueryChange={setNameFilter}
        onOpenQueue={() => setQueueOpen((o) => !o)}
        queueOpen={queueOpen}
        onOpenLicense={() => setLicenseOpen(true)}
      />
      <div className={`db-body ${inspectorVisible && tab === "explore" ? "has-inspector" : ""}`}>
        <div className="db-sidebar-col">
          <Sidebar />
        </div>
        <div className="db-main-col">
          {tab === "explore" && <ExploreView onPreview={openPreview} />}
          {tab === "duplicates" && <DuplicatesView />}
          {tab === "applications" && <ApplicationsView />}
          {tab === "monitor" && <MonitorView />}
          {tab === "snapshots" && <SnapshotsView />}
        </div>
        {inspectorVisible && tab === "explore" && (
          <div className="db-inspector-col">
            <InspectorPanel onPreview={openPreview} />
          </div>
        )}
      </div>

      {previewId != null && (
        <PreviewOverlay
          generation={generation}
          id={previewId}
          onClose={() => setPreviewId(null)}
          onOpenDefault={(id) => {
            void invoke("open_node", { generation, id }).catch(() => undefined);
            setPreviewId(null);
          }}
        />
      )}

      <CleanupQueuePopover open={queueOpen} onClose={() => setQueueOpen(false)} anchor="topbar" />
      <LicenseDialog open={licenseOpen} onClose={() => setLicenseOpen(false)} />
      <TourDriver />
    </div>
  );
}


export default function App() {
  return (
    <AppErrorBoundary>
      <AppShell />
    </AppErrorBoundary>
  );
}
