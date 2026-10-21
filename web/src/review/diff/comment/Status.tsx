import { LoaderCircle, Send } from "lucide-react";
import type { WaitingView } from "@/api";

/** What the agent owes, as far as farol was told.
 *
 * "Sent" is all farol knows on its own; "working" is the agent saying so with
 * `farol comment ack`. Nothing finer exists to show: neither harness reports
 * whether it has started. */
export function Status({ waiting }: StatusProps) {
  const working = waiting.stage === "working";
  return (
    <p className="status flex items-center gap-1.5 font-sans text-xs text-comment-ink" role="status">
      {working ? (
        <LoaderCircle className="size-3.5 animate-spin" aria-hidden="true" />
      ) : (
        <Send className="size-3.5" aria-hidden="true" />
      )}
      {working ? `${waiting.on} is working on it` : `Sent to ${waiting.on} · waiting for an answer`}
    </p>
  );
}

type StatusProps = { waiting: WaitingView };
