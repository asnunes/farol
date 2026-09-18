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
    for (
      let i = 0;
      i < Math.min(segment.removed.length, segment.added.length);
      i++
    ) {
      const pair: [number, number] = [segment.removed[i], segment.added[i]];
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
