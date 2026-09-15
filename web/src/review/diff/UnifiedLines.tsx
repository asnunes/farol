import { cn } from "@/lib/utils";
import { Code } from "./Code";
import { LineNotes } from "./LineNotes";
import { LineNumber } from "./LineNumber";
import { AtLine } from "./comment/AtLine";
import { commentedBy, marker, notedBy, notesAt, reachOf, sidesOf } from "./line";
import type { Commentary } from "./line";
import type { Range } from "./intraline";
import type { Token } from "@/highlight/tokens";
import type { DiffLine, FileView, Hunk, Side } from "@/api";

/** One line under another, the way a diff is written down: removals first,
 * then the additions that replaced them. */
export function UnifiedLines({ hunk, file, coloured, marks, commentary }: UnifiedLinesProps) {
  return hunk.lines.map((line, i) => {
    // The row stands for both sides at once, under whichever numbers the line
    // has. The gutter shows one of them, and which one is what `sideOf` says.
    const at = sidesOf(line);
    const side = sideOf(line);

    return (
      <div key={i}>
        <div
          className={cn(
            // The row is what the `+` comes up on, so the row is what names
            // the group: the pointer reading this line is over the code,
            // not over four characters of gutter.
            "row diff-row group/line",
            line.kind === "added" && "add bg-add-bg text-add-ink",
            line.kind === "removed" && "del bg-del-bg text-del-ink",
            notedBy(file, line) && "noted",
            commentedBy(commentary.comments, at) && "commented",
            commentary.select.covers(at) && "picking bg-comment-dim",
          )}
        >
          <LineNumber
            line={line}
            side={side}
            reach={reachOf(hunk, side)}
            commentary={commentary}
          />
          {/* Wraps instead of scrolling sideways: a narrow window would otherwise
              cut the line off, and reading code by dragging a horizontal bar is
              worse than reading it on two lines. */}
          <div className="code break-words whitespace-pre-wrap">
            {marker(line)} <Code tokens={coloured?.[i]} plain={line.content} marks={marks[i]} />
          </div>
        </div>

        <LineNotes notes={notesAt(file, line)} file={file} />
        <AtLine at={at} commentary={commentary} />
      </div>
    );
  });
}

/** The side a unified row counts on, which is the numbering it prints.
 *
 * A removed line is only on the old side, so that is the one it offers. Added
 * lines and context lines offer the new side — context is on both, and the new
 * one is the number in the gutter and what a comment there has always meant. */
function sideOf(line: DiffLine): Side {
  return line.kind === "removed" ? "old" : "new";
}

type UnifiedLinesProps = {
  hunk: Hunk;
  file: FileView;
  coloured: Token[][] | null;
  marks: (Range[] | undefined)[];
  commentary: Commentary;
};
