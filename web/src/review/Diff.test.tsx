import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { Token, Tokenize } from "@/highlight/tokens";
import type { FileDiff, FileView } from "@/api";

/** The grammar is fetched over the network and this is about what reaches the
 * screen, so the tokenizer is stood in for. */
let tokenizer: Tokenize | null = null;
vi.mock("@/hooks/useHighlight", () => ({ useHighlight: () => tokenizer }));

const { Diff } = await import("./Diff");

function file(): FileView {
  return {
    path: "src/a.rs",
    status: "modified",
    additions: 1,
    deletions: 0,
    viewed: false,
    notes: [],
    lineNotes: [],
    tags: [],
    skim: false,
    skimReason: null,
  };
}

function diff(): FileDiff {
  return {
    path: "src/a.rs",
    status: "modified",
    binary: false,
    additions: 1,
    deletions: 0,
    hunks: [
      {
        old_start: 1,
        old_lines: 1,
        new_start: 1,
        new_lines: 2,
        lines: [
          { kind: "context", old_number: 1, new_number: 1, content: "fn main() {}" },
          { kind: "added", old_number: null, new_number: 2, content: "// nota" },
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
      code
        .split("\n")
        .map((line) => [
          { content: line, style: { "--shiki-light": "#7d4fae", "--shiki-dark": "#c39ae8" } },
        ]);

    const { container } = render(<Diff diff={diff()} file={file()} />);

    const tokens = container.querySelectorAll(".tok");
    expect(tokens).toHaveLength(2);
    expect(tokens[0].getAttribute("style")).toContain("--shiki-light: #7d4fae");
    expect(tokens[0].getAttribute("style")).toContain("--shiki-dark: #c39ae8");
  });

  it("draws the code plain while the grammar has not arrived", () => {
    // Every file starts here, and a file whose language farol cannot name
    // stays here. Neither is an error on screen.
    tokenizer = null;

    const { container } = render(<Diff diff={diff()} file={file()} />);

    expect(container.textContent).toContain("fn main() {}");
    expect(container.querySelectorAll(".tok")).toHaveLength(0);
  });
});
