import { WaitingIcon } from "@/review/diff/comment/WaitingIcon";
import { ago, type Working } from "@/hooks/useActivity";
import { Row } from "./Row";

/** A thread an agent owes something on, marked the way its own badge is. */
export function WorkingRow({ item, onJump }: WorkingRowProps) {
  return (
    <Row
      mark={
        <span className="mt-[3px] text-comment-ink">
          <WaitingIcon waiting={item.waiting} className="size-3" />
        </span>
      }
      line={item.line}
      thread={item.thread}
      when={`since ${ago(item.since).replace(" ago", "")}`}
      excerpt={item.excerpt}
      quiet={false}
      onJump={onJump}
    />
  );
}

type WorkingRowProps = { item: Working; onJump: () => void };
