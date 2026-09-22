/**
 * List mode (spec §7.9): virtualized expandable outline — icon, name,
 * mini share-of-parent bar, %, size; disclosure triangles only for
 * non-empty folders; 500 children per level cap.
 */
import { useEffect, useMemo, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ChevronRightIcon, FolderIcon, LockKeyholeIcon, CloudIcon } from "../../components/Icon";
import { categoryIcon } from "../../components/Icon";
import { getListChildren, type ListRowData } from "../../viz/exploreIpc";
import { bytes } from "../../lib/format";
import { Spinner } from "../../components/buttons";

interface FlatRow extends ListRowData {
  level: number;
  expanded: boolean;
  hasKids: boolean;
}

export interface ListModeProps {
  generation: number;
  folder: number;
  filter: string;
  selectedId: number | null;
  onSelect: (id: number | null) => void;
  onOpen: (id: number) => void;
  onContextMenu: (id: number, x: number, y: number) => void;
  onHover: (id: number | null, x: number, y: number) => void;
}

export function ListMode(props: ListModeProps) {
  const [tree, setTree] = useState<FlatRow[] | null>(null);
  const expanded = useRef(new Set<number>());
  const scrollRef = useRef<HTMLDivElement>(null);

  const rebuild = async () => {
    try {
      const children = await getListChildren(props.generation, props.folder, props.filter);
      let out: FlatRow[] = [];
      const walk = async (parent: number, level: number) => {
        const kids = parent === props.folder ? children : await getListChildren(props.generation, parent, props.filter).catch(() => []);
        for (const k of kids.slice(0, 500)) {
          const hasKids = k.isDir && k.hasChildren;
          const isExpanded = expanded.current.has(k.id);
          out.push({ ...k, level, expanded: isExpanded, hasKids });
          if (k.isDir && isExpanded) await walk(k.id, level + 1);
        }
      };
      await walk(props.folder, 0);
      setTree(out);
    } catch {
      setTree([]);
    }
  };

  const key = `${props.generation}:${props.folder}:${props.filter}`;
  useEffect(() => {
    expanded.current.clear();
    void rebuild();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);

  const virtualizer = useVirtualizer({
    count: tree?.length ?? 0,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 38,
    overscan: 12,
  });

  const totalItems = tree?.length ?? 0;
  const abbreviation = useMemo(() => false, []);

  if (!tree) {
    return (
      <div className="db-loading-block">
        <Spinner />
        <span>Building outline…</span>
      </div>
    );
  }

  return (
    <div className="db-list" style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="list-head">
        <span>Name</span>
        <span />
        <span />
        <span>Share</span>
        <span>%</span>
        <span>Items</span>
        <span>Size</span>
      </div>
      <div ref={scrollRef} className="db-scroll" style={{ flex: 1, overflow: "auto" }}>
        <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
          {virtualizer.getVirtualItems().map((vi) => {
            const row = tree[vi.index];
            const Icon = row.isDir ? FolderIcon : categoryIcon(row.category);
            return (
              <button
                key={`${row.id}:${vi.index}`}
                type="button"
                className={`db-list-row ${props.selectedId === row.id ? "is-selected" : ""}`}
                style={{
                  position: "absolute",
                  top: 0,
                  left: 0,
                  width: "100%",
                  transform: `translateY(${vi.start}px)`,
                  paddingLeft: 8 + row.level * 16,
                }}
                onClick={() => props.onSelect(row.id)}
                onDoubleClick={() => {
                  if (row.isDir) props.onOpen(row.id);
                }}
                onContextMenu={(e) => {
                  e.preventDefault();
                  props.onContextMenu(row.id, e.clientX, e.clientY);
                }}
                onPointerEnter={(e) => props.onHover(row.id, e.clientX, e.clientY)}
                onPointerLeave={() => props.onHover(null, 0, 0)}
              >
                {row.hasKids ? (
                  <span
                    className="db-disclose"
                    data-open={row.expanded}
                    role="button"
                    tabIndex={-1}
                    onClick={(e) => {
                      e.stopPropagation();
                      if (row.expanded) expanded.current.delete(row.id);
                      else expanded.current.add(row.id);
                      void rebuild();
                    }}
                  >
                    <ChevronRightIcon size={13} />
                  </span>
                ) : (
                  <span />
                )}
                <Icon size={15} className="db-row-glyph" />
                <strong>
                  {row.name}
                  <span className="db-row-flags">
                    {row.protected && <LockKeyholeIcon size={10} />}
                    {row.cloud && <CloudIcon size={10} />}
                  </span>
                </strong>
                <i>
                  <b style={{ width: `${Math.max(2, Math.min(100, row.share * 100))}%`, background: `#${row.color.toString(16).padStart(6, "0")}` }} />
                </i>
                <em className="tnum">{(row.share * 100).toFixed(1)}%</em>
                <span className="tnum">{row.isDir ? "" : abbreviation ? "" : ""}</span>
                <b className="tnum">{bytes(row.size)}</b>
              </button>
            );
          })}
        </div>
      </div>
      {totalItems === 0 && (
        <div className="db-substate">{props.filter ? `Nothing matches “${props.filter}”.` : "This folder is empty."}</div>
      )}
    </div>
  );
}
