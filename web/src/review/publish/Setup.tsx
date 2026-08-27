import { Check, Copy, ExternalLink } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { useCopy } from "@/hooks/useCopy";
import { Prose } from "@/review/Prose";
import { Token } from "./Token";
import { SAYS } from "./says";
import type { ReadinessView } from "@/api";

/** What is missing, and the place to fix it.
 *
 * One panel per state rather than a checklist, because the states are not steps
 * beside each other: a branch that is not on GitHub cannot have a pull request,
 * so being told about both at once is being told about one thing that is not
 * yet possible. Each state says only what is true now, and asks again when the
 * reader says they have done it. */
export function Setup({ open, onOpenChange, readiness, onCheck, onToken }: SetupProps) {
  // A state this build has no words for is a server newer than the page. Say
  // nothing rather than a wrong thing: the button beside it is disabled
  // either way, which is the part that matters.
  const said = SAYS[readiness.state];
  if (!said) return null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="setup-card border-rule bg-surface sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="font-sans text-base text-ink">{said.title}</DialogTitle>
          {/* `asChild` because the copy is a list as often as a sentence, and a
              list inside the paragraph Radix draws is not valid markup. */}
          <DialogDescription asChild>
            <Prose className="font-serif leading-relaxed text-ink-soft">
              {said.body(readiness.branch)}
            </Prose>
          </DialogDescription>
        </DialogHeader>

        {said.command && <Command line={said.command(readiness.branch)} />}
        {said.after && (
          <Prose className="font-serif leading-relaxed text-ink-soft">{said.after}</Prose>
        )}
        {readiness.openAt && <Open at={readiness.openAt} />}
        {said.token && <Token onToken={onToken} />}

        {said.check && (
          <DialogFooter>
            <Button
              size="sm"
              variant="outline"
              onClick={() => void onCheck()}
            >
              Check again
            </Button>
          </DialogFooter>
        )}
      </DialogContent>
    </Dialog>
  );
}

/** A command to run, with the means to take it away. Retyping a branch name by
 * eye is how a push ends up on the wrong branch. */
function Command({ line }: { line: string }) {
  const { copied, copy } = useCopy();

  return (
    <div className="cmd flex items-center gap-2 rounded border border-rule bg-sunken px-3 py-2">
      <code className="min-w-0 flex-1 font-mono text-xs break-all text-ink">{line}</code>
      <Button
        size="icon-xs"
        variant="ghost"
        className="text-faint hover:text-ink"
        aria-label="Copy this command"
        onClick={() => void copy(line)}
      >
        {copied ? (
          <Check className="size-3.5" aria-hidden="true" />
        ) : (
          <Copy className="size-3.5" aria-hidden="true" />
        )}
      </Button>
    </div>
  );
}

/** The page that opens a pull request, prefilled with the branch. */
function Open({ at }: { at: string }) {
  return (
    <a
      className="flex items-center gap-1.5 font-sans text-sm text-accent underline"
      href={at}
      target="_blank"
      rel="noreferrer"
    >
      <ExternalLink className="size-3.5" aria-hidden="true" />
      Open a pull request for this branch
    </a>
  );
}

type SetupProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  readiness: ReadinessView;
  onCheck: () => Promise<void>;
  onToken: (token: string) => Promise<void>;
};
