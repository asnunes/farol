import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { Activity } from "./Activity";
import { comment } from "@/review/diff/testing";
import type { Activity as ActivityState } from "@/hooks/useActivity";

describe("the activity box", () => {
  it("says on the launcher what is new and what is under way", () => {
    render(<Activity activity={state()} open={false} onOpen={vi.fn()} onJump={vi.fn()} />);

    const launcher = screen.getByRole("button", { name: /new/ });
    expect(launcher.textContent).toContain("1 new");
    expect(launcher.textContent).toContain("1 working");
  });

  it("takes the reader to the thread and marks the answer read", () => {
    const activity = state();
    const onJump = vi.fn();
    render(<Activity activity={activity} open onOpen={vi.fn()} onJump={onJump} />);

    fireEvent.click(screen.getByText("Claude Code answered"));

    expect(onJump).toHaveBeenCalledWith(activity.updates[0].thread);
    expect(activity.read).toHaveBeenCalledWith(activity.updates[0]);
  });

  it("lists work in progress apart from what is done", () => {
    render(<Activity activity={state()} open onOpen={vi.fn()} onJump={vi.fn()} />);

    expect(screen.getByText("In progress")).toBeTruthy();
    expect(screen.getByText("Updates")).toBeTruthy();
    expect(screen.getByText("Codex is working on it")).toBeTruthy();
  });
});

function state(): ActivityState {
  const thread = comment({ id: "t1", path: "src/follow_up/module.go", from: 26, to: 26 });
  const update = {
    key: "t1:reply:0",
    thread,
    who: "Claude Code",
    what: "answered",
    excerpt: "Parece, mas são coisas diferentes.",
    at: Date.now() / 1000 - 120,
    read: false,
  };
  return {
    working: [{ thread: comment({ id: "t2" }), who: "Codex", what: "is working on it", excerpt: "Se o dispatch falhar?", since: Date.now() / 1000 - 60 }],
    updates: [update],
    fresh: 1,
    read: vi.fn(),
    readAll: vi.fn(),
  };
}
