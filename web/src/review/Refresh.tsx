import type { RefreshReason } from "@/hooks/useReview";
import { Button } from "@/components/ui/button";

/** Announces a changed branch or map, and takes it when clicked.
 *
 * Announced rather than applied on its own: a map that changes while somebody
 * is halfway through moves the blocks and the file they are reading. */
export function Refresh({ reason, onRefresh }: RefreshProps) {
  return (
    <Button
      variant="ghost"
      size="xs"
      className="refresh rounded-full bg-highlight-dim font-mono text-highlight hover:bg-highlight-dim hover:opacity-80"
      onClick={onRefresh}
      title={reason === "map" ? "A newer map was written. Click to read it." : "The current branch changed. Click to refresh the review."}
    >
      {reason === "map" ? "new map" : "branch changed"} · refresh
    </Button>
  );
}

type RefreshProps = { reason: RefreshReason; onRefresh: () => void };
