import { useEffect } from "react";
import { blockOf, type FileView, type ReviewView } from "@/api";

type Shortcuts = {
  review: ReviewView | null;
  order: FileView[];
  index: number;
  current: string | null;
  setCurrent: (path: string) => void;
  toggleViewed: (path: string, viewed: boolean) => Promise<void>;
  setHelpOpen: (fn: (open: boolean) => boolean) => void;
};

/** The keyboard is the primary way through a review; the mouse is the fallback. */
export function useShortcuts({
  review,
  order,
  index,
  current,
  setCurrent,
  toggleViewed,
  setHelpOpen,
}: Shortcuts) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      const target = e.target as HTMLElement;
      if (target.tagName === "INPUT" || target.tagName === "TEXTAREA") return;
      if (!review) return;

      const go = (i: number) => {
        const next = order[Math.max(0, Math.min(order.length - 1, i))];
        if (next) setCurrent(next.path);
      };

      switch (e.key) {
        case "j":
          go(index + 1);
          break;
        case "k":
          go(index - 1);
          break;
        case "n": {
          // Wrap: the last unread may be behind you after marking things read.
          const next =
            order.slice(index + 1).find((f) => !f.viewed) ?? order.find((f) => !f.viewed);
          if (next) setCurrent(next.path);
          break;
        }
        case "e": {
          const file = order[index];
          if (file) void toggleViewed(file.path, !file.viewed);
          break;
        }
        case "[":
        case "]": {
          const here = blockOf(review, current ?? "");
          if (!here) break;
          const target = review.blocks[here.index + (e.key === "]" ? 1 : -1)];
          if (target?.files[0]) setCurrent(target.files[0].path);
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
  }, [review, order, index, current, setCurrent, toggleViewed, setHelpOpen]);
}
