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

    fireEvent.click(screen.getByLabelText("Close this thread"));

    expect(actions.close).not.toHaveBeenCalled();
    expect(screen.getByLabelText("Press again to close this thread")).toBeTruthy();
  });

  it("closes on the second press", () => {
    const actions = noComments();
    render(<Comment comment={comment()} actions={actions} />);

    fireEvent.click(screen.getByLabelText("Close this thread"));
    fireEvent.click(screen.getByLabelText("Press again to close this thread"));

    expect(actions.close).toHaveBeenCalledWith("18cb-3731", false);
  });

  it("copies where it was and what it said, in one paste", () => {
    // Pasted into a chat or an issue it has to stand on its own — the same
    // shape `farol thread` writes on the terminal.
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
      <Comment comment={comment({ waiting: { on: "Codex", stage: "sent", purpose: "answer" } })} actions={noComments()} />,
    );
    expect(screen.getByRole("status").textContent).toContain("Sent to Codex");

    rerender(<Comment comment={comment({ waiting: { on: "Codex", stage: "working", purpose: "answer" } })} actions={noComments()} />);
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
    expect(container.querySelector(".thread")?.lastElementChild?.contains(reply)).toBe(true);
  });

  it("says a reply was not sent, and sends it from there", async () => {
    const actions = { ...noComments(), agent: "Codex" };
    render(
      <Comment
        comment={comment({
          replies: [
            { from: "agent", name: "Codex", body: "Because." },
            { from: "reviewer", name: "You", body: "Why not an env key?" },
          ],
        })}
        actions={actions}
      />,
    );

    expect(screen.getByRole("status").textContent).toContain("not sent to Codex");
    fireEvent.click(screen.getByRole("button", { name: "Send it now" }));
    await waitFor(() => expect(actions.send).toHaveBeenCalledWith("18cb-3731"));
  });

  it("says a comment saved without sending was not sent", () => {
    render(<Comment comment={comment()} actions={{ ...noComments(), agent: "Codex" }} />);

    expect(screen.getByRole("status").textContent).toContain("Not sent to Codex");
  });

  it("says nothing about sending when no session is connected or the agent has answered", () => {
    const { rerender } = render(<Comment comment={comment()} actions={noComments()} />);
    expect(screen.queryByRole("status")).toBeNull();

    rerender(
      <Comment
        comment={comment({ replies: [{ from: "agent", name: "Codex", body: "Because." }] })}
        actions={{ ...noComments(), agent: "Codex" }}
      />,
    );
    expect(screen.queryByRole("status")).toBeNull();
  });

  it("offers no Reply while the agent owes an answer", () => {
    render(
      <Comment comment={comment({ waiting: { on: "Codex", stage: "sent", purpose: "answer" } })} actions={noComments()} />,
    );

    expect(screen.queryByRole("button", { name: "Reply" })).toBeNull();
  });

  it("holds Send while the agent owes an answer and once it has given one", () => {
    const actions = { ...noComments(), agent: "Claude Code" };
    const { rerender } = render(
      <Comment comment={comment({ waiting: { on: "Claude Code", stage: "working", purpose: "answer" } })} actions={actions} />,
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

  describe("in a peer review", () => {
    const peer = () => ({ ...noComments(), agent: "Codex", pendingOffered: true });

    it("offers a pending comment from the thread, the turns and the question", () => {
      render(
        <Comment
          comment={comment({ replies: [{ from: "agent", name: "Codex", body: "Rename it." }] })}
          actions={peer()}
        />,
      );

      expect(screen.getByRole("button", { name: "Generate pending comment" })).toBeTruthy();
      expect(screen.getAllByRole("button", { name: "Use as pending comment" })).toHaveLength(2);
    });

    it("takes a turn of the conversation as the pending comment", () => {
      const actions = peer();
      render(
        <Comment
          comment={comment({ replies: [{ from: "agent", name: "Codex", body: "Rename it." }] })}
          actions={actions}
        />,
      );

      fireEvent.click(screen.getAllByRole("button", { name: "Use as pending comment" })[1]);

      expect(actions.writePending).toHaveBeenCalledWith("18cb-3731", "Rename it.");
    });

    it("shows the pending comment as GitHub has it, and closing takes it along", () => {
      const actions = {
        ...peer(),
        pendingOf: () => ({ state: "pending" as const, body: "Please rename it.", url: null }),
      };
      render(<Comment comment={comment({ pending: { by: "agent", name: "Codex" } })} actions={actions} />);

      expect(screen.getByText("Please rename it.")).toBeTruthy();
      expect(screen.getByText("written by Codex")).toBeTruthy();
      expect(screen.getByRole("button", { name: "Regenerate" })).toBeTruthy();
      expect(screen.queryByRole("button", { name: "Generate pending comment" })).toBeNull();
      expect(screen.queryByRole("button", { name: "Use as pending comment" })).toBeNull();

      fireEvent.click(screen.getByLabelText("Close this thread"));
      fireEvent.click(screen.getByLabelText("Press again to close this thread"));
      expect(actions.close).toHaveBeenCalledWith("18cb-3731", true);
    });

    it("offers nothing to change on a comment that went with the review", () => {
      const actions = {
        ...peer(),
        pendingOf: () => ({ state: "published" as const, body: "Done.", url: "https://x/1" }),
      };
      const { container } = render(
        <Comment comment={comment({ pending: { by: "reviewer", name: "you" } })} actions={actions} />,
      );

      expect(container.querySelector(".pending")?.textContent).toContain("Published");
      expect(screen.queryByRole("button", { name: "Edit" })).toBeNull();
      expect(screen.queryByRole("button", { name: "Remove" })).toBeNull();
    });

    it("says what the agent is writing while it owes a pending comment", () => {
      render(
        <Comment
          comment={comment({ waiting: { on: "Codex", stage: "working", purpose: "pending" } })}
          actions={peer()}
        />,
      );

      expect(screen.getByText("Codex is writing a pending comment")).toBeTruthy();
      expect(screen.queryByRole("button", { name: "Generate pending comment" })).toBeNull();
    });

    it("asks the agent about a pending comment the reviewer wrote, instead of regenerating it", () => {
      const actions = {
        ...peer(),
        pendingOf: () => ({ state: "pending" as const, body: "Rename it.", url: null }),
      };
      render(<Comment comment={comment({ pending: { by: "reviewer", name: "you" } })} actions={actions} />);

      expect(screen.queryByRole("button", { name: "Regenerate" })).toBeNull();
      fireEvent.click(screen.getByRole("button", { name: "Ask Codex about it" }));
      expect(actions.send).toHaveBeenCalledWith("18cb-3731");
    });

    it("says where the thread is on its way to the pull request", () => {
      const { container } = render(<Comment comment={comment()} actions={peer()} />);

      expect(container.querySelector(".stage")?.textContent).toBe("Thread · Pending · Published");
    });
  });
});
