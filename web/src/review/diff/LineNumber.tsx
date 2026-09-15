import { Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { numberOn } from "./line";
import type { Commentary, Reach } from "./line";
import type { Span } from "./useLineSelection";
import type { DiffLine, Side } from "@/api";

/** The gutter cell, and the way into a comment.
 *
 * Pressing here and dragging down the numbers picks the passage; the `+` on
 * hover is the same thing for one line, and the part a reader who has not
 * discovered the drag will find. The cell belongs to one side of the diff and
 * shows that side's number, so a line the other side alone has offers no way in
 * — there is no number here to hang a comment on.
 *
 * **On the `+`, shift with an arrow reaches for more lines.** A drag is a mouse
 * and nothing else, and a passage is what most comments are about, so the one
 * control the keyboard can already get to is where the keyboard picks the
 * passage: each press moves the far end of the span by a line, the rows light
 * up as they do under a drag, and Enter opens the box over what was reached.
 * The reach stays inside this hunk and on this side — the `+` belongs to one
 * column, so there is no crossing to do — and leaving the button gives it up.
 *
 * The label says which side as well as which line, because the same number sits
 * on both and a reader using it has nothing else to tell the two apart. It says
 * the span once there is one, which is the only way a reader who cannot see the
 * rows light up knows what the arrows have reached.
 *
 * The `+` comes up on the whole line rather than on this cell, which is why the
 * group it answers to is declared by whoever draws the line and not here: the
 * row in unified, one column of it in split. The gutter is four characters wide
 * and the reader's pointer is on the code, so a control that only appeared over
 * the numbers was a control nobody found. */
export function LineNumber({ line, side, tint, reach, commentary }: LineNumberProps) {
  const at = numberOn(line, side);
  const open = at !== null && commentary !== undefined;
  const span = open ? reached(commentary.select.picking, side, at) : null;

  return (
    <div
      className={cn(
        "ln relative shrink-0 pr-3 text-right text-ink-muted select-none",
        tint,
        open && "cursor-pointer",
      )}
      onMouseDown={
        open
          ? (e) => {
              // Or the browser starts selecting text down the page instead.
              e.preventDefault();
              commentary.select.start(side, at);
            }
          : undefined
      }
      onMouseEnter={commentary ? () => commentary.select.extend(line) : undefined}
    >
      {open && (
        <Button
          variant="ghost"
          size="icon-xs"
          /* Two things have to be told apart, and one pair of colours does
             both. The comment ink against the surface the page is drawn on is
             the widest gap the palette has — 6.7:1 in either theme — so it
             separates the glyph from the chip; and because the ink is a cold
             blue while a changed line is washed green or red, the same chip
             stands at 5.5:1 or better on every row it can appear on, in both
             themes, including the deeper wash a drag puts under the lines it
             covers. A halo in the surface colour was tried on top of that and
             measured 1.1:1 against the washes, which is a class that does
             nothing. */
          className="absolute top-1/2 left-1 size-4 -translate-y-1/2 rounded bg-comment-ink text-surface opacity-0 group-hover/line:opacity-100 hover:bg-comment-ink/90 hover:text-surface focus-visible:opacity-100"
          aria-label={named(side, at, span)}
          title="Comment on this line — shift ↑/↓ reaches for more"
          onClick={() => commentary.select.open(side, at)}
          onKeyDown={(e) => {
            if (!e.shiftKey || (e.key !== "ArrowUp" && e.key !== "ArrowDown")) return;
            // Always, so the page never scrolls out from under a reader who is
            // at the end of what the hunk can offer.
            e.preventDefault();
            const step = e.key === "ArrowDown" ? 1 : -1;
            const bound = reach ?? { from: at, to: at };
            const to = Math.min(Math.max((span?.to ?? at) + step, bound.from), bound.to);
            commentary.select.reach(side, at, to);
          }}
          // A span nobody opened is a span nobody is looking at any more: the
          // rows would otherwise stay lit under a reader who has tabbed on.
          onBlur={() => commentary.select.drop()}
        >
          {/* Heavier than lucide's default: at 12px the standard stroke lands
              on one device pixel and the plus reads as a smudge. */}
          <Plus className="size-3" strokeWidth={3} aria-hidden="true" />
        </Button>
      )}
      {at}
    </div>
  );
}

/** What this `+` now opens, which is one line until the arrows have reached
 * for more. Only a span anchored here is this button's: the one being reached
 * from a `+` further down the file is that button's business. */
function reached(picking: Span | null, side: Side, at: number): Span | null {
  return picking && picking.side === side && picking.from === at ? picking : null;
}

function named(side: Side, at: number, span: Span | null): string {
  if (span === null || span.to === at) return `Comment on ${side} line ${at}`;
  const [from, to] = span.to < at ? [span.to, at] : [at, span.to];
  return `Comment on ${side} lines ${from}–${to}`;
}

type LineNumberProps = {
  /** The line this cell is the gutter of. */
  line: DiffLine;
  /** Which side of the diff the cell counts on, which is the column it is in. */
  side: Side;
  tint?: string;
  /** How far a span reached from this `+` may grow, which is the hunk. Absent
   * where no comment can be written at all. */
  reach?: Reach;
  commentary?: Commentary;
};
