/**
 * CanvasMode host (spec §7): wraps CanvasViz for the 5 canvas modes,
 * bridging the vizUi store (mode/color/depth/abbrev) into it and
 * connecting the stage-level interactions.
 */
import { CanvasViz } from "../../viz/CanvasViz";
import { useVizUiStore } from "../../state/vizUi";
import type { Mode } from "../../state/vizUi";

const CANVAS_IDS: Record<string, "treemap" | "sunburst" | "flame" | "bubbles" | "mind-map"> = {
  Treemap: "treemap",
  Sunburst: "sunburst",
  Flame: "flame",
  Bubbles: "bubbles",
  "Mind Map": "mind-map",
};

export interface CanvasModeProps {
  generation: number;
  folder: number;
  mode: Mode;
  selectedId: number | null;
  onSelect: (id: number | null) => void;
  onOpen: (id: number) => void;
  onContextMenu: (id: number, x: number, y: number) => void;
  onHover: (id: number | null, x: number, y: number) => void;
}

export function CanvasMode(props: CanvasModeProps) {
  const colorMode = useVizUiStore((s) => s.colorMode);
  const depth = useVizUiStore((s) => s.depth);
  const abbrev = useVizUiStore((s) => s.abbreviate);
  const canvasId = CANVAS_IDS[props.mode];

  return (
    <CanvasViz
      generation={props.generation}
      node={props.folder}
      mode={canvasId}
      colorMode={colorMode}
      depth={depth}
      abbreviateLabels={abbrev}
      selectedId={props.selectedId}
      onSelect={props.onSelect}
      onOpen={props.onOpen}
      onContextMenu={props.onContextMenu}
      onHover={props.onHover}
    />
  );
}
