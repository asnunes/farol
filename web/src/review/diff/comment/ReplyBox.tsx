import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";

/** Writing back under a comment.
 *
 * With a session connected, "Reply & send" hands the whole conversation to the
 * agent again; plain "Reply" only adds the turn, for a note to self. */
export function ReplyBox({ agent, onReply, onCancel }: ReplyBoxProps) {
  const [text, setText] = useState("");
  const [saving, setSaving] = useState(false);
  const box = useRef<HTMLTextAreaElement>(null);

  useEffect(() => box.current?.focus(), []);

  async function save(andSend: boolean) {
    if (!text.trim() || saving) return;
    setSaving(true);
    await onReply(text, andSend && agent !== null);
  }

  return (
    <div className="replybox flex flex-col gap-2">
      <Textarea
        ref={box}
        aria-label="Reply to this comment"
        className="resize-y border-rule-strong bg-surface font-sans text-sm text-ink"
        rows={2}
        placeholder={agent ? "⌘↵ to reply and send, ⇧⌘↵ to only reply, esc to close." : "⌘↵ to reply, esc to close."}
        value={text}
        disabled={saving}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
            e.preventDefault();
            // The plain shortcut does what the highlighted button does.
            void save(!e.shiftKey);
          }
          if (e.key === "Escape") {
            e.preventDefault();
            onCancel();
          }
        }}
      />
      <div className="flex gap-2">
        {agent && (
          <Button
            size="xs"
            className="bg-comment-ink text-surface hover:bg-comment-ink/90"
            title={`Reply and send the conversation to ${agent}`}
            disabled={!text.trim() || saving}
            onClick={() => void save(true)}
          >
            Reply &amp; send
          </Button>
        )}
        <Button size="xs" variant={agent ? "outline" : "default"} disabled={!text.trim() || saving} onClick={() => void save(false)}>
          Reply
        </Button>
        <Button size="xs" variant="ghost" className="text-ink-muted" onClick={onCancel}>
          Cancel
        </Button>
      </div>
    </div>
  );
}

type ReplyBoxProps = {
  agent: string | null;
  onReply: (body: string, andSend: boolean) => Promise<void>;
  onCancel: () => void;
};
