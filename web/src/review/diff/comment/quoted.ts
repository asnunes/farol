import { linesOf, type CommentView, type Side } from "@/api";

/** Where it was and what it said, in one paste — enough to be answered
 * somewhere the file is not open.
 *
 * The same shape `Comment::quoted` writes on the terminal side, so a comment
 * reads the same however it was copied. The two are separate implementations of
 * one format, and they have to stay in step. */
export function quoted(comment: CommentView): string {
  return `${comment.at}\n\n${comment.body.trim()}`;
}

/** Where a thread sits, short enough for a row: `module.go:26`, or the lens or
 * block it is about. */
export function where(thread: CommentView): string {
  const lines = linesOf(thread);
  return lines ? `${lines.path.split("/").pop()}:${span(lines, "–")}` : thread.at;
}

/** How a span of lines is written down, in the one place both the label on
 * screen and the clipboard read it from.
 *
 * The side is written only when it is the old one: both sides number from one,
 * so the numbers alone would name two places, and the unmarked form has to keep
 * meaning what it has always meant — the file as it now reads. */
export function span(comment: { side: Side; from: number; to: number }, dash = "-"): string {
  const lines =
    comment.from === comment.to ? `${comment.from}` : `${comment.from}${dash}${comment.to}`;
  return comment.side === "old" ? `${lines} (old)` : lines;
}

/** The name of a box opened on lines: "Thread on line 12", "Thread on lines
 * 12–14". */
export function threadOn(lines: { side: Side; from: number; to: number }): string {
  return lines.from === lines.to ? `Thread on line ${span(lines, "–")}` : `Thread on lines ${span(lines, "–")}`;
}
