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
 * on both and a reader using it has nothing else to tell the two apart. */
export function LineNumber({ line, side, tint, commentary }: LineNumberProps) {
  const at = numberOn(line, side);
  const open = at !== null && commentary !== undefined;

  return (
    <div
      className={cn(
        "ln group/ln relative shrink-0 pr-3 text-right text-faint select-none",
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
          className="absolute top-1/2 left-1 size-4 -translate-y-1/2 rounded bg-comment-ink text-surface opacity-0 group-hover/ln:opacity-100 hover:bg-comment-ink/90 hover:text-surface focus-visible:opacity-100"
          aria-label={`Comment on ${side} line ${at}`}
          title="Comment on this line"
          onClick={() => commentary.select.open(side, at)}
        >
          <Plus className="size-3" aria-hidden="true" />
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
