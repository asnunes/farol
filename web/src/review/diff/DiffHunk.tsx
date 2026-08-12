import { useMemo } from "react";
import { colourHunk } from "@/highlight/tokens";
import { changedRanges, type Range } from "./intraline";
import { segments } from "./split";
import { SplitLines } from "./SplitLines";
import { UnifiedLines } from "./UnifiedLines";
import type { Tokenize } from "@/highlight/tokens";
import type { DiffView } from "@/hooks/useDiffView";
import type { FileView, Hunk } from "@/api";

/** One run of changed lines, in whichever layout the reader chose. */
export function DiffHunk({ hunk, file, tokenize, view }: DiffHunkProps) {
  // Both sides of the hunk go through the tokenizer once, not once per render:
  // navigation redraws this on every keystroke.
  const coloured = useMemo(
    () => (tokenize ? colourHunk(hunk, tokenize) : null),
    [hunk, tokenize],
  );

  // What changed inside each line, for every pair the layout puts face to
  // face. Whether a pair is the same line edited or two lines that merely
  // ended up at the same position is decided one pair at a time, by how much
  // of the line survives.
  const marks = useMemo(() => {
    const found: (Range[] | undefined)[] = [];

    for (const segment of segments(hunk.lines)) {
      if ("context" in segment) continue;

      for (let k = 0; k < Math.min(segment.removed.length, segment.added.length); k++) {
        const [before, after] = [segment.removed[k], segment.added[k]];
        const changed = changedRanges(hunk.lines[before].content, hunk.lines[after].content);
        if (!changed) continue;

        found[before] = changed.before;
        found[after] = changed.after;
      }
    }

    return found;
  }, [hunk]);

  const Lines = view === "split" ? SplitLines : UnifiedLines;

  return (
    <div>
      <div className="hunk bg-sunken px-6 py-1 text-xs text-faint">
        @@ -{hunk.old_start},{hunk.old_lines} +{hunk.new_start},{hunk.new_lines} @@
      </div>
      <Lines hunk={hunk} file={file} coloured={coloured} marks={marks} />
    </div>
  );
}

type DiffHunkProps = {
  hunk: Hunk;
  file: FileView;
  tokenize: Tokenize | null;
  view: DiffView;
};
