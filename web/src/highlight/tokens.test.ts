import { describe, expect, it } from "vitest";
import { colourHunk, type Token } from "./tokens";
import type { DiffLine, Hunk } from "@/api";

/** A tokenizer that says which side it was given and where in it each line
 * fell, so the mapping can be checked instead of the colours. */
const numbering = (code: string): Token[][] =>
  code.split("\n").map((line, i) => [{ content: line, style: { at: String(i) } }]);

function hunk(...lines: [DiffLine["kind"], string][]): Hunk {
  return {
    oldStart: 1,
    oldLines: lines.length,
    newStart: 1,
    newLines: lines.length,
    lines: lines.map(([kind, content]) => ({
      kind,
      oldNumber: null,
      newNumber: null,
      content,
    })),
  };
}

const text = (rows: Token[][]) => rows.map((r) => r.map((t) => t.content).join(""));

describe("colouring the two sides of a hunk", () => {
  it("gives every row the tokens for its own line", () => {
    const rows = colourHunk(
      hunk(["context", "um"], ["removed", "dois"], ["added", "três"], ["context", "quatro"]),
      numbering,
    );

    expect(text(rows)).toEqual(["um", "dois", "três", "quatro"]);
  });

  it("reads a deleted line from the old side and an added one from the new", () => {
    // The two sides are different texts. Taking a deleted line from the new
    // side would hand the row somebody else's code.
    const rows = colourHunk(hunk(["removed", "antes"], ["added", "depois"]), numbering);

    expect(rows[0][0].style).toEqual({ at: "0" });
    expect(rows[1][0].style).toEqual({ at: "0" });
  });

  it("keeps the sides in step through context lines", () => {
    // Context is in both texts, so a run of deletions must not slide the new
    // side out of line with the rows that follow.
    const rows = colourHunk(
      hunk(
        ["context", "a"],
        ["removed", "b"],
        ["removed", "c"],
        ["added", "d"],
        ["context", "e"],
      ),
      numbering,
    );

    expect(text(rows)).toEqual(["a", "b", "c", "d", "e"]);
    // The new side reads a, d, e — so the closing context is its line 2.
    expect(rows[4][0].style).toEqual({ at: "2" });
  });

  it("hands the whole side to the tokenizer at once, not line by line", () => {
    // The reason the mapping exists at all: a tokenizer carries state across
    // lines, and one that only ever sees a single line reads an open block
    // comment as code.
    const seen: string[] = [];
    colourHunk(hunk(["context", "/*"], ["added", "dentro"], ["context", "*/"]), (code) => {
      seen.push(code);
      return numbering(code);
    });

    expect(seen).toHaveLength(2);
    expect(seen[0]).toBe("/*\n*/");
    expect(seen[1]).toBe("/*\ndentro\n*/");
  });

  it("falls back to the plain line when the tokenizer comes up short", () => {
    const rows = colourHunk(hunk(["context", "a"], ["context", "b"]), () => [
      [{ content: "a" }],
    ]);

    expect(text(rows)).toEqual(["a", "b"]);
    expect(rows[1][0].style).toBeUndefined();
  });
});
