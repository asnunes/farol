import { Columns2, Rows3 } from "lucide-react";
import { cn } from "@/lib/utils";
import type { DiffView } from "@/hooks/useDiffView";

/** Choose how the diff is laid out.
 *
 * Rows against columns: the pair is the shape of each layout rather than a
 * picture of what it means, which is the only thing a pictogram can carry here.
 * The words are still on the button for anyone who needs them, in the label a
 * screen reader announces and in the tooltip. */
export function ViewToggle({ view, onChange }: ViewToggleProps) {
  return (
    <div className="viewtoggle flex items-center rounded border border-rule p-0.5">
      {LAYOUTS.map(({ option, Icon, says }) => (
        <button
          key={option}
          className={cn(
            "grid size-6 cursor-pointer place-items-center rounded transition-colors",
            view === option ? "bg-sunken text-ink" : "text-muted hover:text-ink",
          )}
          aria-pressed={view === option}
          aria-label={says}
          title={says}
          onClick={() => onChange(option)}
        >
          <Icon className="size-3.5" aria-hidden="true" />
        </button>
      ))}
    </div>
  );
}

const LAYOUTS = [
  { option: "unified", Icon: Rows3, says: "Unified: one line under another" },
  { option: "split", Icon: Columns2, says: "Split: the old side and the new one" },
] as const;

type ViewToggleProps = { view: DiffView; onChange: (view: DiffView) => void };
