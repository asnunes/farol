import { render, screen } from "@testing-library/react";
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

  it("keeps a closed comment on the page and says it is closed", () => {
    // Closing is answering, not withdrawing. Hiding it would take away the
    // record of why the code looks the way it does.
    const { container } = render(
      <LineComments comments={[comment({ resolved: true })]} actions={noComments()} />,
    );

    expect(container.textContent).toContain("Why this order?");
    expect(container.textContent).toContain("closed");
    expect(container.querySelector(".comment")?.className).toContain("resolved");
  });

  it("offers to reopen a closed one, and to close an open one", () => {
    const actions = noComments();
    render(<LineComments comments={[comment()]} actions={actions} />);

    screen.getByLabelText("Close this comment").click();

    expect(actions.resolve).toHaveBeenCalledWith("18cb-3731", true);
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

    screen.getByLabelText("Copy this comment").click();

    expect(writeText).toHaveBeenCalledWith("src/a.rs:82-116\n\nWhy this order?");
    vi.unstubAllGlobals();
  });
});
