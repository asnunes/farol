import { ExternalLink } from "lucide-react";
import { DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import type { OpenedPullView } from "@/api";

/** What the dialog says once the pull request is open: which one, and a way
 * to it. Opening it and saying nothing left the reviewer to go and find it. */
export function Opened({ opened }: OpenedProps) {
  return (
    <DialogHeader>
      <DialogTitle className="font-sans text-base text-ink">Pull request #{opened.number} opened</DialogTitle>
      <DialogDescription className="text-ink-muted">
        <a
          href={opened.url}
          target="_blank"
          rel="noreferrer"
          className="opened-pull inline-flex items-center gap-1 text-highlight underline"
        >
          {opened.url}
          <ExternalLink className="size-3" aria-hidden="true" />
        </a>
      </DialogDescription>
    </DialogHeader>
  );
}

type OpenedProps = { opened: OpenedPullView };
