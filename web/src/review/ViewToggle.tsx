import { cn } from "@/lib/utils";
import type { DiffView } from "@/hooks/useDiffView";

/** Choose how the diff is laid out. Two words rather than two icons: the
 * difference between the layouts is not something a pictogram says. */
export function ViewToggle({ view, onChange }: ViewToggleProps) {
  return (
    <div className="viewtoggle flex items-center rounded border border-rule p-0.5 font-mono text-xs">
      {(["unified", "split"] as const).map((option) => (
        <button
          key={option}
          className={cn(
            "cursor-pointer rounded px-2 py-0.5 transition-colors",
            view === option ? "bg-sunken text-ink" : "text-muted hover:text-ink",
          )}
          aria-pressed={view === option}
          onClick={() => onChange(option)}
        >
          {option}
        </button>
      ))}
    </div>
  );
}

type ViewToggleProps = { view: DiffView; onChange: (view: DiffView) => void };
