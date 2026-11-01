import { useEffect } from "react";
import { CircleCheck } from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";

/** How long a done thing is said before it goes. */
const SHOWN_FOR = 4000;

/** Something the reader asked for that went through.
 *
 * Where a failure is said, over the review, so the bar it was asked from keeps
 * its shape. Unlike a failure it goes on its own: missing it costs nothing,
 * since what it reports already happened. */
export function Toast({ what, onGone }: ToastProps) {
  useEffect(() => {
    const going = window.setTimeout(onGone, SHOWN_FOR);
    return () => window.clearTimeout(going);
  }, [what, onGone]);

  return (
    <Alert
      role="status"
      className="toast fixed right-4 bottom-24 left-4 z-50 flex w-auto max-w-[34rem] items-center gap-3 border-rule bg-surface shadow-lg sm:left-auto"
    >
      <CircleCheck className="text-add-ink" />
      <AlertDescription className="font-sans text-sm text-ink">{what}</AlertDescription>
    </Alert>
  );
}

type ToastProps = {
  what: string;
  onGone: () => void;
};
