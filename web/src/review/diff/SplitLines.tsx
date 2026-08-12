import { useMemo } from "react";
import { cn } from "@/lib/utils";
import { Code } from "./Code";
import { LineNotes } from "./LineNotes";
import { marker, notedBy, notesAt } from "./line";
import { splitRows } from "./split";
import type { Range } from "./intraline";
import type { Token } from "@/highlight/tokens";
import type { FileView, Hunk } from "@/api";

/** The old side and the new one, facing each other. */
export function SplitLines({ hunk, file, coloured, marks }: SplitLinesProps) {
  const rows = useMemo(() => splitRows(hunk.lines), [hunk]);

  return rows.map((row, i) => {
    // A note belongs to the new side, and falls back to the old one for a row
    // that only removes.
    const anchor = row.right ?? row.left;
    const line = anchor === null ? null : hunk.lines[anchor];

    return (
      <div key={i}>
        <div className={cn("row split-row", line && notedBy(file, line) && "noted")}>
          <Side index={row.left} hunk={hunk} coloured={coloured} marks={marks} side="old" />
          <Side index={row.right} hunk={hunk} coloured={coloured} marks={marks} side="new" />
        </div>
        {line && <LineNotes notes={notesAt(file, line)} file={file} />}
      </div>
    );
  });
}

/** One side of a row: its number and its code, or an empty pair where the other
 * side has a line and this one does not.
 *
 * The two cells are separate grid children so the columns line up across every
 * row of the hunk, however the lines wrap. */
function Side({ index, hunk, coloured, marks, side }: SideProps) {
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
      <div className={cn("ln shrink-0 pr-3 text-right text-faint select-none", tint)}>
        {side === "old" ? line.old_number : line.new_number}
      </div>
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
};

type SideProps = {
  index: number | null;
  hunk: Hunk;
  coloured: Token[][] | null;
  marks: (Range[] | undefined)[];
  side: "old" | "new";
};
