import { act, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { comment, file, noComments } from "./testing";
import type { DiffView } from "@/hooks/useDiffView";
import type { Token, Tokenize } from "@/highlight/tokens";
import type { CommentView, FileDiff } from "@/api";

/** The grammar is fetched over the network and this is about what reaches the
 * screen, so the tokenizer is stood in for. */
let tokenizer: Tokenize | null = null;
vi.mock("@/hooks/useHighlight", () => ({ useHighlight: () => tokenizer }));

const { Diff } = await import("./Diff");


function diff(): FileDiff {
  return {
    path: "src/a.rs",
    status: "modified",
    binary: false,
    additions: 1,
    deletions: 0,
    lineCount: 2,
    hunks: [
      {
        oldStart: 1,
        oldLines: 1,
        newStart: 1,
        newLines: 2,
        lines: [
          { kind: "context", oldNumber: 1, newNumber: 1, content: "fn main() {}" },
          { kind: "added", oldNumber: null, newNumber: 2, content: "// nota" },
        ],
      },
    ],
  };
}

/** The same file, with a line taken out above what stayed. */
function withARemoval(): FileDiff {
  const one = diff();
  one.hunks[0].oldLines = 2;
  one.hunks[0].lines.unshift({
    kind: "removed",
    oldNumber: 1,
    newNumber: null,
    content: "fn main() { }",
  });
  one.hunks[0].lines[1].oldNumber = 2;
  return one;
}

/** A file the change deleted whole: two old lines and no new side at all. */
function deleted(): FileDiff {
  return {
    path: "src/gone.rs",
    status: "deleted",
    binary: false,
    additions: 0,
    deletions: 2,
    lineCount: 0,
    hunks: [
      {
        oldStart: 1,
        oldLines: 2,
        newStart: 0,
        newLines: 0,
        lines: [
          { kind: "removed", oldNumber: 1, newNumber: null, content: "fn gone() {}" },
          { kind: "removed", oldNumber: 2, newNumber: null, content: "// and this" },
        ],
      },
    ],
  };
}

/** Two lines rewritten in place, so line 1 exists on both sides and means two
 * different things. The case a comment that did not carry its side cannot
 * survive. */
function rewritten(): FileDiff {
  return {
    path: "src/a.rs",
    status: "modified",
    binary: false,
    additions: 2,
    deletions: 2,
    lineCount: 2,
    hunks: [
      {
        oldStart: 1,
        oldLines: 2,
        newStart: 1,
        newLines: 2,
        lines: [
          { kind: "removed", oldNumber: 1, newNumber: null, content: "let a = 1;" },
          { kind: "removed", oldNumber: 2, newNumber: null, content: "let b = 2;" },
          { kind: "added", oldNumber: null, newNumber: 1, content: "let a = 3;" },
          { kind: "added", oldNumber: null, newNumber: 2, content: "let b = 4;" },
        ],
      },
    ],
  };
}

describe("the diff on screen", () => {
  it("carries both palettes on the token, so the theme is a css matter", () => {
    // Colour arrives as custom properties rather than a resolved colour: the
    // stylesheet picks light or dark, and moving between them re-tokenizes
    // nothing.
    tokenizer = (code: string): Token[][] =>
      code.split("\n").map((line) => [
        {
          content: line,
          style: { "--shiki-light": "#7d4fae", "--shiki-dark": "#c39ae8" },
        },
      ]);

    const { container } = render(
      <Diff diff={diff()} file={file()} view="unified" comments={[]} actions={noComments()} />,
    );

    const tokens = container.querySelectorAll(".tok");
    expect(tokens).toHaveLength(2);
    expect(tokens[0].getAttribute("style")).toContain("--shiki-light: #7d4fae");
    expect(tokens[0].getAttribute("style")).toContain("--shiki-dark: #c39ae8");
  });

  it("draws the code plain while the grammar has not arrived", () => {
    // Every file starts here, and a file whose language farol cannot name
    // stays here. Neither is an error on screen.
    tokenizer = null;

    const { container } = render(
      <Diff diff={diff()} file={file()} view="unified" comments={[]} actions={noComments()} />,
    );

    expect(container.textContent).toContain("fn main() {}");
    expect(container.querySelectorAll(".tok")).toHaveLength(0);
  });
});

describe("commenting on the diff", () => {
  /** The whole file, rows and everything hanging off them, in the order the
   * page lays them out. */
  function laidOut(container: HTMLElement) {
    return [...container.querySelectorAll(".row, .comment, .commentbox")].map(
      (el) => el.className.split(" ")[0],
    );
  }

  function draw(
    comments = [] as ReturnType<typeof comment>[],
    actions = noComments(),
  ) {
    tokenizer = null;
    return render(
      <Diff
        diff={diff()}
        file={file()}
        view="unified"
        comments={comments}
        actions={actions}
      />,
    );
  }

  it("puts a comment under the last line it covers", () => {
    // Where the reader stopped reading to write it, which is the same place the
    // session's own line notes land.
    const { container } = draw([comment({ from: 1, to: 2 })]);

    expect(laidOut(container)).toEqual(["row", "row", "comment"]);
  });

  it("marks every line the comment covers, not only the last", () => {
    // The rail is what makes the span visible before the prose explains it.
    const { container } = draw([comment({ from: 1, to: 2 })]);

    const railed = [...container.querySelectorAll(".row")].map((r) =>
      r.className.includes("commented"),
    );
    expect(railed).toEqual([true, true]);
  });

  it.each(["unified", "split"] as const)(
    "raises the plus from the whole line, code included, in %s",
    (view) => {
      // The gutter is four characters wide and the reader's pointer is on the
      // code, so a control that only came up over the numbers was a control
      // nobody found. jsdom resolves no `:hover`, so what is checked is the
      // wiring that decides it: the reveal answers to a group, and the element
      // naming that group holds the line's code as well as its gutter.
      tokenizer = null;
      render(
        <Diff diff={diff()} file={file()} view={view} comments={[]} actions={noComments()} />,
      );

      const plus = screen.getByLabelText("Comment on new line 2");
      const line = plus.closest('[class~="group/line"]');

      expect(plus.className).toContain("group-hover/line:opacity-100");
      expect(plus.className).toContain("focus-visible:opacity-100");
      expect(line?.querySelector(".code")?.textContent).toContain("// nota");
    },
  );

  it("keeps each column of a split row to its own plus", () => {
    // The two halves are two different lines. Raising the right-hand control
    // because the pointer is over the left-hand code would offer a comment on
    // code the reader is not looking at.
    // A context line is the case that has both: one row, the same line facing
    // itself, numbered once on each side.
    tokenizer = null;
    render(
      <Diff diff={diff()} file={file()} view="split" comments={[]} actions={noComments()} />,
    );

    const before = screen.getByLabelText("Comment on old line 1");
    const after = screen.getByLabelText("Comment on new line 1");

    expect(before.closest(".row")).toBe(after.closest(".row"));
    expect(before.closest('[class~="group/line"]')).not.toBe(
      after.closest('[class~="group/line"]'),
    );
  });

  it.each([
    ["down the numbers", 0, 1],
    ["back up them", 1, 0],
  ])("picks the passage a drag %s covers", (_way, from, to) => {
    // The hook is tested on its own; this is about the gutter still being
    // wired to it, which is what a change to how the `+` is revealed could
    // quietly undo.
    const { container } = draw();
    const gutters = container.querySelectorAll(".ln");

    fireEvent.mouseDown(gutters[from]);
    fireEvent.mouseOver(gutters[to]);
    fireEvent.mouseUp(window);

    // The label the box opens under is also the box's name, so the span the
    // drag covered is what a screen reader reads out of it.
    expect((screen.getByLabelText("Comment on lines 1–2") as HTMLTextAreaElement).tagName).toBe(
      "TEXTAREA",
    );
    expect(container.querySelector(".commentbox .lbl")?.textContent).toBe("Comment on lines 1–2");
  });

  it("opens the box on the line whose plus was pressed", () => {
    const { container } = draw();

    fireEvent.click(screen.getByLabelText("Comment on new line 2"));

    expect(laidOut(container)).toEqual(["row", "row", "commentbox"]);
  });

  it("sends what was written, against the lines it was written about", () => {
    const actions = noComments();
    draw([], actions);
    fireEvent.click(screen.getByLabelText("Comment on new line 2"));

    fireEvent.change(screen.getByRole("textbox"), {
      target: { value: "Why this order?" },
    });
    fireEvent.click(screen.getByText("Comment"));

    expect(actions.add).toHaveBeenCalledWith(
      "src/a.rs",
      "new",
      2,
      2,
      "Why this order?",
    );
  });

  it("will not send an empty comment", () => {
    // A comment with nothing in it says nothing, and the server refuses it
    // anyway. Refusing here saves the round trip and the error banner.
    const actions = noComments();
    draw([], actions);
    fireEvent.click(screen.getByLabelText("Comment on new line 2"));

    fireEvent.change(screen.getByRole("textbox"), { target: { value: "   " } });
    fireEvent.click(screen.getByText("Comment"));

    expect(actions.add).not.toHaveBeenCalled();
  });

  it("offers a way in on a removed line, counted on the old side", () => {
    // A removed line is not in the file any more, and it is still the thing a
    // reader has a question about — often the only thing. It is offered under
    // the numbering it has, which is the old one.
    tokenizer = null;
    render(
      <Diff
        diff={withARemoval()}
        file={file()}
        view="unified"
        comments={[]}
        actions={noComments()}
      />,
    );

    expect(screen.queryAllByLabelText(/^Comment on/)).toHaveLength(3);
    expect(screen.queryByLabelText("Comment on old line 1")).toBeTruthy();
    expect(screen.queryByLabelText("Comment on new line 1")).toBeTruthy();
    expect(screen.queryByLabelText("Comment on new line 2")).toBeTruthy();
  });
});

describe("commenting on the old side of the diff", () => {
  function draw(diff: FileDiff, view: DiffView, comments: CommentView[], actions = noComments()) {
    tokenizer = null;
    return render(
      <Diff
        diff={diff}
        file={file({ path: diff.path })}
        view={view}
        comments={comments}
        actions={actions}
      />,
    );
  }

  /** The whole file, rows and everything hanging off them, in the order the
   * page lays them out. */
  function laidOut(container: HTMLElement) {
    return [...container.querySelectorAll(".row, .comment, .commentbox")].map(
      (el) => el.className.split(" ")[0],
    );
  }

  it("writes a comment on a removed line against the old numbers", () => {
    const actions = noComments();
    draw(withARemoval(), "unified", [], actions);

    fireEvent.click(screen.getByLabelText("Comment on old line 1"));
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "Why did this go?" } });
    fireEvent.click(screen.getByText("Comment"));

    expect(actions.add).toHaveBeenCalledWith("src/a.rs", "old", 1, 1, "Why did this go?");
  });

  it("covers a span dragged down the removed lines, either way round", () => {
    // The gesture in both directions, because the reader drags from the line
    // that puzzled them and that is as often the last as the first.
    for (const [first, second] of [
      [1, 2],
      [2, 1],
    ]) {
      const actions = noComments();
      const { unmount } = draw(deleted(), "unified", [], actions);
      const rows = screen.getAllByText(/^[12]$/);

      fireEvent.mouseDown(rows[first - 1]);
      fireEvent.mouseEnter(rows[second - 1]);
      fireEvent.mouseUp(window);
      fireEvent.change(screen.getByRole("textbox"), { target: { value: "All of it?" } });
      fireEvent.click(screen.getByText("Comment"));

      expect(actions.add).toHaveBeenCalledWith("src/gone.rs", "old", 1, 2, "All of it?");
      unmount();
    }
  });

  it("hangs a comment on a file that was deleted whole under its last line", () => {
    // The one kind of change with nothing on the new side at all. Anchored to
    // the old numbering, it renders where the reader was looking.
    const { container } = draw(deleted(), "unified", [
      comment({ path: "src/gone.rs", side: "old", from: 1, to: 2 }),
    ]);

    expect(laidOut(container)).toEqual(["row", "row", "comment"]);
    expect(
      [...container.querySelectorAll(".row")].map((r) => r.className.includes("commented")),
    ).toEqual([true, true]);
  });

  it("keeps two comments on the same number apart by the side they are on", () => {
    // Line 1 was rewritten, so it exists on both sides. Read by number alone,
    // each question would land on both rows.
    const { container } = draw(rewritten(), "unified", [
      comment({ id: "1", side: "old", from: 1, to: 1, body: "Why was this here?" }),
      comment({ id: "2", side: "new", from: 1, to: 1, body: "Why is this here?" }),
    ]);

    expect(laidOut(container)).toEqual(["row", "comment", "row", "row", "comment", "row"]);
    const said = [...container.querySelectorAll(".comment .body")].map((el) => el.textContent);
    expect(said).toEqual(["Why was this here?", "Why is this here?"]);
  });

  it("draws each comment in the column it was written in, split", () => {
    // The same two comments, in the other layout. Split pairs the rewritten
    // line with its replacement on one row, so both hang off that row — each
    // from the side it belongs to.
    const { container } = draw(rewritten(), "split", [
      comment({ id: "1", side: "old", from: 2, to: 2, body: "Why was this here?" }),
      comment({ id: "2", side: "new", from: 1, to: 1, body: "Why is this here?" }),
    ]);

    expect(laidOut(container)).toEqual(["row", "comment", "row", "comment"]);
    const said = [...container.querySelectorAll(".comment .body")].map((el) => el.textContent);
    expect(said).toEqual(["Why is this here?", "Why was this here?"]);
  });

  it("offers the old side in the left column and the new side in the right", () => {
    // Split view answers the question by where the reader pressed, so each
    // column offers its own numbering and neither offers the other's.
    draw(rewritten(), "split", []);

    expect(screen.getByLabelText("Comment on old line 1")).toBeTruthy();
    expect(screen.getByLabelText("Comment on old line 2")).toBeTruthy();
    expect(screen.getByLabelText("Comment on new line 1")).toBeTruthy();
    expect(screen.getByLabelText("Comment on new line 2")).toBeTruthy();
  });

  it("says which side a comment on the old one is about", () => {
    // Both sides number from one, so the label has to say more than a range.
    draw(withARemoval(), "unified", [comment({ side: "old", from: 1, to: 2 })]);

    expect(screen.getByText("1–2 (old)")).toBeTruthy();
  });
});

describe("picking the lines a comment is about with the keyboard", () => {
  function draw(one: FileDiff, view: DiffView, actions = noComments()) {
    tokenizer = null;
    render(
      <Diff
        diff={one}
        file={file({ path: one.path })}
        view={view}
        comments={[]}
        actions={actions}
      />,
    );
    return actions;
  }

  /** Shift and an arrow, on the `+` that has focus. */
  function stretch(plus: HTMLElement, key: "ArrowUp" | "ArrowDown", times = 1) {
    for (let i = 0; i < times; i++) fireEvent.keyDown(plus, { key, shiftKey: true });
  }

  it("reaches down the lines from the plus, and opens the box over all of them", () => {
    // The drag is a mouse and nothing else. This is the same passage, picked
    // from the one control the keyboard can already reach.
    const actions = draw(diff(), "unified");
    const plus = screen.getByLabelText("Comment on new line 1");

    stretch(plus, "ArrowDown");

    expect(plus.getAttribute("aria-label")).toBe("Comment on new lines 1–2");
    fireEvent.click(plus);
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "Both of these?" } });
    fireEvent.click(screen.getByText("Comment"));

    expect(actions.add).toHaveBeenCalledWith("src/a.rs", "new", 1, 2, "Both of these?");
  });

  it("marks every line reached, the way a drag does", () => {
    // The reader has to see the passage grow, or they are choosing it blind.
    const { container } = render(
      <Diff
        diff={diff()}
        file={file()}
        view="unified"
        comments={[]}
        actions={noComments()}
      />,
    );

    stretch(screen.getByLabelText("Comment on new line 1"), "ArrowDown");

    expect([...container.querySelectorAll(".row")].map((r) => r.className.includes("picking"))).toEqual([
      true,
      true,
    ]);
  });

  it("stops at the end of the hunk rather than reaching for lines nobody printed", () => {
    // The box hangs off a row. Reached past what the hunk prints, the span
    // would cover lines that are not on screen and the box would have nowhere
    // to open.
    draw(diff(), "unified");
    const plus = screen.getByLabelText("Comment on new line 1");

    stretch(plus, "ArrowDown", 4);

    expect(plus.getAttribute("aria-label")).toBe("Comment on new lines 1–2");
  });

  it("reaches back up from the line that puzzled the reader", () => {
    draw(diff(), "unified");
    const plus = screen.getByLabelText("Comment on new line 2");

    stretch(plus, "ArrowUp");

    expect(plus.getAttribute("aria-label")).toBe("Comment on new lines 1–2");
  });

  it("keeps a reach on the old side counted on the old numbers", () => {
    // A file deleted whole: every line is on the old side and nowhere else.
    const actions = draw(deleted(), "unified");
    const plus = screen.getByLabelText("Comment on old line 1");

    stretch(plus, "ArrowDown");
    fireEvent.click(plus);
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "All of it?" } });
    fireEvent.click(screen.getByText("Comment"));

    expect(actions.add).toHaveBeenCalledWith("src/gone.rs", "old", 1, 2, "All of it?");
  });

  it("will not reach across into the other side, however far it is pushed", () => {
    // Two lines rewritten in place: 1 and 2 exist on both sides and mean two
    // different things. The `+` belongs to one column, and so does its span.
    const actions = draw(rewritten(), "unified");
    const plus = screen.getByLabelText("Comment on old line 1");

    stretch(plus, "ArrowDown", 4);

    expect(plus.getAttribute("aria-label")).toBe("Comment on old lines 1–2");
    fireEvent.click(plus);
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "Why both?" } });
    fireEvent.click(screen.getByText("Comment"));
    expect(actions.add).toHaveBeenCalledWith("src/a.rs", "old", 1, 2, "Why both?");
  });

  it("picks a passage in the left column of a split view too", () => {
    const actions = draw(rewritten(), "split");
    const plus = screen.getByLabelText("Comment on old line 1");

    stretch(plus, "ArrowDown");
    fireEvent.click(plus);
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "Why both?" } });
    fireEvent.click(screen.getByText("Comment"));

    expect(actions.add).toHaveBeenCalledWith("src/a.rs", "old", 1, 2, "Why both?");
  });

  it("gives the span up when the reader tabs off the plus", () => {
    const { container } = render(
      <Diff
        diff={diff()}
        file={file()}
        view="unified"
        comments={[]}
        actions={noComments()}
      />,
    );
    const plus = screen.getByLabelText("Comment on new line 1");
    stretch(plus, "ArrowDown");

    fireEvent.blur(plus);

    expect(container.querySelector(".picking")).toBeNull();
  });

  it("takes no notice of an arrow pressed without shift", () => {
    // Shift is what separates reaching for lines from every other arrow the
    // reader might press while the `+` happens to have focus.
    draw(diff(), "unified");
    const plus = screen.getByLabelText("Comment on new line 1");

    fireEvent.keyDown(plus, { key: "ArrowDown" });

    expect(plus.getAttribute("aria-label")).toBe("Comment on new line 1");
  });
});

describe("where the keyboard is left when a comment box closes", () => {
  /** The box, opened from the `+` with the keyboard on it — which is the only
   * way there is a control to hand focus back to. */
  function open() {
    tokenizer = null;
    const page = render(
      <Diff diff={diff()} file={file()} view="unified" comments={[]} actions={noComments()} />,
    );
    const plus = screen.getByLabelText("Comment on new line 2");
    plus.focus();
    fireEvent.click(plus);
    return { ...page, plus };
  }

  it("opens ready to type", () => {
    open();

    expect(document.activeElement).toBe(screen.getByRole("textbox"));
  });

  it("keeps its name once there is something written in it", () => {
    // A box with no label of its own is named by its placeholder, and only in
    // a browser generous enough to do it — which is a name that goes away at
    // the first keystroke. The line it is about is the name here, and the keys
    // are what the box is described by.
    open();
    const box = screen.getByRole("textbox");

    fireEvent.change(box, { target: { value: "Why this order?" } });

    expect(screen.getByLabelText("Comment on line 2")).toBe(box);
    const said = box.getAttribute("aria-describedby");
    expect(document.getElementById(said ?? "")?.textContent).toContain("⌘↵");
  });

  it("keeps that description inside the box and not off the foot of the page", () => {
    // `sr-only` is absolute with no offsets, so it lands at its static position
    // inside the nearest positioned ancestor — and with none, that is the page.
    // A box opened far down a scrolled pane put a one-pixel span hundreds of
    // pixels below the screen, and the page grew a scrollbar for it. jsdom does
    // no layout, so what is checked is the containing block that prevents it.
    const { container } = open();
    const box = screen.getByRole("textbox");
    const said = document.getElementById(box.getAttribute("aria-describedby") ?? "");

    const wrapper = container.querySelector(".commentbox");
    expect(wrapper?.className).toContain("relative");
    expect(wrapper?.contains(said)).toBe(true);
  });

  it("hands it back to the plus it was opened from when the box is cancelled", () => {
    // Dropped on BODY instead, the reader is back at the top of the review,
    // a page away from the line they had just read.
    const { plus } = open();

    fireEvent.click(screen.getByText("Cancel"));

    expect(document.activeElement).toBe(plus);
  });

  it("hands it back when the comment has been saved, too", async () => {
    const { plus } = open();
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "Why this order?" } });

    await act(async () => void fireEvent.click(screen.getByText("Comment")));

    expect(document.activeElement).toBe(plus);
  });
});
