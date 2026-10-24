import { WaitingIcon } from "./WaitingIcon";
import { waitingLine } from "./waiting";
import type { WaitingView } from "@/api";

/** What the agent owes, as far as farol was told.
 *
 * Asked is all farol knows on its own; started is the agent saying so with
 * `farol thread ack`. Nothing finer exists to show: neither harness reports
 * whether it has started. Which thing it owes is said too, because a thread
 * waiting for a pending comment and one waiting for an answer look alike. */
export function Status({ waiting }: StatusProps) {
  return (
    <p className="status flex items-center gap-1.5 font-sans text-xs text-comment-ink" role="status">
      <WaitingIcon waiting={waiting} />
      {waitingLine(waiting)}
    </p>
  );
}

type StatusProps = { waiting: WaitingView };
