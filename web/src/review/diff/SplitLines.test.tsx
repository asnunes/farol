import { fireEvent, render, screen } from "@testing-library/react";
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

describe("picking a span with the keyboard where the columns are stacked", () => {
  /** The narrow layout, drawn, with the actions a finished comment reports to. */
  function narrow() {
    screenIs(390);
    const actions = noComments();
    render(
      <Diff diff={diff()} file={file()} view="split" comments={[]} actions={actions} />,
    );
    return actions;
  }

  /** Shift and an arrow, on the `+` that has focus. */
  function stretch(plus: HTMLElement, key: "ArrowUp" | "ArrowDown", times = 1) {
    for (let i = 0; i < times; i++) fireEvent.keyDown(plus, { key, shiftKey: true });
  }

  /** Open the box the `+` now offers and send what is written in it. */
  function write(plus: HTMLElement, body: string) {
    fireEvent.click(plus);
    fireEvent.change(screen.getByRole("textbox"), { target: { value: body } });
    fireEvent.click(screen.getByText("Comment"));
  }

  it("reaches down the old side from the line printed once for both", () => {
    // Stacked, one row carries both gutters, and each of them has to be told
    // how far its own side runs. Told nothing, the keyboard offered the one
    // line under the cursor and no more — on the screen where there is no drag
    // to fall back on.
    const actions = narrow();
    const plus = screen.getByLabelText("Comment on old line 1");

    stretch(plus, "ArrowDown");

    expect(plus.getAttribute("aria-label")).toBe("Comment on old lines 1–2");
    write(plus, "Why did this go?");
    expect(actions.add).toHaveBeenCalledWith("src/a.rs", "old", 1, 2, "Why did this go?");
  });

  it("reaches down the new side of that same row, on the new numbering", () => {
    // The other gutter of the one row, counted separately: line 1 is on both
    // sides here and means two different things.
    const actions = narrow();
    const plus = screen.getByLabelText("Comment on new line 1");

    stretch(plus, "ArrowDown");

    expect(plus.getAttribute("aria-label")).toBe("Comment on new lines 1–2");
    write(plus, "Why is this here?");
    expect(actions.add).toHaveBeenCalledWith("src/a.rs", "new", 1, 2, "Why is this here?");
  });

  it("reaches back up from a line the change touched", () => {
    // A changed line sits on a row of its own with the other gutter blank, and
    // that gutter is a third place the reach has to arrive at.
    const actions = narrow();
    const plus = screen.getByLabelText("Comment on old line 2");

    stretch(plus, "ArrowUp");

    expect(plus.getAttribute("aria-label")).toBe("Comment on old lines 1–2");
    write(plus, "This pair, then?");
    expect(actions.add).toHaveBeenCalledWith("src/a.rs", "old", 1, 2, "This pair, then?");
  });

  it("stops at the end of the hunk rather than crossing to the other side", () => {
    // Both sides number 1 and 2 here. A reach that ran past its own side would
    // land on the other one's numbering, which is a span nobody chose.
    narrow();
    const plus = screen.getByLabelText("Comment on old line 1");

    stretch(plus, "ArrowDown", 4);

    expect(plus.getAttribute("aria-label")).toBe("Comment on old lines 1–2");
  });
});
