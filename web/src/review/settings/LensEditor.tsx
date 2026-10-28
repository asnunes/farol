import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";

/** A lens's title and prompt, edited as a draft and written only on Save. */
export function LensEditor({ title, prompt, saving, onSave, onCancel }: LensEditorProps) {
  const [draft, setDraft] = useState({ title, prompt });

  return (
    <div className="lens-editor mt-2 flex flex-col gap-2 text-sm text-ink">
      <label className="flex flex-col gap-1">
        <span className="text-xs text-ink-muted">Title</span>
        <Input value={draft.title} disabled={saving} onChange={(e) => setDraft({ ...draft, title: e.target.value })} />
      </label>
      <label className="flex flex-col gap-1">
        <span className="text-xs text-ink-muted">Prompt</span>
        <Textarea rows={5} value={draft.prompt} disabled={saving} onChange={(e) => setDraft({ ...draft, prompt: e.target.value })} />
        <span className="text-xs text-ink-muted">What the agent follows to write it. It can be just "Use the X skill".</span>
      </label>
      <div className="flex justify-end gap-2">
        <Button variant="ghost" size="xs" disabled={saving} onClick={onCancel}>
          Cancel
        </Button>
        <Button
          size="xs"
          disabled={saving || !draft.title.trim() || !draft.prompt.trim()}
          onClick={() => void onSave(draft.title, draft.prompt)}
        >
          Save
        </Button>
      </div>
    </div>
  );
}

type LensEditorProps = {
  title: string;
  prompt: string;
  saving: boolean;
  onSave: (title: string, prompt: string) => Promise<boolean>;
  onCancel: () => void;
};
