/**
 * Mock synthetic tree (DEV/TEST ONLY — never shipped in a Tauri build).
 * A deterministic, Windows-shaped file system model (~700 nodes) with
 * sizes, categories, ages, cloud + protected flags, so the whole UI can
 * be exercised in a plain browser against the exact command surface.
 */

/** Deterministic PRNG (mulberry32) — stable screenshots. */
function rng(seed: number): () => number {
  let a = seed;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export interface MockNode {
  id: number;
  name: string;
  parent: number;
  isDir: boolean;
  category: number; // 0..8 index
  logical: number;
  onDisk: number;
  modified: number; // unix seconds
  created: number;
  cloud: boolean;
  protected: boolean;
  children: number[];
}

export const CATEGORY_LABELS = [
  "Video",
  "Audio",
  "Images",
  "Documents",
  "Developer",
  "Archives",
  "Applications",
  "System",
  "Other",
];
export const CATEGORY_COLORS = [
  0xfda4af, 0xc4b5fd, 0x7dd3fc, 0xfcd34d, 0x86efac, 0xfdba74, 0x93c5fd, 0x5eead4, 0xcbd5e1,
];

const NOW = Math.floor(Date.now() / 1000);
const DAY = 86400;
const MB = 1024 * 1024;
const GB = 1024 * MB;

const EXT_BY_CATEGORY: Record<number, string[]> = {
  0: ["mp4", "mkv", "mov", "avi", "webm"],
  1: ["mp3", "flac", "wav", "m4a", "ogg"],
  2: ["jpg", "png", "heic", "gif", "tiff", "psd", "raw"],
  3: ["pdf", "docx", "xlsx", "pptx", "txt", "md", "csv", "epub"],
  4: ["rs", "ts", "tsx", "js", "json", "py", "toml", "yml", "lock", "dll"],
  5: ["zip", "7z", "tar", "gz", "rar", "iso"],
  6: ["exe", "msi", "msix", "appx"],
  7: ["db", "sqlite", "log", "dat", "etl", "cab", "sys"],
  8: ["bin", "dat", "bak", "misc"],
};

const FILE_NAME_STEMS = [
  "report", "photo", "backup", "project", "invoice", "video", "track", "dataset",
  "install", "patch", "notes", "export", "render", "capture", "archive", "config",
  "session", "draft", "final", "master", "copy", "cache", "temp", "log",
];

export class MockTree {
  nodes: MockNode[] = [];
  generation = 1;
  scanRootPath = "C:\\Users\\dev";
  scanDurationMs = 5420;
  denied = { count: 3, samples: ["C:\\Users\\dev\\AppData\\Local\\Secrets", "C:\\Recovery", "C:\\System Volume Information"] };
  private rand = rng(20260922);

  constructor() {
    this.build();
  }

  private push(n: Omit<MockNode, "id" | "children">): number {
    const id = this.nodes.length;
    this.nodes.push({ ...n, id, children: [] });
    if (n.parent < id && n.parent >= 0) {
      this.nodes[n.parent].children.push(id);
    }
    return id;
  }

  private file(parent: number, cat: number, size: number, ageDays: number, cloud = false): number {
    const exts = EXT_BY_CATEGORY[cat];
    const ext = exts[Math.floor(this.rand() * exts.length)];
    const stem = FILE_NAME_STEMS[Math.floor(this.rand() * FILE_NAME_STEMS.length)];
    const num = Math.floor(this.rand() * 900 + 100);
    return this.push({
      name: `${stem}-${num}.${ext}`,
      parent,
      isDir: false,
      category: cat,
      logical: size,
      onDisk: cloud ? 0 : Math.round(size * (0.62 + this.rand() * 0.35)),
      modified: NOW - Math.round(ageDays * DAY * (0.5 + this.rand())),
      created: NOW - Math.round(ageDays * DAY * (1.5 + this.rand() * 2)),
      cloud,
      protected: false,
    });
  }

  private folder(parent: number, name: string, protectedDir = false): number {
    return this.push({
      name,
      parent,
      isDir: true,
      category: 8,
      logical: 0,
      onDisk: 0,
      modified: NOW - Math.round(this.rand() * 90 * DAY),
      created: NOW - Math.round((365 + this.rand() * 500) * DAY),
      cloud: false,
      protected: protectedDir,
    });
  }

  /** Generate `count` files in a distribution across categories. */
  private sprinkle(parent: number, count: number, sizeFloor: number, sizeCeil: number): void {
    // Weighted categories: docs/developer/media heavy, apps rare.
    const weights = [12, 10, 16, 20, 18, 7, 3, 9, 5];
    for (let i = 0; i < count; i++) {
      let pick = this.rand() * weights.reduce((a, b) => a + b, 0);
      let cat = 0;
      for (let c = 0; c < weights.length; c++) {
        pick -= weights[c];
        if (pick <= 0) {
          cat = c;
          break;
        }
      }
      const size = Math.round(sizeFloor + this.rand() * (sizeCeil - sizeFloor));
      const age = this.rand() ** 2 * 800; // skewed recent
      const cloud = cat === 3 && this.rand() < 0.08;
      this.file(parent, cat, size, Math.max(0.2, age), cloud);
    }
  }

  private build(): void {
    const root = this.push({
      name: "This PC",
      parent: -1,
      isDir: true,
      category: 8,
      logical: 0,
      onDisk: 0,
      modified: NOW,
      created: NOW - 400 * DAY,
      cloud: false,
      protected: false,
    });

    const c = this.folder(root, "Local Disk (C:)");
    // ── Users ────────────────────────────────────────────────────────
    const users = this.folder(c, "Users");
    const dev = this.folder(users, "dev");
    const desktop = this.folder(dev, "Desktop");
    this.sprinkle(desktop, 26, 400 * 1024, 220 * MB);
    const docs = this.folder(dev, "Documents");
    this.sprinkle(docs, 44, 100 * 1024, 45 * MB);
    const work = this.folder(docs, "Work");
    this.sprinkle(work, 38, 500 * 1024, 30 * MB);
    const invoices = this.folder(work, "Invoices 2026");
    this.sprinkle(invoices, 14, 80 * 1024, 6 * MB);
    const downloads = this.folder(dev, "Downloads");
    this.sprinkle(downloads, 33, 1 * MB, 1.4 * GB);
    const pictures = this.folder(dev, "Pictures");
    this.sprinkle(pictures, 58, 900 * 1024, 48 * MB);
    const cameraRoll = this.folder(pictures, "Camera Roll");
    this.sprinkle(cameraRoll, 41, 3 * MB, 24 * MB);
    const videos = this.folder(dev, "Videos");
    this.sprinkle(videos, 12, 340 * MB, 3.2 * GB);
    const music = this.folder(dev, "Music");
    this.sprinkle(music, 19, 6 * MB, 110 * MB);
    const appdata = this.folder(dev, "AppData");
    const local = this.folder(appdata, "Local");
    const roaming = this.folder(appdata, "Roaming");
    const temp = this.folder(local, "Temp");
    this.sprinkle(temp, 47, 30 * 1024, 900 * MB);
    const caches = this.folder(local, "Caches");
    this.sprinkle(caches, 36, 1 * MB, 650 * MB);
    const google = this.folder(local, "Google");
    const chromeProfile = this.folder(google, "Chrome");
    const userData = this.folder(chromeProfile, "User Data");
    const profile0 = this.folder(userData, "Default");
    const cacheDir = this.folder(profile0, "Cache");
    this.sprinkle(cacheDir, 22, 200 * 1024, 320 * MB);
    const codeCache = this.folder(profile0, "Code Cache");
    this.sprinkle(codeCache, 15, 120 * 1024, 210 * MB);
    this.sprinkle(profile0, 30, 40 * 1024, 70 * MB);
    const msEdge = this.folder(local, "Microsoft");
    const edgeData = this.folder(msEdge, "Edge");
    const edgeProfile = this.folder(edgeData, "User Data");
    const edgeCache = this.folder(edgeProfile, "Default");
    this.sprinkle(edgeCache, 18, 90 * 1024, 160 * MB);
    const packages = this.folder(local, "Packages");
    this.sprinkle(packages, 24, 200 * 1024, 340 * MB);
    const crashDumps = this.folder(local, "CrashDumps");
    this.sprinkle(crashDumps, 7, 30 * MB, 480 * MB);
    const npmCache = this.folder(local, "npm-cache");
    this.sprinkle(npmCache, 42, 100 * 1024, 40 * MB);
    const pipCache = this.folder(local, "pip");
    this.sprinkle(pipCache, 19, 300 * 1024, 55 * MB);
    this.sprinkle(local, 60, 30 * 1024, 60 * MB);
    this.sprinkle(roaming, 48, 20 * 1024, 90 * MB);
    const dotCargo = this.folder(dev, ".cargo");
    const registry = this.folder(dotCargo, "registry");
    this.sprinkle(registry, 31, 500 * 1024, 30 * MB);
    const projects = this.folder(dev, "Projects");
    const projA = this.folder(projects, "website-2026");
    const nmA = this.folder(projA, "node_modules");
    this.sprinkle(nmA, 120, 3 * 1024, 3 * MB);
    const nextCache = this.folder(projA, ".next");
    this.sprinkle(nextCache, 40, 20 * 1024, 2 * MB);
    this.sprinkle(projA, 22, 1 * 1024, 900 * 1024);
    const projB = this.folder(projects, "disk-scanner");
    const targetDir = this.folder(projB, "target");
    this.sprinkle(targetDir, 55, 50 * 1024, 28 * MB);
    const nmB = this.folder(projB, "node_modules");
    this.sprinkle(nmB, 88, 2 * 1024, 2.5 * MB);
    this.sprinkle(projB, 18, 800, 400 * 1024);
    const projC = this.folder(projects, "ml-pipeline");
    const pycache = this.folder(projC, "__pycache__");
    this.sprinkle(pycache, 16, 5 * 1024, 2 * MB);
    this.sprinkle(projC, 21, 1 * 1024, 600 * 1024);
    const virtualvms = this.folder(dev, "VirtualBox VMs");
    this.file(virtualvms, 5, 22 * GB, 120);
    this.file(virtualvms, 5, 8.4 * GB, 200);
    const dotandroid = this.folder(dev, ".android");
    const avd = this.folder(dotandroid, "avd");
    this.sprinkle(avd, 8, 300 * MB, 1.6 * GB);
    this.sprinkle(dev, 24, 1 * 1024, 900 * 1024);

    // ── Program Files ────────────────────────────────────────────────
    const pf = this.folder(c, "Program Files");
    const ide = this.folder(pf, "JetBrains");
    this.sprinkle(ide, 40, 2 * MB, 300 * MB);
    const vscode = this.folder(pf, "Microsoft VS Code");
    this.sprinkle(vscode, 38, 60 * 1024, 40 * MB);
    const tools = this.folder(pf, "Utilities");
    this.sprinkle(tools, 17, 100 * 1024, 25 * MB);
    this.sprinkle(pf, 26, 300 * 1024, 90 * MB);
    const pf86 = this.folder(c, "Program Files (x86)");
    this.sprinkle(pf86, 33, 90 * 1024, 40 * MB);
    const windowsApps = this.folder(pf, "WindowsApps", true);
    this.sprinkle(windowsApps, 21, 500 * 1024, 120 * MB);

    // ── Windows (protected) ──────────────────────────────────────────
    const win = this.folder(c, "Windows", true);
    const winsxs = this.folder(win, "WinSxS", true);
    this.sprinkle(winsxs, 74, 400 * 1024, 28 * MB);
    const installer = this.folder(win, "Installer", true);
    this.sprinkle(installer, 29, 1 * MB, 120 * MB);
    const system32 = this.folder(win, "System32", true);
    this.sprinkle(system32, 96, 60 * 1024, 9 * MB);
    this.sprinkle(win, 58, 20 * 1024, 3 * MB);
    this.file(c, 7, 8 * GB, 3, ); // pagefile.sys
    this.nodes[this.nodes.length - 1].name = "pagefile.sys";
    this.nodes[this.nodes.length - 1].protected = true;
    this.file(c, 7, 6.4 * GB, 40);
    this.nodes[this.nodes.length - 1].name = "hiberfil.sys";
    this.nodes[this.nodes.length - 1].protected = true;
    this.sprinkle(c, 14, 10 * 1024, 2 * MB);

    this.rollup();
  }

  /** Reverse roll-up + largest-first child order (mirrors core rollup). */
  private rollup(): void {
    for (let i = this.nodes.length - 1; i >= 1; i--) {
      const n = this.nodes[i];
      const p = this.nodes[n.parent];
      if (!p) continue;
      p.logical += n.logical;
      p.onDisk += n.onDisk;
      if (!n.isDir) {
        (p as MockNode & { fileCount?: number }).fileCount = ((p as MockNode & { fileCount?: number }).fileCount ?? 0) + 1;
      } else {
        (p as MockNode & { folderCount?: number }).folderCount =
          ((p as MockNode & { folderCount?: number }).folderCount ?? 0) + 1;
      }
      p.modified = Math.max(p.modified, n.modified);
    }
    for (const n of this.nodes) {
      if (!n.isDir) continue;
      n.children.sort((a, b) => this.nodes[b].onDisk - this.nodes[a].onDisk);
    }
  }

  stats(node: number): { logical: number; onDisk: number; files: number; folders: number } {
    const n = this.nodes[node];
    let files = 0;
    let folders = 0;
    for (const c of n.children) {
      const ch = this.nodes[c];
      if (ch.isDir) folders++;
      else files++;
    }
    const agg = this.aggregate(node);
    return { logical: n.logical || agg.logical, onDisk: n.onDisk || agg.onDisk, files: agg.files, folders: agg.folders };
  }

  aggregate(node: number): { files: number; folders: number; logical: number; onDisk: number } {
    let files = 0;
    let folders = 0;
    const walk = (id: number): void => {
      for (const c of this.nodes[id].children) {
        const ch = this.nodes[c];
        if (ch.isDir) {
          folders++;
          walk(c);
        } else {
          files++;
        }
      }
    };
    walk(node);
    const n = this.nodes[node];
    return { files, folders, logical: n.logical, onDisk: n.onDisk };
  }

  pathOf(id: number): string {
    // Parity with core node_path: the synthetic This-PC root yields the
    // LABEL "This PC", not a real path (the mock used to return "C:\",
    // so the frontend's virtual-root staging guard never fired in dev).
    if (id === 0) return "This PC";
    const parts: string[] = [];
    let cur = id;
    while (cur >= 0 && cur !== 0) {
      parts.unshift(this.nodes[cur].name);
      cur = this.nodes[cur].parent;
    }
    return "C:\\" + parts.slice(1).join("\\");
  }

  dominantCategory(id: number): number {
    const counts = new Array(9).fill(0);
    const walk = (nid: number): void => {
      const n = this.nodes[nid];
      if (!n.isDir) counts[n.category] += n.logical;
      for (const c of n.children) walk(c);
    };
    walk(id);
    let best = 8;
    let bestSize = -1;
    for (let i = 0; i < 9; i++) {
      if (counts[i] > bestSize) {
        best = i;
        bestSize = counts[i];
      }
    }
    return best;
  }

  /** Top categories with sizes for folder-card dots / file-types bar. */
  topCategories(id: number, k: number): { category: number; size: number }[] {
    const counts = new Array(9).fill(0);
    const walk = (nid: number): void => {
      const n = this.nodes[nid];
      if (!n.isDir) counts[n.category] += n.logical;
      for (const c of n.children) walk(c);
    };
    walk(id);
    return counts
      .map((size, category) => ({ category, size }))
      .filter((x) => x.size > 0)
      .sort((a, b) => b.size - a.size)
      .slice(0, k);
  }

  allDescendants(id: number): number[] {
    const out: number[] = [];
    const walk = (nid: number): void => {
      out.push(nid);
      for (const c of this.nodes[nid].children) walk(c);
    };
    walk(id);
    return out;
  }

  /** Files anywhere under `id`, largest first. */
  filesAnywhere(id: number, cap: number): { id: number; node: MockNode }[] {
    const out: { id: number; node: MockNode }[] = [];
    for (const nid of this.allDescendants(id)) {
      const n = this.nodes[nid];
      if (!n.isDir && !n.cloud) out.push({ id: nid, node: n });
    }
    out.sort((a, b) => b.node.logical - a.node.logical);
    return out.slice(0, cap);
  }

  /** Folders anywhere under `id`, largest first. */
  foldersAnywhere(id: number, cap: number): { id: number; node: MockNode }[] {
    const out: { id: number; node: MockNode }[] = [];
    for (const nid of this.allDescendants(id)) {
      const n = this.nodes[nid];
      if (n.isDir && nid !== id) out.push({ id: nid, node: n });
    }
    out.sort((a, b) => b.node.onDisk - a.node.onDisk);
    return out.slice(0, cap);
  }

  /** Tree surgery for commit_cleanup: remove subtrees + ancestor subtraction. */
  removeSubtrees(ids: number[]): void {
    const roots = new Set<number>();
    for (const id of ids) {
      let top = id;
      let p = this.nodes[id].parent;
      while (p >= 0 && !ids.includes(p) && !this.removed.has(p)) {
        top = id;
        break;
      }
      if (!this.removed.has(this.nodes[id].parent)) roots.add(top);
    }
    const removed = new Set<number>();
    for (const r of ids) {
      for (const d of this.allDescendants(r)) removed.add(d);
    }
    // subtract from ancestors
    for (const r of ids) {
      const n = this.nodes[r];
      let p = n.parent;
      while (p >= 0) {
        const pn = this.nodes[p];
        pn.logical = Math.max(0, pn.logical - n.logical);
        pn.onDisk = Math.max(0, pn.onDisk - n.onDisk);
        pn.children = pn.children.filter((c) => c !== r && !removed.has(c));
        p = pn.parent;
      }
    }
    for (const r of removed) {
      this.removed.add(r);
    }
    this.generation++;
  }

  removed = new Set<number>();
}
