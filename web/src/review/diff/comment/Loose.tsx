import { Pending } from "./Pending";
import type { DrawnThreadActions } from "@/hooks/useComments";
import type { LoosePendingView } from "@/api";

/** A comment in the pending review with no thread here pointing at it: still
 * what goes when the review is submitted, so still drawn where it will land —
 * under its line, or above the code for one on the file as a whole. */
export function Loose({ one, actions }: LooseProps) {
  return (
    <div
      data-pending={one.id}
      className="comment commented border-b border-comment-rule bg-comment-bg py-2 pr-4 pl-8 md:pr-6 md:pl-16"
    >
      <Pending
        id={one.id}
        thread={null}
        remote={{ state: "pending", body: one.body, url: null }}
        actions={actions}
        busy={false}
      />
    </div>
  );
}

type LooseProps = { one: LoosePendingView; actions: DrawnThreadActions };
