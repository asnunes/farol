import { cn } from "@/lib/utils";
import { Code } from "./Code";
import { LineNotes } from "./LineNotes";
import { marker, notedBy, notesAt } from "./line";
import type { Token } from "@/highlight/tokens";
import type { FileView, Hunk } from "@/api";

/** One line under another, the way a diff is written down: removals first,
 * then the additions that replaced them. */
export function UnifiedLines({ hunk, file, coloured }: UnifiedLinesProps) {
  return hunk.lines.map((line, i) => (
    <div key={i}>
      <div
        className={cn(
          "row diff-row",
          line.kind === "added" && "add bg-add-bg text-add-ink",
          line.kind === "removed" && "del bg-del-bg text-del-ink",
          notedBy(file, line) && "noted",
        )}
      >
        <div className="ln shrink-0 pr-3 text-right text-faint select-none">
          {line.new_number ?? line.old_number ?? ""}
        </div>
        {/* Wraps instead of scrolling sideways: a narrow window would otherwise
            cut the line off, and reading code by dragging a horizontal bar is
            worse than reading it on two lines. */}
        <div className="code break-words whitespace-pre-wrap">
          {marker(line)} <Code tokens={coloured?.[i]} plain={line.content} />
        </div>
      </div>
      <LineNotes notes={notesAt(file, line)} file={file} />
    </div>
  ));
}

type UnifiedLinesProps = {
  hunk: Hunk;
  file: FileView;
  coloured: Token[][] | null;
};
