import { useState } from "react";
import { MessageSquareReply } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Prose } from "@/review/Prose";
import { Close } from "./Close";
import { Copy } from "./Copy";
import { Generate } from "./Generate";
import { Pending } from "./Pending";
import { Replies } from "./Replies";
import { ReplyBox } from "./ReplyBox";
import { Send } from "./Send";
import { Status } from "./Status";
import { Unsent } from "./Unsent";
import { UseAsPending } from "./UseAsPending";
import { span } from "./quoted";
import type { CommentActions } from "@/hooks/useComments";
import type { CommentView } from "@/api";

/** One thread: the question the reviewer left, under the last line it is
 * about, the conversation it started, and — in a peer review — the comment for
 * the pull request that came out of it.
 *
 * Anchored and railed the same way the session's line notes are, and coloured
 * differently: one is the author explaining, the other is the reader asking, and
 * the page has to say which without a legend. */
export function Comment({ comment, actions }: CommentProps) {
  const [replying, setReplying] = useState(false);
  const remote = actions.pendingOf(comment.id);
  const peer = actions.pendingOffered;
  // Only a comment still pending on the pull request goes when the thread
  // does; one that went with the review stays there whatever happens here.
  const discardsPending = comment.pending !== null && remote?.state === "pending";
  const use = (body: string) => actions.writePending(comment.id, body);

  return (
    <div className="comment commented flex flex-col gap-2 border-b border-comment-rule bg-comment-bg py-2 pr-4 pl-8 md:pr-6 md:pl-16">
      <div className="flex items-start gap-2 md:gap-3">
        <span className="lbl mt-0.5 shrink-0 font-mono text-xs text-comment-ink">
          {span(comment, "–")}
        </span>

        <div className="min-w-0 flex-1">
          <Prose className="body font-sans text-sm leading-relaxed text-ink-soft">{comment.body}</Prose>
          {peer && !comment.waiting && comment.pending === null && (
            <UseAsPending onUse={() => use(comment.body)} />
          )}
        </div>

        <div className="acts flex shrink-0 items-center gap-0.5">
          {actions.agent && (
            <Send comment={comment} agent={actions.agent} actions={actions} held={held(comment)} />
          )}
          <Copy comment={comment} />
          <Close comment={comment} actions={actions} discardsPending={discardsPending} />
        </div>
      </div>

      {/* Reply sits after the last turn, where the reader is when they decide
          to answer — not up with the actions on the question. While an agent
          owes something there is no turn to take: a reply then would land in
          the middle of its work. */}
      <div className="thread flex flex-col gap-2 pl-6 md:pl-10">
        {comment.replies.length > 0 && (
          <Replies
            replies={comment.replies}
            onUse={peer && !comment.waiting ? use : undefined}
          />
        )}
        {comment.waiting && <Status waiting={comment.waiting} />}
        {actions.agent && unsent(comment) && (
          <Unsent
            agent={actions.agent}
            reply={comment.replies.length > 0}
            onSend={() => actions.send(comment.id)}
          />
        )}
        {comment.pending !== null && (
          <Pending comment={comment} remote={remote} actions={actions} busy={comment.waiting !== null} />
        )}
        {replying ? (
          <ReplyBox
            agent={actions.agent}
            onReply={async (body, andSend) => {
              await actions.reply(comment.id, body, andSend);
              setReplying(false);
            }}
            onCancel={() => setReplying(false)}
          />
        ) : comment.waiting ? null : (
          <div className="foot flex flex-wrap items-center gap-1">
            <Button
              variant="ghost"
              size="xs"
              className="opener text-comment-faint hover:bg-sunken hover:text-comment-ink"
              onClick={() => setReplying(true)}
            >
              <MessageSquareReply className="size-3.5" aria-hidden="true" />
              Reply
            </Button>
            {/* Once there is a pending comment, a new one is Regenerate, on
                the comment itself. */}
            {peer && actions.agent && comment.pending === null && (
              <Generate
                label="Generate pending comment"
                onGenerate={(note) => actions.generate(comment.id, note)}
              />
            )}
          </div>
        )}
      </div>
    </div>
  );
}

/** Why Send has nothing to do, or null when it has.
 *
 * Sending again what the agent already has, or already answered, would only
 * hand it the same conversation twice. Something new to say goes through
 * Reply & send, which is always there. */
function held(comment: CommentView): string | null {
  if (comment.waiting) return `Already sent to ${comment.waiting.on} — waiting for it`;
  const last = comment.replies[comment.replies.length - 1];
  if (last?.from === "agent") return `${last.name} answered — reply to continue`;
  return null;
}

/** The reviewer had the last word and it never left: a thread saved with
 * plain Comment, or a reply written but not sent. Said on the thread, because
 * otherwise it looks exactly like one the agent is about to answer.
 *
 * A thread whose only purpose was the pull request — written straight as a
 * pending comment — was never meant for the agent, so it is not unsent. */
function unsent(comment: CommentView): boolean {
  if (comment.waiting) return false;
  const last = comment.replies[comment.replies.length - 1];
  if (last === undefined) return comment.pending === null;
  return last.from === "reviewer";
}

type CommentProps = { comment: CommentView; actions: CommentActions };
