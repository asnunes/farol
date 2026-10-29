import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Box } from "./Box";
import { threadOn } from "./quoted";

const span = { side: "new" as const, from: 9, to: 9 };

describe("the box a comment is written in", () => {
  it("offers only Comment when no agent session is connected", () => {
    render(<Box label={threadOn(span)} onSave={vi.fn()} onCancel={vi.fn()} agent={null} onSaveAndSend={vi.fn()} />);

    expect(screen.queryByRole("button", { name: "Ask Claude Code" })).toBeNull();
  });

  it("saves and sends in one press when a session can take it", () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    const onSaveAndSend = vi.fn().mockResolvedValue(undefined);
    render(<Box label={threadOn(span)} onSave={onSave} onCancel={vi.fn()} agent="Claude Code" onSaveAndSend={onSaveAndSend} />);

    fireEvent.change(screen.getByRole("textbox"), { target: { value: "Why this order?" } });
    fireEvent.click(screen.getByRole("button", { name: "Ask Claude Code" }));

    expect(onSaveAndSend).toHaveBeenCalledWith("Why this order?");
    expect(onSave).not.toHaveBeenCalled();
  });

  it("puts Ask first when a session is connected", () => {
    render(<Box label={threadOn(span)} onSave={vi.fn()} onCancel={vi.fn()} agent="Claude Code" onSaveAndSend={vi.fn()} />);

    const buttons = screen.getAllByRole("button").map((b) => b.textContent);
    expect(buttons.slice(0, 2)).toEqual(["Ask Claude Code", "Comment"]);
  });

  it("hands focus back without moving the page", () => {
    const opener = document.createElement("button");
    document.body.appendChild(opener);
    opener.focus();
    const focus = vi.spyOn(opener, "focus");
    const { unmount } = render(<Box label={threadOn(span)} onSave={vi.fn()} onCancel={vi.fn()} agent={null} onSaveAndSend={vi.fn()} />);

    unmount();

    expect(focus).toHaveBeenCalledWith({ preventScroll: true });
    opener.remove();
  });

  it("sends on the plain save shortcut when a session is connected, like the highlighted button", () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    const onSaveAndSend = vi.fn().mockResolvedValue(undefined);
    render(<Box label={threadOn(span)} onSave={onSave} onCancel={vi.fn()} agent="Codex" onSaveAndSend={onSaveAndSend} />);
    const box = screen.getByRole("textbox");
    fireEvent.change(box, { target: { value: "Why?" } });

    fireEvent.keyDown(box, { key: "Enter", metaKey: true });

    expect(onSaveAndSend).toHaveBeenCalledWith("Why?");
    expect(onSave).not.toHaveBeenCalled();
  });

  it("only saves with shift held", () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    const onSaveAndSend = vi.fn().mockResolvedValue(undefined);
    render(<Box label={threadOn(span)} onSave={onSave} onCancel={vi.fn()} agent="Codex" onSaveAndSend={onSaveAndSend} />);
    const box = screen.getByRole("textbox");
    fireEvent.change(box, { target: { value: "Why?" } });

    fireEvent.keyDown(box, { key: "Enter", metaKey: true, shiftKey: true });

    expect(onSave).toHaveBeenCalledWith("Why?");
    expect(onSaveAndSend).not.toHaveBeenCalled();
  });

  it("only saves on the plain shortcut when no session is connected", () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    render(<Box label={threadOn(span)} onSave={onSave} onCancel={vi.fn()} agent={null} onSaveAndSend={vi.fn()} />);
    const box = screen.getByRole("textbox");
    fireEvent.change(box, { target: { value: "Why?" } });

    fireEvent.keyDown(box, { key: "Enter", metaKey: true });

    expect(onSave).toHaveBeenCalledWith("Why?");
  });

  it("offers the thread as a pending comment only when asked to, and on the shifted key", () => {
    const onAddAsPending = vi.fn().mockResolvedValue(undefined);
    const { unmount } = render(
      <Box label={threadOn(span)} onSave={vi.fn()} onCancel={vi.fn()} agent="Codex" onSaveAndSend={vi.fn()} />,
    );
    expect(screen.queryByRole("button", { name: "Add as pending comment" })).toBeNull();
    unmount();

    render(
      <Box
        label={threadOn(span)}
        onSave={vi.fn()}
        onCancel={vi.fn()}
        agent="Codex"
        onSaveAndSend={vi.fn()}
        onAddAsPending={onAddAsPending}
      />,
    );
    const box = screen.getByRole("textbox");
    fireEvent.change(box, { target: { value: "Rename it." } });
    fireEvent.keyDown(box, { key: "Enter", metaKey: true, shiftKey: true });

    expect(onAddAsPending).toHaveBeenCalledWith("Rename it.");
  });
});
