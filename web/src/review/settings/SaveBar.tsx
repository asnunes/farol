import { Button } from "@/components/ui/button";

/** One place to write what a section holds, there only while it holds
 * something: a Save beside every field was a row of buttons to hunt along. */
export function SaveBar({ count, saving, onSave, onDiscard }: SaveBarProps) {
  return (
    <div role="region" aria-label="Unsaved changes" className="savebar flex items-center justify-between gap-3 border-t border-rule bg-surface px-4 py-2.5 md:px-8">
      <span className="font-sans text-sm text-ink-muted">
        {count} unsaved {count === 1 ? "change" : "changes"}
      </span>
      <div className="flex gap-2">
        <Button variant="ghost" size="sm" disabled={saving} onClick={onDiscard}>
          Discard
        </Button>
        <Button size="sm" disabled={saving} onClick={onSave}>
          {saving ? "Saving…" : "Save"}
        </Button>
      </div>
    </div>
  );
}

type SaveBarProps = {
  count: number;
  saving: boolean;
  onSave: () => void;
  onDiscard: () => void;
};
