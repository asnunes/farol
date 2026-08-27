import type { FileDiff, Hunk } from "@/api";

/** A stretch of the file between two hunks, or before the first, or after the
 * last: the lines the diff did not print and the reader can ask to see.
 *
 * Kept as new-side numbers, with the offset to the old side alongside. Inside a
 * gap the two sides run in step, so one number and one offset name both. */
export type Gap = {
  /** First and last line of what is still closed, on the new side. */
  from: number;
  to: number;
  /** Old number minus new number, constant across the gap. */
  shift: number;
  /** Whether there is a hunk on each side of it. The gap before the first hunk
   * has nothing above it and the one after the last has nothing below, which is
   * what decides how many ways in it can offer. */
  under: boolean;
  over: boolean;
};

/** Every gap in a file, in reading order, including the ones at the ends.
 *
 * A file with no hunks has no gaps: there is nothing to open around, and the
 * whole file would be one enormous one. */
export function gapsOf(diff: FileDiff): Gap[] {
  if (diff.hunks.length === 0) return [];

  const gaps: Gap[] = [];
  const first = diff.hunks[0];
  if (first.new_start > 1) {
    gaps.push({
      from: 1,
      to: first.new_start - 1,
      shift: first.old_start - first.new_start,
      under: false,
      over: true,
    });
  }

  for (let i = 0; i < diff.hunks.length - 1; i++) {
    const [above, below] = [diff.hunks[i], diff.hunks[i + 1]];
    const from = ends(above);
    const to = below.new_start - 1;
    if (from <= to) gaps.push({ from, to, shift: shiftAfter(above), under: true, over: true });
  }

  const last = diff.hunks[diff.hunks.length - 1];
  if (ends(last) <= diff.line_count) {
    gaps.push({
      from: ends(last),
      to: diff.line_count,
      shift: shiftAfter(last),
      under: true,
      over: false,
    });
  }

  return gaps;
}

/** What is left of a gap once these ranges have been opened.
 *
 * Opening from the top and from the bottom eats the gap from both ends, so what
 * remains is a gap of the same shape, only shorter. When nothing is left the
 * gap is gone and its controls go with it. */
export function stillClosed(gap: Gap, opened: Range[]): Gap | null {
  let { from, to } = gap;

  for (const range of opened) {
    if (range.from <= from && from <= range.to) from = range.to + 1;
    if (range.from <= to && to <= range.to) to = range.from - 1;
  }

  return from > to ? null : { ...gap, from, to };
}

/** The step a chevron takes, and what the reader gets when the gap is smaller
 * than a step: all of it, in one press. Twenty is what GitHub opens, and it is
 * about a screenful of code without being a page of it. */
export const STEP = 20;

export type Range = { from: number; to: number };

/** What the arrow pointing up asks for: the lines just above the hunk below. */
export function fromBelow(gap: Gap): Range {
  return { from: Math.max(gap.from, gap.to - STEP + 1), to: gap.to };
}

/** And the arrow pointing down: the lines just under the hunk above. */
export function fromAbove(gap: Gap): Range {
  return { from: gap.from, to: Math.min(gap.to, gap.from + STEP - 1) };
}

/** Whether one press would open the whole thing, which is when the two arrows
 * and the third gesture would all do the same and only one belongs on screen. */
export function fitsInOneStep(gap: Gap): boolean {
  return gap.to - gap.from + 1 <= STEP;
}

function ends(hunk: Hunk): number {
  return hunk.new_start + hunk.new_lines;
}

function shiftAfter(hunk: Hunk): number {
  return hunk.old_start + hunk.old_lines - (hunk.new_start + hunk.new_lines);
}

/** A gap as it stands after some of it has been opened: the stretches that are
 * on screen, in order, and the one that is still closed.
 *
 * The closed part stays in one piece because of the gestures on offer: reading
 * from the top, from the bottom, or all of it. None of them can open a hole in
 * the middle, so what is left is always a single run. */
export function opening<T extends Range>(gap: Gap, opened: T[]): Opening<T> {
  const mine = opened
    .filter((r) => gap.from <= r.from && r.to <= gap.to)
    .sort((a, b) => a.from - b.from);

  return { above: mine, closed: stillClosed(gap, mine) };
}

export type Opening<T extends Range = Range> = { above: T[]; closed: Gap | null };
