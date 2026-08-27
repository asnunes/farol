import { Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { Commentary } from "./line";

/** The gutter cell, and the way into a comment.
 *
 * Pressing here and dragging down the numbers picks the passage; the `+` on
 * hover is the same thing for one line, and the part a reader who has not
 * discovered the drag will find. A line with no number on the new side gets
 * neither: a comment hangs off the code as it now reads, and a removed line is
 * not there to hang it on. */
export function LineNumber({ number, on, tint, commentary }: LineNumberProps) {
  const open = on !== null && commentary !== undefined;

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
              commentary.select.start(on);
            }
          : undefined
      }
      onMouseEnter={open ? () => commentary.select.extend(on) : undefined}
    >
      {open && (
        <Button
          variant="ghost"
          size="icon-xs"
          className="absolute top-1/2 left-1 size-4 -translate-y-1/2 rounded bg-comment-ink text-surface opacity-0 group-hover/ln:opacity-100 hover:bg-comment-ink/90 hover:text-surface focus-visible:opacity-100"
          aria-label={`Comment on line ${on}`}
          title="Comment on this line"
          onClick={() => commentary.select.open(on)}
        >
          <Plus className="size-3" aria-hidden="true" />
        </Button>
      )}
      {number}
    </div>
  );
}

type LineNumberProps = {
  /** What the cell shows, which in a split diff is the number of its own side. */
  number: number | null;
  /** The line a comment written here would be about, or null if there is none. */
  on: number | null;
  tint?: string;
  commentary?: Commentary;
};
