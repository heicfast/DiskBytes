/**
 * Sidebar composition (spec §6): scan targets → recent → storage →
 * current view → quick wins → file types, plus the unreadable notice.
 */
import { ScanSection } from "./ScanSection";
import { RecentSection } from "./RecentSection";
import { StorageSection } from "./StorageSection";
import { CurrentViewSection } from "./CurrentViewSection";
import { QuickWinsSection } from "./QuickWinsSection";
import { FileTypesSection } from "./FileTypesSection";
import { UnreadableNotice } from "./UnreadableNotice";

export function Sidebar() {
  return (
    <aside className="db-sidebar db-scroll" aria-label="Sidebar">
      <ScanSection />
      <RecentSection />
      <StorageSection />
      <CurrentViewSection />
      <QuickWinsSection />
      <FileTypesSection />
    </aside>
  );
}

export { UnreadableNotice };
