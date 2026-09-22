# DiskBytes — §18 Acceptance Evidence (M11 closeout)

> BuildPrompt §18 + doc 09 §3 spec-conformance matrix, item by item, with
> evidence collected on 2026-09-21 (session 004). Two evidence classes:
>
> - **CI** — the `windows-latest` battery (runs on every push to `main`):
>   fmt → clippy `-D warnings` (workspace, all targets) → `cargo test
>   --workspace` → 1M-node bench (<2 s) → npm ci → typecheck → vitest →
>   R7.1 grep → R2.2 grep → frontend build → NSIS bundle + portable zip +
>   SHA256SUMS artifact.
>   - **Authoritative gates run: 35638411781 on commit `1223082` (M11 head)
>     — `Static gates + core tests` job SUCCESS** (all 14 gate steps green,
>     first green run of the complete M0–M11 tree: fmt, clippy with the
>     crate's pedantic profile, full workspace tests incl. the M10
>     license/analytics and M8 applications suites, bench, typecheck,
>     vitest 25, both greps, frontend build). Ran on `chainproof-dev`
>     before its Actions quota exhausted.
>   - **Authoritative bundle run: 35642632772 on commit `981c077` —
>     `Windows NSIS bundle (real build)` job SUCCESS on `tensorkernel`
>     (the migrated home account)**: brand icon (programmatic 1024px
>     source → `npx tauri icon` fan-out) → `tauri build` NSIS per-user →
>     portable zip → SHA256SUMS → artifact `diskbytes-windows-release`
>     (id 10660126921). Triggered via the `nsis-bundle.yml`
>     dispatch-only workflow (gates not re-run — already green above,
>     per owner instruction). Artifacts downloaded and hash-verified
>     locally: `DiskBytes-0.1.0-x64-setup.exe` = 4,808,624 bytes
>     sha256 `249fe20c…`, `DiskBytes-0.1.0-x64-portable.zip` = 3,911,294
>     bytes sha256 `15a3daff…` — byte-exact vs the CI log.
>   - **NSIS bundle on the M9 head** was also green earlier (run
>     35627021735, `chainproof-dev`) — the pipeline was never the
>     problem; only the account quota was.
> - **Host** — commands executed in the dev sandbox this session, output
>   pasted in `/home/z/my-project/worklog.md` entries S4-4..S4-7.
>
> Windows-GUI items (real interactive verification) are listed as **PENDING
> Windows session** — they require a human at a Windows desktop, per the
> owner's model: CI verifies everything automatable on windows-latest; a
> Windows session gives the final sign-off.

## §18 checklist (BuildPrompt, verbatim order)

| # | Item | Status | Evidence |
|---|------|--------|----------|
| 1 | `npm install && npm run tauri dev` opens the window; `cargo clippy -- -D warnings`, `cargo test`, `npm run typecheck` pass | PARTIAL — gates PASS, window PENDING | clippy/test/typecheck = **CI run 35638411781 (1223082), gates job SUCCESS** (workspace clippy `-D warnings` incl. pedantic profile, `cargo test --workspace`, typecheck); typecheck + vitest also host-verified this session. Window opening = Windows GUI session (needs WebView2 + interactive desktop). |
| 2 | direct-delete grep finds nothing | PASS | Host: `git grep -nE "remove_file\|remove_dir\|remove_dir_all\|DeleteFileW\|RemoveDirectoryW\|SHEmptyRecycleBin" src-tauri/src` → empty. CI R7.1 gate identical pattern, runs on every push. |
| 3 | Profile scan shows sizes, counts, Quick Wins, File Types | PENDING Windows session | Scan pipeline (NtQueryDirectoryFile engine + worker pool + incremental tree updates) covered by core tests (124) + app-crate tests on CI; visual confirmation needs the GUI. |
| 4 | This PC across two drives | PENDING Windows session | Drive enumeration + This PC root implemented (S3-x worklog; root naming regression-tested after the CI-found bug). |
| 5 | Turbo vs standard totals within 1% (idle drive, DISKBYTES_VERIFY) | PENDING Windows session | Two-engine comparison code + turbo-report shipped in M7 (commit d1c1333); the ≤1% agreement requires a real NTFS drive with elevation. |
| 6 | Non-elevated Turbo toggle explains; standard runs | PENDING Windows session | ELEVATION_REQUIRED gate + user-visible fallback event + standard rerun (M7); visual + real-elevation check on Windows. |
| 7 | All 9 visualizations render; hover chip, click-select, double-click-open, depth, color modes, abbreviation | PENDING Windows session | 9 modes implemented M4 (S3-4/S3-5 worklogs); abbrev + cell-flag unit tests in vitest (src/viz/abbrev.test.ts); rendering needs the GPU/canvas environment. |
| 8 | Staging from inspector/Age Map/Quick Wins/Duplicates/uninstaller updates the badge; Clear and ✕ update instantly | PARTIAL — logic PASS, visual PENDING | vitest `cleanup.test.ts`: "badge count updates instantly on unstage/remove/clear (spec pitfall)" — the exact §18 pitfall. All five staging sources wired (InspectorPanel, AgeMapView, QuickWins, DuplicatesView, ApplicationsView leftovers). |
| 9 | Recycle: red button, confirm-first, recycles, views update without rescan, refuses non-recyclable | PENDING Windows session | IFileOperation → Recycle Bin ONLY (recycle.rs, R7.1 grep backs it); refusal paths incl. E_ACCESSDENIED admin hint (CI-found dead-table bug fixed, commit e8d6a2d); interactive proof on Windows. |
| 10 | Cloud placeholders never opened, previewed or hashed | PASS (code-review) | Scanner flags placeholders (FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS/_OPEN, offline); dupes.rs skips `is_cloud_placeholder()` before opening (never hashed, R7.3); preview gates on the flag; grep `is_cloud_placeholder` across scan/dupes/explore confirms the guards. |
| 11 | Monitor updates every 2 s; snapshots save, list, compare, delete | PENDING Windows session | 2 s sampler thread (commands/monitor.rs, StrictMode-safe start/stop); snapshots round-trip tests (S3-6); cadence + UI on Windows. |
| 12 | Light/Dark toggle persists, no startup flash | PENDING Windows session | Theme tokens + `useTheme` + inline pre-mount script in index.html (M1); persistence via localStorage; visual check on Windows. |
| 13 | `scripts/build.ps1 -Arch x64` → per-user NSIS installer + SHA256SUMS.txt | **PASS** | Bundle run 35642632772 on `981c077` (tensorkernel): icon → `tauri build` NSIS per-user (currentUser, embedBootstrapper) → `DiskBytes-0.1.0-x64-setup.exe` (4,808,624 B) + `DiskBytes-0.1.0-x64-portable.zip` (3,911,294 B) + SHA256SUMS.txt, artifact `diskbytes-windows-release`. Downloaded this session; local sha256 matches the CI-emitted SHA256SUMS byte-exact (`249fe20c…` / `15a3daff…`). Same pipeline as `scripts/build.ps1 -Arch x64`; per-user SmartScreen guidance in docs/DISTRIBUTION.md. |

## doc 09 §3 matrix additions (items beyond §18)

| # | Check | Status | Evidence |
|---|------|--------|----------|
| 14 | React pitfalls audit (doc 05 §7) | PASS | Performed S4-4: (1) slice selectors everywhere (`s => s.items`), badge from `s.items.length` + vitest pitfall test; (2) CleanupQueue popover documented opaque `var(--background)` + Esc/click-outside; (3) hover state in refs + overlay canvas (CanvasViz/VizView useRef, no re-render); (4) StrictMode attach-once guard in scan.ts + idempotent monitor_start/stop on mount/unmount; (5) generation-tagged IPC (scan.ts/exploreIpc/layoutIpc) with stale-drop; (6) no object-creating selectors. |
| 15 | PostHog events land, opt-out works, CSP clean | PENDING live keys | Wiring complete per doc 07 (hybrid, EVENTS dictionary, opt-out both layers, CSP `connect-src … https://us.i.posthog.com` in tauri.conf.json — verified present). Keys are env-configured (`VITE_POSTHOG_KEY` / `DISKBYTES_POSTHOG_KEY`); absent keys = disabled, never errors (decision-logged). Live dashboard verification needs a real project key. |
| 16 | Dodo TEST-mode flows + grace unit tests | PARTIAL — unit PASS, live PENDING | 12 license.rs tests + 3 commands/license.rs tests run in CI's `cargo test --workspace`: status mapping (201/404/403/422/500 → typed errors with spec'd copy), posture grace day-14 boundary + degrade, newer-of-validated/known-good (clock rollback), state machine, hardware-id shape, commit-gate tiers. Live TEST-mode activate/422 seat flow needs a dashboard key pair. |
| 17 | Platform seam: only platform/win.rs-adjacent files list Win32 | PASS | Fixed this session (commit 4d50a3a): `hardlink_identity` FFI moved from commands/dupes.rs into platform/win.rs; seam grep now lists zero non-platform files. |
| 18 | Performance: 1.2M files ≤12 s standard / ≤4 s turbo; 60 fps hover; startup <2 s | PARTIAL — core PASS, drive PENDING | 1M-node bench runs in CI (<2 s gate; 62 ms at M2). Real 1.2M-file drive timing + fps + startup need the Windows session. |

## Security gates (doc 09 §2, license/IPC-touching milestone)

- `git grep -nEi "dodo_sk|whsec_|sk_live|Bearer [A-Za-z0-9]{20,}" -- . ':!*.lock'` → **empty** (run this session).
- `strip = true` in the release profile (Cargo.toml; symbol-leak prevention per RE finding).

## Windows-GUI sign-off checklist (the final session)

1. `npm run tauri dev` — window opens, no console errors (§18-1).
2. Scan the user profile: sizes/counts/Quick Wins/File Types; This PC across 2 drives (§18-3/4).
3. `DISKBYTES_VERIFY=1` turbo-vs-standard ≤1% + non-elevated toggle explanation (§18-5/6).
4. Walk all 9 visualizations + hover/click/dblclick/depth/color/abbreviation (§18-7).
5. Stage from all five sources; Clear/✕ instant badge update (§18-8).
6. Recycle commit end-to-end incl. a non-recyclable refusal (§18-9).
7. OneDrive folder scan — placeholders skipped everywhere (§18-10).
8. Monitor 2 s cadence; snapshot take/list/diff/delete (§18-11).
9. Theme toggle + relaunch persistence, no flash (§18-12).
10. Install the CI NSIS artifact per-user; verify SHA256SUMS (§18-13).
11. Live Dodo TEST-mode + PostHog event landing with real keys (§3-15/16).
12. Real-drive performance timings: 1.2M files, hover fps, startup (§3-18).

## Honest state summary

All milestone code M0–M11 is complete and committed. The ENTIRE automatable
§18 battery is now GREEN across the two home accounts:

1. **Gates** — run 35638411781 on `1223082` (chainproof-dev, before quota
   exhaustion): fmt, clippy (pedantic profile), `cargo test --workspace`
   (42 app + 124 core, 0 failed — first Windows execution of the M10
   license/analytics + M8 applications suites), 1M-node bench 122 ms,
   typecheck, vitest 25, R7.1/R2.2 greps, frontend build.
2. **NSIS bundle** — run 35642632772 on `981c077` (tensorkernel, the
   migrated home after chainproof-dev's billing wall): per-user installer +
   portable zip + SHA256SUMS, artifact downloaded and hash-verified
   byte-exact. The account migration mirrored all 7 DiskBytes repos with
   full git history + release assets; `tensorkernel` is the push/CI home
   for all further work.

The one remaining item is the **Windows-GUI sign-off session** (the
interactive checklist above: real scans, 9 visualizations, recycle
round-trip, turbo verify, live Dodo TEST-mode + PostHog with real keys,
drive timings) — plus the optional `tauri dev` window check (§18-1).
