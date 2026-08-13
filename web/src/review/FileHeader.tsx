import { ChevronDown, ChevronRight, MessageSquare } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { CopyPath } from "@/review/CopyPath";
import { commentsOn } from "@/review/diff/line";
import { splitPath } from "@/lib/path";
import type { CommentView, FileView } from "@/api";

/** The file being read: the tick on the left, the path, the churn on the right.
 *
 * Sticks to the top of the pane while the file scrolls under it, so marking a
 * file read never means scrolling back up to find the box. The block band above
 * it does not stick: it is read once, at the start of the block, and pinning it
 * would spend the top of the screen on prose the reader has already finished. */
export function FileHeader({ file, open, onToggleOpen, onToggleViewed, comments }: FileHeaderProps) {
  const { dir, name } = splitPath(file.path);
  const unanswered = commentsOn(comments, file.path).length;

  return (
    <div className="filehead sticky top-0 z-10 flex items-center justify-between gap-4 border-b border-rule bg-surface px-6 py-2.5">
      <div className="left flex min-w-0 items-center gap-3">
        <button
          className="fold grid size-5 shrink-0 cursor-pointer place-items-center rounded text-faint transition-colors hover:text-ink focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
          aria-expanded={open}
          aria-label={open ? "Collapse this file" : "Expand this file"}
          title={open ? "Collapse this file" : "Expand this file"}
          onClick={onToggleOpen}
        >
          {open ? (
            <ChevronDown className="size-3.5" aria-hidden="true" />
          ) : (
            <ChevronRight className="size-3.5" aria-hidden="true" />
          )}
        </button>

        <button
          className="markbox grid size-5 shrink-0 cursor-pointer place-items-center rounded border border-rule-strong text-xs text-transparent transition-colors hover:border-accent focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none aria-pressed:border-accent aria-pressed:bg-accent aria-pressed:text-surface"
          aria-pressed={file.viewed}
          title="Mark as read — key ;"
          onClick={onToggleViewed}
        >
          <span aria-hidden="true">✓</span>
        </button>

        <div className="path truncate font-mono text-sm font-semibold text-ink">
          <span className="dir font-normal text-faint">{dir}</span>
          {name}
        </div>

        {/* Keyed by path so moving to another file cannot inherit the tick from
            the one before it. */}
        <CopyPath key={file.path} path={file.path} />

        {/* Only worth showing when the file belongs to more than one block —
            otherwise the tag says what the block band already said. */}
        {file.tags.length > 1 && (
          <div className="tags flex shrink-0 gap-1">
            {file.tags.map((t) => (
              <Badge
                key={t}
                variant="secondary"
                className="tag border-transparent bg-sunken font-mono text-[0.6875rem] font-normal text-muted"
              >
                {t}
              </Badge>
            ))}
          </div>
        )}
      </div>

      <div className="right flex shrink-0 items-center gap-4">
        {/* Marking a file read folds it away. Without this the questions still
            waiting for an answer would fold away with it. */}
        {unanswered > 0 && (
          <span
            className="open flex items-center gap-1 font-mono text-xs text-comment-ink"
            title={`${unanswered} comment${unanswered > 1 ? "s" : ""} still open`}
          >
            <MessageSquare className="size-3.5" aria-hidden="true" />
            {unanswered}
          </span>
        )}

        <div className="churn font-mono text-xs">
          <span className="a text-add-ink">+{file.additions}</span>{" "}
          <span className="d text-del-ink">−{file.deletions}</span>
        </div>
      </div>
    </div>
  );
}

type FileHeaderProps = {
  file: FileView;
  open: boolean;
  onToggleOpen: () => void;
  onToggleViewed: () => void;
  comments: CommentView[];
};
