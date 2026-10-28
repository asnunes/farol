import { Circle, CircleCheck } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { LensView } from "@/api";

/** One lens in the sidebar, beside the files: seen or not, and a way to it. */
export function LensRow({ lens, onPick }: LensRowProps) {
  return (
    <li>
      <Button
        variant="ghost"
        className="lensitem h-auto w-full justify-start gap-2 px-2 py-1 font-sans text-[0.8125rem] font-normal text-ink hover:bg-sunken hover:text-ink"
        data-seen={lens.seen ? "true" : undefined}
        onClick={onPick}
      >
        {lens.seen ? (
          <CircleCheck className="chk size-3.5 shrink-0 text-add-ink" aria-hidden="true" />
        ) : (
          <Circle className="chk size-3.5 shrink-0 text-faint" aria-hidden="true" />
        )}
        <span className={cn("nm truncate", lens.seen && "text-faint line-through")}>{lens.title}</span>
        {lens.state === "outdated" && (
          <span className="ml-auto shrink-0 text-[0.625rem] text-highlight">outdated</span>
        )}
      </Button>
    </li>
  );
}

type LensRowProps = {
  lens: LensView;
  onPick: () => void;
};
