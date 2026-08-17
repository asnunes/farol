import { useState } from "react";
import { Info, Send } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Send as SendReview } from "./Send";
import { Setup } from "./Setup";
import type { Publishing } from "@/hooks/usePublishing";
import type { CommentView } from "@/api";

/** Sending the review to its pull request, from the top bar.
 *
 * One control with two behaviours, because there is only ever one thing to do:
 * ready, it opens the review; not ready, the (i) beside it opens the panel that
 * says what is missing and lets the reader fix it there. The button stays
 * visible either way — hidden, there would be nothing to explain, and the
 * reader would conclude farol cannot do this at all. */
export function Publish({ publishing, comments, onError }: PublishProps) {
  const [sending, setSending] = useState(false);
  const [explaining, setExplaining] = useState(false);
  const readiness = publishing.readiness;
  const ready = readiness?.state === "ready";

  // Nothing is drawn until the first answer arrives. A button that starts
  // disabled and enables itself a moment later reads as broken, and a button
  // that starts enabled and disables itself is worse.
  if (!readiness) return null;

  const waiting = comments.filter((c) => c.published === null);

  return (
    <div className="publish flex items-center gap-1">
      <Button
        size="xs"
        variant="ghost"
        className="cursor-pointer gap-1.5 font-mono text-xs text-muted hover:bg-sunken hover:text-ink disabled:cursor-default disabled:opacity-50"
        disabled={!ready}
        title={
          ready ? `Send this review to pull request #${readiness.pullRequest}` : undefined
        }
        onClick={() => setSending(true)}
      >
        <Send className="size-3.5" aria-hidden="true" />
        Send review
        {waiting.length > 0 && <span className="text-accent">{waiting.length}</span>}
      </Button>

      {!ready && (
        <Button
          size="icon-xs"
          variant="ghost"
          className="cursor-pointer text-faint hover:bg-sunken hover:text-ink"
          aria-label="Why this review cannot be sent yet"
          onClick={() => setExplaining(true)}
        >
          <Info className="size-3.5" aria-hidden="true" />
        </Button>
      )}

      <SendReview
        open={sending}
        onOpenChange={setSending}
        waiting={waiting.length}
        pullRequest={readiness.pullRequest}
        publish={publishing.publish}
        onError={onError}
      />
      <Setup
        open={explaining}
        onOpenChange={setExplaining}
        readiness={readiness}
        onCheck={publishing.ask}
        onToken={publishing.saveToken}
      />
    </div>
  );
}

type PublishProps = {
  publishing: Publishing;
  /** Every comment on the review, so the control can count the ones that have
   * not gone yet — the number the reader is about to send. */
  comments: CommentView[];
  onError: (message: string) => void;
};
