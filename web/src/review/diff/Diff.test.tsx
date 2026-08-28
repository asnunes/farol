import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { comment, file, noComments } from "./testing";
import type { Token, Tokenize } from "@/highlight/tokens";
import type { FileDiff } from "@/api";

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

    fireEvent.click(screen.getByLabelText("Comment on line 2"));

    expect(laidOut(container)).toEqual(["row", "row", "commentbox"]);
  });

  it("sends what was written, against the lines it was written about", () => {
    const actions = noComments();
    draw([], actions);
    fireEvent.click(screen.getByLabelText("Comment on line 2"));

    fireEvent.change(screen.getByRole("textbox"), {
      target: { value: "Why this order?" },
    });
    fireEvent.click(screen.getByText("Comment"));

    expect(actions.add).toHaveBeenCalledWith(
      "src/a.rs",
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
    fireEvent.click(screen.getByLabelText("Comment on line 2"));

    fireEvent.change(screen.getByRole("textbox"), { target: { value: "   " } });
    fireEvent.click(screen.getByText("Comment"));

    expect(actions.add).not.toHaveBeenCalled();
  });

  it("offers no way in on a line that is not in the file any more", () => {
    // A comment hangs off the code as it now reads. A removed line is not
    // there to hang one on.
    tokenizer = null;
    const withRemoval = diff();
    withRemoval.hunks[0].lines.unshift({
      kind: "removed",
      oldNumber: 1,
      newNumber: null,
      content: "fn main() { }",
    });

    render(
      <Diff diff={withRemoval} file={file()} view="unified" comments={[]} actions={noComments()} />,
    );

    expect(screen.queryAllByLabelText(/^Comment on line/)).toHaveLength(2);
    expect(screen.queryByLabelText("Comment on line 1")).toBeTruthy();
    expect(screen.queryByLabelText("Comment on line 2")).toBeTruthy();
  });
});
