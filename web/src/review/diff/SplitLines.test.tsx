import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { file, noComments } from "./testing";
import type { FileDiff } from "@/api";

/** The grammar is fetched over the network and this is about what reaches the
 * screen, so the tokenizer is stood in for. */
vi.mock("@/hooks/useHighlight", () => ({ useHighlight: () => null }));

const { Diff } = await import("./Diff");

/** A screen of a given width, as the page is able to ask about it.
 *
 * jsdom has no layout, so `matchMedia` is the whole of what the component can
 * see — which is also why the split row decides this with a query rather than
 * by measuring. */
function screenIs(px: number) {
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches: query.includes("48rem") ? px < 768 : false,
    addEventListener: () => {},
    removeEventListener: () => {},
  }));
}

/** One unchanged line, then a line rewritten in place. */
function diff(): FileDiff {
  return {
    path: "src/a.rs",
    status: "modified",
    binary: false,
    additions: 1,
    deletions: 1,
    lineCount: 2,
    hunks: [
      {
        oldStart: 1,
        oldLines: 2,
        newStart: 1,
        newLines: 2,
        lines: [
          { kind: "context", oldNumber: 1, newNumber: 1, content: "fn main() {" },
          { kind: "removed", oldNumber: 2, newNumber: null, content: "  old();" },
          { kind: "added", oldNumber: null, newNumber: 2, content: "  new();" },
        ],
      },
    ],
  };
}

function draw() {
  render(
    <Diff
      diff={diff()}
      file={file()}
      view="split"
      comments={[]}
      actions={noComments()}
    />,
  );
}

describe("split on a screen with room for one column of code", () => {
  it("prints a line the change did not touch once, under both its numbers", () => {
    // Stacked, the same line on both sides arrives twice — the whole unchanged
    // half of a file read through again on the screen with least room for it.
    screenIs(390);
    draw();

    expect(screen.getAllByText("fn main() {")).toHaveLength(1);
    // Both numberings are still there, and so is a way into each side.
    expect(screen.getByLabelText("Comment on old line 1")).toBeTruthy();
    expect(screen.getByLabelText("Comment on new line 1")).toBeTruthy();
  });

  it("keeps a line the change did touch on its own side", () => {
    // What the two columns were saying is said by which gutter the number is
    // in, now that the sides are above each other rather than beside.
    screenIs(390);
    const { container } = render(
      <Diff diff={diff()} file={file()} view="split" comments={[]} actions={noComments()} />,
    );

    const changed = [...container.querySelectorAll(".stack-row")].at(-1);
    const gutters = [...(changed?.querySelectorAll(".ln") ?? [])];

    // Old on the first line with the second gutter empty, new on the second
    // with the first gutter empty.
    expect(gutters.map((g) => g.textContent?.trim())).toEqual(["2", "", "", "2"]);
    expect(gutters[1].classList.contains("blank")).toBe(true);
    expect(gutters[2].classList.contains("blank")).toBe(true);
  });

  it("faces the two columns where there is width for two columns", () => {
    // The wide screen is not touched by any of this: one grid, four cells, and
    // the unchanged line in both columns the way side by side means.
    screenIs(1440);
    const { container } = render(
      <Diff diff={diff()} file={file()} view="split" comments={[]} actions={noComments()} />,
    );

    expect(container.querySelectorAll(".stack-row")).toHaveLength(0);
    expect(container.querySelectorAll(".split-row").length).toBeGreaterThan(0);
    expect(screen.getAllByText("fn main() {")).toHaveLength(2);
  });
});
