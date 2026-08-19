import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { noComments } from "./testing";
import type { CommentView, FileDiff, FileView } from "@/api";

vi.mock("@/hooks/useHighlight", () => ({ useHighlight: () => null }));

/** What the server hands back for a range: the lines, saying which they are. */
const lines = vi.fn(async (_path: string, from: number, to: number) =>
  Array.from({ length: to - from + 1 }, (_, i) => `line ${from + i}`),
);
vi.mock("@/api", async (real) => ({ ...(await real<object>()), api: { lines: (...a: [string, number, number]) => lines(...a) } }));

const { Diff } = await import("./Diff");

beforeEach(() => lines.mockClear());

describe("opening what the diff did not print", () => {
  it("offers three ways into a long gap and one into a short one", () => {
    // Three buttons that all do the same thing is three ways to wonder which
    // one is different. The file opens with seven lines above the first hunk
    // and forty-nine between the two.
    const { container } = render(<Diff {...props(twoHunks(100))} />);

    expect(within(gapAt(container, 0)).getAllByRole("button")).toHaveLength(1);
    expect(within(gapAt(container, 1)).getAllByRole("button")).toHaveLength(3);
  });

  it("opens twenty lines under the hunk above, and asks for exactly those", async () => {
    const { container } = render(<Diff {...props(twoHunks(100))} />);

    pull(container, "Open the lines under the hunk above");

    await waitFor(() => expect(lines).toHaveBeenCalledWith("src/a.rs", 11, 30));
    expect(await screen.findByText("line 11")).toBeTruthy();
  });

  it("opens the last twenty of the gap when pulled from below", async () => {
    const { container } = render(<Diff {...props(twoHunks(100))} />);

    pull(container, "Open the lines over the hunk below");

    await waitFor(() => expect(lines).toHaveBeenCalledWith("src/a.rs", 40, 59));
  });

  it("numbers an opened line on both sides", async () => {
    // The old side runs behind by whatever the hunk above added, and a reader
    // comparing against another checkout needs the number that side uses.
    const { container } = render(<Diff {...props(twoHunks(100), "split")} />);

    pull(container, "Open the lines under the hunk above");

    // Twice over, because a context line is the same on both sides. Which is
    // the point of the test: the two numbers beside it are not.
    await waitFor(() => expect(screen.getAllByText("line 11")).toHaveLength(2));
    const row = [...container.querySelectorAll(".split-row")].find((r) =>
      r.textContent?.includes("line 11"),
    );
    const numbers = [...row!.querySelectorAll(".ln")].map((n) => n.textContent);
    // Two lines were added by the hunk above, so line 11 was line 9 before.
    expect(numbers).toEqual(["9", "11"]);
  });

  it("gives an opened line no way to be commented on", async () => {
    // A comment can only sit where the diff reaches, and these lines are the
    // definition of where it does not.
    const { container } = render(<Diff {...props(twoHunks(100))} />);

    pull(container, "Open the lines under the hunk above");
    await screen.findByText("line 11");

    const row = [...container.querySelectorAll(".diff-row")].find((r) =>
      r.textContent?.includes("line 11"),
    );
    expect(screen.queryByLabelText("Comment on line 11")).toBeNull();
    expect(row!.querySelector(".ln")!.className).not.toContain("cursor-pointer");
  });

  it("shows a line note that was written where the diff never reached", async () => {
    // Written, counted in the sidebar, and invisible until the region opens.
    const hidden = file();
    hidden.lineNotes = [{ block: "core", from: 11, to: 12, text: "vale ler junto" }];

    const { container } = render(<Diff {...props(twoHunks(100), "unified", hidden)} />);
    expect(screen.queryByText("vale ler junto")).toBeNull();

    pull(container, "Open the lines under the hunk above");

    expect(await screen.findByText("vale ler junto")).toBeTruthy();
  });

  it("takes the whole gap in one press, and the band goes with it", async () => {
    const { container } = render(<Diff {...props(twoHunks(100))} />);

    fireEvent.click(screen.getByLabelText("Open all 49 lines hidden here"));

    await screen.findByText("line 11");
    // The leading gap and the trailing one are still there; the one that was
    // opened is not.
    expect(container.querySelectorAll(".gap")).toHaveLength(2);
  });
});

/** Press a control in the gap between the two hunks, which is the one the
 * tests are about. The same labels sit in the gap above the first hunk and the
 * one after the last. */
function pull(container: HTMLElement, label: string) {
  fireEvent.click(within(gapAt(container, 1)).getByLabelText(label));
}

/** The gaps in reading order: above the first hunk, between the two, after the
 * last. */
function gapAt(container: HTMLElement, i: number): HTMLElement {
  return container.querySelectorAll<HTMLElement>(".gap")[i];
}

function props(diff: FileDiff, view: "unified" | "split" = "unified", of = file()) {
  return { diff, file: of, view, comments: [] as CommentView[], actions: noComments() };
}

/** A file with a hunk at the top, a hunk in the middle and a tail after it. */
function twoHunks(count: number, secondAt = 60): FileDiff {
  return {
    path: "src/a.rs",
    status: "modified",
    binary: false,
    additions: 2,
    deletions: 0,
    line_count: count,
    hunks: [
      {
        old_start: 8,
        old_lines: 1,
        new_start: 8,
        new_lines: 3,
        lines: [
          { kind: "context", old_number: 8, new_number: 8, content: "fn main() {}" },
          { kind: "added", old_number: null, new_number: 9, content: "// um" },
          { kind: "added", old_number: null, new_number: 10, content: "// dois" },
        ],
      },
      {
        old_start: secondAt - 2,
        old_lines: 2,
        new_start: secondAt,
        new_lines: 2,
        lines: [
          { kind: "context", old_number: secondAt - 2, new_number: secondAt, content: "fim" },
          { kind: "context", old_number: secondAt - 1, new_number: secondAt + 1, content: "}" },
        ],
      },
    ],
  };
}

function file(): FileView {
  return {
    path: "src/a.rs",
    status: "modified",
    additions: 2,
    deletions: 0,
    viewed: false,
    notes: [],
    lineNotes: [],
    tags: [],
    skim: false,
    skimReason: null,
  };
}
