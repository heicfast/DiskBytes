# Disk Storage Analyzer — Design & Functional Specification

Reverse-engineered from 9 screenshots of a macOS disk-usage analysis app (DaisyDisk/GrandPerspective/OmniDiskSweeper-class tool). This document specifies layout, components, color system, typography, and per-view behavior in enough detail for an agent to rebuild the UI.

---

## 1. Overall App Shell

Three-pane persistent layout, present on every screen:

```
┌───────────────────────────────────────────────────────────────────────────┐
│  TOP BAR (full width)                                                      │
├───────────────┬───────────────────────────────────────┬────────────────────┤
│               │                                         │                    │
│  LEFT SIDEBAR │            MAIN CONTENT PANE            │  RIGHT DETAILS     │
│  (fixed, dark)│      (view switcher + visualization)     │  PANEL (selected  │
│               │                                         │  item inspector)   │
│               │                                         │                    │
└───────────────┴───────────────────────────────────────┴────────────────────┘
```

Approx column widths (at ~1080px viewport): sidebar 250px, main pane flexible/center (~530px content column), right panel ~250px fixed.

### 1.1 Top Bar
- Left-aligned segmented/tab navigation, each item = icon + label, pill-shaped when active:
  - **Explore** (compass/search icon) — active state shown throughout: solid orange/coral pill (`#F4623A`-ish), white text, rounded-full
  - **Duplicates** (overlapping-squares icon)
  - **Applications** (app-window icon)
  - **Monitor** (gauge/speedometer icon)
  - **Snapshots** (camera icon)
  - All inactive tabs: plain text/icon, gray (`#8A8F98`), no background, on hover likely gets light gray bg
- Vertical divider, then breadcrumb/back control: `‹ Macintosh HD` (chevron-left + disk name, clickable back nav)
- Right-aligned: search input "Filter by name..." (pill/rounded, gray placeholder, magnifier icon prefix), print icon button, theme toggle (sun/moon icon), right-panel-toggle icon (sidebar icon)
- Background: white/very light gray, thin bottom border separating from content, sits above both side panels (full-bleed row)

### 1.2 Left Sidebar
Dark theme, independent of main-content light theme (fixed dark navy/charcoal, approx `#1B1D2A`–`#20222E`). Sections top to bottom:

1. **Scan Full Mac** — full-width primary button, warm orange-to-red gradient (`#FF7A50` → `#F0453A`), white bold text, rounded-lg (~10px), subtle icon (arrows-in) at left.
2. **Home** / **Folder...** — two secondary buttons side-by-side, dark-gray fill, white icon+text, rounded, equal width, subtle border.
3. **RECENT** — section label (small caps, muted gray, letter-spaced). One row: disk icon + "Macintosh HD".
4. **DISK STORAGE** card:
   - Header: disk icon + "Macintosh HD" label
   - Large circular ring/donut chart (radial progress), orange arc representing used %, center shows big bold percentage ("90.1%") + "USED" caption below in small caps
   - To the right of ring: stacked stat rows — "Total" 245 GB, "Used" 221 GB (orange value text), "Free" 24.4 GB (green value text)
5. **CURRENT VIEW** section:
   - Small caps label + elapsed-scan-time badge on the right ("7.1s scan")
   - Bold disk/folder name ("Macintosh HD"), path breadcrumb below ("/")
   - Two buttons: **Reveal**, **Copy Path** (side-by-side, secondary dark buttons with icons)
6. **QUICK WINS** section:
   - Header: "QUICK WINS" label + aggregate total on right ("54.6 GB")
   - Scrollable list rows, each: colored square icon (category-coded), title (bold), item count (muted, small, below title), size (right-aligned, bold), chevron `>` at far right
   - Categories observed: Caches & logs (blue icon, 319 items, 18.0 GB), Large media (red/pink icon, 32 items, 15.9 GB), node_modules (green icon, 400 items, 13.3 GB), Build artifacts (orange icon, 400 items, 7.26 GB), Downloads (blue icon, 20 items, 119 MB), iOS Simulators (teal icon, 8 items, 5.62 MB), Xcode DerivedData (purple/pencil icon, 1 item, 61.4 KB)
   - Each row is a clickable list item, likely navigates to that category's file set.

### 1.3 Right Details Panel (Inspector)
Light theme, appears/updates whenever an item (folder/file/node) is selected in the main pane. Structure top→bottom:

1. **Header row**: small folder/file-type icon + item name (bold, large), type caption below ("Folder")
2. **Path** line, small muted monospace-ish text (e.g. `/Library`)
3. **Size hero**: very large bold number ("5.20 GB") + small muted caption ("3.2% of scan") beneath
4. **DETAILS** card (label/value two-column rows, right-aligned values):
   - Size on disk
   - Logical size
   - Compressed by (value in green, e.g. "3.27 GB")
   - Files (count)
   - Folders (count)
   - Of parent (percentage)
   - Modified (relative time, e.g. "3 minutes ago")
   - Created (relative time, e.g. "2 months ago")
5. **LARGEST INSIDE** card:
   - Header: label + item count on right ("65 items")
   - List rows: small colored dot bullet (category color) + name (truncate/ellipsis if long) + size right-aligned
6. **Action buttons** — 2×2 grid of secondary buttons with icon+label: **Reveal**, **Quick Look**, **Focus**, **Copy Path**
7. **Add to Cleanup** — full-width primary destructive-style button, red/orange, pinned to bottom of panel (partially cut off in some screenshots, implying it's sticky/anchored at panel bottom)

### 1.4 Main Content Pane (shared header across all views)
1. Title row: large bold folder name ("Macintosh HD") + inline stat string ("162 GB · 16,47,622 files · 2,20,118 folders") + small square icon button top-right (maybe "export/share")
2. **View switcher toolbar**: row of icon-only buttons (small square icons in a light gray pill container) representing each visualization mode, plus one large pill button showing the **active view's name** in orange (matches top-bar active-tab color language), and a right-aligned italic/muted one-line description of what the current view shows (e.g. "Every item as an expandable outline", "Where your bytes sit on a timeline", "The biggest items, ranked", "Browse folder by folder, sized as you go")
   - View icon order (inferred from icon strip): List/outline, Age Map (clock), Top Sizes (bar chart), Mind Map (tree), Bubbles (circles), Flame (flame), Sunburst (radial), Folders (grid), Treemap (squares) — 9 total modes, matching the 9 screenshots
3. **Warning/alert banner** (conditional, dismissible): light red/pink background, red left icon (lock), bold message ("205 folders couldn't be read — sizes may be incomplete"), muted sub-text explaining why (macOS Full Disk Access), a white "Open Privacy Settings" button (with gear icon) on the right, and an "×" dismiss button at far right.
4. View-specific visualization body (see Section 2).

---

## 2. Per-View Breakdown

### 2.1 List View (Image 1)
- Sub-header: "Every item as an expandable outline"
- Table with columns: expand-chevron, folder/file icon, name, horizontal proportion bar (colored, filled left-to-right relative to size), percentage (right-aligned, muted), size (right-aligned, bold)
- Rows sorted descending by size
- Selected/hovered row: light purple/lavender background highlight with rounded corners (seen on "Library" row)
- Example data: Users 76.5% 124GB · Applications 15.4% 24.9GB · private 4.1% 6.65GB · Library 3.2% 5.20GB · usr 0.5% 834MB · opt 0.3% 428MB · bin <0.1% 4.90MB · sbin <0.1% 2.50MB · plus zero-byte special entries (.resolve, .file, .vol, .nofollow) shown as flat rows (no bar, "0.0%", "0 B")
- Proportion bars use a muted blue-gray fill.

### 2.2 Age Map View (Image 2)
- Sub-header: "Where your bytes sit on a timeline", plus a small "A" icon badge top-right of toolbar
- **Panel A — "HOW OLD ARE THESE BYTES?"**: horizontal stacked/grouped bar list, one row per age bucket:
  - Last 7 days — green bar
  - 8–30 days — blue bar
  - 1–3 months — blue/lavender bar
  - 3–12 months — purple bar (longest/largest, 71.0GB / 43.8%)
  - 1–2 years — pink/magenta bar
  - Over 2 years — small orange/red bar
  - Each row: label (left, fixed width), bar (fills proportionally), size value, percentage (right-aligned)
- **Panel B — "BYTES BY LAST-MODIFIED MONTH"**: calendar-heatmap grid. Columns = months (J F M A M J J A S O N D), rows = years (2019 through 2026, most recent on top). Cells are small rounded squares shaded by intensity (light gray = ~0, deeper blue = larger size); a few cells in the current year show size labels directly inside the cell (e.g., "20.9 GB", "18.3 GB", "10.8 GB", "32.7 GB", "26.4 GB" in 2026 row, "12.7 GB" in 2025 row) with a darker blue fill and border to denote emphasis/selection.

### 2.3 Top Sizes View (Image 3)
- Sub-header: "The biggest items, ranked"
- Segmented sub-tabs directly under toolbar: **In this folder** (active, underlined/bold), **Biggest files anywhere**, **Biggest folders anywhere** — plus a right-aligned count ("12 shown")
- Ranked table: numeric rank column (1–12), icon, name, file-count column, percentage column, size column (right-aligned)
- Selected row (Library, rank 4) shown with a purple outline/border box around it rather than a full background fill — distinct selection style from List view.

### 2.4 Mind Map View (Image 4)
- Radial node-link / tree diagram centered in the pane.
- Root node: filled orange circle, center, labeled "Macintosh HD" + total size, larger radius than children.
- Primary branches radiate outward in straight lines to color-coded top-level folder nodes (e.g., red for one cluster, teal/green for Applications, purple for private, orange/yellow for Library, gray/blue for others), each branch line colored to match its cluster.
- Each top-level node has further child nodes (smaller circles) branching outward with connecting lines, each labeled with folder name and size (e.g., "Library 5.20 GB", "Contents", "Xcode.app 5.25 GB").
- A highlighted/selected node (Library) shown as a hollow ring (white center, colored border) versus solid-filled dots for others.
- Small utility icon row top-right of canvas (loop/refresh, folder, circle, "A" toggle) — likely layout/zoom controls.

### 2.5 Bubbles View (Image 5)
- Circle-packing chart. Outer boundary circle = "Macintosh HD" (thin gray outline, label top-left inside).
- Nested circles sized by folder size, packed within color-zoned regions:
  - Blue zone: Users / hariprasad, with children Library, Documents, Caches (smaller bubbles inside)
  - Teal/green zone: Applications, with smaller packed sub-bubbles
  - Purple zone: private
  - A standalone bubble "Library" (orange-tinted, top area) appears highlighted/selected, distinct color from its parent context, larger stroke.
- Labels appear inside larger bubbles only; smallest bubbles are unlabeled dots.

### 2.6 Flame View (Icicle/Flame Graph — Image 6)
- Sub-toolbar: three segmented buttons **By type**, **By folder** (active), **By age**, plus a horizontal slider control (depth/zoom, shown as a small track with a filled portion and draggable dot, labeled "5") and an "A" icon toggle.
- Chart type: horizontal icicle / flame-graph layers. Each row = one depth level in the folder tree; each row is subdivided into colored horizontal segments proportional to size (root "/" full width top row, then Users|Applications, then hariprasad, then Library|Documents, then Application Support|Caches, then Google|Claude, etc.)
- Segment colors pastel-coded by top-level ancestor (blue=Users lineage, teal=Applications lineage, purple/lavender=Library-Google lineage).
- Selected segment (Google, bottom-most visible row) has a bold dark border/outline distinguishing it.
- Right panel updates to show the selected "Google" folder (path `/Users/hariprasad/Library/Application Support/Google`, 12.4 GB, 7.6% of scan, Files 69,278 / Folders 3,932, Largest Inside: Chrome 11.6GB, GoogleUpdater 796MB, RLZ 4.10KB, Chrome for Testing 4.10KB, GoogleUpdate.app 0B, Chrome-headless 0B).
- "Add to Cleanup" primary red button now fully visible at panel bottom.

### 2.7 Sunburst View (Image 7)
- Same sub-toolbar (By type / By folder-active / By age, slider, "A") and same selected folder (Google) as Flame view — confirms these two views share state/selection with Flame.
- Concentric radial rings: center circle = root ("Macintosh HD" + size), each successive ring = one folder-tree depth, ring arcs sized angularly by proportion of parent size, color-coded by top-level lineage (same palette as Bubbles/Mind Map: blue/teal/purple/orange families).
- Outer-ring labels only where arc is wide enough (deepest visible level shows small folder names).
- Selected arc (Google, outer ring) has a bold dark stroke outline for emphasis.

### 2.8 Folders (Grid) View (Image 8)
- Sub-header: "Browse folder by folder, sized as you go"
- Section label "Folders" with count badge ("11")
- Grid of large rounded pastel cards (2 columns visible, more below fold), one per top-level folder:
  - Each card: two small colored dots top-left (status/type indicators — teal + yellow dot pattern seen), folder name (bold), item count (muted, e.g. "11,88,912 items"), large bold size bottom-right of card
  - Card background tint matches folder's assigned category color at low opacity: Users = light blue/lavender, Applications = light mint/teal, private = light purple, Library = light yellow, usr = light pink, opt = light green
  - Cards observed: Users 109GB, Applications 24.9GB, private 6.56GB, Library 5.17GB, usr 834MB, opt 428MB (grid continues below viewport)
- Right panel in this view shows the **root** ("Macintosh HD") selected by default — 147 GB, 100.0% of scan, with Largest Inside listing all 11 top folders.

### 2.9 Treemap View (Image 9 — wider/resized window)
- Same By type / By folder(active) / By age + slider + "A" sub-toolbar as Flame/Sunburst.
- Classic squarified treemap: nested rectangles, area proportional to size, recursively subdivided.
- Top split: Users (left, larger) | Applications (right, smaller), each subdivided further (Users → hariprasad → Library/Documents/Screenshot → Application Support/Code/Comet/etc. → Google/Claude/Caches/Arduino15/Containers/.npm/etc.)
- Each rectangle labeled with folder name (top-left of cell) and size where space allows; deeply nested/small cells show no label, just color block.
- Color-coding consistent with other views: blue family = Users lineage, teal = Applications lineage, purple/lavender = Library/Google/Claude area (Google cell has bold dark selection border, matching selected state in Flame/Sunburst).
- Right panel again shows Google folder detail identical to Flame/Sunburst state — confirms Treemap, Flame, and Sunburst are variants sharing one "By type/folder/age" toolbar and selection.
- Note: sidebar is not visible in this screenshot — window appears to be a wider/different viewport where sidebar may be collapsed or this is a cropped capture; treat sidebar presence as constant across all views regardless.

---

## 3. Design System

### 3.1 Color Palette (approximate hex, inferred from screenshots)
| Role | Color | Usage |
|---|---|---|
| Primary accent (brand) | `#F4623A` / gradient `#FF7A50`→`#EB4530` | Active tab pill, Scan button, used-space ring, active view button |
| Danger/cleanup | `#E8483A` (similar to primary, slightly more red) | "Add to Cleanup" button, alert banner accents |
| Success/green | `#3DB16B` | Free space stat, "Compressed by" value |
| Warning banner bg | `#FCE9E7` (pale red/pink) | Alert banner background |
| Warning banner icon/text | `#D9463A` | Alert banner icon + heading text |
| Sidebar background | `#1C1E2A`–`#22242F` (dark navy/charcoal) | Left sidebar only |
| Sidebar card background | `#2A2C3A` (slightly lighter than sidebar bg) | Quick Wins rows, buttons |
| Main background | `#FFFFFF` / `#FAFAFB` | Main content + right panel |
| Muted text/gray | `#8A8F98`–`#9CA0A8` | Secondary labels, captions, placeholders |
| Body text (dark) | `#1A1B23` | Headings, values |
| Borders/dividers | `#E8E9ED` | Card borders, table row separators |
| Category blue | `#5B8DEF`/`#AFC8FA` | Users lineage |
| Category teal/green | `#2FBFA0`/`#B7E9DD` | Applications lineage |
| Category purple/lavender | `#8B7CF6`/`#D9D3FB` | private, Library/Google lineage |
| Category yellow | `#F2C94C`/`#FBEBB5` | Library (grid card) |
| Category pink | `#F27EAE`/`#FAD1E4` | usr (grid card) |

### 3.2 Typography
- Sans-serif system font (SF Pro / Inter-like).
- Size scale (approx): 11px micro-labels (small caps section headers, tracked/letter-spaced, uppercase) · 12–13px body/table text · 14px default UI text · 16–18px card headings/item names · 22–28px hero numbers (size values, percentages) · 20px main pane title.
- Numeric values (sizes, percentages, counts) consistently bold/semibold; labels/captions regular weight, muted color.
- Section headers use uppercase + letter-spacing + small gray color (e.g., "RECENT", "QUICK WINS", "DETAILS", "LARGEST INSIDE").

### 3.3 Spacing & Shape
- Base radius: 8–12px on cards/buttons; fully rounded (pill) on the active-tab buttons and the primary Scan button.
- Consistent 12–16px internal card padding; 8–12px gaps between stacked list rows.
- Thin 1px borders (`#E8E9ED`) separate cards from background rather than heavy shadows; subtle/no drop shadow on most elements (flat design with light elevation only on hover/selection states).

### 3.4 Iconography
- Line icons throughout (not filled), ~16–20px, stroke-based, consistent weight.
- Folder icon: simple rounded folder glyph, colored to match category where used as a bullet/marker.
- Toolbar view-switcher icons are small monochrome pictograms representing each chart type (list lines, calendar/clock, bar chart, branching tree, circles, flame, radial rings, grid squares, nested squares).

### 3.5 Selection / Interaction States
- Table/list row selected: full-row light-purple background fill with rounded corners (List view style).
- Ranked-list row selected: outline/border box only, no fill (Top Sizes style).
- Diagram node/segment selected: bold dark stroke/outline added to that shape, no fill change (Mind Map, Bubbles, Flame, Sunburst, Treemap style) — this is the dominant selection pattern for chart-based views, while table-based views (List, Top Sizes, Folders) use background/border highlight instead.
- Right panel content is reactive: it always reflects whatever node/row/segment is currently selected in the main pane, regardless of which of the 9 view modes is active — this is a single shared "selection" state driving both panes.

---

## 4. Implied Data Model

Each filesystem entry (folder or file) carries:
- `name`, `path`, `type` (folder/file/special)
- `sizeOnDisk`, `logicalSize`, `compressedBy`
- `fileCount`, `folderCount` (if folder)
- `percentOfParent`, `percentOfScan`
- `modifiedAt`, `createdAt` (relative-time display)
- `children[]` (recursive tree)
- `categoryColor` (assigned per top-level ancestor for consistent cross-view coloring)
- A separate "Quick Wins" classifier tags items into categories: Caches & logs, Large media, node_modules, Build artifacts, Downloads, iOS Simulators, Xcode DerivedData, etc. — likely rule-based (path/name pattern matching) independent of the folder tree.
- Global scan metadata: total size, total files, total folders, elapsed scan time, list of unreadable/permission-denied folder count (for the warning banner).

---

## 5. Rebuild Checklist for an Agent

1. Build the 3-pane shell (dark sidebar / light main / light inspector) as a persistent layout wrapping all views.
2. Implement the top bar tab navigation (Explore/Duplicates/Applications/Monitor/Snapshots) with pill-active styling.
3. Implement sidebar: Scan button, Home/Folder buttons, Recent, animated/static disk-usage donut ring, Current View block, Quick Wins scrollable list.
4. Implement the shared main-pane header: title + stats line, 9-icon view switcher + active-view pill + description text, dismissible warning banner.
5. Implement the shared right inspector panel bound to a single "selectedItem" state: name/type/path header, size hero, Details card, Largest Inside card, 2×2 action buttons, sticky "Add to Cleanup" button.
6. Build each of the 9 visualizations as interchangeable body components, all reading from the same hierarchical file-tree data and writing to the same `selectedItem` state on click:
   - List (expandable outline table)
   - Age Map (bucketed bar chart + month/year heatmap)
   - Top Sizes (ranked table with 3 sub-tab filters)
   - Mind Map (radial node-link tree)
   - Bubbles (circle packing)
   - Flame (icicle/flame graph, with By type/folder/age + depth slider toolbar)
   - Sunburst (radial hierarchy, same sub-toolbar as Flame)
   - Folders (card grid, one level at a time, drill-down navigation)
   - Treemap (squarified treemap, same sub-toolbar as Flame/Sunburst)
7. Apply the shared color system: assign a stable color per top-level ancestor folder and reuse it across every chart type so the same folder always reads as the same color regardless of view.
8. Apply consistent selection affordances per view family (fill for tables, outline for ranked lists, stroke-outline for diagrams).
