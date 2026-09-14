import { fireEvent, render, screen } from "@testing-library/react";
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

  it("leaves the comments of other files alone", () => {
    // Every comment in the review is handed down; the diff shows its own.
    const { container } = draw([
      comment({ path: "src/elsewhere.rs", from: 1, to: 1 }),
    ]);

    expect(container.querySelector(".comment")).toBeNull();
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
