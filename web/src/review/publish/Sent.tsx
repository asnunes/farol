import { DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import type { SentView } from "@/api";

/** Where the review went, once it has gone.
 *
 * The address is the whole of this panel: it is the one thing the page cannot
 * work out for itself, and the reader is about to go and look at it. */
export function Sent({ sent, onDone }: SentProps) {
  return (
    <>
      <DialogHeader>
        <DialogTitle className="font-sans text-base text-ink">Review sent</DialogTitle>
        <DialogDescription className="font-serif text-ink-soft">
          {sent.comments === 0
            ? "Your verdict is on the pull request."
            : `${sent.comments} comment${sent.comments === 1 ? "" : "s"} went with it.`}{" "}
          They stay here too, still open, until you close them.
          {sent.readFailed ? (
            // The review is on the pull request and cannot be taken back, so
            // this is a note beside it rather than a failure in place of it.
            <>
              {" "}
              The files you had read were not ticked: {sent.readFailed}
            </>
          ) : (
            sent.read > 0 && (
              <>
                {" "}
                The {sent.read} file{sent.read === 1 ? "" : "s"} you had read went up ticked,
                so a second round shows what changed.
              </>
            )
          )}
        </DialogDescription>
      </DialogHeader>

      <a
        className="font-mono text-sm break-all text-highlight underline"
        href={sent.url}
        target="_blank"
        rel="noreferrer"
      >
        {sent.url}
      </a>

      <DialogFooter>
        <Button size="sm" onClick={onDone}>
          Done
        </Button>
      </DialogFooter>
    </>
  );
}

type SentProps = {
  sent: SentView;
  onDone: () => void;
};
