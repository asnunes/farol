import { useState } from "react";
import { Button } from "@/components/ui/button";

/** Take something already said in the thread as its comment for the pull
 * request, when it is already what the author should read. */
export function UseAsPending({ onUse }: UseAsPendingProps) {
  const [busy, setBusy] = useState(false);

  return (
    <Button
      variant="link"
      size="xs"
      className="use-as-pending h-auto p-0 text-xs text-comment-faint hover:text-comment-ink"
      disabled={busy}
      onClick={async () => {
        setBusy(true);
        await onUse();
        setBusy(false);
      }}
    >
      Use as pending comment
    </Button>
  );
}

type UseAsPendingProps = { onUse: () => Promise<boolean> };
