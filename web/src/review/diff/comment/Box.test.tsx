import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Box } from "./Box";

const span = { side: "new" as const, from: 9, to: 9 };

describe("the box a comment is written in", () => {
  it("offers only Comment when no agent session is connected", () => {
    render(<Box span={span} onSave={vi.fn()} onCancel={vi.fn()} agent={null} onSaveAndSend={vi.fn()} />);

    expect(screen.queryByRole("button", { name: "Comment & send" })).toBeNull();
  });

  it("saves and sends in one press when a session can take it", () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    const onSaveAndSend = vi.fn().mockResolvedValue(undefined);
    render(<Box span={span} onSave={onSave} onCancel={vi.fn()} agent="Claude Code" onSaveAndSend={onSaveAndSend} />);

    fireEvent.change(screen.getByRole("textbox"), { target: { value: "Why this order?" } });
    fireEvent.click(screen.getByRole("button", { name: "Comment & send" }));

    expect(onSaveAndSend).toHaveBeenCalledWith("Why this order?");
    expect(onSave).not.toHaveBeenCalled();
  });

  it("sends with shift held on the save shortcut, and only saves without it", () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    const onSaveAndSend = vi.fn().mockResolvedValue(undefined);
    render(<Box span={span} onSave={onSave} onCancel={vi.fn()} agent="Codex" onSaveAndSend={onSaveAndSend} />);
    const box = screen.getByRole("textbox");
    fireEvent.change(box, { target: { value: "Why?" } });

    fireEvent.keyDown(box, { key: "Enter", metaKey: true, shiftKey: true });

    expect(onSaveAndSend).toHaveBeenCalledWith("Why?");
    expect(onSave).not.toHaveBeenCalled();
  });
});
