/**
 * DiskBytes icon system.
 *
 * Standard glyphs are re-exported from **lucide-react** — the professional,
 * MIT-licensed icon set (Feather/Lucide, 24×24 grid, stroke 2, round
 * caps/joins, currentColor) — so every generic icon is a pixel-exact,
 * industry-standard render instead of a hand-drawn approximation.
 *
 * Two groups remain purpose-drawn (no lucide equivalent exists):
 *   1. View-mode pictograms (treemap / sunburst / bubbles / mind-map) —
 *      geometric chart glyphs drawn to the same 24-grid / stroke language.
 *   2. Windows caption glyphs (minimize / maximize / restore / close) —
 *      the exact Windows 11 caption geometry (thin 1.7 stroke, square
 *      corners, L-clipped restore square).
 */
import type { ComponentType, SVGProps } from "react";
import {
  Activity as ActivityIcon, AppWindow as AppWindowIcon, Archive as ArchiveIcon,
  ArrowDown as ArrowDownIcon, ArrowUp as ArrowUpIcon, Box as BoxIcon,
  CalendarClock as CalendarClockIcon, Camera as CameraIcon, Check as CheckIcon,
  ChevronDown as ChevronDownIcon, ChevronLeft as ChevronLeftIcon,
  ChevronRight as ChevronRightIcon, ChevronsRightLeft as ChevronsRightLeftIcon,
  CircleDot as CircleDotIcon, Clock3 as Clock3Icon, Cloud as CloudIcon,
  Copy as CopyIcon, Cpu as CpuIcon, Database as DatabaseIcon,
  ExternalLink as ExternalLinkIcon, Eye as EyeIcon,
  File as FileIcon, FileArchive as FileArchiveIcon, FileAudio as FileAudioIcon,
  FileCode2 as FileCode2Icon, FileImage as FileImageIcon, FileText as FileTextIcon,
  FileVideo as FileVideoIcon, Flame as FlameIcon, Folder as FolderIcon,
  FolderOpen as FolderOpenIcon, Gauge as GaugeIcon, Grid2x2 as Grid2x2Icon,
  HardDrive as HardDriveIcon, Home as HomeIcon, Key as KeyIcon, List as ListIcon,
  ListTree as ListTreeIcon, LockKeyhole as LockKeyholeIcon, LayoutGrid as LayoutGridIcon,
  Maximize2 as Maximize2Icon, MemoryStick as MemoryStickIcon, Minus as MinusIcon,
  Moon as MoonIcon, Network as NetworkIcon, PackageOpen as PackageOpenIcon,
  PanelRight as PanelRightIcon, Plus as PlusIcon, RefreshCw as RefreshCwIcon,
  ScanLine as ScanLineIcon, Search as SearchIcon, Settings as SettingsIcon,
  Shield as ShieldIcon, Sparkles as SparklesIcon, Square as SquareIcon,
  Sun as SunIcon, Trash2 as Trash2Icon, Wifi as WifiIcon, X as XIcon,
} from "lucide-react";

export type IconProps = SVGProps<SVGSVGElement> & { size?: number | string };

/** Anything renderable as `<Icon size={n} />` — lucide glyphs and the
 *  purpose-drawn pictograms alike. */
export type AnyIcon = ComponentType<SVGProps<SVGSVGElement> & { size?: number | string }>;

/* ── View-mode pictograms (no lucide equivalent — drawn to the lucide
 *    grid: 24×24, stroke 2, round caps/joins, currentColor) ─────────── */

/** Treemap — nested rectangles of unequal area (classic squarified look). */
export const TreemapIcon = ({ size = 16, ...p }: IconProps) => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}
    strokeLinecap="round" strokeLinejoin="round" width={size} height={size}
    aria-hidden focusable="false" {...p}>
    <rect x="3" y="3" width="18" height="18" rx="1.5" />
    <path d="M12 3v18" />
    <path d="M12 10h9" />
    <path d="M12 15h9" />
    <path d="M3 8h9" />
  </svg>
);

/** Sunburst — concentric rings radiating from a filled center disc. */
export const SunburstIcon = ({ size = 16, ...p }: IconProps) => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}
    strokeLinecap="round" strokeLinejoin="round" width={size} height={size}
    aria-hidden focusable="false" {...p}>
    <circle cx="12" cy="12" r="10" />
    <circle cx="12" cy="12" r="6.25" />
    <circle cx="12" cy="12" r="2.5" fill="currentColor" stroke="none" />
  </svg>
);

/** Bubbles — three packed circles of decreasing size. */
export const BubblesIcon = ({ size = 16, ...p }: IconProps) => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}
    strokeLinecap="round" strokeLinejoin="round" width={size} height={size}
    aria-hidden focusable="false" {...p}>
    <circle cx="9" cy="8.5" r="5.5" />
    <circle cx="17.25" cy="15.25" r="4" />
    <circle cx="6.75" cy="18.75" r="2.25" />
  </svg>
);

/** Mind map — organic radial tree: filled hub with curved branches
 *  fanning out to leaf dots (asymmetric, like a sketched mind map). */
export const MindMapIcon = ({ size = 16, ...p }: IconProps) => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}
    strokeLinecap="round" strokeLinejoin="round" width={size} height={size}
    aria-hidden focusable="false" {...p}>
    <circle cx="7.5" cy="12" r="2.6" fill="currentColor" stroke="none" />
    <path d="M9.6 10.7C11.5 8.6 14.4 7 17.2 6.6" />
    <path d="M10.1 12h9.4" />
    <path d="M9.6 13.3c1.9 2.1 4.8 3.7 7.6 4.1" />
    <circle cx="19" cy="5.5" r="1.7" />
    <circle cx="20.3" cy="12" r="1.7" />
    <circle cx="19" cy="18.5" r="1.7" />
  </svg>
);

/** Top Sizes — ranked bars, biggest first (descending heights off a
 *  shared baseline — the reference's "bar chart" metaphor). */
export const TopSizesIcon = ({ size = 16, ...p }: IconProps) => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}
    strokeLinecap="round" width={size} height={size}
    aria-hidden focusable="false" {...p}>
    <path d="M6 19V8" /><path d="M12 19v-8" /><path d="M18 19v-5" />
    <path d="M3.5 19h17" />
  </svg>
);

/* ── Windows caption glyphs (Windows 11 geometry: thin square-corner
 *    strokes on the 24 grid; sized/centered like Segoe Fluent caption
 *    icons so the chrome reads native) ───────────────────────────────── */

export const CaptionMinimizeIcon = ({ size = 16, ...p }: IconProps) => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.7}
    strokeLinecap="round" width={size} height={size} aria-hidden focusable="false" {...p}>
    <path d="M5.5 12h13" />
  </svg>
);

export const CaptionMaximizeIcon = ({ size = 16, ...p }: IconProps) => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.7}
    width={size} height={size} aria-hidden focusable="false" {...p}>
    <rect x="5.5" y="5.5" width="13" height="13" />
  </svg>
);

export const CaptionRestoreIcon = ({ size = 16, ...p }: IconProps) => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.7}
    width={size} height={size} aria-hidden focusable="false" {...p}>
    {/* back square, clipped to the L visible around the front square */}
    <path d="M14.5 5H5v9.5h5" />
    {/* front square */}
    <rect x="10" y="10" width="9" height="9" />
  </svg>
);

export const CaptionCloseIcon = ({ size = 16, ...p }: IconProps) => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.7}
    strokeLinecap="round" width={size} height={size} aria-hidden focusable="false" {...p}>
    <path d="M6 6l12 12" /><path d="M18 6L6 18" />
  </svg>
);

/** Legacy composite (kept for the Snapshots header art) — a copy square. */
export const CopySquareIcon = ({ size = 16, ...p }: IconProps) => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}
    strokeLinecap="round" strokeLinejoin="round" width={size} height={size}
    aria-hidden focusable="false" {...p}>
    <path d="M10 4h4a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6c0-1.1.9-2 2-2h2" />
    <rect width="8" height="8" x="8" y="8" rx="1" />
  </svg>
);

export {
  ActivityIcon, AppWindowIcon, ArchiveIcon, ArrowDownIcon, ArrowUpIcon,
  BoxIcon, CalendarClockIcon, CameraIcon, CheckIcon, ChevronDownIcon,
  ChevronLeftIcon, ChevronRightIcon, ChevronsRightLeftIcon, CircleDotIcon,
  Clock3Icon, CloudIcon, CopyIcon, CpuIcon, DatabaseIcon, ExternalLinkIcon,
  EyeIcon, FileArchiveIcon, FileAudioIcon, FileCode2Icon, FileIcon,
  FileImageIcon, FileTextIcon, FileVideoIcon, FlameIcon, FolderIcon,
  FolderOpenIcon, GaugeIcon, Grid2x2Icon, HardDriveIcon, HomeIcon, KeyIcon,
  LayoutGridIcon, ListIcon, ListTreeIcon, LockKeyholeIcon, Maximize2Icon,
  MemoryStickIcon, MinusIcon, MoonIcon, NetworkIcon, PackageOpenIcon,
  PanelRightIcon, PlusIcon, RefreshCwIcon, ScanLineIcon, SearchIcon,
  SettingsIcon, ShieldIcon, SparklesIcon, SquareIcon, SunIcon, Trash2Icon,
  WifiIcon, XIcon,
};

/** Category label (Rust `FileCategory::label`) → icon component. */
export function categoryIcon(label: string): AnyIcon {
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
