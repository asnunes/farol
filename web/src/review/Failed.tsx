import { TriangleAlert, X } from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";

/** Something the reader tried that did not happen.
 *
 * Over the review rather than in it: the page keeps its shape, nothing below
 * jumps, and what they were reading when it failed is still where they left it.
 * It sits above the key bar and waits to be dismissed, because a message that
 * fades on its own is one they can miss while looking at the code. */
export function Failed({ what, onClose }: FailedProps) {
  return (
    <Alert
      variant="destructive"
      role="alert"
      className="failed fixed right-4 bottom-24 left-4 z-50 flex w-auto max-w-[34rem] items-start gap-3 border-rule bg-surface shadow-lg sm:left-auto"
    >
      <TriangleAlert />
      <AlertDescription className="min-w-0 flex-1 font-mono text-sm whitespace-pre-wrap">
        {what}
      </AlertDescription>
      <Button
        size="icon-xs"
        variant="ghost"
        className="text-faint hover:text-ink"
        aria-label="Dismiss"
        onClick={onClose}
      >
        <X className="size-3.5" aria-hidden="true" />
      </Button>
    </Alert>
  );
}

type FailedProps = {
  what: string;
  onClose: () => void;
};
