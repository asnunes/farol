import { Check, Copy, RotateCcw, Trash2 } from "lucide-react";
import Markdown from "react-markdown";
import { useCopy } from "@/hooks/useCopy";
import { cn } from "@/lib/utils";
import type { CommentActions } from "@/hooks/useComments";
import type { CommentView } from "@/api";

/** What the reviewer wrote, under the last line it is about.
 *
 * Anchored and railed the same way the session's line notes are, and coloured
 * differently: one is the author explaining, the other is the reader asking, and
 * the page has to say which without a legend. */
export function LineComments({ comments, actions }: LineCommentsProps) {
  return comments.map((comment) => (
    <div
      key={comment.id}
      className={cn(
        "comment commented flex items-start gap-3 border-b border-comment-rule bg-comment-bg py-2 pr-6 pl-16",
        comment.resolved && "resolved opacity-55",
      )}
    >
      <span className="lbl mt-0.5 shrink-0 font-mono text-xs text-comment-ink">
        {comment.from === comment.to ? comment.from : `${comment.from}–${comment.to}`}
        {comment.resolved && " · closed"}
      </span>

      {/* `prose` is not in play here: the body is a sentence or two, and a
          typography reset would give a lone paragraph margins it does not
          need. Only the marks that actually turn up in a review are styled. */}
      <div className="body min-w-0 flex-1 font-sans text-sm leading-relaxed text-ink-soft [&_a]:text-accent [&_a]:underline [&_code]:rounded [&_code]:bg-sunken [&_code]:px-1 [&_code]:font-mono [&_code]:text-[0.8125rem] [&_li]:ml-4 [&_li]:list-disc [&_p+p]:mt-2 [&_pre]:mt-2 [&_pre]:overflow-x-auto [&_pre]:rounded [&_pre]:bg-sunken [&_pre]:p-2 [&_pre_code]:bg-transparent [&_pre_code]:p-0">
        <Markdown>{comment.body}</Markdown>
      </div>

      <div className="acts flex shrink-0 gap-0.5">
        <CopyComment comment={comment} />
        <Act
          label={comment.resolved ? "Reopen this comment" : "Close this comment"}
          onClick={() => void actions.resolve(comment.id, !comment.resolved)}
        >
          {comment.resolved ? (
            <RotateCcw className="size-3.5" aria-hidden="true" />
          ) : (
            <Check className="size-3.5" aria-hidden="true" />
          )}
        </Act>
        <Act label="Delete this comment" onClick={() => void actions.remove(comment.id)}>
          <Trash2 className="size-3.5" aria-hidden="true" />
        </Act>
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

function CopyComment({ comment }: { comment: CommentView }) {
  const { copied, copy } = useCopy();

  return (
    <Act
      label={copied ? "Comment copied" : "Copy this comment"}
      onClick={() => void copy(quoted(comment))}
    >
      {copied ? (
        <Check className="size-3.5 text-add-ink" aria-hidden="true" />
      ) : (
        <Copy className="size-3.5" aria-hidden="true" />
      )}
    </Act>
  );
}

function Act({ label, onClick, children }: ActProps) {
  return (
    <button
      className="grid size-6 cursor-pointer place-items-center rounded text-faint transition-colors hover:bg-sunken hover:text-comment-ink focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
      aria-label={label}
      title={label}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

type LineCommentsProps = { comments: CommentView[]; actions: CommentActions };

type ActProps = { label: string; onClick: () => void; children: React.ReactNode };
