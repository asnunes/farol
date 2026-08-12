import { cn } from "@/lib/utils";
import type { Range } from "./intraline";
import type { Token } from "@/highlight/tokens";

/** The line, coloured if its grammar has arrived and plain until then.
 *
 * The palette for both themes rides on the token as custom properties, so the
 * stylesheet decides which one applies and nothing is tokenized twice. */
export function Code({ tokens, plain, marks }: CodeProps) {
  const pieces = tokens ?? [{ content: plain }];

  return (
    <>
      {cut(pieces, marks ?? []).map((piece, i) => (
        <span
          key={i}
          className={cn(piece.style && "tok", piece.changed && "changed")}
          style={piece.style}
        >
          {piece.content}
        </span>
      ))}
    </>
  );
}

/** Split the syntax tokens at the edges of the marks.
 *
 * Two things want to segment the same line: the grammar, which decides the
 * colour, and the word diff, which decides what changed. Neither can give way,
 * so the line is cut wherever either of them says. */
function cut(tokens: Token[], marks: Range[]): Piece[] {
  const pieces: Piece[] = [];
  let at = 0;

  for (const token of tokens) {
    const start = at;
    at += token.content.length;

    // Every boundary inside this token, in order, so the slices come out in
    // reading order and concatenate back into the token.
    const edges = [start, ...marks.flatMap((m) => [m.from, m.to]), at]
      .filter((edge) => edge >= start && edge <= at)
      .sort((a, b) => a - b);

    for (let i = 0; i < edges.length - 1; i++) {
      const [from, to] = [edges[i], edges[i + 1]];
      if (from === to) continue;
      pieces.push({
        content: token.content.slice(from - start, to - start),
        style: token.style,
        changed: marks.some((m) => m.from <= from && to <= m.to),
      });
    }
  }

  return pieces;
}

type Piece = { content: string; style?: Record<string, string>; changed: boolean };

type CodeProps = { tokens?: Token[]; plain: string; marks?: Range[] };
