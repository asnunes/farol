import { useMemo } from "react";
import { cn } from "@/lib/utils";
import { useNarrow } from "@/hooks/useNarrow";
import { Code } from "./Code";
import { LineNotes } from "./LineNotes";
import { LineNumber } from "./LineNumber";
import { AtLine } from "./comment/AtLine";
import { commentedBy, marker, notedBy, notesAt } from "./line";
import { splitRows } from "./split";
import type { SplitRow } from "./split";
import type { Anchor, Commentary } from "./line";
import type { Range } from "./intraline";
import type { Token } from "@/highlight/tokens";
import type { DiffLine, FileView, Hunk, Side as DiffSide } from "@/api";

/** The old side and the new one, facing each other — or stacked, on a screen
 * with room for one column of code rather than two. */
export function SplitLines({ hunk, file, coloured, marks, commentary }: SplitLinesProps) {
  const rows = useMemo(() => splitRows(hunk.lines), [hunk]);
  const stacked = useNarrow();

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
            "row",
            stacked ? "stack-row" : "split-row",
            line && notedBy(file, line) && "noted",
            commentedBy(commentary.comments, at) && "commented",
            commentary.select.covers(at) && "picking bg-comment-dim",
          )}
        >
          {stacked ? (
            <Stacked
              row={row}
              hunk={hunk}
              coloured={coloured}
              marks={marks}
              commentary={commentary}
            />
          ) : (
            <>
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
            </>
          )}
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
 * much a part of the change as what replaced it.
 *
 * Which is also why the `+` answers to this half and not to the row. The two
 * columns are two different lines, and raising the right-hand control because
 * the pointer is over the left-hand code would offer a comment on code the
 * reader is not looking at. The wrapper is `contents` so it names the half
 * without becoming a box: the four cells stay grid children of the row and the
 * columns still line up. */
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
  const tint = tintOf(line);

  return (
    <div className="group/line contents">
      <LineNumber line={line} side={side} tint={tint} commentary={commentary} />
      <div className={cn("code", tint)}>
        {marker(line)} <Code tokens={coloured?.[index]} plain={line.content} marks={marks[index]} />
      </div>
    </div>
  );
}

/** The same row, where there is width for one column of code rather than two.
 *
 * **A line the change did not touch is printed once.** It is the same line on
 * both sides, and stacked it would arrive twice — the whole unchanged half of a
 * file read through a second time on the screen with least room for it. It
 * keeps both its numbers, one in each gutter, so the old and the new numbering
 * are both there and a comment still has two places to go.
 *
 * A line the change did touch keeps its side. Its number sits in its own
 * gutter and the other gutter is left empty, which is what says which side it
 * is once the two are above each other instead of beside each other — the
 * colour says it too, and the empty gutter says it where colour cannot. */
function Stacked({ row, hunk, coloured, marks, commentary }: StackedProps) {
  // `splitRows` gives a context line the same index on both sides, which is the
  // question being asked here: is this one line or two?
  if (row.left !== null && row.left === row.right) {
    const line = hunk.lines[row.left];

    return (
      <div className="group/line contents">
        <LineNumber line={line} side="old" commentary={commentary} />
        <LineNumber line={line} side="new" commentary={commentary} />
        <div className="code">
          {marker(line)}{" "}
          <Code tokens={coloured?.[row.left]} plain={line.content} marks={marks[row.left]} />
        </div>
      </div>
    );
  }

  return (
    <>
      {row.left !== null && (
        <Stack
          index={row.left}
          side="old"
          hunk={hunk}
          coloured={coloured}
          marks={marks}
          commentary={commentary}
        />
      )}
      {row.right !== null && (
        <Stack
          index={row.right}
          side="new"
          hunk={hunk}
          coloured={coloured}
          marks={marks}
          commentary={commentary}
        />
      )}
    </>
  );
}

/** One changed line on a line of its own: its number under its own side, the
 * other gutter empty, and the code. Three cells, which is one row of the grid. */
function Stack({ index, side, hunk, coloured, marks, commentary }: StackProps) {
  const line = hunk.lines[index];
  const tint = tintOf(line);
  const number = <LineNumber line={line} side={side} tint={tint} commentary={commentary} />;
  const empty = <div className={cn("ln blank", tint)} aria-hidden="true" />;

  return (
    <div className="group/line contents">
      {side === "old" ? number : empty}
      {side === "old" ? empty : number}
      <div className={cn("code", tint)}>
        {marker(line)} <Code tokens={coloured?.[index]} plain={line.content} marks={marks[index]} />
      </div>
    </div>
  );
}

/** What a line is washed in, which both layouts ask for the same way. */
function tintOf(line: DiffLine): string {
  return line.kind === "added"
    ? "add bg-add-bg text-add-ink"
    : line.kind === "removed"
      ? "del bg-del-bg text-del-ink"
      : "";
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

type StackedProps = {
  row: SplitRow;
  hunk: Hunk;
  coloured: Token[][] | null;
  marks: (Range[] | undefined)[];
  commentary: Commentary;
};

type StackProps = {
  index: number;
  side: DiffSide;
  hunk: Hunk;
  coloured: Token[][] | null;
  marks: (Range[] | undefined)[];
  commentary: Commentary;
};
