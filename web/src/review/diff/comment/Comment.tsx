import { Conversation } from "./Conversation";
import { Question } from "./Question";
import type { CommentActions } from "@/hooks/useComments";
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

  return (
    <div
      data-thread={comment.id}
      className="comment commented flex scroll-mt-32 flex-col gap-2 border-b border-comment-rule bg-comment-bg py-2 pr-4 pl-8 transition-[box-shadow,background-color] duration-300 md:pr-6 md:pl-16"
    >
      <Question
        comment={comment}
        actions={actions}
        minimized={minimized}
        offerUse={offerUse}
      />
      {!minimized && (
        <Conversation comment={comment} actions={actions} remote={remote} offerUse={offerUse} />
      )}
    </div>
  );
}

type CommentProps = { comment: CommentView; actions: CommentActions };
