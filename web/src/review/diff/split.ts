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
  const rows: SplitRow[] = [];
  let i = 0;

  while (i < lines.length) {
    if (lines[i].kind === "context") {
      rows.push({ left: i, right: i });
      i++;
      continue;
    }

    const removed: number[] = [];
    const added: number[] = [];
    while (i < lines.length && lines[i].kind !== "context") {
      (lines[i].kind === "removed" ? removed : added).push(i);
      i++;
    }

    for (let k = 0; k < Math.max(removed.length, added.length); k++) {
      rows.push({ left: removed[k] ?? null, right: added[k] ?? null });
    }
  }

  return rows;
}

/** One row of the two column layout: which line sits on each side, if any. */
export type SplitRow = { left: number | null; right: number | null };
