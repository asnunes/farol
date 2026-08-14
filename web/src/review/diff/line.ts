import type { CommentActions } from "@/hooks/useComments";
import type { LineSelection } from "./useLineSelection";
import type { CommentView, DiffLine, FileView, TaggedLineNote } from "@/api";

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

/** Whether a note is about this line.
 *
 * Marking the whole span is what makes it visible before the note explains it:
 * the range printed on the note says which lines it covers, but nobody counts
 * line numbers to find them. */
export function notedBy(file: FileView, line: DiffLine): boolean {
  return file.lineNotes.some(
    (n) => line.new_number !== null && n.from <= line.new_number && line.new_number <= n.to,
  );
}

/** The notes that belong under this line.
 *
 * A note is anchored to the last line of its span, which is where the reader
 * has finished reading the thing it is about. */
export function notesAt(file: FileView, line: DiffLine): TaggedLineNote[] {
  return file.lineNotes.filter((n) => line.new_number !== null && n.to === line.new_number);
}

/** Whether a comment covers this line. The reviewer's writing gets the same
 * treatment as the session's: the span is marked, and the prose sits under it. */
export function commentedBy(comments: CommentView[], line: DiffLine): boolean {
  return comments.some(
    (c) => line.new_number !== null && c.from <= line.new_number && line.new_number <= c.to,
  );
}

/** The comments that belong under this line, oldest first — questions read in
 * the order they were asked. Ids carry the clock, so sorting by id is sorting
 * by when. */
export function commentsAt(comments: CommentView[], line: DiffLine): CommentView[] {
  return comments
    .filter((c) => line.new_number !== null && c.to === line.new_number)
    .sort((a, b) => a.id.localeCompare(b.id));
}

/** The comments left on one file. */
export function commentsOn(comments: CommentView[], path: string): CommentView[] {
  return comments.filter((c) => c.path === path);
}
