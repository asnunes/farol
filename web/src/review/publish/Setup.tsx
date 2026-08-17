import { useState } from "react";
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
import { Textarea } from "@/components/ui/textarea";
import { useCopy } from "@/hooks/useCopy";
import { Prose } from "@/review/Prose";
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
  // nothing rather than a wrong thing — the button beside it is disabled
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
              className="cursor-pointer"
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

/** What the panel says, per state: one line of why, then the steps as a list.
 *
 * Markdown, because somebody standing in front of a blocked button is looking
 * for what to do rather than reading — a paragraph makes them find the steps
 * inside it, and a list hands them over. The why still comes first: a step
 * nobody understands is a step done wrong. */
const SAYS: Record<ReadinessView["state"], Said> = {
  ready: {
    title: "Ready to send",
    body: () => "The pull request is there and the token works.",
  },

  noRemote: {
    title: "This repository has no remote",
    body: () =>
      "There is nowhere to send a review to.\n\n" +
      "farol still reads the change here, and your comments still live under " +
      "the branch's own store. They just have no pull request to go to.",
  },

  noToken: {
    title: "farol needs a token of your own",
    body: () =>
      "The comments you left here go up as a review on this branch's pull " +
      "request, on the same lines and in the same words. To post them, farol " +
      "needs a token of your own.\n\n" +
      "- Create a **fine-grained personal access token** on GitHub.\n" +
      "- Give it **Pull requests: Read and write** on this repository.\n" +
      "- Paste it below.",
    token: true,
  },

  tokenRefused: {
    title: "GitHub would not take the token",
    body: () =>
      "One of two things:\n\n" +
      "- It expired.\n" +
      "- It does not carry **Pull requests: Read and write** on this " +
      "repository, which is what posting a review needs.\n\n" +
      "Create a new one and paste it below. It replaces the one farol has.",
    token: true,
  },

  branchNotPushed: {
    title: "The branch is not on GitHub yet",
    body: (branch) =>
      `A review is posted onto a pull request, and \`${branch}\` has none: ` +
      "GitHub has never seen the branch. Two steps, in this order:\n\n" +
      "- **Push it**, with the command below.\n" +
      "- **Open a pull request for it**, yourself on GitHub or through the " +
      "session that wrote the code.",
    command: (branch) => `git push -u origin ${branch}`,
    after: "Press the button below once both are done.",
    check: true,
  },

  noPullRequest: {
    title: "The branch has no pull request",
    body: (branch) =>
      `\`${branch}\` is on GitHub with nothing open on it, and a review is ` +
      "posted onto a pull request.\n\n" +
      "- Open one yourself, with the link below.\n" +
      "- Or ask the session that wrote the code. It knows what the change " +
      "was for, which is most of a description.",
    check: true,
  },
};

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
        className="cursor-pointer text-faint hover:text-ink"
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

/** Where the token goes in. A textarea and not an input: a fine-grained token
 * runs to ninety characters, and a single line hides all but the tail of it
 * exactly when the reader wants to check they pasted the whole thing.
 *
 * `break-all` is what makes that true. A token has no spaces in it, so soft
 * wrapping treats the whole thing as one word and runs it off the side, which
 * is the very problem the textarea was chosen to avoid. */
function Token({ onToken }: { onToken: (token: string) => Promise<void> }) {
  const [token, setToken] = useState("");
  const [saving, setSaving] = useState(false);

  async function save() {
    if (!token.trim() || saving) return;
    setSaving(true);
    try {
      await onToken(token);
      // Out of the page as soon as it is out of the box. It is not coming back
      // from the server, and there is no reason for it to sit in a form.
      setToken("");
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="token">
      <Textarea
        className="resize-none break-all border-rule-strong bg-surface font-mono text-xs text-ink"
        rows={3}
        placeholder="github_pat_…"
        autoComplete="off"
        spellCheck={false}
        value={token}
        disabled={saving}
        onChange={(e) => setToken(e.target.value)}
      />
      <Button
        size="sm"
        className="mt-2 cursor-pointer"
        disabled={!token.trim() || saving}
        onClick={() => void save()}
      >
        {saving ? "Saving…" : "Save token"}
      </Button>
    </div>
  );
}

type Said = {
  title: string;
  body: (branch: string) => string;
  command?: (branch: string) => string;
  after?: string;
  token?: boolean;
  check?: boolean;
};

type SetupProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  readiness: ReadinessView;
  onCheck: () => Promise<void>;
  onToken: (token: string) => Promise<void>;
};
