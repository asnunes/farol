import { Comment } from "@/review/diff/comment/Comment";
import type { CommentActions } from "@/hooks/useComments";
import type { CommentView } from "@/api";

/** Threads whose lines the diff no longer shows, at the end of the block their
 * file is read in — or, with `heading`, in a block of their own at the end of
 * the page, for files that left the review.
 *
 * Drawn whole and answered like any other: the conversation is still going,
 * often because the agent's fix for the thread is what moved its lines. Only
 * the comment for the pull request is not offered, because GitHub takes one
 * only on a line of the diff. */
export function DriftedThreads({ threads, actions, heading }: DriftedThreadsProps) {
  if (threads.length === 0) return null;
  const offDiff: CommentActions = { ...actions, pendingOffered: false };

  return (
    <section className={heading ? "drifted standalone mt-6 md:mt-10" : "drifted"}>
      {heading ? (
        <div className="blockbar border-y border-rule bg-surface px-4 py-3 md:px-6 md:py-4">
          <h2 className="font-sans text-lg font-semibold text-ink">{heading}</h2>
          <p className="mt-2 max-w-[68ch] font-serif text-[0.9375rem] leading-relaxed text-ink-soft">
            The files these threads were about are no longer in the review. They can still be answered and
            closed.
          </p>
        </div>
      ) : (
        <p className="border-b border-comment-rule bg-comment-bg px-4 pt-3 pb-1 font-sans text-xs text-comment-faint md:px-6">
          No longer on the diff — these lines changed since the thread was opened.
        </p>
      )}
      {threads.map((thread) => (
        <div key={thread.id}>
          <p className="border-t border-comment-rule bg-comment-bg px-8 pt-2 font-mono text-xs text-comment-faint md:px-16">
            {thread.at}
          </p>
          <Comment comment={thread} actions={offDiff} />
        </div>
      ))}
    </section>
  );
}

type DriftedThreadsProps = {
  threads: CommentView[];
  actions: CommentActions;
  /** Set for the block of their own; absent at the end of a block. */
  heading?: string;
};
