/**
 * Explore header / mode toolbar (spec §7): icon-only mode capsule with
 * the active mode expanding its label (framer layoutId slide), caption,
 * color-mode segmented control (5 canvas modes), depth slider 2–10
 * (treemap/sunburst/flame), "A" abbreviate toggle; responsive degrade
 * via CSS breakpoints (caption → depth → wrap).
 */
import { motion } from "framer-motion";
import { BlocksIcon, CircleDotIcon, Clock3Icon, FlameIcon, FolderIcon, GaugeIcon, Grid2x2Icon, ListTreeIcon, NetworkIcon } from "../components/Icon";
import {
  COLORED_MODES, DEPTH_MODES, MODES, MODE_CAPTIONS, useVizUiStore, type Mode,
} from "../state/vizUi";
import { EVENTS, track } from "../lib/analytics";

const MODE_ICONS: Record<Mode, (p: { size?: number }) => JSX.Element> = {
  Folders: FolderIcon,
  Treemap: Grid2x2Icon,
  Sunburst: CircleDotIcon,
  Flame: FlameIcon,
  Bubbles: BlocksIcon,
  "Mind Map": NetworkIcon,
  "Top Sizes": GaugeIcon,
  "Age Map": Clock3Icon,
  List: ListTreeIcon,
};

export function ExploreHeader() {
  const mode = useVizUiStore((s) => s.mode);
  const setMode = useVizUiStore((s) => s.setMode);
  const colorMode = useVizUiStore((s) => s.colorMode);
  const setColorMode = useVizUiStore((s) => s.setColorMode);
  const depth = useVizUiStore((s) => s.depth);
  const setDepth = useVizUiStore((s) => s.setDepth);
  const abbrev = useVizUiStore((s) => s.abbreviate);
  const setAbbrev = useVizUiStore((s) => s.setAbbreviate);

  return (
    <div className="db-toolbar-row" role="toolbar" aria-label="Visualization controls">
      <div className="db-mode-picker" role="group" aria-label="Mode">
        {MODES.map((m) => {
          const Icon = MODE_ICONS[m];
          return (
            <button
              key={m}
              type="button"
              data-active={mode === m}
              onClick={() => {
                setMode(m);
                track(EVENTS.vizModeSelected, { mode: m });
              }}
              title={m}
              aria-label={m}
            >
              {mode === m && <motion.span layoutId="db-mode-pill" className="db-mode-pill" transition={{ type: "spring", stiffness: 500, damping: 40 }} />}
              <Icon size={15} />
              {mode === m && <span>{m}</span>}
            </button>
          );
        })}
      </div>

      {COLORED_MODES.has(mode) && (
        <div className="db-segmented" role="group" aria-label="Color mode">
          {(["by-folder", "by-type", "by-age"] as const).map((c) => (
            <button
              key={c}
              type="button"
              data-active={colorMode === c}
              onClick={() => {
                setColorMode(c);
                track(EVENTS.colorModeChanged, { mode: c });
              }}
            >
              {c === "by-folder" ? "By folder" : c === "by-type" ? "By type" : "By age"}
            </button>
          ))}
        </div>
      )}

      {DEPTH_MODES.has(mode) && (
        <label className="db-depth" aria-label={`Depth ${depth}`}>
          <BlocksIcon size={14} />
          <input
            type="range"
            min={2}
            max={10}
            value={depth}
            onChange={(e) => {
              setDepth(Number(e.target.value));
              track(EVENTS.depthSliderMoved, { depth: Number(e.target.value) });
            }}
          />
          <b className="tnum">{depth}</b>
        </label>
      )}

      <p className="db-toolbar-caption">{MODE_CAPTIONS[mode]}</p>

      {(COLORED_MODES.has(mode) || mode === "List" || mode === "Top Sizes") && (
        <button
          type="button"
          className="db-abbreviate"
          data-active={abbrev}
          onClick={() => setAbbrev(!abbrev)}
          title="Toggle abbreviated labels"
          aria-pressed={abbrev}
        >
          A
        </button>
      )}
    </div>
  );
}
