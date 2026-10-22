import { useState } from "react";
import { MessageSquareReply } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Prose } from "@/review/Prose";
import { Close } from "./Close";
import { Copy } from "./Copy";
import { Published } from "./Published";
import { Replies } from "./Replies";
import { ReplyBox } from "./ReplyBox";
import { Send } from "./Send";
import { Status } from "./Status";
import { Unsent } from "./Unsent";
import { span } from "./quoted";
import type { CommentActions } from "@/hooks/useComments";
import type { CommentView } from "@/api";

/** One question the reviewer left, under the last line it is about, and the
 * conversation it started.
 *
 * Anchored and railed the same way the session's line notes are, and coloured
 * differently: one is the author explaining, the other is the reader asking, and
 * the page has to say which without a legend. */
export function Comment({ comment, actions }: CommentProps) {
  const [replying, setReplying] = useState(false);

  return (
    <div className="comment commented flex flex-col gap-2 border-b border-comment-rule bg-comment-bg py-2 pr-4 pl-8 md:pr-6 md:pl-16">
      <div className="flex items-start gap-2 md:gap-3">
        <span className="lbl mt-0.5 shrink-0 font-mono text-xs text-comment-ink">
          {span(comment, "–")}
        </span>

        <Prose className="body min-w-0 flex-1 font-sans text-sm leading-relaxed text-ink-soft">
          {comment.body}
        </Prose>

        <div className="acts flex shrink-0 items-center gap-0.5">
          <Published comment={comment} />
          {actions.agent && (
            <Send comment={comment} agent={actions.agent} actions={actions} held={held(comment)} />
          )}
          <Copy comment={comment} />
          <Close comment={comment} actions={actions} />
        </div>
      </div>

      {/* Reply sits after the last turn, where the reader is when they decide
          to answer — not up with the actions on the question. While an agent
          owes an answer there is no turn to take: a reply then would land in
          the middle of its work. */}
      <div className="thread flex flex-col gap-2 pl-6 md:pl-10">
        {comment.replies.length > 0 && <Replies replies={comment.replies} />}
        {comment.waiting && <Status waiting={comment.waiting} />}
        {actions.agent && unsent(comment) && (
          <Unsent
            agent={actions.agent}
            reply={comment.replies.length > 0}
            onSend={() => actions.send(comment.id)}
          />
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
          <Button
            variant="ghost"
            size="xs"
            className="opener self-start text-comment-faint hover:bg-sunken hover:text-comment-ink"
            onClick={() => setReplying(true)}
          >
            <MessageSquareReply className="size-3.5" aria-hidden="true" />
            Reply
          </Button>
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
  if (comment.waiting) return `Already sent to ${comment.waiting.on} — waiting for an answer`;
  const last = comment.replies[comment.replies.length - 1];
  if (last?.from === "agent") return `${last.name} answered — reply to continue`;
  return null;
}

/** The reviewer had the last word and it never left: a comment saved with
 * plain Comment, or a reply written but not sent. Said on the comment, because
 * otherwise it looks exactly like one the agent is about to answer.
 *
 * Nothing pending and no answer means it was never sent: a send leaves the
 * comment waiting until the agent replies, and the reply is a turn. */
function unsent(comment: CommentView): boolean {
  if (comment.waiting) return false;
  const last = comment.replies[comment.replies.length - 1];
  return last === undefined || last.from === "reviewer";
}

type CommentProps = { comment: CommentView; actions: CommentActions };
