import { useState } from "react";

/** Unified or side by side, remembered between visits.
 *
 * In the browser rather than in the review state on disk: it is a preference of
 * the person reading, not of the change being read, and it should follow them
 * from one review to the next. */
export function useDiffView(): [DiffView, (view: DiffView) => void] {
  const [view, setView] = useState<DiffView>(remembered);

  return [
    view,
    (next: DiffView) => {
      setView(next);
      try {
        localStorage.setItem(KEY, next);
      } catch {
        // Storage can be refused, and a preference that fails to persist is
        // not worth breaking the screen over.
      }
    },
  ];
}

export type DiffView = "unified" | "split";

const KEY = "farol:diff-view";

function remembered(): DiffView {
  try {
    return localStorage.getItem(KEY) === "split" ? "split" : "unified";
  } catch {
    return "unified";
  }
}
