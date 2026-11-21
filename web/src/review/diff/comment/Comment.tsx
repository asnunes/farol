import { cn } from "@/lib/utils";
import { Conversation } from "./Conversation";
import { Question } from "./Question";
import type { DrawnThreadActions } from "@/hooks/useComments";
import type { CommentView } from "@/api";

/** One thread: the question the reviewer left, under the last line it is
 * about, the conversation it started, and — in a peer review — the comment for
 * the pull request that came out of it.
 *
 * Anchored and railed the same way the session's line notes are, and coloured
 * differently: one is the author explaining, the other is the reader asking, and
 * the page has to say which without a legend.
 *
 * What is decided here is what both halves need to agree on; each half draws
 * itself. */
export function Comment({ comment, actions }: CommentProps) {
  const remote = actions.pendingOf(comment.id);
  // Once there is a pending comment, Edit and Regenerate are how it changes;
  // offering every turn as a replacement too would be a third way.
  const offerUse = actions.pendingOffered && !comment.waiting && comment.pending === null;
  const minimized = actions.isMinimized(comment.id);
  const onCode = comment.about.kind === "lines";

  return (
    <div
      data-thread={comment.id}
      className={cn(
        "comment commented flex scroll-mt-32 flex-col gap-2 border-b border-comment-rule bg-comment-bg py-2 transition-[box-shadow,background-color] duration-300",
        // Under code, in from the gutter, so the question starts where the
        // code does. Under a lens or a block's text there is no gutter to
        // clear, and the same indent only pushed the question away from it.
        onCode ? "pr-4 pl-8 md:pr-6 md:pl-16" : "px-3",
      )}
    >
      <Question
        comment={comment}
        actions={actions}
        minimized={minimized}
        offerUse={offerUse}
      />
      {!minimized && (
        <Conversation comment={comment} actions={actions} remote={remote} offerUse={offerUse} under={onCode} />
      )}
    </div>
  );
}

type CommentProps = { comment: CommentView; actions: DrawnThreadActions };
