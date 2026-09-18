import { useMemo } from "react";
import { useNarrow } from "@/hooks/useNarrow";
import { useHighlight } from "@/hooks/useHighlight";
import { changedRanges, type Range } from "./intraline";
import { Gap } from "./Gap";
import { SplitLines } from "./SplitLines";
import { UnifiedLines } from "./UnifiedLines";
import { DiffWindow } from "./DiffWindow";
import { colourWindow, hunkText, hunkWindows } from "./hunkWindows";
import type { HunkText, HunkWindow } from "./hunkWindows";
import type { Commentary } from "./line";
import type { Gap as GapRange, Range as LineRange } from "./gaps";
import type { DiffView } from "@/hooks/useDiffView";
import type { FileView, Hunk } from "@/api";

/** Keep the hunk's numbering and selection intact while only painting nearby rows. */
export function DiffHunk({ hunk, file, view, commentary, gap }: DiffHunkProps) {
  const narrow = useNarrow();
  const windows = useMemo(() => hunkWindows(hunk, view), [hunk, view]);
  const text = useMemo(() => hunkText(hunk), [hunk]);
  const held = commentary.select.picking ?? commentary.select.composing;

  return (
    <div>
      {gap && (
        <Gap
          gap={gap.gap}
          onOpen={gap.onOpen}
        >{`@@ -${hunk.oldStart},${hunk.oldLines} +${hunk.newStart},${hunk.newLines} @@`}</Gap>
      )}
      {windows.map((window, i) => {
        const pinned =
          !!held &&
          window.indices.some((index) => {
            const line = hunk.lines[index];
            const at = held.side === "old" ? line.oldNumber : line.newNumber;
            return (
              at !== null &&
              Math.min(held.from, held.to) <= at &&
              at <= Math.max(held.from, held.to)
            );
          });
        return (
          <DiffWindow
            key={`${view}-${i}`}
            rows={
              narrow
                ? window.indices.length
                : (window.rows?.length ?? window.indices.length)
            }
            pinned={pinned}
            layout={`${view}-${narrow}`}
          >
            <WindowContents
              hunk={hunk}
              file={file}
              view={view}
              commentary={commentary}
              window={window}
              text={text}
            />
          </DiffWindow>
        );
      })}
    </div>
  );
}

function WindowContents({
  hunk,
  file,
  view,
  commentary,
  window,
  text,
}: WindowContentsProps) {
  const tokenize = useHighlight(file.path);
  const coloured = useMemo(
    () => (tokenize ? colourWindow(text, window.indices, tokenize) : null),
    [text, window, tokenize],
  );
  const marks = useMemo(() => {
    const found: (Range[] | undefined)[] = [];
    const handled = new Set<number>();
    for (const index of window.indices) {
      const pair = text.pairs.get(index);
      if (!pair || handled.has(pair[0])) continue;
      handled.add(pair[0]);
      const changed = changedRanges(
        hunk.lines[pair[0]].content,
        hunk.lines[pair[1]].content,
      );
      if (changed) {
        found[pair[0]] = changed.before;
        found[pair[1]] = changed.after;
      }
    }
    return found;
  }, [hunk, window, text]);
  return view === "split" ? (
    <SplitLines
      hunk={hunk}
      file={file}
      coloured={coloured}
      marks={marks}
      commentary={commentary}
      rows={window.rows}
    />
  ) : (
    <UnifiedLines
      hunk={hunk}
      file={file}
      coloured={coloured}
      marks={marks}
      commentary={commentary}
      indices={window.indices}
    />
  );
}

type DiffHunkProps = {
  hunk: Hunk;
  file: FileView;
  view: DiffView;
  commentary: Commentary;
  gap?: { gap: GapRange; onOpen: (range: LineRange) => Promise<void> };
};
type WindowContentsProps = Omit<DiffHunkProps, "gap"> & {
  window: HunkWindow;
  text: HunkText;
};
