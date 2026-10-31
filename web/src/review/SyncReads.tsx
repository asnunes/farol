import { useState } from "react";
import { RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { SyncedView } from "@/api";

/** Make the pull request's read marks say what the ones here say.
 *
 * What it did is told by the caller, over the review, and not beside the
 * button: words appearing here pushed the bar about and then took it back. A
 * refusal goes where every other one does. */
export function SyncReads({ onSync, onSynced, blocked }: SyncReadsProps) {
  const [busy, setBusy] = useState(false);

  async function sync() {
    setBusy(true);
    const synced = await onSync();
    setBusy(false);
    if (synced) onSynced(described(synced));
  }

  return (
    <Button
      variant="ghost"
      size="icon-xs"
      className="sync-reads text-faint hover:bg-sunken hover:text-ink"
      aria-label="Sync read marks to GitHub"
      title={
        blocked
          ? "The pull request is not at the commit you read: the mark beside this says why"
          : "Sync read marks to GitHub"
      }
      disabled={busy || blocked}
      onClick={() => void sync()}
    >
      <RefreshCw className={busy ? "size-3.5 animate-spin" : "size-3.5"} aria-hidden="true" />
    </Button>
  );
}

/** What the sync did, in the fewest words that say it. */
function described(synced: SyncedView): string {
  const parts = [`${synced.marked} marked`];
  if (synced.unmarked > 0) parts.push(`${synced.unmarked} unmarked`);
  return `Read marks synced: ${parts.join(", ")} on #${synced.number}`;
}

type SyncReadsProps = {
  /** What the sync did, or null when it did not happen. */
  onSync: () => Promise<SyncedView | null>;
  /** Say what it did, somewhere that does not move the bar. */
  onSynced: (said: string) => void;
  /** The pull request shows another commit, so there is nothing true to send. */
  blocked: boolean;
};
