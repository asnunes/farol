import Markdown from "react-markdown";
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

      {/* `prose` is not in play here: the body is a sentence or two, and a
          typography reset would give a lone paragraph margins it does not
          need. Only the marks that actually turn up in a review are styled. */}
      <div className="body min-w-0 flex-1 font-sans text-sm leading-relaxed text-ink-soft [&_a]:text-accent [&_a]:underline [&_code]:rounded [&_code]:bg-sunken [&_code]:px-1 [&_code]:font-mono [&_code]:text-[0.8125rem] [&_li]:ml-4 [&_li]:list-disc [&_p+p]:mt-2 [&_pre]:mt-2 [&_pre]:overflow-x-auto [&_pre]:rounded [&_pre]:bg-sunken [&_pre]:p-2 [&_pre_code]:bg-transparent [&_pre_code]:p-0">
        <Markdown>{comment.body}</Markdown>
      </div>

      <div className="acts flex shrink-0 items-center gap-0.5">
        <Published comment={comment} />
        <Copy comment={comment} />
        <Close comment={comment} actions={actions} />
      </div>
    </div>
  );
}

type CommentProps = { comment: CommentView; actions: CommentActions };
