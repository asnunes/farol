import { useEffect, useRef, useState } from "react";
import { Check, Copy } from "lucide-react";
import Markdown from "react-markdown";
import { Button } from "@/components/ui/button";
import { useCopy } from "@/hooks/useCopy";
import type { CommentActions } from "@/hooks/useComments";
import type { CommentView } from "@/api";

/** What the reviewer asked, under the last line it is about.
 *
 * Anchored and railed the same way the session's line notes are, and coloured
 * differently: one is the author explaining, the other is the reader asking, and
 * the page has to say which without a legend. */
export function LineComments({ comments, actions }: LineCommentsProps) {
  return comments.map((comment) => (
    <div
      key={comment.id}
      className="comment commented flex items-start gap-3 border-b border-comment-rule bg-comment-bg py-2 pr-6 pl-16"
    >
      <span className="lbl mt-0.5 shrink-0 font-mono text-xs text-comment-ink">
        {comment.from === comment.to ? comment.from : `${comment.from}–${comment.to}`}
      </span>

      {/* `prose` is not in play here: the body is a sentence or two, and a
          typography reset would give a lone paragraph margins it does not
          need. Only the marks that actually turn up in a review are styled. */}
      <div className="body min-w-0 flex-1 font-sans text-sm leading-relaxed text-ink-soft [&_a]:text-accent [&_a]:underline [&_code]:rounded [&_code]:bg-sunken [&_code]:px-1 [&_code]:font-mono [&_code]:text-[0.8125rem] [&_li]:ml-4 [&_li]:list-disc [&_p+p]:mt-2 [&_pre]:mt-2 [&_pre]:overflow-x-auto [&_pre]:rounded [&_pre]:bg-sunken [&_pre]:p-2 [&_pre_code]:bg-transparent [&_pre_code]:p-0">
        <Markdown>{comment.body}</Markdown>
      </div>

      <div className="acts flex shrink-0 items-center gap-0.5">
        <CopyComment comment={comment} />
        <CloseComment comment={comment} actions={actions} />
      </div>
    </div>
  ));
}

/** Where it was and what it said, in one paste — enough to be answered
 * somewhere the file is not open. The same shape `Comment::quoted` writes on
 * the terminal side, so a comment reads the same however it was copied. */
export function quoted(comment: CommentView): string {
  const lines = comment.from === comment.to ? `${comment.from}` : `${comment.from}-${comment.to}`;
  return `${comment.path}:${lines}\n\n${comment.body.trim()}`;
}

/** How long the button stays armed. Long enough to press it twice on purpose,
 * short enough that one left armed by accident is safe again by the time
 * anybody comes back to it. */
const ARMED_FOR = 4000;

/** Closing answers the comment and drops it, in that order and with no undo:
 * the file is deleted, and it was never in git to be recovered from.
 *
 * So it asks first. Two presses rather than a dialog, because a dialog over a
 * diff covers the code the question is about — and the button that copies sits
 * a few pixels away, which is the misclick worth guarding against. */
function CloseComment({ comment, actions }: CloseCommentProps) {
  const [armed, setArmed] = useState(false);
  const disarming = useRef<number | undefined>(undefined);

  useEffect(() => () => window.clearTimeout(disarming.current), []);

  function press() {
    if (armed) {
      window.clearTimeout(disarming.current);
      void actions.close(comment.id);
      return;
    }
    setArmed(true);
    disarming.current = window.setTimeout(() => setArmed(false), ARMED_FOR);
  }

  return (
    <Button
      variant="ghost"
      size={armed ? "xs" : "icon-xs"}
      className={
        armed
          ? "closer cursor-pointer bg-comment-ink text-[0.6875rem] text-surface hover:bg-comment-ink/90 hover:text-surface"
          : "closer cursor-pointer text-faint hover:bg-sunken hover:text-comment-ink"
      }
      aria-label={armed ? "Press again to close this comment" : "Close this comment"}
      title={armed ? "Press again — closing removes it" : "Close this comment"}
      onClick={press}
      onBlur={() => setArmed(false)}
    >
      <Check className="size-3.5" aria-hidden="true" />
      {armed && "Sure?"}
    </Button>
  );
}

function CopyComment({ comment }: { comment: CommentView }) {
  const { copied, copy } = useCopy();

  return (
    <Button
      variant="ghost"
      size="icon-xs"
      className="cursor-pointer text-faint hover:bg-sunken hover:text-comment-ink"
      aria-label={copied ? "Comment copied" : "Copy this comment"}
      title="Copy this comment"
      onClick={() => void copy(quoted(comment))}
    >
      {copied ? (
        <Check className="size-3.5 text-add-ink" aria-hidden="true" />
      ) : (
        <Copy className="size-3.5" aria-hidden="true" />
      )}
    </Button>
  );
}

type LineCommentsProps = { comments: CommentView[]; actions: CommentActions };

type CloseCommentProps = { comment: CommentView; actions: CommentActions };
