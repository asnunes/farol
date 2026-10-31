import { useEffect, useRef, useState } from "react";
import { Check, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { SyncedView } from "@/api";

/** How long what the sync did stays beside the button. */
const SAID_FOR = 4000;

/** Make the pull request's read marks say what the ones here say.
 *
 * What it did is said beside it for a moment and then goes: it is an answer to
 * a press, not a state of the review. A refusal goes where every other one
 * does, through the caller. */
export function SyncReads({ onSync, blocked }: SyncReadsProps) {
  const [busy, setBusy] = useState(false);
  const [said, setSaid] = useState<string | null>(null);
  const clearing = useRef<number | undefined>(undefined);

  useEffect(() => () => window.clearTimeout(clearing.current), []);

  async function sync() {
    setBusy(true);
    const synced = await onSync();
    setBusy(false);
    if (!synced) return;
    setSaid(described(synced));
    window.clearTimeout(clearing.current);
    clearing.current = window.setTimeout(() => setSaid(null), SAID_FOR);
  }

  return (
    <>
      <Button
        variant="ghost"
        size="icon-xs"
        className="sync-reads text-faint hover:bg-sunken hover:text-ink"
        aria-label="Sync read marks to GitHub"
        title={
          blocked
            ? "The pull request is not at the commit you read: the line below says why"
            : "Sync read marks to GitHub"
        }
        disabled={busy || blocked}
        onClick={() => void sync()}
      >
        {said ? (
          <Check className="size-3.5 text-add-ink" aria-hidden="true" />
        ) : (
          <RefreshCw className={busy ? "size-3.5 animate-spin" : "size-3.5"} aria-hidden="true" />
        )}
      </Button>
      {said && (
        <span className="synced font-sans text-xs text-ink-muted" role="status">
          {said}
        </span>
      )}
    </>
  );
}

/** What the sync did, in the fewest words that say it. */
function described(synced: SyncedView): string {
  const parts = [`${synced.marked} marked`];
  if (synced.unmarked > 0) parts.push(`${synced.unmarked} unmarked`);
  return `${parts.join(", ")} on #${synced.number}`;
}

type SyncReadsProps = {
  /** What the sync did, or null when it did not happen. */
  onSync: () => Promise<SyncedView | null>;
  /** The pull request shows another commit, so there is nothing true to send. */
  blocked: boolean;
};
