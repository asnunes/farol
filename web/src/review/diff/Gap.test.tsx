import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { file, noComments } from "./testing";
import type { CommentView, FileDiff } from "@/api";

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

  it("leaves out the direction with no hunk to walk from", () => {
    // The gap after the last hunk has no hunk below it, so there is nothing for
    // an up arrow to walk up from. Only the way down out of the hunk above.
    const { container } = render(<Diff {...props(twoHunks(100))} />);
    const tail = gapAt(container, 2);

    expect(
      within(tail).getByLabelText("Open lines 62–81, down from the hunk above"),
    ).toBeTruthy();
    expect(
      within(tail).queryByLabelText("Open lines 81–100, up from the hunk below"),
    ).toBeNull();
  });

  it("walks down out of the hunk above, taking the lines it runs into", async () => {
    // The down chevron names the hunk it leaves, not the side of the band the
    // code lands on: those lines are drawn between that hunk and the band,
    // which is above the band, whichever way the arrow points.
    const { container } = render(<Diff {...props(twoHunks(100))} />);

    pull(container, "Open lines 11–30, down from the hunk above");

    await waitFor(() => expect(lines).toHaveBeenCalledWith("src/a.rs", 11, 30));
    expect(await screen.findByText("line 11")).toBeTruthy();
  });

  it("walks up into the hunk below, taking the lines that run into it", async () => {
    const { container } = render(<Diff {...props(twoHunks(100))} />);

    pull(container, "Open lines 40–59, up from the hunk below");

    await waitFor(() => expect(lines).toHaveBeenCalledWith("src/a.rs", 40, 59));
  });

  it("draws each step against the hunk its arrow walked from", async () => {
    // The whole meaning of the two chevrons. What the down arrow takes is drawn
    // against the hunk above, and what the up arrow takes against the hunk
    // below, with what is still closed between them. The band sits at the far
    // end of the stretch rather than in the middle of it, so which side of the
    // band the code lands on says nothing and is not what is asserted.
    const { container } = render(<Diff {...props(twoHunks(100))} />);

    pull(container, "Open lines 11–30, down from the hunk above");
    await screen.findByText("line 11");
    expect(side("line 11", gapAt(container, 1))).toBe("above");

    pull(container, "Open lines 40–59, up from the hunk below");
    await screen.findByText("line 40");
    expect(side("line 40", gapAt(container, 1))).toBe("below");
    expect(side("line 11", gapAt(container, 1))).toBe("above");
  });

  it("keeps offering the same way in, so a long gap is walked a step at a time", async () => {
    // Twenty lines is a step, not a limit. Pressing down again takes the next
    // twenty, and the arrow has to survive the first press for that to be how
    // anyone reads a long stretch.
    const { container } = render(<Diff {...props(twoHunks(100))} />);

    pull(container, "Open lines 11–30, down from the hunk above");
    await screen.findByText("line 11");

    pull(container, "Open lines 31–50, down from the hunk above");

    await waitFor(() => expect(lines).toHaveBeenCalledWith("src/a.rs", 31, 50));
  });

  it("numbers an opened line on both sides", async () => {
    // The old side runs behind by whatever the hunk above added, and a reader
    // comparing against another checkout needs the number that side uses.
    const { container } = render(<Diff {...props(twoHunks(100), "split")} />);

    pull(container, "Open lines 11–30, down from the hunk above");

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

    pull(container, "Open lines 11–30, down from the hunk above");
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

    pull(container, "Open lines 11–30, down from the hunk above");

    expect(await screen.findByText("vale ler junto")).toBeTruthy();
  });

  it("drops the band once the code above runs into the hunk", async () => {
    // The band announces that the file jumps here. Opened, it does not jump,
    // and announcing a jump that is not there sends the reader looking for a
    // discontinuity nobody made.
    const { container } = render(<Diff {...props(twoHunks(100))} />);
    expect(container.textContent).toContain("@@ -58,2 +60,2 @@");

    fireEvent.click(screen.getByLabelText("Open all 49 lines hidden here"));

    await screen.findByText("line 11");
    expect(container.textContent).not.toContain("@@ -58,2 +60,2 @@");
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

/** Which side of the band a line came out on. */
function side(line: string, band: HTMLElement): "above" | "below" {
  const where = band.compareDocumentPosition(screen.getByText(line));
  return where & Node.DOCUMENT_POSITION_FOLLOWING ? "below" : "above";
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
    lineCount: count,
    hunks: [
      {
        oldStart: 8,
        oldLines: 1,
        newStart: 8,
        newLines: 3,
        lines: [
          { kind: "context", oldNumber: 8, newNumber: 8, content: "fn main() {}" },
          { kind: "added", oldNumber: null, newNumber: 9, content: "// um" },
          { kind: "added", oldNumber: null, newNumber: 10, content: "// dois" },
        ],
      },
      {
        oldStart: secondAt - 2,
        oldLines: 2,
        newStart: secondAt,
        newLines: 2,
        lines: [
          { kind: "context", oldNumber: secondAt - 2, newNumber: secondAt, content: "fim" },
          { kind: "context", oldNumber: secondAt - 1, newNumber: secondAt + 1, content: "}" },
        ],
      },
    ],
  };
}

