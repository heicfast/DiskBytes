/**
 * DiskBytes icon system — lucide-style stroke glyphs (24×24 grid,
 * stroke-width 2, round caps/joins, currentColor). Hand-drawn to match
 * the owner-approved prototype visual language. No icon library
 * dependency (BuildPrompt §2: allowlist locked).
 */
import type { SVGProps } from "react";

export type IconProps = SVGProps<SVGSVGElement> & { size?: number };

function base(size: number): Omit<SVGProps<SVGSVGElement>, "children"> {
  return {
    width: size,
    height: size,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 2,
    strokeLinecap: "round",
    strokeLinejoin: "round",
    "aria-hidden": true,
    focusable: false,
  };
}

/* ── Navigation & tabs ─────────────────────────────────────────────── */
export const LayoutGridIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><rect width="7" height="7" x="3" y="3" rx="1.5"/><rect width="7" height="7" x="14" y="3" rx="1.5"/><rect width="7" height="7" x="14" y="14" rx="1.5"/><rect width="7" height="7" x="3" y="14" rx="1.5"/></svg>
);
export const CopyIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><rect width="14" height="14" x="8" y="8" rx="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg>
);
export const AppWindowIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M6 2h12a2 2 0 0 1 2 2v16a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2Z"/><path d="M4 7h16"/><circle cx="7" cy="4.6" r=".4" fill="currentColor"/><circle cx="9.4" cy="4.6" r=".4" fill="currentColor"/></svg>
);
export const GaugeIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="m12 14 4-4"/><path d="M3.34 19a10 10 0 1 1 17.32 0"/></svg>
);
export const Clock3Icon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg>
);

/* ── Brand ──────────────────────────────────────────────────────────── */
export const DatabaseIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p} strokeWidth={p.strokeWidth ?? 2.4}><ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M3 5v14a9 3 0 0 0 18 0V5"/><path d="M3 12a9 3 0 0 0 18 0"/></svg>
);

/* ── Breadcrumb / chevrons ─────────────────────────────────────────── */
export const ChevronLeftIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="m15 18-6-6 6-6"/></svg>
);
export const ChevronRightIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="m9 18 6-6-6-6"/></svg>
);
export const ChevronDownIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="m6 9 6 6 6-6"/></svg>
);
export const ChevronsRightLeftIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="m6 8-4 4 4 4"/><path d="m18 8 4 4-4 4"/><path d="M10 12h4"/></svg>
);

/* ── Actions ────────────────────────────────────────────────────────── */
export const SearchIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>
);
export const XIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
);
export const Trash2Icon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M3 6h18"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/><line x1="10" x2="10" y1="11" y2="17"/><line x1="14" x2="14" y1="11" y2="17"/></svg>
);
export const MoonIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"/></svg>
);
export const SunIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/></svg>
);
export const PanelRightIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><rect width="18" height="18" x="3" y="3" rx="2"/><path d="M15 3v18"/></svg>
);
export const RefreshCwIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M3 21v-5h5"/></svg>
);
export const Maximize2Icon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><polyline points="15 3 21 3 21 9"/><polyline points="9 21 3 21 3 15"/><line x1="21" x2="14" y1="3" y2="10"/><line x1="3" x2="10" y1="21" y2="14"/></svg>
);
export const MinusIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M5 12h14"/></svg>
);
export const SquareIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><rect width="18" height="18" x="3" y="3" rx="2"/></svg>
);
export const CopySquareIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M10 4h4a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6c0-1.1.9-2 2-2h2"/><rect width="8" height="8" x="8" y="8" rx="1"/></svg>
);
export const CheckIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M20 6 9 17l-5-5"/></svg>
);
export const PlusIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M5 12h14"/><path d="M12 5v14"/></svg>
);
export const LockKeyholeIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><circle cx="12" cy="16" r="1"/><path d="m8.4 12.6-.8-2c-.6-1.5.5-3.2 2.1-3.4l4.5-.7c1.6-.2 3 1.2 2.7 2.8l-.4 2.3"/><path d="M5 19a2 2 0 0 1-1.4-3.4C4.6 14.6 5 13.7 5 12a7 7 0 0 1 14 0c0 1.7.4 2.6 1.4 3.6A2 2 0 0 1 19 19Z"/></svg>
);
export const ShieldIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/></svg>
);
export const SettingsIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/></svg>
);
export const ExternalLinkIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/></svg>
);
export const EyeIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M2.06 12.35a1 1 0 0 1 0-.7 10.75 10.75 0 0 1 19.88 0 1 1 0 0 1 0 .7 10.75 10.75 0 0 1-19.88 0"/><circle cx="12" cy="12" r="3"/></svg>
);
export const ScanLineIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M3 7V5a2 2 0 0 1 2-2h2"/><path d="M17 3h2a2 2 0 0 1 2 2v2"/><path d="M21 17v2a2 2 0 0 1-2 2h-2"/><path d="M7 21H5a2 2 0 0 1-2-2v-2"/><path d="M7 12h10"/></svg>
);
export const HardDriveIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><line x1="22" x2="2" y1="12" y2="12"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/><line x1="6" x2="6.01" y1="16" y2="16"/><line x1="10" x2="10.01" y1="16" y2="16"/></svg>
);
export const FolderIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/></svg>
);
export const FolderOpenIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2"/></svg>
);
export const FileIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/></svg>
);
export const HomeIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><path d="M9 22V12h6v10"/></svg>
);
export const SparklesIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M9.9 2.6 11.2 6a1 1 0 0 0 .6.6l3.4 1.3a1 1 0 0 1 0 1.9l-3.4 1.3a1 1 0 0 0-.6.6l-1.3 3.4a1 1 0 0 1-1.9 0L6.7 11.7a1 1 0 0 0-.6-.6L2.7 9.8a1 1 0 0 1 0-1.9l3.4-1.3a1 1 0 0 0 .6-.6l1.3-3.4a1 1 0 0 1 1.9 0Z"/><path d="M18 14.5 18.7 16.4a1 1 0 0 0 .6.6l1.9.7a1 1 0 0 1 0 1.8l-1.9.7a1 1 0 0 0-.6.6l-.7 1.9a1 1 0 0 1-1.8 0l-.7-1.9a1 1 0 0 0-.6-.6l-1.9-.7a1 1 0 0 1 0-1.8l1.9-.7a1 1 0 0 0 .6-.6l.7-1.9a1 1 0 0 1 1.8 0Z"/></svg>
);
export const CircleDotIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="1.6" fill="currentColor" stroke="none"/></svg>
);
export const BlocksIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><rect width="7" height="7" x="3" y="3" rx="1"/><rect width="7" height="7" x="14" y="3" rx="1"/><rect width="7" height="7" x="3" y="14" rx="1"/><rect width="7" height="7" x="14" y="14" rx="1"/></svg>
);
export const NetworkIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><rect x="16" y="16" width="6" height="6" rx="1"/><rect x="2" y="16" width="6" height="6" rx="1"/><rect x="9" y="2" width="6" height="6" rx="1"/><path d="M5 16v-3a1 1 0 0 1 1-1h12a1 1 0 0 1 1 1v3"/><path d="M12 12V8"/></svg>
);
export const ListTreeIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M21 12h-8"/><path d="M21 6H8"/><path d="M21 18h-8"/><path d="M3 6v4a2 2 0 0 0 2 2h3"/><path d="M3 10v6a2 2 0 0 0 2 2h1"/></svg>
);
export const Grid2x2Icon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><rect width="7" height="7" x="3" y="3" rx="1"/><rect width="7" height="7" x="14" y="3" rx="1"/><rect width="7" height="7" x="14" y="14" rx="1"/><rect width="7" height="7" x="3" y="14" rx="1"/></svg>
);
export const FlameIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M8.5 14.5A2.5 2.5 0 0 0 11 12c0-1.38-.5-2-1-3-1.072-2.143-.224-4.054 2-6 .5 2.5 2 4.9 4 6.5 2 1.6 3 3.5 3 5.5a7 7 0 1 1-14 0c0-1.153.433-2.294 1-3a2.5 2.5 0 0 0 2.5 2.5z"/></svg>
);

/* ── Data / files ───────────────────────────────────────────────────── */
export const FileImageIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><circle cx="10" cy="12" r="1.6"/><path d="m8.5 17 2-2.5 2 2.5 2-2.9 2.5 2.9z"/></svg>
);
export const FileVideoIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="m10 12 5 3-5 3z"/></svg>
);
export const FileAudioIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="M9 13v5l3-2v-5z"/><path d="M15 10v5"/></svg>
);
export const FileTextIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="M16 13H8"/><path d="M16 17H8"/><path d="M10 9H8"/></svg>
);
export const FileCode2Icon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="m10 12-2 2 2 2"/><path d="m14 12 2 2-2 2"/></svg>
);
export const FileArchiveIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><circle cx="12" cy="13" r="1.4"/><path d="M12 14.4V17"/></svg>
);
export const PackageOpenIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M12.6 2.8 2.9 7.4a1 1 0 0 0-.4 1.3l3.6 7.2a1 1 0 0 0 1.4.4l9.7-4.6a1 1 0 0 0 .4-1.3l-3.6-7.2a1 1 0 0 0-1.4-.4Z"/><path d="m2 8 10 5 10-5"/><path d="M12 13v9"/><path d="M2 8v9l10 5 10-5V8"/></svg>
);
export const BoxIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z"/><path d="m3.3 7 8.7 5 8.7-5"/><path d="M12 22V12"/></svg>
);
export const ArchiveIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><rect width="20" height="5" x="2" y="3" rx="1"/><path d="M4 8v11a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8"/><path d="M10 12h4"/></svg>
);

/* ── System / misc ──────────────────────────────────────────────────── */
export const CloudIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z"/></svg>
);
export const ActivityIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M22 12h-2.48a2 2 0 0 0-1.93 1.46l-2.35 8.36a.25.25 0 0 1-.48 0L9.24 2.18a.25.25 0 0 0-.48 0l-2.35 8.36A2 2 0 0 1 4.49 12H2"/></svg>
);
export const CpuIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><rect x="4" y="4" width="16" height="16" rx="2"/><rect x="9" y="9" width="6" height="6" rx="1"/><path d="M15 2v2"/><path d="M15 20v2"/><path d="M2 15h2"/><path d="M2 9h2"/><path d="M20 15h2"/><path d="M20 9h2"/><path d="M9 2v2"/><path d="M9 20v2"/></svg>
);
export const MemoryStickIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M6 19v-3"/><path d="M10 19v-3"/><path d="M14 19v-3"/><path d="M18 19v-3"/><path d="M8 11V9"/><path d="M16 11V9"/><path d="M12 11V9"/><path d="M2 15h20"/><path d="M2 7a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1z"/></svg>
);
export const WifiIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M12 20h.01"/><path d="M8.5 16.4a5 5 0 0 1 7 0"/><path d="M5 12.9a10 10 0 0 1 14 0"/><path d="M1.8 9.4a15 15 0 0 1 20.4 0"/></svg>
);
export const ArrowDownIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M12 5v14"/><path d="m19 12-7 7-7-7"/></svg>
);
export const ArrowUpIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="M12 19V5"/><path d="m5 12 7-7 7 7"/></svg>
);
export const KeyIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p}><path d="m15.5 7.5 2.3 2.3a1 1 0 0 0 1.4 0l2.1-2.1a1 1 0 0 0 0-1.4L19 4"/><path d="m21 2-9.6 9.6"/><circle cx="7.5" cy="15.5" r="5.5"/></svg>
);
export const CameraIcon = ({ size = 16, ...p }: IconProps) => (
  <svg {...base(size)} {...p} strokeWidth={p.strokeWidth ?? 2.2}><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="3.2"/><path d="M12 2v3.4"/><path d="M12 18.6V22"/><path d="M2 12h3.4"/><path d="M18.6 12H22"/></svg>
);

/** Category label (Rust `FileCategory::label`) → icon component. */
export function categoryIcon(label: string): (p: IconProps) => JSX.Element {
  switch (label) {
    case "Video": return FileVideoIcon;
    case "Audio": return FileAudioIcon;
    case "Images": return FileImageIcon;
    case "Documents": return FileTextIcon;
    case "Developer": return FileCode2Icon;
    case "Archives": return FileArchiveIcon;
    case "Applications": return AppWindowIcon;
    case "System": return CpuIcon;
    default: return FileIcon;
  }
}

/** By-folder tone key from the layout color (index into the 8 families). */
export const TONE_KEYS = ["blue", "mint", "violet", "amber", "rose", "green", "sky", "slate"] as const;
export type ToneKey = (typeof TONE_KEYS)[number];
