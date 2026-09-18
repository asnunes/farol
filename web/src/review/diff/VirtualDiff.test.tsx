import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
} from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { Diff } from "./Diff";
import { file, noComments } from "./testing";
import type { FileDiff } from "@/api";

const { tokenize } = vi.hoisted(() => ({
  tokenize: vi.fn((code: string) =>
    code.split("\n").map((content) => [{ content }]),
  ),
}));
vi.mock("@/hooks/useHighlight", () => ({ useHighlight: () => tokenize }));
const callbacks = new Map<Element, IntersectionObserverCallback>();
beforeEach(() => {
  tokenize.mockClear();
  callbacks.clear();
  vi.stubGlobal(
    "IntersectionObserver",
    class {
      callback: IntersectionObserverCallback;
      constructor(callback: IntersectionObserverCallback) {
        this.callback = callback;
      }
      observe(el: Element) {
        callbacks.set(el, this.callback);
      }
      unobserve(el: Element) {
        callbacks.delete(el);
      }
      disconnect() {}
    },
  );
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
function near(el: Element, visible: boolean) {
  callbacks.get(el)!(
    [{ target: el, isIntersecting: visible } as IntersectionObserverEntry],
    {} as IntersectionObserver,
  );
}
function diff(): FileDiff {
  return {
    path: "sample.rs",
    status: "added",
    binary: false,
    additions: 240,
    deletions: 0,
    lineCount: 240,
    hunks: [
      {
        oldStart: 0,
        oldLines: 0,
        newStart: 1,
        newLines: 240,
        lines: Array.from({ length: 240 }, (_, i) => ({
          kind: "added" as const,
          oldNumber: null,
          newNumber: i + 1,
          content: `line ${i + 1}`,
        })),
      },
    ],
  };
}
it("only colours nearby portions of one large hunk and reuses unchanged visible colours", () => {
  const source = diff(),
    subject = file({ path: source.path }),
    comments: [] = [],
    actions = noComments();
  const { container, rerender } = render(
    <Diff
      diff={source}
      file={subject}
      view="unified"
      comments={comments}
      actions={actions}
    />,
  );
  const windows = container.querySelectorAll(".diff-window");
  expect(windows).toHaveLength(3);
  expect(tokenize).not.toHaveBeenCalled();
  act(() => near(windows[0], true));
  expect(container.querySelectorAll(".row")).toHaveLength(80);
  const calls = tokenize.mock.calls.length;
  rerender(
    <Diff
      diff={source}
      file={subject}
      view="unified"
      comments={comments}
      actions={actions}
    />,
  );
  expect(tokenize).toHaveBeenCalledTimes(calls);
  act(() => {
    near(windows[0], false);
    near(windows[1], true);
  });
  expect(container.querySelectorAll(".row")).toHaveLength(80);
  expect(screen.queryByText("line 1")).toBeNull();
  expect(screen.getByText("line 81")).toBeTruthy();
});
it("keeps a draft and its selected range while the editor is offscreen", () => {
  const source = diff();
  const actions = noComments();
  const { container } = render(
    <Diff
      diff={source}
      file={file({ path: source.path })}
      view="unified"
      comments={[]}
      actions={actions}
    />,
  );
  const windows = container.querySelectorAll(".diff-window");
  act(() => near(windows[0], true));
  const button = screen.getByRole("button", { name: "Comment on new line 79" });
  fireEvent.keyDown(button, { key: "ArrowDown", shiftKey: true });
  fireEvent.keyDown(button, { key: "ArrowDown", shiftKey: true });
  fireEvent.click(button);
  const editor = screen.getByRole("textbox");
  fireEvent.change(editor, { target: { value: "Keep this draft" } });
  act(() => {
    near(windows[0], false);
    near(windows[1], false);
  });
  expect((screen.getByRole("textbox") as HTMLTextAreaElement).value).toBe(
    "Keep this draft",
  );
  fireEvent.click(screen.getByRole("button", { name: "Comment" }));
  expect(actions.add).toHaveBeenCalledWith(
    "sample.rs",
    "new",
    79,
    81,
    "Keep this draft",
  );
});
it("does not accumulate rendered rows after visiting 180 files in a 352-file review", () => {
  const source = diff(),
    actions = noComments();
  const { container } = render(
    <main className="pane">
      {Array.from({ length: 352 }, (_, i) => (
        <Diff
          key={i}
          diff={source}
          file={file({ path: `file-${i}.rs` })}
          view="unified"
          comments={[]}
          actions={actions}
        />
      ))}
    </main>,
  );
  const windows = container.querySelectorAll(".diff-window");
  for (let i = 0; i < 180; i++)
    act(() => {
      if (i) near(windows[(i - 1) * 3], false);
      near(windows[i * 3], true);
    });
  expect(container.querySelectorAll(".row")).toHaveLength(80);
  expect(container.querySelectorAll('[data-rendered="true"]')).toHaveLength(1);
}, 20000);
