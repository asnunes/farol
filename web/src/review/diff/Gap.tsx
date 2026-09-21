import { useRef } from "react";
import { ChevronDown, ChevronUp, UnfoldVertical } from "lucide-react";
import { Button } from "@/components/ui/button";
import { fitsInOneStep, fromAbove, fromBelow } from "./gaps";
import type { Gap as Stretch, Range } from "./gaps";

/** The lines between two hunks that nobody has asked to see yet, and the way to
 * ask.
 *
 * It sits where the `@@` band sits, because that is the seam: the band already
 * announces that the file jumps here, and this turns the announcement into
 * something the reader can do. Twenty lines above it, twenty below, or the
 * whole thing.
 *
 * **An arrow says which hunk it walks from**, and that is the only thing it
 * means. Up walks up from the hunk below, taking the twenty lines that run into
 * it; down walks down from the hunk above, taking the twenty that follow it.
 * Press the same one again and it takes twenty more.
 *
 * The arrows used to name the side of the band the code came out on instead,
 * which reads well only while the band sits in the middle of what is hidden. It
 * does not: a stretch still reaching down to its hunk has no band of its own and
 * borrows that hunk's `@@` header, which sits under all of it. Both directions
 * then put their lines above the header, and the one pointing down pointed at a
 * side nothing ever came out on.
 *
 * At the ends of a file one direction has no hunk to walk from and is left out:
 * the gap before the first hunk only walks up into it, the gap after the last
 * only walks down out of it. A gap shorter than one step gets one control, since
 * three buttons that all do the same thing are three ways to wonder which one is
 * different. */
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
      className="gap flex items-center gap-1 bg-sunken px-3 py-1 text-xs text-ink-muted"
    >
      {fitsInOneStep(gap) ? (
        <Pull label={`Open the ${gap.to - gap.from + 1} lines hidden here`} onPull={() => pull(whole)}>
          <UnfoldVertical className="size-3.5" aria-hidden="true" />
        </Pull>
      ) : (
        <>
          {gap.over && (
            <Pull label={up(gap)} onPull={() => pull(fromBelow(gap))}>
              <ChevronUp className="size-3.5" aria-hidden="true" />
            </Pull>
          )}
          {gap.under && (
            <Pull label={down(gap)} onPull={() => pull(fromAbove(gap))}>
              <ChevronDown className="size-3.5" aria-hidden="true" />
            </Pull>
          )}
          <Pull
            label={`Open all ${gap.to - gap.from + 1} lines hidden here`}
            onPull={() => pull(whole)}
          >
            <UnfoldVertical className="size-3.5" aria-hidden="true" />
          </Pull>
        </>
      )}
      <span className="ml-2">{children}</span>
    </div>
  );
}

/** The labels name the lines themselves and the hunk they are being walked
 * from, which is the part a reader cannot see: the band says where the file
 * jumps, not which end of the jump a press takes. */
function up(gap: Stretch): string {
  const { from, to } = fromBelow(gap);
  return `Open lines ${from}–${to}, up from the hunk below`;
}

function down(gap: Stretch): string {
  const { from, to } = fromAbove(gap);
  return `Open lines ${from}–${to}, down from the hunk above`;
}

function Pull({ label, onPull, children }: PullProps) {
  return (
    <Button
      size="icon-xs"
      variant="ghost"
      className="text-faint hover:bg-surface hover:text-ink"
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
