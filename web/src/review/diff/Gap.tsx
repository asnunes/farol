import { ChevronDown, ChevronsUpDown, ChevronUp } from "lucide-react";
import { Button } from "@/components/ui/button";
import { fitsInOneStep, fromAbove, fromBelow } from "./gaps";
import type { Gap as Stretch, Range } from "./gaps";

/** The lines between two hunks that nobody has asked to see yet, and the way to
 * ask.
 *
 * It sits where the `@@` band sits, because that is the seam: the band already
 * announces that the file jumps here, and this turns the announcement into
 * something the reader can do. Three gestures, in the order the hand reaches
 * for them: pull down from the hunk above, pull up from the hunk below, or take
 * the whole thing.
 *
 * A gap shorter than one step gets one control. Three buttons that all do the
 * same thing is three ways to wonder which one is different. */
export function Gap({ gap, onOpen, children }: GapProps) {
  const whole = { from: gap.from, to: gap.to };

  return (
    <div className="gap flex items-center gap-1 bg-sunken px-3 py-1 text-xs text-faint">
      {fitsInOneStep(gap) ? (
        <Pull label={`Open the ${gap.to - gap.from + 1} lines hidden here`} onPull={() => onOpen(whole)}>
          <ChevronsUpDown className="size-3.5" aria-hidden="true" />
        </Pull>
      ) : (
        <>
          <Pull label="Open the lines under the hunk above" onPull={() => onOpen(fromAbove(gap))}>
            <ChevronDown className="size-3.5" aria-hidden="true" />
          </Pull>
          <Pull label="Open the lines over the hunk below" onPull={() => onOpen(fromBelow(gap))}>
            <ChevronUp className="size-3.5" aria-hidden="true" />
          </Pull>
          <Pull
            label={`Open all ${gap.to - gap.from + 1} lines hidden here`}
            onPull={() => onOpen(whole)}
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
  onOpen: (range: Range) => void;
  /** What the band says beside the controls, which between hunks is the `@@`
   * header the diff came with. */
  children?: React.ReactNode;
};

type PullProps = { label: string; onPull: () => void; children: React.ReactNode };
