import { describe, expect, it } from "vitest";
import { segments, splitRows } from "./split";
import type { DiffLine } from "@/api";

function lines(...kinds: [DiffLine["kind"], string][]): DiffLine[] {
  return kinds.map(([kind, content]) => ({
    kind,
    old_number: null,
    new_number: null,
    content,
  }));
}

/** What each row holds, as the content of the two sides. */
const shape = (rows: ReturnType<typeof splitRows>, from: DiffLine[]) =>
  rows.map((r) => [
    r.left === null ? null : from[r.left].content,
    r.right === null ? null : from[r.right].content,
  ]);

describe("laying a hunk out in two columns", () => {
  it("puts a context line on both sides", () => {
    const l = lines(["context", "um"], ["context", "dois"]);

    expect(shape(splitRows(l), l)).toEqual([
      ["um", "um"],
      ["dois", "dois"],
    ]);
  });

  it("faces a removed line with the added one that replaced it", () => {
    const l = lines(["context", "antes"], ["removed", "velho"], ["added", "novo"]);

    expect(shape(splitRows(l), l)).toEqual([
      ["antes", "antes"],
      ["velho", "novo"],
    ]);
  });

  it("pairs a run by position, leaving the shorter side blank", () => {
    // Three lines out, one in. The first faces its replacement and the other
    // two face nothing, which is what says the code was dropped rather than
    // rewritten.
    const l = lines(
      ["removed", "a"],
      ["removed", "b"],
      ["removed", "c"],
      ["added", "d"],
    );

    expect(shape(splitRows(l), l)).toEqual([
      ["a", "d"],
      ["b", null],
      ["c", null],
    ]);
  });

  it("leaves the old side blank where a run only adds", () => {
    const l = lines(["context", "topo"], ["added", "novo"], ["added", "outro"]);

    expect(shape(splitRows(l), l)).toEqual([
      ["topo", "topo"],
      [null, "novo"],
      [null, "outro"],
    ]);
  });

  it("starts a new run after every context line", () => {
    // Without this the second change would be paired against the first, and
    // lines that have nothing to do with each other would end up facing.
    const l = lines(
      ["removed", "a"],
      ["added", "b"],
      ["context", "meio"],
      ["removed", "c"],
      ["added", "d"],
    );

    expect(shape(splitRows(l), l)).toEqual([
      ["a", "b"],
      ["meio", "meio"],
      ["c", "d"],
    ]);
  });

  it("keeps every line of the hunk, on one side or the other", () => {
    const l = lines(
      ["context", "a"],
      ["removed", "b"],
      ["removed", "c"],
      ["added", "d"],
      ["context", "e"],
    );

    const seen = new Set(splitRows(l).flatMap((r) => [r.left, r.right]));
    seen.delete(null);

    expect(seen.size).toBe(l.length);
  });
});

describe("the segments a hunk is made of", () => {
  it("keeps each context line on its own and gathers the change between them", () => {
    const l = lines(
      ["context", "a"],
      ["removed", "b"],
      ["added", "c"],
      ["added", "d"],
      ["context", "e"],
    );

    expect(segments(l)).toEqual([
      { context: 0 },
      { removed: [1], added: [2, 3] },
      { context: 4 },
    ]);
  });

  it("gathers a run that only adds, with nothing on the old side", () => {
    const l = lines(["added", "a"], ["added", "b"]);

    expect(segments(l)).toEqual([{ removed: [], added: [0, 1] }]);
  });
});
