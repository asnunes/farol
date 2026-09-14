import { Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { numberOn } from "./line";
import type { Commentary } from "./line";
import type { DiffLine, Side } from "@/api";

/** The gutter cell, and the way into a comment.
 *
 * Pressing here and dragging down the numbers picks the passage; the `+` on
 * hover is the same thing for one line, and the part a reader who has not
 * discovered the drag will find. The cell belongs to one side of the diff and
 * shows that side's number, so a line the other side alone has offers no way in
 * — there is no number here to hang a comment on.
 *
 * The label says which side as well as which line, because the same number sits
 * on both and a reader using it has nothing else to tell the two apart.
 *
 * The `+` comes up on the whole line rather than on this cell, which is why the
 * group it answers to is declared by whoever draws the line and not here: the
 * row in unified, one column of it in split. The gutter is four characters wide
 * and the reader's pointer is on the code, so a control that only appeared over
 * the numbers was a control nobody found. */
export function LineNumber({ line, side, tint, commentary }: LineNumberProps) {
  const at = numberOn(line, side);
  const open = at !== null && commentary !== undefined;

  return (
    <div
      className={cn(
        "ln relative shrink-0 pr-3 text-right text-faint select-none",
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
          aria-label={`Comment on ${side} line ${at}`}
          title="Comment on this line"
          onClick={() => commentary.select.open(side, at)}
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

type LineNumberProps = {
  /** The line this cell is the gutter of. */
  line: DiffLine;
  /** Which side of the diff the cell counts on, which is the column it is in. */
  side: Side;
  tint?: string;
  commentary?: Commentary;
};
