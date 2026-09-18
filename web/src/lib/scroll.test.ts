import { afterEach, expect, it, vi } from "vitest";
import { scrollToFile } from "./scroll";

afterEach(() => {
  window.dispatchEvent(new Event("wheel"));
  document.body.innerHTML = "";
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

it("keeps the selected file in place when an earlier lazy diff expands, until reader input", () => {
  let changed: ResizeObserverCallback = () => {};
  const disconnect = vi.fn();
  vi.stubGlobal(
    "ResizeObserver",
    class {
      constructor(callback: ResizeObserverCallback) {
        changed = callback;
      }
      observe() {}
      disconnect = disconnect;
    },
  );
  vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
    callback(0);
    return 1;
  });
  vi.stubGlobal("CSS", { escape: (value: string) => value });
  document.body.innerHTML =
    '<main class="pane"><section></section><section data-path="last.rs"></section></main>';
  const pane = document.querySelector<HTMLElement>("main")!;
  const target = document.querySelector<HTMLElement>("[data-path]")!;
  target.scrollIntoView = vi.fn();
  let top = 50;
  vi.spyOn(target, "getBoundingClientRect").mockImplementation(
    () => ({ top }) as DOMRect,
  );
  scrollToFile("last.rs");
  top = 550;
  changed([], {} as ResizeObserver);
  expect(pane.scrollTop).toBe(500);
  window.dispatchEvent(new Event("wheel"));
  expect(disconnect).toHaveBeenCalled();
});
