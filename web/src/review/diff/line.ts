import type { DiffLine, FileView, TaggedLineNote } from "@/api";

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
