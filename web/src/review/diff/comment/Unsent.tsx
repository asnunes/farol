import { useState } from "react";
import { CircleAlert } from "lucide-react";
import { Button } from "@/components/ui/button";

/** What the reviewer wrote and did not send, and the way to send it. */
export function Unsent({ agent, reply, onSend }: UnsentProps) {
  const [sending, setSending] = useState(false);

  return (
    <p className="unsent flex items-center gap-1.5 font-sans text-xs text-comment-ink" role="status">
      <CircleAlert className="size-3.5" aria-hidden="true" />
      {reply ? `Your reply was not sent to ${agent}.` : `Not sent to ${agent}.`}
      <Button
        variant="link"
        size="xs"
        className="h-auto p-0 text-xs text-comment-ink underline"
        disabled={sending}
        onClick={async () => {
          setSending(true);
          await onSend();
          setSending(false);
        }}
      >
        Send it now
      </Button>
    </p>
  );
}

type UnsentProps = {
  agent: string;
  /** Whether what is unsent is a reply, rather than the comment itself. */
  reply: boolean;
  onSend: () => Promise<boolean>;
};
