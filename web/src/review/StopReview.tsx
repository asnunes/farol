import { useState } from "react";
import { Power } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { api } from "@/api";
import { said } from "@/lib/utils";

/** Stop the server from the page, when the review is done. Beside Submit, at
 * the end of the review bar, because it is the last thing the reviewer does —
 * and quieter than Submit, with a confirmation, because it ends the page. */
export function StopReview({ branch, onStopped }: StopReviewProps) {
  const [open, setOpen] = useState(false);
  const [stopping, setStopping] = useState(false);
  const [failed, setFailed] = useState<string | null>(null);

  async function stop() {
    setStopping(true);
    setFailed(null);
    try {
      await api.stop();
      onStopped();
    } catch (e) {
      setFailed(said(e));
      setStopping(false);
    }
  }

  return (
    <>
      <Button
        size="xs"
        variant="outline"
        className="stop-review gap-1 border-rule text-ink-muted hover:text-ink"
        title="Stop this review"
        onClick={() => setOpen(true)}
      >
        <Power className="size-3.5" aria-hidden="true" />
        Stop
      </Button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="border-rule bg-surface sm:max-w-md">
          <DialogHeader>
            <DialogTitle className="font-sans text-base text-ink">Stop this review?</DialogTitle>
            <DialogDescription className="text-ink-muted">
              The server for <code className="font-mono text-ink">{branch}</code> stops and this page stops updating.
              What you read and wrote is kept; run <code className="font-mono text-ink">farol serve</code> to open it
              again.
            </DialogDescription>
          </DialogHeader>
          {failed && <p className="font-mono text-xs text-del-ink">{failed}</p>}
          <DialogFooter>
            <Button size="xs" variant="ghost" className="text-ink-muted" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button
              size="xs"
              className="bg-del-ink text-surface hover:bg-del-ink/90"
              disabled={stopping}
              onClick={() => void stop()}
            >
              Stop review
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

type StopReviewProps = {
  branch: string;
  /** Called once the server has said it is stopping. */
  onStopped: () => void;
};
