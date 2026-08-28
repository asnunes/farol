import { Check, Copy as CopyIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useCopy } from "@/hooks/useCopy";
import { quoted } from "./quoted";
import type { CommentView } from "@/api";

/** Put the whole comment on the clipboard: path, lines and text. */
export function Copy({ comment }: CopyProps) {
  const { copied, copy } = useCopy();

  return (
    <Button
      variant="ghost"
      size="icon-xs"
      className="text-faint hover:bg-sunken hover:text-comment-ink"
      aria-label={copied ? "Comment copied" : "Copy this comment"}
      title="Copy this comment"
      onClick={() => void copy(quoted(comment))}
    >
      {copied ? (
        <Check className="size-3.5 text-add-ink" aria-hidden="true" />
      ) : (
        <CopyIcon className="size-3.5" aria-hidden="true" />
      )}
    </Button>
  );
}

type CopyProps = { comment: CommentView };
