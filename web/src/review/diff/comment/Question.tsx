import { Button } from "@/components/ui/button";
import { Prose } from "@/review/Prose";
import { Close } from "./Close";
import { Copy } from "./Copy";
import { Send } from "./Send";
import { UseAsPending } from "./UseAsPending";
import { span } from "./quoted";
import { linesOf } from "@/api";
import type { CommentActions } from "@/hooks/useComments";
import type { CommentView } from "@/api";

/** The line a thread opens on: where it is, what was asked, and what can be
 * done to the thread as a whole — send it, copy it, close it, fold it. */
export function Question({ comment, actions, minimized, offerUse, discardsPending }: QuestionProps) {
  return (
    <div className="flex items-start gap-2 md:gap-3">
      <span className="lbl mt-0.5 shrink-0 font-mono text-xs text-comment-ink">
        {threadLabel(comment)}
      </span>

      <div className="min-w-0 flex-1">
        {minimized ? (
          <p className="body truncate font-sans text-sm text-ink-muted">{comment.body}</p>
        ) : (
          <Prose className="body font-sans text-sm leading-relaxed text-ink-soft">{comment.body}</Prose>
        )}
        {offerUse && !minimized && (
          <UseAsPending onUse={() => actions.writePending(comment.id, comment.body)} />
        )}
      </div>

      <div className="acts flex shrink-0 items-center gap-0.5">
        {actions.agent && (
          <Send comment={comment} agent={actions.agent} actions={actions} held={held(comment)} />
        )}
        <Copy comment={comment} />
        <Close comment={comment} actions={actions} discardsPending={discardsPending} />
        {/* Folds the thread to its question, for a reader who is done with it
            for now; the activity box opens it again when something arrives. */}
        <Button
          variant="ghost"
          size="icon-xs"
          className="minimize font-mono text-comment-faint hover:bg-sunken hover:text-comment-ink"
          aria-label={minimized ? "Expand this thread" : "Minimize this thread"}
          aria-expanded={!minimized}
          title={minimized ? "Expand this thread" : "Minimize this thread"}
          onClick={() => actions.toggleMinimized(comment.id)}
        >
          {minimized ? "+" : "–"}
        </Button>
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

type QuestionProps = {
  comment: CommentView;
  actions: CommentActions;
  minimized: boolean;
  /** Whether the question itself may become the pending comment. */
  offerUse: boolean;
  /** The thread's pending comment is still on the pull request, and goes too. */
  discardsPending: boolean;
};

/** The lines a thread is on, or what it is about when it sits on no line. */
function threadLabel(comment: CommentView): string {
  const lines = linesOf(comment);
  return lines ? span(lines, "–") : "Thread";
}
