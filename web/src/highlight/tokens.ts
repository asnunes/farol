import type { DiffLine, Hunk } from "@/api";

/** Colour the two sides of a hunk and hand each row the tokens for its line.
 *
 * A whole side at a time, never line by line: a tokenizer carries state across
 * lines, so a block comment or a multi-line string tokenized one line at a time
 * comes out as code. Reading each side whole gets that right within the hunk,
 * which is as far as a hunk can see — a comment that opened before the first
 * line of the hunk is still beyond it, and the file would have to travel for
 * that to be fixed.
 *
 * Deleted lines belong to the old side and added ones to the new; context lines
 * are in both, which is what keeps the two sides in step. */
export function colourHunk(hunk: Hunk, tokenize: Tokenize): Token[][] {
  const side = (drop: DiffLine["kind"]) =>
    hunk.lines.filter((l) => l.kind !== drop).map((l) => l.content);

  const old = tokenize(side("added").join("\n"));
  const fresh = tokenize(side("removed").join("\n"));

  const at = { old: 0, fresh: 0 };
  return hunk.lines.map((line) => {
    const plain = [{ content: line.content }];

    if (line.kind === "removed") return old[at.old++] ?? plain;
    if (line.kind === "added") return fresh[at.fresh++] ?? plain;

    // Context is in both texts, so it moves both cursors. Advancing only the
    // one it was read from leaves the other behind by a line for every context
    // line, and every row after the first deletion shows the wrong code.
    const row = fresh[at.fresh] ?? plain;
    at.old++;
    at.fresh++;
    return row;
  });
}

/** A run of code that shares one colour. `style` carries both palettes at once,
 * as custom properties, so switching theme is a CSS matter and nothing is
 * tokenized twice. */
export type Token = { content: string; style?: Record<string, string> };

/** Turns a block of code into one array of tokens per line. */
export type Tokenize = (code: string) => Token[][];
