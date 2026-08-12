import { useEffect } from "react";
import { blockOf, type FileView, type ReviewView } from "@/api";

/** The keyboard is the primary way through a review; the mouse is the fallback. */
export function useShortcuts({
  review,
  order,
  index,
  current,
  goTo,
  toggleViewed,
  setHelpOpen,
}: Shortcuts) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
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
        case "Enter": {
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
  }, [review, order, index, current, goTo, toggleViewed, setHelpOpen]);
}

type Shortcuts = {
  review: ReviewView | null;
  order: FileView[];
  index: number;
  current: string | null;
  goTo: (path: string) => void;
  toggleViewed: (path: string, viewed: boolean) => Promise<void>;
  setHelpOpen: (fn: (open: boolean) => boolean) => void;
};
