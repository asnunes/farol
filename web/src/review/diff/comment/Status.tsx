import { LoaderCircle, Send } from "lucide-react";
import type { WaitingView } from "@/api";

/** What the agent owes, as far as farol was told.
 *
 * "Sent" is all farol knows on its own; "working" is the agent saying so with
 * `farol thread ack`. Nothing finer exists to show: neither harness reports
 * whether it has started. Which thing it owes is said too, because a thread
 * waiting for a pending comment and one waiting for an answer look alike. */
export function Status({ waiting }: StatusProps) {
  const working = waiting.stage === "working";
  return (
    <p className="status flex items-center gap-1.5 font-sans text-xs text-comment-ink" role="status">
      {working ? (
        <LoaderCircle className="size-3.5 animate-spin" aria-hidden="true" />
      ) : (
        <Send className="size-3.5" aria-hidden="true" />
      )}
      {waiting.purpose === "pending"
        ? working
          ? `${waiting.on} is writing a pending comment`
          : `Sent to ${waiting.on} · waiting for a pending comment`
        : working
          ? `${waiting.on} is working on it`
          : `Sent to ${waiting.on} · waiting for an answer`}
    </p>
  );
}

type StatusProps = { waiting: WaitingView };
