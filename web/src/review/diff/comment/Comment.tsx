import { Prose } from "@/review/Prose";
import { Close } from "./Close";
import { Copy } from "./Copy";
import { Published } from "./Published";
import { span } from "./quoted";
import type { CommentActions } from "@/hooks/useComments";
import type { CommentView } from "@/api";

/** One question the reviewer left, under the last line it is about.
 *
 * Anchored and railed the same way the session's line notes are, and coloured
 * differently: one is the author explaining, the other is the reader asking, and
 * the page has to say which without a legend. */
export function Comment({ comment, actions }: CommentProps) {
  return (
    <div className="comment commented flex items-start gap-3 border-b border-comment-rule bg-comment-bg py-2 pr-6 pl-16">
      <span className="lbl mt-0.5 shrink-0 font-mono text-xs text-comment-ink">
        {span(comment, "–")}
      </span>

      <Prose className="body min-w-0 flex-1 font-sans text-sm leading-relaxed text-ink-soft">
        {comment.body}
      </Prose>

      <div className="acts flex shrink-0 items-center gap-0.5">
        <Published comment={comment} />
        <Copy comment={comment} />
        <Close comment={comment} actions={actions} />
      </div>
    </div>
  );
}

type CommentProps = { comment: CommentView; actions: CommentActions };
