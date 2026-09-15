import { useEffect, useId, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { span } from "./quoted";
import type { Span } from "../useLineSelection";

/** How the box is worked, which is a description of it and never its name. */
const KEYS = "Markdown. ⌘↵ to save, esc to close.";

/** Where a comment gets written: a plain box under the lines it is about.
 *
 * Markdown goes in as it is typed and is rendered only once it is saved. A
 * preview tab would double the surface for something most comments never use —
 * a sentence and a question mark.
 *
 * The line the box already printed is its label rather than a caption beside
 * it. A box whose only name was its placeholder had no name at all once a word
 * was typed into it, and the placeholder is where the keys live, which is a
 * description of how to use the box and not a name for it. */
export function Box({ span: lines, onSave, onCancel }: BoxProps) {
  const field = useId();
  const keys = useId();
  const [text, setText] = useState("");
  const [saving, setSaving] = useState(false);
  const box = useRef<HTMLTextAreaElement>(null);

  // The box opens because the reader asked for it, so it opens ready to type —
  // and hands focus back to the control it was opened from when it goes.
  // Saving and cancelling both end here, and both used to leave the keyboard
  // on BODY, at the top of the review, a page away from the line just read.
  useEffect(() => {
    const opener = document.activeElement as HTMLElement | null;
    box.current?.focus();
    return () => {
      if (opener?.isConnected) opener.focus();
    };
  }, []);

  async function save() {
    if (!text.trim() || saving) return;
    setSaving(true);
    await onSave(text);
  }

  return (
    // `relative` for the description below: `sr-only` is absolute with no
    // offsets, so it lands at its static position inside the nearest positioned
    // ancestor. With none, that is the page — and a box opened far down a
    // scrolled pane hung a one-pixel span hundreds of pixels past the bottom of
    // the screen, which the page then grew a scrollbar for.
    <div className="commentbox commented relative border-b border-comment-rule bg-comment-bg py-3 pr-4 pl-8 md:pr-6 md:pl-16">
      <label htmlFor={field} className="lbl mb-1.5 block font-mono text-xs text-comment-ink">
        {about(lines)}
      </label>

      <Textarea
        ref={box}
        id={field}
        aria-describedby={keys}
        className="resize-y border-rule-strong bg-surface font-sans text-sm text-ink"
        rows={3}
        placeholder={KEYS}
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
      {/* The same sentence the placeholder shows, read out as the box's
          description instead of as its name — and still there once the box has
          something in it and the placeholder has gone. */}
      <span id={keys} className="sr-only">
        {KEYS}
      </span>

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

/** What the box is for, in the words the finished comment will carry — the
 * span first and the side after it, which is how `span` writes one everywhere
 * else. Worded the other way round from the `+` in the gutter on purpose: the
 * two sit on the same line while the box is open, and one name over both would
 * leave nothing to tell the control apart from the box it opened. */
function about(lines: Span): string {
  return lines.from === lines.to
    ? `Comment on line ${span(lines, "–")}`
    : `Comment on lines ${span(lines, "–")}`;
}

type BoxProps = {
  span: Span;
  onSave: (body: string) => Promise<void>;
  onCancel: () => void;
};
