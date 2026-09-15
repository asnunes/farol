import { useEffect } from "react";
import { blockOf, type FileView, type ReviewView } from "@/api";

/** How the modifier is written on this machine, as the prefix it is: `⌘B` on a
 * Mac, `Ctrl+B` anywhere else. Spelled out by the key bar, the help and the
 * button's own tooltip. */
export const MOD = typeof navigator !== "undefined" && navigator.userAgent.includes("Mac")
  ? "⌘"
  : "Ctrl+";

/** The keyboard is the primary way through a review; the mouse is the fallback. */
export function useShortcuts({
  review,
  order,
  index,
  current,
  goTo,
  toggleViewed,
  setHelpOpen,
  toggleMap,
}: Shortcuts) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // Above every guard below it: hiding the map is chrome, not reading, and
      // the reader wants it while typing a comment as much as while moving
      // through files. It is also the one key here with a modifier, which is
      // what keeps it out of the way of a comment being written.
      if ((e.metaKey || e.ctrlKey) && !e.altKey && e.key.toLowerCase() === "b") {
        // Firefox opens its bookmarks sidebar on this, which is the wrong
        // sidebar.
        e.preventDefault();
        toggleMap();
        return;
      }
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      const target = e.target as HTMLElement;
      if (target.tagName === "INPUT" || target.tagName === "TEXTAREA") return;
      // A focused button already answers to Enter, and pressing it would both
      // press the button and mark the file read.
      if (target.tagName === "BUTTON") return;
      if (!review) return;

      const go = (i: number) => {
        const next = order[Math.max(0, Math.min(order.length - 1, i))];
        if (next) goTo(next.path);
      };

      // The letters are the short way, the arrows and Enter the obvious one.
      // Both are here because the reader who knows the keys and the reader who
      // is guessing are the same person on different days.
      switch (e.key) {
        case "j":
        case "ArrowDown":
          e.preventDefault();
          go(index + 1);
          break;
        case "k":
        case "ArrowUp":
          e.preventDefault();
          go(index - 1);
          break;
        case "n": {
          // Wrap: the last unread may be behind you after marking things read.
          const next =
            order.slice(index + 1).find((f) => !f.viewed) ?? order.find((f) => !f.viewed);
          if (next) goTo(next.path);
          break;
        }
        case ";":
        case " ": {
          // Space scrolls by default, and it cannot do both.
          e.preventDefault();
          const file = order[index];
          if (file) void toggleViewed(file.path, !file.viewed);
          break;
        }
        // The page keys are left alone on purpose. They are the only way to
        // scroll a long file by the screenful, and the brackets already move
        // between blocks.
        case "[":
        case "]": {
          e.preventDefault();
          const here = blockOf(review, current ?? "");
          if (!here) break;
          const target = review.blocks[here.index + (e.key === "]" ? 1 : -1)];
          if (target?.files[0]) goTo(target.files[0].path);
          break;
        }
        case "?":
          setHelpOpen((v) => !v);
          break;
        case "Escape":
          setHelpOpen(() => false);
          break;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [review, order, index, current, goTo, toggleViewed, setHelpOpen, toggleMap]);
}

type Shortcuts = {
  review: ReviewView | null;
  order: FileView[];
  index: number;
  current: string | null;
  goTo: (path: string) => void;
  toggleViewed: (path: string, viewed: boolean) => Promise<void>;
  setHelpOpen: (fn: (open: boolean) => boolean) => void;
  toggleMap: () => void;
};
