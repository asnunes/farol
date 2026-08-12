import type { DiffLine } from "@/api";

/** Lay a hunk out as two columns, the old side and the new one.
 *
 * A unified hunk lists removals and then additions; side by side they have to
 * face each other. So a run of changed lines is paired off by position, and
 * whichever side runs out first gets blanks for the rest of the run. Pairing by
 * position rather than by content is what git itself does, and it keeps a
 * rewritten line next to the line it replaced.
 *
 * The rows carry indices into the hunk rather than the lines themselves,
 * because the tokens for the code are indexed the same way. */
export function splitRows(lines: DiffLine[]): SplitRow[] {
  return segments(lines).flatMap((segment) => {
    if ("context" in segment) return [{ left: segment.context, right: segment.context }];

    const { removed, added } = segment;
    return Array.from({ length: Math.max(removed.length, added.length) }, (_, k) => ({
      left: removed[k] ?? null,
      right: added[k] ?? null,
    }));
  });
}

/** A hunk as the reader meets it: single context lines, and runs of change
 * between them.
 *
 * The runs are what both the layout and the word marks are built on, and one
 * walk means the two cannot come to disagree about where a change begins. */
export function segments(lines: DiffLine[]): Segment[] {
  const found: Segment[] = [];
  let i = 0;

  while (i < lines.length) {
    if (lines[i].kind === "context") {
      found.push({ context: i });
      i++;
      continue;
    }

    const removed: number[] = [];
    const added: number[] = [];
    while (i < lines.length && lines[i].kind !== "context") {
      (lines[i].kind === "removed" ? removed : added).push(i);
      i++;
    }
    found.push({ removed, added });
  }

  return found;
}

export type Segment = { context: number } | { removed: number[]; added: number[] };

/** One row of the two column layout: which line sits on each side, if any. */
export type SplitRow = { left: number | null; right: number | null };
