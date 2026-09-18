import { expect, it } from "vitest";
import { load, tokenizerFor } from "@/highlight/highlighter";
import { colourHunk } from "@/highlight/tokens";
import { colourWindow, hunkText, hunkWindows } from "./hunkWindows";
import type { Hunk } from "@/api";

it("preserves paired rows and syntax state on both sides across chunk boundaries", async () => {
  await load("rust");
  const tokenize = tokenizerFor("rust")!;
  const hunk: Hunk = {
    oldStart: 1,
    oldLines: 122,
    newStart: 1,
    newLines: 122,
    lines: [
      {
        kind: "context",
        oldNumber: 1,
        newNumber: 1,
        content: "/* comment begins",
      },
      ...Array.from({ length: 120 }, (_, i) => ({
        kind: "removed" as const,
        oldNumber: i + 2,
        newNumber: null,
        content: `old comment ${i}`,
      })),
      ...Array.from({ length: 120 }, (_, i) => ({
        kind: "added" as const,
        oldNumber: null,
        newNumber: i + 2,
        content: `new comment ${i}`,
      })),
      { kind: "context", oldNumber: 122, newNumber: 122, content: "end */" },
    ],
  };
  const windows = hunkWindows(hunk, "split");
  expect(windows).toHaveLength(2);
  expect(windows.every((window) => window.rows!.length <= 80)).toBe(true);
  const complete = colourHunk(hunk, tokenize);
  const text = hunkText(hunk);
  for (const window of [...windows].reverse()) {
    const partial = colourWindow(text, window.indices, tokenize);
    for (const index of window.indices)
      expect(partial[index]).toEqual(complete[index]);
    for (const row of window.rows!) {
      if (row.left !== null && row.right !== null) {
        expect(hunk.lines[row.left].oldNumber).toBe(
          hunk.lines[row.right].newNumber,
        );
      }
    }
  }
});
