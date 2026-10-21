import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Comment } from "./Comment";
import { quoted } from "./quoted";
import { comment, noComments } from "../testing";

describe("a comment on the page", () => {
  it("renders the markdown it was written in", () => {
    // The body is stored as it was typed; the screen is where it becomes
    // readable. `**` on screen would mean the file is the only place it reads
    // properly.
    const { container } = render(
      <Comment comment={comment({ body: "Not **here** — see `foo()`." })} actions={noComments()} />,
    );

    expect(container.querySelector("strong")?.textContent).toBe("here");
    expect(container.querySelector("code")?.textContent).toBe("foo()");
  });

  it("says which lines it is about", () => {
    render(<Comment comment={comment({ from: 82, to: 116 })} actions={noComments()} />);

    expect(screen.getByText("82–116")).toBeTruthy();
  });

  it("asks before closing, because closing removes it for good", () => {
    // The file is deleted and it was never in git. One press arms, the second
    // one does it — and the button that copies is a few pixels away.
    const actions = noComments();
    render(<Comment comment={comment()} actions={actions} />);

    fireEvent.click(screen.getByLabelText("Close this comment"));

    expect(actions.close).not.toHaveBeenCalled();
    expect(screen.getByLabelText("Press again to close this comment")).toBeTruthy();
  });

  it("closes on the second press", () => {
    const actions = noComments();
    render(<Comment comment={comment()} actions={actions} />);

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

  it("offers no Send when no agent session is connected", () => {
    render(<Comment comment={comment()} actions={noComments()} />);

    expect(screen.queryByLabelText(/^Send to/)).toBeNull();
    expect(screen.getByLabelText("Copy this comment")).toBeTruthy();
  });

  it("names the harness on Send and hands over only the comment's id", async () => {
    const actions = { ...noComments(), agent: "Claude Code" };
    render(<Comment comment={comment()} actions={actions} />);

    fireEvent.click(screen.getByLabelText("Send to Claude Code"));

    expect(actions.send).toHaveBeenCalledWith("18cb-3731");
    expect(await screen.findByLabelText("Sent to Claude Code")).toBeTruthy();
  });

  it("shows no tick when the harness did not take it", async () => {
    const actions = { ...noComments(), agent: "Codex", send: vi.fn().mockResolvedValue(false) };
    render(<Comment comment={comment()} actions={actions} />);

    fireEvent.click(screen.getByLabelText("Send to Codex"));

    await waitFor(() => expect(actions.send).toHaveBeenCalled());
    expect(screen.queryByLabelText("Sent to Codex")).toBeNull();
  });

  it("shows the conversation under the question, signed by who wrote each turn", () => {
    render(
      <Comment
        comment={comment({
          replies: [
            { from: "agent", name: "Claude Code", body: "Because the **legacy** format comes first." },
            { from: "reviewer", name: "You", body: "And the new one?" },
          ],
        })}
        actions={noComments()}
      />,
    );

    expect(screen.getByText("Claude Code")).toBeTruthy();
    expect(screen.getByText("legacy").tagName).toBe("STRONG");
    expect(screen.getByText("And the new one?")).toBeTruthy();
  });

  it("says what the agent owes: sent, then working once it acknowledges", () => {
    const { rerender } = render(
      <Comment comment={comment({ waiting: { on: "Codex", stage: "sent" } })} actions={noComments()} />,
    );
    expect(screen.getByRole("status").textContent).toContain("Sent to Codex");

    rerender(<Comment comment={comment({ waiting: { on: "Codex", stage: "working" } })} actions={noComments()} />);
    expect(screen.getByRole("status").textContent).toContain("Codex is working on it");
  });

  it("offers Reply after the last turn, not among the actions on the question", () => {
    const { container } = render(
      <Comment
        comment={comment({ replies: [{ from: "agent", name: "Claude Code", body: "Because." }] })}
        actions={noComments()}
      />,
    );

    const reply = screen.getByRole("button", { name: "Reply" });
    expect(container.querySelector(".acts")?.contains(reply)).toBe(false);
    expect(container.querySelector(".thread")?.lastElementChild).toBe(reply);
  });

  it("holds Send while the agent owes an answer and once it has given one", () => {
    const actions = { ...noComments(), agent: "Claude Code" };
    const { rerender } = render(
      <Comment comment={comment({ waiting: { on: "Claude Code", stage: "working" } })} actions={actions} />,
    );
    const send = () => screen.getByLabelText("Send to Claude Code") as HTMLButtonElement;
    expect(send().disabled).toBe(true);

    rerender(
      <Comment
        comment={comment({ replies: [{ from: "agent", name: "Claude Code", body: "Because." }] })}
        actions={actions}
      />,
    );
    expect(send().disabled).toBe(true);
    expect(send().title).toContain("reply to continue");

    rerender(
      <Comment
        comment={comment({
          replies: [
            { from: "agent", name: "Claude Code", body: "Because." },
            { from: "reviewer", name: "You", body: "And this?" },
          ],
        })}
        actions={actions}
      />,
    );
    expect(send().disabled).toBe(false);
  });

  it("replies and sends the conversation on in one press", async () => {
    const actions = { ...noComments(), agent: "Claude Code" };
    render(<Comment comment={comment()} actions={actions} />);

    fireEvent.click(screen.getByRole("button", { name: "Reply" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Reply to this comment" }), {
      target: { value: "And the new one?" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Reply & send" }));

    await waitFor(() => expect(actions.reply).toHaveBeenCalledWith("18cb-3731", "And the new one?", true));
  });

  it("only replies when no session is connected", async () => {
    const actions = noComments();
    render(<Comment comment={comment()} actions={actions} />);

    fireEvent.click(screen.getByRole("button", { name: "Reply" }));
    expect(screen.queryByRole("button", { name: "Reply & send" })).toBeNull();
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "Note to self." } });
    fireEvent.click(screen.getByRole("button", { name: "Reply" }));

    await waitFor(() => expect(actions.reply).toHaveBeenCalledWith("18cb-3731", "Note to self.", false));
  });

  it("puts it on the clipboard when the button is pressed", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", { clipboard: { writeText } });
    render(<Comment comment={comment({ from: 82, to: 116 })} actions={noComments()} />);

    fireEvent.click(screen.getByLabelText("Copy this comment"));

    expect(writeText).toHaveBeenCalledWith("src/a.rs:82-116\n\nWhy this order?");
    vi.unstubAllGlobals();
  });
});
