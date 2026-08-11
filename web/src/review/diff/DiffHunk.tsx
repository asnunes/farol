import { useMemo } from "react";
import { colourHunk } from "@/highlight/tokens";
import { changedRanges, type Range } from "./intraline";
import { splitRows } from "./split";
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

  // What changed inside each line, for the rows where a removal and an
  // addition face each other. The pairing is the one the split layout uses, so
  // both layouts mark the same words.
  const marks = useMemo(() => {
    const found: (Range[] | undefined)[] = [];
    for (const row of splitRows(hunk.lines)) {
      if (row.left === null || row.right === null) continue;
      const changed = changedRanges(hunk.lines[row.left].content, hunk.lines[row.right].content);
      if (!changed) continue;
      found[row.left] = changed.before;
      found[row.right] = changed.after;
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
