import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
} from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { DiffWindow } from "./DiffWindow";

const callbacks = new Map<Element, IntersectionObserverCallback>();
const disconnect = vi.fn();
let observers = 0;

beforeEach(() => {
  callbacks.clear();
  observers = 0;
  disconnect.mockClear();
  vi.stubGlobal(
    "IntersectionObserver",
    class {
      callback: IntersectionObserverCallback;
      constructor(callback: IntersectionObserverCallback) {
        this.callback = callback;
        observers++;
      }
      observe(element: Element) {
        callbacks.set(element, this.callback);
      }
      unobserve(element: Element) {
        callbacks.delete(element);
      }
      disconnect() {
        disconnect();
      }
    },
  );
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

function near(element: Element, isIntersecting: boolean) {
  act(() =>
    callbacks.get(element)!(
      [{ target: element, isIntersecting } as IntersectionObserverEntry],
      {} as IntersectionObserver,
    ),
  );
}

it("releases distant code and keeps its measured height for the return trip", () => {
  const { container } = render(
    <DiffWindow rows={80} layout="unified">
      <p>Visible code</p>
    </DiffWindow>,
  );
  const box = container.querySelector(".diff-window")!;
  vi.spyOn(box, "getBoundingClientRect").mockReturnValue({
    height: 1730,
  } as DOMRect);
  expect(screen.queryByText("Visible code")).toBeNull();
  near(box, true);
  expect(screen.getByText("Visible code")).toBeTruthy();
  near(box, false);
  expect(screen.queryByText("Visible code")).toBeNull();
  expect((box as HTMLElement).style.height).toBe("1730px");
  near(box, true);
  expect(screen.getByText("Visible code")).toBeTruthy();
});

it("keeps a focused control mounted when scrolling it out of the window", () => {
  const { container } = render(
    <>
      <DiffWindow rows={1} layout="unified">
        <button>Comment on line</button>
      </DiffWindow>
      <button>Outside</button>
    </>,
  );
  const box = container.querySelector(".diff-window")!;
  near(box, true);
  act(() => screen.getByText("Comment on line").focus());
  near(box, false);
  expect(screen.getByText("Comment on line")).toBe(document.activeElement);
  act(() => screen.getByText("Outside").focus());
  expect(screen.queryByText("Comment on line")).toBeNull();
});

it("suspends ordinary windows in a hidden tab while preserving an editing window", () => {
  const { container } = render(
    <>
      <DiffWindow rows={1} layout="unified">
        <p>Idle code</p>
      </DiffWindow>
      <DiffWindow rows={1} layout="unified" pinned>
        <textarea defaultValue="Unsent draft" />
      </DiffWindow>
    </>,
  );
  const box = container.querySelector(".diff-window")!;
  near(box, true);
  const state = vi
    .spyOn(document, "visibilityState", "get")
    .mockReturnValue("hidden");
  fireEvent(document, new Event("visibilitychange"));
  expect(screen.queryByText("Idle code")).toBeNull();
  expect((screen.getByRole("textbox") as HTMLTextAreaElement).value).toBe(
    "Unsent draft",
  );
  state.mockReturnValue("visible");
  fireEvent(document, new Event("visibilitychange"));
  expect(screen.getByText("Idle code")).toBeTruthy();
});

it("shares the observer and releases every observed element when the review closes", () => {
  const { unmount } = render(
    <main className="pane">
      {Array.from({ length: 30 }, (_, i) => (
        <DiffWindow key={i} rows={80} layout="unified">
          <p>{i}</p>
        </DiffWindow>
      ))}
    </main>,
  );
  expect(observers).toBe(1);
  expect(callbacks.size).toBe(30);
  unmount();
  expect(callbacks.size).toBe(0);
  expect(disconnect).toHaveBeenCalledTimes(1);
});
