import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { LineComments, quoted } from "./LineComments";
import { comment, noComments } from "./testing";

describe("a comment on the page", () => {
  it("renders the markdown it was written in", () => {
    // The body is stored as it was typed; the screen is where it becomes
    // readable. `**` on screen would mean the file is the only place it reads
    // properly.
    const { container } = render(
      <LineComments
        comments={[comment({ body: "Not **here** — see `foo()`." })]}
        actions={noComments()}
      />,
    );

    expect(container.querySelector("strong")?.textContent).toBe("here");
    expect(container.querySelector("code")?.textContent).toBe("foo()");
  });

  it("says which lines it is about", () => {
    render(<LineComments comments={[comment({ from: 82, to: 116 })]} actions={noComments()} />);

    expect(screen.getByText("82–116")).toBeTruthy();
  });

  it("asks before closing, because closing removes it for good", () => {
    // The file is deleted and it was never in git. One press arms, the second
    // one does it — and the button that copies is a few pixels away.
    const actions = noComments();
    render(<LineComments comments={[comment()]} actions={actions} />);

    fireEvent.click(screen.getByLabelText("Close this comment"));

    expect(actions.close).not.toHaveBeenCalled();
    expect(screen.getByLabelText("Press again to close this comment")).toBeTruthy();
  });

  it("closes on the second press", () => {
    const actions = noComments();
    render(<LineComments comments={[comment()]} actions={actions} />);

    fireEvent.click(screen.getByLabelText("Close this comment"));
    fireEvent.click(screen.getByLabelText("Press again to close this comment"));

    expect(actions.close).toHaveBeenCalledWith("18cb-3731");
  });

  it("copies where it was and what it said, in one paste", () => {
    // Pasted into a chat or an issue it has to stand on its own — the same
    // shape `farol comment` writes on the terminal.
    expect(quoted(comment({ from: 82, to: 116 }))).toBe("src/a.rs:82-116\n\nWhy this order?");
    expect(quoted(comment({ from: 9, to: 9 }))).toBe("src/a.rs:9\n\nWhy this order?");
  });

  it("puts it on the clipboard when the button is pressed", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", { clipboard: { writeText } });
    render(<LineComments comments={[comment({ from: 82, to: 116 })]} actions={noComments()} />);

    fireEvent.click(screen.getByLabelText("Copy this comment"));

    expect(writeText).toHaveBeenCalledWith("src/a.rs:82-116\n\nWhy this order?");
    vi.unstubAllGlobals();
  });
});
