import { useState } from "react";
import { MessageSquareReply } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Generate } from "./Generate";
import { Pending } from "./Pending";
import { Replies } from "./Replies";
import { ReplyBox } from "./ReplyBox";
import { Stage } from "./Stage";
import { Status } from "./Status";
import { Unsent } from "./Unsent";
import type { CommentActions } from "@/hooks/useComments";
import type { CommentView, RemotePendingView } from "@/api";

/** Everything under the question: the turns, what the agent owes, the
 * thread's pending comment, and the way to take the next turn.
 *
 * Reply sits after the last turn, where the reader is when they decide to
 * answer — not up with the actions on the question. While an agent owes
 * something there is no turn to take: a reply then would land in the middle of
 * its work. */
export function Conversation({ comment, actions, remote, offerUse }: ConversationProps) {
  const [replying, setReplying] = useState(false);
  const peer = actions.pendingOffered;

  return (
    <div className="thread flex flex-col gap-2 pl-6 md:pl-10">
      {comment.replies.length > 0 && (
        <Replies
          replies={comment.replies}
          onUse={offerUse ? (body) => actions.writePending(comment.id, body) : undefined}
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
            variant="outline"
            size="xs"
            className="opener h-6 px-2 font-sans"
            onClick={() => setReplying(true)}
          >
            <MessageSquareReply className="size-3.5" aria-hidden="true" />
            Reply
          </Button>
          {/* Once there is a pending comment, a new one is Regenerate, on the
              comment itself. */}
          {peer && actions.agent && comment.pending === null && (
            <Generate
              label="Generate pending comment"
              onGenerate={(note) => actions.generate(comment.id, note)}
            />
          )}
        </div>
      )}
      {peer && <Stage thread={comment} remote={remote} />}
    </div>
  );
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

type ConversationProps = {
  comment: CommentView;
  actions: CommentActions;
  /** The thread's pending comment as GitHub has it, once GitHub has answered. */
  remote: RemotePendingView | undefined;
  /** Whether a turn may become the pending comment. */
  offerUse: boolean;
};
