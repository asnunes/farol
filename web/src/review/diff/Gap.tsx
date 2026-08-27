import { useRef } from "react";
import { ChevronDown, ChevronsUpDown, ChevronUp } from "lucide-react";
import { Button } from "@/components/ui/button";
import { fitsInOneStep, fromAbove, fromBelow } from "./gaps";
import type { Gap as Stretch, Range } from "./gaps";

/** The lines between two hunks that nobody has asked to see yet, and the way to
 * ask.
 *
 * It sits where the `@@` band sits, because that is the seam: the band already
 * announces that the file jumps here, and this turns the announcement into
 * something the reader can do. Twenty lines from the top, twenty from the
 * bottom, or the whole thing.
 *
 * At the ends of a file one of the two directions has no hunk to walk away
 * from, and it is left out: the gap before the first hunk opens upward from
 * what follows it, the gap after the last opens downward from what precedes
 * it. A gap shorter than one step gets one control, since three buttons that
 * all do the same thing are three ways to wonder which one is different. */
export function Gap({ gap, onOpen, children }: GapProps) {
  const band = useRef<HTMLDivElement>(null);
  const whole = { from: gap.from, to: gap.to };

  /** Open, and keep the band under the cursor where it was.
   *
   * Lines that arrive above it push it down by their own height, which on a
   * full step is half a screen: the reader presses a button and the button
   * walks away from the pointer, taking the code they were about to read with
   * it. Scrolling by the same amount puts it back. When the gap opened whole
   * the band is gone and there is nothing to hold still, which is fine — that
   * press ended the reading rather than continuing it. */
  async function pull(range: Range) {
    const pane = band.current?.closest(".pane");
    const before = band.current?.getBoundingClientRect().top;
    await onOpen(range);
    requestAnimationFrame(() => {
      const after = band.current?.getBoundingClientRect().top;
      if (pane && before !== undefined && after !== undefined) {
        pane.scrollTop += after - before;
      }
    });
  }

  return (
    <div
      ref={band}
      className="gap flex items-center gap-1 bg-sunken px-3 py-1 text-xs text-faint"
    >
      {fitsInOneStep(gap) ? (
        <Pull label={`Open the ${gap.to - gap.from + 1} lines hidden here`} onPull={() => pull(whole)}>
          <ChevronsUpDown className="size-3.5" aria-hidden="true" />
        </Pull>
      ) : (
        <>
          {gap.under && (
            <Pull label="Open twenty lines from the top" onPull={() => pull(fromAbove(gap))}>
              <ChevronDown className="size-3.5" aria-hidden="true" />
            </Pull>
          )}
          {gap.over && (
            <Pull label="Open twenty lines from the bottom" onPull={() => pull(fromBelow(gap))}>
              <ChevronUp className="size-3.5" aria-hidden="true" />
            </Pull>
          )}
          <Pull
            label={`Open all ${gap.to - gap.from + 1} lines hidden here`}
            onPull={() => pull(whole)}
          >
            <ChevronsUpDown className="size-3.5" aria-hidden="true" />
          </Pull>
        </>
      )}
      <span className="ml-2">{children}</span>
    </div>
  );
}

function Pull({ label, onPull, children }: PullProps) {
  return (
    <Button
      size="icon-xs"
      variant="ghost"
      className="cursor-pointer text-faint hover:bg-surface hover:text-ink"
      aria-label={label}
      title={label}
      onClick={() => void onPull()}
    >
      {children}
    </Button>
  );
}

type GapProps = {
  gap: Stretch;
  onOpen: (range: Range) => Promise<void>;
  /** What the band says beside the controls, which between hunks is the `@@`
   * header the diff came with. */
  children?: React.ReactNode;
};

type PullProps = { label: string; onPull: () => void; children: React.ReactNode };
