import { useEffect, useRef, useState } from "react";
import { Check, Send as SendIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { CommentActions } from "@/hooks/useComments";
import type { CommentView } from "@/api";

/** As long as the copy's tick: the two sit side by side and should answer alike. */
const CONFIRM_FOR = 1500;

/** Hand the comment to the agent session that implemented the branch.
 *
 * The tick means the harness took it, not that the agent read it — neither
 * harness says that much. A failure goes to the page's alert, and Copy stays
 * beside this for whoever would rather paste it. */
export function Send({ comment, agent, actions }: SendProps) {
  const [sending, setSending] = useState(false);
  const [sent, setSent] = useState(false);
  const clearing = useRef<number | undefined>(undefined);

  useEffect(() => () => window.clearTimeout(clearing.current), []);

  async function press() {
    setSending(true);
    const took = await actions.send(comment.id);
    setSending(false);
    if (!took) return;
    setSent(true);
    window.clearTimeout(clearing.current);
    clearing.current = window.setTimeout(() => setSent(false), CONFIRM_FOR);
  }

  return (
    <Button
      variant="ghost"
      size="icon-xs"
      className="sender text-comment-faint hover:bg-sunken hover:text-comment-ink"
      aria-label={sent ? `Sent to ${agent}` : `Send to ${agent}`}
      title={`Send to ${agent}`}
      disabled={sending}
      onClick={() => void press()}
    >
      {sent ? (
        <Check className="size-3.5 text-add-ink" aria-hidden="true" />
      ) : (
        <SendIcon className="size-3.5" aria-hidden="true" />
      )}
    </Button>
  );
}

type SendProps = { comment: CommentView; agent: string; actions: CommentActions };
