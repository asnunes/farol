import type { CommentActions } from "@/hooks/useComments";
import type { LineSelection } from "./useLineSelection";
import type { CommentView, DiffLine, FileView, Hunk, Side, TaggedLineNote } from "@/api";

/** The comment layer over one file's diff: what is already written, what can be
 * done to it, and which lines the reader is choosing right now.
 *
 * Carried as one value down to the rows because all three are needed in the
 * same place — the gutter that starts a selection is a cell away from the prose
 * that answers it. */
export type Commentary = {
  path: string;
  comments: CommentView[];
  actions: CommentActions;
  select: LineSelection;
};

/** What a line is marked with in the gutter of a unified diff. */
export function marker(line: DiffLine) {
  return line.kind === "added" ? "+" : line.kind === "removed" ? "−" : " ";
}

/** The line a row carries on each side, and where a comment written on that
 * side hangs.
 *
 * Split view fills the two halves from the two columns. Unified view puts the
 * same line in both, because one row there stands for whichever sides that line
 * has a number on: a removed line is the old side only, an added line the new
 * side only, and a context line is both at once under two different numbers. */
export type Anchor = { old: DiffLine | null; new: DiffLine | null };

/** A unified row, as the two sides it stands for. */
export function sidesOf(line: DiffLine): Anchor {
  return { old: line, new: line };
}

/** What a line is numbered on one side, or nothing when it is not on that side
 * at all. The single place the two numbers on a line are told apart. */
export function numberOn(line: DiffLine | null, side: Side): number | null {
  if (line === null) return null;
  return side === "old" ? line.oldNumber : line.newNumber;
}

/** How far a comment picked from the keyboard may grow on one side: the first
 * and last number that side carries across a hunk.
 *
 * The bound is the hunk rather than the file because the box hangs off a row,
 * and a hunk is exactly what has a row for every number in it. Reached past,
 * the span would cover lines the diff never printed and the box would have
 * nowhere to open. A drag is bounded by the same thing without being told: the
 * pointer can only be over a row that exists. */
export function reachOf(hunk: Hunk, side: Side): Reach {
  return side === "old"
    ? { from: hunk.oldStart, to: hunk.oldStart + hunk.oldLines - 1 }
    : { from: hunk.newStart, to: hunk.newStart + hunk.newLines - 1 };
}

export type Reach = { from: number; to: number };

/** Whether a note is about this line.
 *
 * Marking the whole span is what makes it visible before the note explains it:
 * the range printed on the note says which lines it covers, but nobody counts
 * line numbers to find them. */
export function notedBy(file: FileView, line: DiffLine): boolean {
  return file.lineNotes.some(
    (n) => line.newNumber !== null && n.from <= line.newNumber && line.newNumber <= n.to,
  );
}

/** The notes that belong under this line.
 *
 * A note is anchored to the last line of its span, which is where the reader
 * has finished reading the thing it is about. */
export function notesAt(file: FileView, line: DiffLine): TaggedLineNote[] {
  return file.lineNotes.filter((n) => line.newNumber !== null && n.to === line.newNumber);
}

/** Whether a comment covers this row. The reviewer's writing gets the same
 * treatment as the session's: the span is marked, and the prose sits under it.
 *
 * Each comment is asked about the side it was written on, so line 12 as it was
 * and line 12 as it now reads mark the rows they are actually about. */
export function commentedBy(comments: CommentView[], at: Anchor): boolean {
  return comments.some((c) => {
    const line = numberOn(at[c.side], c.side);
    return line !== null && c.from <= line && line <= c.to;
  });
}

/** The comments that belong under this row.
 *
 * They arrive oldest first and stay that way: the order is the server's to
 * decide, like the grouping that put them on this file in the first place. */
export function commentsAt(comments: CommentView[], at: Anchor): CommentView[] {
  return comments.filter((c) => numberOn(at[c.side], c.side) === c.to);
}

