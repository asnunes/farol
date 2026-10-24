import { cn } from "@/lib/utils";
import { ago, type Update } from "@/hooks/useActivity";
import { Row } from "./Row";

/** Something an agent finished, with a dot while it is new. */
export function UpdateRow({ item, onJump }: UpdateRowProps) {
  return (
    <Row
      mark={
        <span
          className={cn("mt-1.5 size-[7px] rounded-full", item.read ? "border border-rule-strong" : "bg-highlight")}
          aria-label={item.read ? undefined : "new"}
        />
      }
      line={`${item.who} ${item.what}`}
      thread={item.thread}
      when={ago(item.at)}
      excerpt={item.excerpt}
      quiet={item.read}
      onJump={onJump}
    />
  );
}

type UpdateRowProps = { item: Update; onJump: () => void };
