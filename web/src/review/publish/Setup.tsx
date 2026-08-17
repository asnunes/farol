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
          <DialogDescription className="font-serif leading-relaxed whitespace-pre-line text-ink-soft">
            {said.body(readiness.branch)}
          </DialogDescription>
        </DialogHeader>

        {said.command && <Command line={said.command(readiness.branch)} />}
        {said.after && (
          <p className="font-serif leading-relaxed text-ink-soft">{said.after}</p>
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
              I have done that — check again
            </Button>
          </DialogFooter>
        )}
      </DialogContent>
    </Dialog>
  );
}

/** What the panel says, per state. Prose and not a table of parts: the reason
 * each state is separate is that each needs different words. */
const SAYS: Record<ReadinessView["state"], Said> = {
  ready: {
    title: "Ready to send",
    body: () => "The pull request is there and the token works.",
  },

  noRemote: {
    title: "This repository has no remote",
    body: () =>
      "There is nowhere to send a review to. farol still reads the change " +
      "here, and your comments still live under the branch's own store — " +
      "they just have no pull request to go to.",
  },

  noToken: {
    title: "farol needs a token of your own",
    body: () =>
      "This is the one step here that nobody else can do for you: a " +
      "credential handed to an agent is a credential the agent has.\n\n" +
      "Create a fine-grained personal access token carrying Pull requests: " +
      "Read and write on this repository, then paste it below. farol keeps " +
      "it under ~/.config/farol, readable by nobody else, and never shows " +
      "it back to you.",
    token: true,
  },

  tokenRefused: {
    title: "GitHub would not take the token",
    body: () =>
      "It may have expired, or it may not carry Pull requests: Read and " +
      "write on this repository — that permission is what posting a review " +
      "needs.\n\n" +
      "Create a new one and paste it below. It replaces the one farol has.",
    token: true,
  },

  branchNotPushed: {
    title: "The branch is not on GitHub yet",
    body: (branch) =>
      "A review is posted onto a pull request, and there is no pull request " +
      `for ${branch} because GitHub has never seen the branch. Two things ` +
      "have to happen, and in this order.\n\n" +
      "First, push it:",
    command: (branch) => `git push -u origin ${branch}`,
    after:
      "Then open a pull request for it — yourself on GitHub, or ask the " +
      "session that wrote the code to do it. Come back and press the button " +
      "below once both are done.",
    check: true,
  },

  noPullRequest: {
    title: "The branch has no pull request",
    body: (branch) =>
      `${branch} is on GitHub, and nothing is open on it. A review is posted ` +
      "onto a pull request, so that is the piece that is missing.\n\n" +
      "Open it yourself, or ask the session that wrote the code to — it " +
      "knows what the change was for, which is most of what a description is.",
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
 * exactly when the reader wants to check they pasted the whole thing. */
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
        className="resize-none border-rule-strong bg-surface font-mono text-xs text-ink"
        rows={2}
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
