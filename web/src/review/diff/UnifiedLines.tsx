import { cn } from "@/lib/utils";
import { Code } from "./Code";
import { LineNotes } from "./LineNotes";
import { LineNumber } from "./LineNumber";
import { AtLine } from "./comment/AtLine";
import { commentedBy, marker, notedBy, notesAt } from "./line";
import type { Commentary } from "./line";
import type { Range } from "./intraline";
import type { Token } from "@/highlight/tokens";
import type { FileView, Hunk } from "@/api";

/** One line under another, the way a diff is written down: removals first,
 * then the additions that replaced them. */
export function UnifiedLines({ hunk, file, coloured, marks, commentary }: UnifiedLinesProps) {
  return hunk.lines.map((line, i) => (
    <div key={i}>
      <div
        className={cn(
          "row diff-row",
          line.kind === "added" && "add bg-add-bg text-add-ink",
          line.kind === "removed" && "del bg-del-bg text-del-ink",
          notedBy(file, line) && "noted",
          commentedBy(commentary.comments, line) && "commented",
          commentary.select.covers(line.newNumber ?? -1) && "picking bg-comment-dim",
        )}
      >
        <LineNumber
          number={line.newNumber ?? line.oldNumber}
          on={line.newNumber}
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
      <AtLine line={line} commentary={commentary} />
    </div>
  ));
}

type UnifiedLinesProps = {
  hunk: Hunk;
  file: FileView;
  coloured: Token[][] | null;
  marks: (Range[] | undefined)[];
  commentary: Commentary;
};
