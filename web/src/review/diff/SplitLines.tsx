import { useMemo } from "react";
import { cn } from "@/lib/utils";
import { Code } from "./Code";
import { LineNotes } from "./LineNotes";
import { LineNumber } from "./LineNumber";
import { Thread } from "./Thread";
import { commentedBy, marker, notedBy, notesAt } from "./line";
import { splitRows } from "./split";
import type { Commentary } from "./line";
import type { Range } from "./intraline";
import type { Token } from "@/highlight/tokens";
import type { FileView, Hunk } from "@/api";

/** The old side and the new one, facing each other. */
export function SplitLines({ hunk, file, coloured, marks, commentary }: SplitLinesProps) {
  const rows = useMemo(() => splitRows(hunk.lines), [hunk]);

  return rows.map((row, i) => {
    // A note belongs to the new side, and falls back to the old one for a row
    // that only removes.
    const anchor = row.right ?? row.left;
    const line = anchor === null ? null : hunk.lines[anchor];

    return (
      <div key={i}>
        <div
          className={cn(
            "row split-row",
            line && notedBy(file, line) && "noted",
            line && commentedBy(commentary.comments, line) && "commented",
            line && commentary.select.covers(line.new_number ?? -1) && "picking bg-comment-dim",
          )}
        >
          <Side index={row.left} hunk={hunk} coloured={coloured} marks={marks} side="old" />
          <Side
            index={row.right}
            hunk={hunk}
            coloured={coloured}
            marks={marks}
            side="new"
            commentary={commentary}
          />
        </div>

        {line && (
          <>
            <LineNotes notes={notesAt(file, line)} file={file} />
            <Thread line={line} commentary={commentary} />
          </>
        )}
      </div>
    );
  });
}

/** One side of a row: its number and its code, or an empty pair where the other
 * side has a line and this one does not.
 *
 * The two cells are separate grid children so the columns line up across every
 * row of the hunk, however the lines wrap. Only the new side takes a comment:
 * a comment is about the code as it now reads, and the left column is the code
 * that is gone. */
function Side({ index, hunk, coloured, marks, side, commentary }: SideProps) {
  if (index === null) {
    return (
      <>
        <div className="ln absent bg-sunken" />
        <div className="code absent bg-sunken" />
      </>
    );
  }

  const line = hunk.lines[index];
  const tint =
    line.kind === "added"
      ? "add bg-add-bg text-add-ink"
      : line.kind === "removed"
        ? "del bg-del-bg text-del-ink"
        : "";

  return (
    <>
      <LineNumber
        number={side === "old" ? line.old_number : line.new_number}
        on={side === "new" ? line.new_number : null}
        tint={tint}
        commentary={commentary}
      />
      <div className={cn("code break-words whitespace-pre-wrap", tint)}>
        {marker(line)} <Code tokens={coloured?.[index]} plain={line.content} marks={marks[index]} />
      </div>
    </>
  );
}

type SplitLinesProps = {
  hunk: Hunk;
  file: FileView;
  coloured: Token[][] | null;
  marks: (Range[] | undefined)[];
  commentary: Commentary;
};

type SideProps = {
  index: number | null;
  hunk: Hunk;
  coloured: Token[][] | null;
  marks: (Range[] | undefined)[];
  side: "old" | "new";
  commentary?: Commentary;
};
