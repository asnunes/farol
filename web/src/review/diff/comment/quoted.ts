import type { CommentView } from "@/api";

/** Where it was and what it said, in one paste — enough to be answered
 * somewhere the file is not open.
 *
 * The same shape `Comment::quoted` writes on the terminal side, so a comment
 * reads the same however it was copied. The two are separate implementations of
 * one format, and they have to stay in step. */
export function quoted(comment: CommentView): string {
  return `${comment.path}:${span(comment)}\n\n${comment.body.trim()}`;
}

/** How a span of lines is written down, in the one place both the label on
 * screen and the clipboard read it from. */
export function span(comment: { from: number; to: number }, dash = "-"): string {
  return comment.from === comment.to ? `${comment.from}` : `${comment.from}${dash}${comment.to}`;
}
