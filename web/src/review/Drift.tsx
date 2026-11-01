import { useState } from "react";
import { CloudUpload, TriangleAlert } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import type { DriftView } from "@/api";

/** Why the pull request is not at the commit under review, and the way out.
 *
 * Only one of the three is a push, and only on your own pull request: pushing
 * somebody else's branch is not one click away, and the other two need a pull
 * or a commit, which belong in a terminal. The push asks first, because it
 * publishes code. */
export function Drift({ drift, mine, number, branch, onPush }: DriftProps) {
  const [open, setOpen] = useState(false);
  const [pushing, setPushing] = useState(false);
  const pushable = drift.kind === "unpushed" && mine;

  async function push() {
    setPushing(true);
    if (await onPush()) setOpen(false);
    setPushing(false);
  }

  return (
    <span className="drift flex items-center font-sans text-xs text-highlight">
      {/* An icon like the rest of the bar, the words in its tooltip. The push
          keeps its count in sight, since it is the one that publishes. */}
      {pushable ? (
        <>
          <Tooltip delayDuration={0}>
            <TooltipTrigger asChild>
              <Button
                size="xs"
                variant="ghost"
                className="h-5 gap-1 px-1.5 text-xs text-highlight hover:bg-sunken hover:text-highlight"
                aria-label={`Push ${commits(drift.commits)}`}
                onClick={() => setOpen(true)}
              >
                <CloudUpload className="size-3.5" aria-hidden="true" />
                {drift.commits}
              </Button>
            </TooltipTrigger>
            <TooltipContent>Push {commits(drift.commits)} to the pull request to sync read marks</TooltipContent>
          </Tooltip>
          <Dialog open={open} onOpenChange={setOpen}>
            <DialogContent className="border-rule bg-surface sm:max-w-md">
              <DialogHeader>
                <DialogTitle className="font-sans text-base text-ink">Push {branch}?</DialogTitle>
                <DialogDescription className="text-ink-muted">
                  {commits(drift.commits)} go to origin and onto pull request #{number}, where everyone on it sees
                  them.
                </DialogDescription>
              </DialogHeader>
              <DialogFooter>
                <Button size="xs" variant="ghost" className="text-ink-muted" onClick={() => setOpen(false)}>
                  Cancel
                </Button>
                <Button
                  size="xs"
                  className="bg-comment-ink text-surface hover:bg-comment-ink/90"
                  disabled={pushing}
                  onClick={() => void push()}
                >
                  Push
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        </>
      ) : (
        <Tooltip delayDuration={0}>
          <TooltipTrigger asChild>
            <span className="inline-flex cursor-default" role="img" aria-label={said(drift)} tabIndex={0}>
              <TriangleAlert className="size-3.5" aria-hidden="true" />
            </span>
          </TooltipTrigger>
          <TooltipContent>{said(drift)}</TooltipContent>
        </Tooltip>
      )}
    </span>
  );
}

/** The drift in the reader's terms. */
function said(drift: DriftView): string {
  switch (drift.kind) {
    case "uncommitted":
      return "Uncommitted changes: commit them to sync read marks";
    case "unpushed":
      return `${commits(drift.commits)} not on the pull request yet`;
    case "newer":
      return "The pull request has commits you do not: pull to read them";
  }
}

function commits(count: number | null): string {
  return count === 1 ? "1 commit" : `${count ?? "Some"} commits`;
}

type DriftProps = {
  drift: DriftView;
  /** Whether the pull request is the reviewer's own, which is when Push is offered. */
  mine: boolean;
  number: number | null;
  branch: string;
  /** True once the branch went. */
  onPush: () => Promise<boolean>;
};
