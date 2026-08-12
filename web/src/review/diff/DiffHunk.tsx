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

  // What changed inside a line, and only where one line was replaced by one
  // line. In a longer run the third removal faces the third addition because
  // they sit at the same position and for no other reason, so the words that
  // differ are not the words that were edited, and the marks land on lines
  // nobody rewrote.
  const marks = useMemo(() => {
    const found: (Range[] | undefined)[] = [];

    for (const segment of segments(hunk.lines)) {
      if ("context" in segment) continue;
      if (segment.removed.length !== 1 || segment.added.length !== 1) continue;

      const [before, after] = [segment.removed[0], segment.added[0]];
      const changed = changedRanges(hunk.lines[before].content, hunk.lines[after].content);
      if (!changed) continue;

      found[before] = changed.before;
      found[after] = changed.after;
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
