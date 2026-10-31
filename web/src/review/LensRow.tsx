import { ScanSearch } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { Check } from "./Check";
import type { LensView } from "@/api";

/** One lens in the sidebar, beside the files: seen or not, and a way to it. */
export function LensRow({ lens, current, onPick, onCheck }: LensRowProps) {
  return (
    <li className="relative">
      {/* Ticks the lens as seen, or takes it back, without opening, folding or
          moving to it — the same as a file's mark beside it. */}
      <Check
        done={lens.seen}
        label={`${lens.seen ? "Mark as not seen" : "Mark as seen"}: ${lens.title}`}
        onClick={onCheck}
      >
        {/* A lens, not a ring: the ring is a file's, and a lens beside the
            files read as one more of them. Green once seen, as a file's is. */}
        <ScanSearch className={cn("chk size-3.5", lens.seen ? "text-add-ink" : "text-faint")} aria-hidden="true" />
      </Check>
      <Button
        variant="ghost"
        className="lensitem h-auto w-full justify-start gap-2 py-1 pr-2 pl-[30px] font-sans text-[0.8125rem] font-normal text-ink hover:bg-sunken hover:text-ink aria-[current=true]:bg-highlight-dim aria-[current=true]:text-ink"
        data-seen={lens.seen ? "true" : undefined}
        aria-current={current ? "true" : undefined}
        onClick={onPick}
      >
        <span className={cn("nm truncate", lens.seen && "text-faint line-through", current && "font-semibold")}>{lens.title}</span>
        {lens.state === "outdated" && (
          <span className="ml-auto shrink-0 text-[0.625rem] text-highlight">outdated</span>
        )}
      </Button>
    </li>
  );
}

type LensRowProps = {
  lens: LensView;
  /** The lens the reader is on, from where the pane is scrolled. */
  current: boolean;
  onPick: () => void;
  /** Flip the seen mark, and nothing else. */
  onCheck: () => void;
};
