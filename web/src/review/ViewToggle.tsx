import { Columns2, Rows3 } from "lucide-react";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import type { DiffView } from "@/hooks/useDiffView";

/** Choose how the diff is laid out.
 *
 * Rows against columns: the pair is the shape of each layout rather than a
 * picture of what it means, which is the only thing a pictogram can carry here.
 * The words are still on the button for anyone who needs them, in the label a
 * screen reader announces and in the tooltip. */
export function ViewToggle({ view, onChange }: ViewToggleProps) {
  return (
    <ToggleGroup
      type="single"
      value={view}
      // A group of one choice, so letting go of the pressed one would leave the
      // diff with no layout at all. Pressing the current one again does
      // nothing, which is what the reader expects from a segmented control.
      onValueChange={(next) => next && onChange(next as DiffView)}
      className="viewtoggle rounded border border-rule p-0.5"
    >
      {LAYOUTS.map(({ option, Icon, says }) => (
        <ToggleGroupItem
          key={option}
          value={option}
          size="sm"
          aria-label={says}
          title={says}
          className="size-6 min-w-0 text-ink-muted hover:text-ink data-[state=on]:bg-sunken data-[state=on]:text-ink"
        >
          <Icon className="size-3.5" aria-hidden="true" />
        </ToggleGroupItem>
      ))}
    </ToggleGroup>
  );
}

const LAYOUTS = [
  { option: "unified", Icon: Rows3, says: "Unified: one line under another" },
  { option: "split", Icon: Columns2, says: "Split: the old side and the new one" },
] as const;

type ViewToggleProps = { view: DiffView; onChange: (view: DiffView) => void };
