#!/usr/bin/env python3
"""Insert the quickwins resolver helpers before the commands registry,
then replace the two handlers. Run AFTER port_quickwins.py if needed —
this version does both in one pass from the CURRENT file state."""

path = "src/mock/commands.ts"
data = open(path).read()

# 1) Insert helpers before `const commands: Record<string, Cmd> = {`
anchor = "const commands: Record<string, Cmd> = {"
assert anchor in data

helpers = '''// ── Quick Wins engine (port of core quickwins.rs, Windows table) ───
const CATEGORY_CAP = 400; // core quickwins::CATEGORY_CAP
const QW_USERPROFILE = "C:\\\\Users\\\\dev";
const QW_LOCALAPPDATA = "C:\\\\Users\\\\dev\\\\AppData\\\\Local";

interface QwCat {
  id: string;
  title: string;
  icon: string;
  items: number[];
  size: number;
  reviewOnly: boolean;
  extra: string | null;
}

function resolveQuickWins(): QwCat[] {
  const eqCi = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();
  const findByPath = (p: string): number => {
    for (let i = 1; i < tree.nodes.length; i++) {
      if (eqCi(tree.pathOf(i), p)) return i;
    }
    return 0; // Rust: unwrap_or(tree.root)
  };
  // match_pattern parity: final-segment matches under root; '*' = one segment.
  const matchPattern = (root: string, segments: string[]): number[] => {
    let current = [findByPath(root)];
    for (const seg of segments) {
      const next: number[] = [];
      for (const cur of current) {
        for (const c of tree.nodes[cur].children) {
          const cn = tree.nodes[c];
          if (seg === "*" || eqCi(cn.name, seg)) next.push(c);
        }
      }
      current = next;
      if (current.length === 0) break;
    }
    return current.filter((id) => id !== 0);
  };
  // find_named parity (any depth, dirs-only option).
  const findNamed = (name: string, dirsOnly: boolean): number[] => {
    const out: number[] = [];
    for (let i = 1; i < tree.nodes.length; i++) {
      const n = tree.nodes[i];
      if ((!dirsOnly || n.isDir) && eqCi(n.name, name)) out.push(i);
    }
    return out;
  };
  const isDescendantOf = (desc: number, anc: number): boolean => {
    let cur = tree.nodes[desc].parent;
    while (cur > 0) {
      if (cur === anc) return true;
      cur = tree.nodes[cur].parent;
    }
    return false;
  };
  const nodeSize = (id: number): number => tree.nodes[id].onDisk || 0;
  // push_cat parity: drop items nested inside a same-category match,
  // cap at 400, sum on-disk, drop empty categories.
  const pushCat = (
    id: string, title: string, icon: string, reviewOnly: boolean,
    extra: string | null, items: number[],
  ): QwCat | null => {
    const filtered: number[] = [];
    for (const it of items) {
      if (filtered.some((f) => isDescendantOf(it, f))) continue;
      filtered.push(it);
      if (filtered.length >= CATEGORY_CAP) break;
    }
    if (filtered.length === 0) return null;
    return {
      id, title, icon, items: filtered,
      size: filtered.reduce((a, x) => a + nodeSize(x), 0),
      reviewOnly, extra,
    };
  };

  const out: QwCat[] = [];
  // Pattern categories (Windows table, verbatim locations + env roots).
  const browsers = ["Google\\\\Chrome", "Microsoft\\\\Edge", "BraveSoftware\\\\Brave-Browser"];
  const cacheSegs = ["Cache", "Code Cache", "GPUCache"];
  const patternCats: [string, string, string, string, string[]][] = [
    ["downloads", "Downloads", "download", QW_USERPROFILE, ["Downloads"]],
    ["temp_caches", "Temp & caches", "temp", QW_LOCALAPPDATA, ["Temp"]],
    ["temp_caches", "Temp & caches", "temp", QW_LOCALAPPDATA, ["Microsoft", "Windows", "INetCache"]],
    ["temp_caches", "Temp & caches", "temp", QW_LOCALAPPDATA, ["CrashDumps"]],
    ["temp_caches", "Temp & caches", "temp", QW_LOCALAPPDATA, ["D3DSCache"]],
    ["temp_caches", "Temp & caches", "temp", QW_LOCALAPPDATA, ["Microsoft", "Windows", "WER"]],
    ...browsers.flatMap((b) =>
      cacheSegs.map((cache) => [
        "browser_caches", "Browser caches", "browser", QW_LOCALAPPDATA,
        [...b.split("\\\\"), "*", cache],
      ] as [string, string, string, string, string[]]),
    ),
    ["browser_caches", "Browser caches", "browser", QW_LOCALAPPDATA, ["Mozilla", "Firefox", "Profiles", "*", "cache2"]],
    ["dev_caches", "Developer caches", "code", QW_USERPROFILE, [".nuget", "packages"]],
    ["dev_caches", "Developer caches", "code", QW_USERPROFILE, [".cargo", "registry"]],
    ["dev_caches", "Developer caches", "code", QW_USERPROFILE, [".gradle", "caches"]],
    ["dev_caches", "Developer caches", "code", QW_LOCALAPPDATA, ["npm-cache"]],
    ["dev_caches", "Developer caches", "code", QW_LOCALAPPDATA, ["pip", "Cache"]],
    ["dev_caches", "Developer caches", "code", QW_LOCALAPPDATA, ["pnpm", "store"]],
    ["dev_caches", "Developer caches", "code", QW_LOCALAPPDATA, ["Yarn", "Cache"]],
    ["android_emulators", "Android emulators", "phone", QW_USERPROFILE, [".android", "avd"]],
  ];
  const buckets = new Map<string, number[]>();
  for (const [cat, , , root, segs] of patternCats) {
    for (const id of matchPattern(root, segs)) {
      const b = buckets.get(cat);
      if (b) b.push(id);
      else buckets.set(cat, [id]);
    }
  }
  const catMeta: [string, string, string][] = [
    ["downloads", "Downloads", "download"],
    ["temp_caches", "Temp & caches", "temp"],
    ["browser_caches", "Browser caches", "browser"],
    ["dev_caches", "Developer caches", "code"],
    ["android_emulators", "Android emulators", "phone"],
  ];
  for (const [id, title, icon] of catMeta) {
    const items = buckets.get(id);
    if (items) {
      const cat = pushCat(id, title, icon, false, null, items);
      if (cat) out.push(cat);
    }
  }

  // node_modules: any depth, dirs named node_modules.
  {
    const cat = pushCat("node_modules", "node_modules", "code", false, null, findNamed("node_modules", true));
    if (cat) out.push(cat);
  }
  // Build artifacts: unconditional names + sibling-ruled target/bin/obj.
  {
    const BUILD_ARTIFACT_NAMES = [
      "build", ".build", "dist", ".next", ".nuxt", ".turbo", ".parcel-cache",
      ".terraform", "__pycache__", ".pytest_cache", ".mypy_cache", ".ruff_cache",
      ".tox", ".gradle",
    ];
    const ba: number[] = [];
    for (let i = 1; i < tree.nodes.length; i++) {
      const n = tree.nodes[i];
      if (!n.isDir) continue;
      if (BUILD_ARTIFACT_NAMES.some((f) => eqCi(n.name, f))) { ba.push(i); continue; }
      if (n.parent <= 0) continue;
      const siblings = tree.nodes[n.parent].children.map((c) => tree.nodes[c].name);
      if (eqCi(n.name, "target")) {
        if (siblings.some((s) => s === "Cargo.toml" || s === "pom.xml")) ba.push(i);
      } else if (eqCi(n.name, "bin") || eqCi(n.name, "obj")) {
        if (siblings.some((s) => s === "project.json" || /\\.(csproj|vcxproj)$/i.test(s))) ba.push(i);
      }
    }
    const cat = pushCat("build_artifacts", "Build artifacts", "hammer", false, null, ba);
    if (cat) out.push(cat);
  }
  // Large media: video/audio/image files >= 10 MB (cats 0/1/2).
  {
    const lm: number[] = [];
    for (let i = 1; i < tree.nodes.length; i++) {
      const n = tree.nodes[i];
      if (!n.isDir && n.logical >= 10 * MB && (n.category === 0 || n.category === 1 || n.category === 2)) lm.push(i);
    }
    const cat = pushCat("large_media", "Large media", "video", false, null, lm);
    if (cat) out.push(cat);
  }
  // VM disks (review-only): VM_DISK_ROOTS + any .vhdx >= 1 GB.
  {
    const vmRoots: [string, string[]][] = [
      [QW_LOCALAPPDATA, ["Packages", "*", "LocalState"]],
      [QW_LOCALAPPDATA, ["Docker"]],
      [QW_USERPROFILE, ["VirtualBox VMs"]],
    ];
    const vm: number[] = [];
    for (const [root, segs] of vmRoots) vm.push(...matchPattern(root, segs));
    for (let i = 1; i < tree.nodes.length; i++) {
      const n = tree.nodes[i];
      if (!n.isDir && n.logical >= GB && /\\.vhdx$/i.test(n.name)) vm.push(i);
    }
    const cat = pushCat("vm_disks", "VM disks", "server", true, null, vm);
    if (cat) out.push(cat);
  }
  // Previous Windows install (review-only + storagesense link).
  {
    const cat = pushCat("windows_old", "Previous Windows install", "clock", true, "ms-settings:storagesense", findNamed("Windows.old", true));
    if (cat) out.push(cat);
  }
  out.sort((a, b) => b.size - a.size);
  return out;
}

/** Row shape for the UI (items stripped — ids are engine-internal). */
function stripItems(c: QwCat): Record<string, unknown> {
  return {
    id: c.id,
    title: c.title,
    icon: c.icon,
    count: c.items.length,
    size: c.size,
    reviewOnly: c.reviewOnly,
    extra: c.extra,
    biggestMatch: c.items[0],
  };
}

'''

data = data.replace(anchor, helpers + anchor, 1)

# 2) Replace the two handlers (current state after port_quickwins.py may
#    already have the new handlers; handle both old and new).
start = data.find("  // ── sidebar data ───")
end = data.find("  file_types: () => {")
if start > 0 and end > start:
    new_handlers = '''  // ── sidebar data ──────────────────────────────────────────────────
  quick_wins: () => resolveQuickWins().map(stripItems),
  quick_win_items: (a) => {
    // Parity with the Rust command: re-resolve, find the category,
    // cap at CATEGORY_CAP, return {id, path, size: on_disk}.
    const cat = resolveQuickWins().find((c) => c.id === String(a.categoryId));
    if (!cat) return [];
    return cat.items
      .slice(0, CATEGORY_CAP)
      .map((id) => ({ id, path: fmtPath(id), size: tree.nodes[id].onDisk || 0 }));
  },
'''
    data = data[:start] + new_handlers + data[end:]

open(path, "w").write(data)
print("done: helpers inserted + handlers replaced")
