import { splitRows, segments } from "./split";
import type { SplitRow } from "./split";
import type { Hunk } from "@/api";
import type { DiffView } from "@/hooks/useDiffView";
import type { Token, Tokenize } from "@/highlight/tokens";

/** Chunk display rows, keeping paired old/new lines together in split view. */
export function hunkWindows(hunk: Hunk, view: DiffView): HunkWindow[] {
  const rows = view === "split" ? splitRows(hunk.lines) : null;
  const count = rows?.length ?? hunk.lines.length;
  const windows: HunkWindow[] = [];
  for (let from = 0; from < count; from += WINDOW_LINES) {
    const to = Math.min(from + WINDOW_LINES, count);
    const slice = rows?.slice(from, to);
    const indices = slice
      ? [
          ...new Set(
            slice.flatMap((row) =>
              [row.left, row.right].filter((i): i is number => i !== null),
            ),
          ),
        ].sort((a, b) => a - b)
      : Array.from({ length: to - from }, (_, i) => from + i);
    windows.push({ indices, rows: slice });
  }
  return windows;
}

/** The two syntax streams and paired edits, built without tokenizing hidden code. */
export function hunkText(hunk: Hunk): HunkText {
  const old: string[] = [];
  const fresh: string[] = [];
  const positions = hunk.lines.map((line) => {
    const before = line.kind === "added" ? null : old.push(line.content) - 1;
    const after = line.kind === "removed" ? null : fresh.push(line.content) - 1;
    return { old: before, fresh: after };
  });
  const pairs = new Map<number, [number, number]>();
  for (const segment of segments(hunk.lines)) {
    if ("context" in segment) continue;
    for (const pair of paired(segment.removed, segment.added, hunk)) {
      pairs.set(pair[0], pair);
      pairs.set(pair[1], pair);
    }
  }
  return { old, fresh, positions, pairs };
}

export function colourWindow(
  text: HunkText,
  indices: number[],
  tokenize: Tokenize,
): Token[][] {
  const coloured: Token[][] = [];
  for (const side of ["old", "fresh"] as const) {
    const wanted = indices.filter((i) => text.positions[i][side] !== null);
    if (!wanted.length) continue;
    const from = Math.min(...wanted.map((i) => text.positions[i][side]!));
    const to = Math.max(...wanted.map((i) => text.positions[i][side]!)) + 1;
    const tokens = tokenize.range
      ? tokenize.range(text[side], from, to)
      : tokenize(text[side].slice(0, to).join("\n")).slice(from, to);
    for (const i of wanted)
      coloured[i] = tokens[text.positions[i][side]! - from];
  }
  return coloured;
}

export const WINDOW_LINES = 80;

export type HunkWindow = { indices: number[]; rows?: SplitRow[] };
export type HunkText = {
  old: string[];
  fresh: string[];
  positions: { old: number | null; fresh: number | null }[];
  pairs: Map<number, [number, number]>;
};

/** Which removed line is the same line as which added one, within one run of
 * changes.
 *
 * Pairing by position alone shifts every pair after a line that was inserted
 * or dropped, so a block gofmt realigned around one new name came out with each
 * line compared against its neighbour. Lines equal but for blank space anchor
 * the pairing first; only what sits between two anchors is paired by position. */
function paired(removed: number[], added: number[], hunk: Hunk): [number, number][] {
  const key = (i: number) => hunk.lines[i].content.replace(/\s+/g, "");
  const a = removed.map(key);
  const b = added.map(key);
  const table: number[][] = Array.from({ length: a.length + 1 }, () =>
    new Array(b.length + 1).fill(0),
  );
  for (let i = a.length - 1; i >= 0; i--)
    for (let j = b.length - 1; j >= 0; j--)
      table[i][j] =
        a[i] === b[j] ? table[i + 1][j + 1] + 1 : Math.max(table[i + 1][j], table[i][j + 1]);

  const pairs: [number, number][] = [];
  let i = 0;
  let j = 0;
  let gapI = 0;
  let gapJ = 0;
  const closeGap = () => {
    for (let k = 0; k < Math.min(i - gapI, j - gapJ); k++)
      pairs.push([removed[gapI + k], added[gapJ + k]]);
  };
  while (i < a.length && j < b.length) {
    if (a[i] === b[j]) {
      closeGap();
      pairs.push([removed[i++], added[j++]]);
      gapI = i;
      gapJ = j;
    } else if (table[i + 1][j] >= table[i][j + 1]) i++;
    else j++;
  }
  i = a.length;
  j = b.length;
  closeGap();
  return pairs;
}
