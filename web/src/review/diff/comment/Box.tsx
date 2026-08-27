import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { span } from "./quoted";

/** Where a comment gets written: a plain box under the lines it is about.
 *
 * Markdown goes in as it is typed and is rendered only once it is saved. A
 * preview tab would double the surface for something most comments never use —
 * a sentence and a question mark. */
export function Box({ span: lines, onSave, onCancel }: BoxProps) {
  const [text, setText] = useState("");
  const [saving, setSaving] = useState(false);
  const box = useRef<HTMLTextAreaElement>(null);

  // The box opens because the reader asked for it, so it opens ready to type.
  useEffect(() => box.current?.focus(), []);

  async function save() {
    if (!text.trim() || saving) return;
    setSaving(true);
    await onSave(text);
  }

  return (
    <div className="commentbox commented border-b border-comment-rule bg-comment-bg py-3 pr-6 pl-16">
      <div className="lbl mb-1.5 font-mono text-xs text-comment-ink">
        {span(lines, "–")}
      </div>

      <Textarea
        ref={box}
        className="resize-y border-rule-strong bg-surface font-sans text-sm text-ink"
        rows={3}
        placeholder="Markdown. ⌘↵ to save, esc to close."
        value={text}
        disabled={saving}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          // Enter alone has to stay a newline: this is prose, and a comment
          // worth writing usually runs to a second line.
          if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
            e.preventDefault();
            void save();
          }
          if (e.key === "Escape") {
            e.preventDefault();
            onCancel();
          }
        }}
      />

      <div className="mt-2 flex gap-2">
        <Button
          size="xs"
          className="bg-comment-ink text-surface hover:bg-comment-ink/90"
          disabled={!text.trim() || saving}
          onClick={() => void save()}
        >
          Comment
        </Button>
        <Button size="xs" variant="ghost" className="text-ink-muted" onClick={onCancel}>
          Cancel
        </Button>
      </div>
    </div>
  );
}

type BoxProps = {
  span: { from: number; to: number };
  onSave: (body: string) => Promise<void>;
  onCancel: () => void;
};
