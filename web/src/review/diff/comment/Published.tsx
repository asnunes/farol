import { ExternalLink } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { CommentView } from "@/api";

/** That this comment has gone, and where it went.
 *
 * Marked rather than removed or greyed: publishing is not answering. The
 * question is still open, it just has a second home now, and the answer will
 * arrive there. What the mark is for is the second send — so the reader can see
 * which of these is about to go again, and which already has. */
export function Published({ comment }: { comment: CommentView }) {
  if (comment.published === null) return null;

  return (
    <Button
      asChild
      variant="ghost"
      size="icon-xs"
      className="published text-comment-faint hover:bg-sunken hover:text-comment-ink"
      title="Published — read it on the pull request"
    >
      <a
        href={comment.published}
        target="_blank"
        rel="noreferrer"
        aria-label="Read this comment on the pull request"
      >
        <ExternalLink className="size-3.5" aria-hidden="true" />
      </a>
    </Button>
  );
}
