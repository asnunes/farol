/** What changed inside a line, when a removal and an addition are the same line
 * edited rather than two unrelated lines.
 *
 * Compared word by word instead of character by character: characters find
 * matches inside words and produce a rash of one letter marks, which is noisier
 * than marking nothing at all.
 *
 * The ranges are over the characters of the line, because that is what the
 * renderer has to cut the syntax tokens at. */
export function changedRanges(before: string, after: string): Changed | null {
  const a = words(before);
  const b = words(after);
  const same = commonRun(a, b);

  // Two lines that share almost nothing are a rewrite, not an edit. Marking
  // them leaves both lines lit end to end, which says less than the row colour
  // already said.
  const kept = same.reduce((n, [i]) => n + (i === null ? 0 : a[i].length), 0);
  if (kept < Math.max(before.length, after.length) * KEEPS_AT_LEAST) return null;

  return {
    before: rangesOf(before, a, same.map(([i]) => i)),
    after: rangesOf(after, b, same.map(([, j]) => j)),
  };
}

export type Range = { from: number; to: number };

export type Changed = { before: Range[]; after: Range[] };

/** How much of the longer line has to survive for the two to count as the same
 * line edited. Below this the marks stop telling the reader anything. */
const KEEPS_AT_LEAST = 0.25;

/** Words, runs of whitespace, and punctuation one character at a time. Keeping
 * the separators means the pieces still concatenate back into the line. */
function words(text: string): string[] {
  return text.match(/\w+|\s+|[^\w\s]/g) ?? [];
}

/** The longest common subsequence, as pairs of indices into the two sides.
 *
 * Lines are short enough that the table costs nothing, and the alternative,
 * matching greedily from both ends, misses a word that moved. */
function commonRun(a: string[], b: string[]): [number | null, number | null][] {
  const table: number[][] = Array.from({ length: a.length + 1 }, () =>
    new Array(b.length + 1).fill(0),
  );

  for (let i = a.length - 1; i >= 0; i--) {
    for (let j = b.length - 1; j >= 0; j--) {
      table[i][j] =
        a[i] === b[j] ? table[i + 1][j + 1] + 1 : Math.max(table[i + 1][j], table[i][j + 1]);
    }
  }

  const pairs: [number | null, number | null][] = [];
  let i = 0;
  let j = 0;
  while (i < a.length && j < b.length) {
    if (a[i] === b[j]) {
      pairs.push([i, j]);
      i++;
      j++;
    } else if (table[i + 1][j] >= table[i][j + 1]) {
      i++;
    } else {
      j++;
    }
  }
  return pairs;
}

/** The character ranges of every word that is *not* in the common run. */
function rangesOf(text: string, pieces: string[], kept: (number | null)[]): Range[] {
  const survives = new Set(kept.filter((i): i is number => i !== null));
  const ranges: Range[] = [];
  let at = 0;

  for (let i = 0; i < pieces.length; i++) {
    const end = at + pieces[i].length;
    if (!survives.has(i)) ranges.push({ from: at, to: end });
    at = end;
  }

  return trimmed(text, joined(text, ranges));
}

/** Marks with nothing but blank space between them become one mark.
 *
 * The common run happily matches a space in the middle of an edit, which would
 * otherwise cut `, view, onView` into two boxes with a gap where a space
 * happened to line up. */
function joined(text: string, ranges: Range[]): Range[] {
  return ranges.reduce<Range[]>((kept, range) => {
    const last = kept[kept.length - 1];
    if (last && text.slice(last.to, range.from).trim() === "") last.to = range.to;
    else kept.push({ ...range });
    return kept;
  }, []);
}

/** Blank space at the edge of a mark is space the reader cannot see lit, so it
 * only widens the box. A mark that is nothing but space disappears with it. */
function trimmed(text: string, ranges: Range[]): Range[] {
  return ranges
    .map((range) => {
      let { from, to } = range;
      while (from < to && /\s/.test(text[from])) from++;
      while (to > from && /\s/.test(text[to - 1])) to--;
      return { from, to };
    })
    .filter((range) => range.to > range.from);
}
