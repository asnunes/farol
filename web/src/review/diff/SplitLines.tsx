import { useMemo } from "react";
import { cn } from "@/lib/utils";
import { Code } from "./Code";
import { LineNotes } from "./LineNotes";
import { LineNumber } from "./LineNumber";
import { AtLine } from "./comment/AtLine";
import { commentedBy, marker, notedBy, notesAt } from "./line";
import { splitRows } from "./split";
import type { Anchor, Commentary } from "./line";
import type { Range } from "./intraline";
import type { Token } from "@/highlight/tokens";
import type { FileView, Hunk, Side as DiffSide } from "@/api";

/** The old side and the new one, facing each other. */
export function SplitLines({ hunk, file, coloured, marks, commentary }: SplitLinesProps) {
  const rows = useMemo(() => splitRows(hunk.lines), [hunk]);

  return rows.map((row, i) => {
    // A note belongs to the new side, and falls back to the old one for a row
    // that only removes.
    const anchor = row.right ?? row.left;
    const line = anchor === null ? null : hunk.lines[anchor];

    // A comment belongs to the column it was written in, so each side of the
    // row is offered as itself rather than one standing in for the other.
    const at: Anchor = {
      old: row.left === null ? null : hunk.lines[row.left],
      new: row.right === null ? null : hunk.lines[row.right],
    };

    return (
      <div key={i}>
        <div
          className={cn(
            "row split-row",
            line && notedBy(file, line) && "noted",
            commentedBy(commentary.comments, at) && "commented",
            commentary.select.covers(at) && "picking bg-comment-dim",
          )}
        >
          <Side
            index={row.left}
            hunk={hunk}
            coloured={coloured}
            marks={marks}
            side="old"
            commentary={commentary}
          />
          <Side
            index={row.right}
            hunk={hunk}
            coloured={coloured}
            marks={marks}
            side="new"
            commentary={commentary}
          />
        </div>

        {line && <LineNotes notes={notesAt(file, line)} file={file} />}
        <AtLine at={at} commentary={commentary} />
      </div>
    );
  });
}

/** One side of a row: its number and its code, or an empty pair where the other
 * side has a line and this one does not.
 *
 * The two cells are separate grid children so the columns line up across every
 * row of the hunk, however the lines wrap. Both sides take a comment, each on
 * its own numbering: the left column is the code that is going, which is as
 * much a part of the change as what replaced it. */
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
      <LineNumber line={line} side={side} tint={tint} commentary={commentary} />
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
  side: DiffSide;
  commentary: Commentary;
};
