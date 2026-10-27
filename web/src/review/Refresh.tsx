import { RefreshCw } from "lucide-react";
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
      title={TITLES[reason]}
    >
      <RefreshCw className="size-3" aria-hidden="true" />
      {LABELS[reason]}
    </Button>
  );
}

const LABELS: Record<RefreshReason, string> = {
  map: "new map available",
  head: "branch changed",
  lenses: "new lenses written",
};

const TITLES: Record<RefreshReason, string> = {
  map: "A newer map was written. Click to read it.",
  head: "The current branch changed. Click to refresh the review.",
  lenses: "The session wrote lenses. Click to read them.",
};

type RefreshProps = { reason: RefreshReason; onRefresh: () => void };
