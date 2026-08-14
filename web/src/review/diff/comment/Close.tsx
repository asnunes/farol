import { useEffect, useRef, useState } from "react";
import { Check } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { CommentActions } from "@/hooks/useComments";
import type { CommentView } from "@/api";

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
export function Close({ comment, actions }: CloseProps) {
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

type CloseProps = { comment: CommentView; actions: CommentActions };
