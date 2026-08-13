import { useEffect, useRef, useState } from "react";

/** Where a comment gets written: a plain box under the lines it is about.
 *
 * Markdown goes in as it is typed and is rendered only once it is saved. A
 * preview tab would double the surface for something most comments never use —
 * a sentence and a question mark. */
export function CommentBox({ span, onSave, onCancel }: CommentBoxProps) {
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
        {span.from === span.to ? span.from : `${span.from}–${span.to}`}
      </div>

      <textarea
        ref={box}
        className="w-full resize-y rounded border border-rule-strong bg-surface px-3 py-2 font-sans text-sm text-ink focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
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

      <div className="mt-2 flex gap-2 font-sans text-xs">
        <button
          className="cursor-pointer rounded bg-comment-ink px-3 py-1 text-surface disabled:cursor-default disabled:opacity-40"
          disabled={!text.trim() || saving}
          onClick={() => void save()}
        >
          Comment
        </button>
        <button className="cursor-pointer rounded px-3 py-1 text-muted" onClick={onCancel}>
          Cancel
        </button>
      </div>
    </div>
  );
}

type CommentBoxProps = {
  span: { from: number; to: number };
  onSave: (body: string) => Promise<void>;
  onCancel: () => void;
};
