import { ScrollText } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { LensView } from "@/api";

/** One lens in the sidebar, beside the files: seen or not, and a way to it. */
export function LensRow({ lens, current, onPick }: LensRowProps) {
  return (
    <li>
      <Button
        variant="ghost"
        className="lensitem h-auto w-full justify-start gap-2 px-2 py-1 font-sans text-[0.8125rem] font-normal text-ink hover:bg-sunken hover:text-ink aria-[current=true]:bg-highlight-dim aria-[current=true]:text-ink"
        data-seen={lens.seen ? "true" : undefined}
        aria-current={current ? "true" : undefined}
        onClick={onPick}
      >
        {/* Text, not a ring: the ring is a file's, and a lens beside the
            files read as one more of them. Green once seen, as a file's is. */}
        <ScrollText
          className={cn("chk size-3.5 shrink-0", lens.seen ? "text-add-ink" : "text-faint")}
          aria-hidden="true"
        />
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
};
