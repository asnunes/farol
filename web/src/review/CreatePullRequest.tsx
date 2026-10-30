import { useEffect, useState } from "react";
import { Sparkles } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import type { GitHubState } from "@/hooks/useGitHub";
import type { PullRequestView } from "@/api";

/** Open the pull request from the review, where Submit review sits once there
 * is one. The title and description are the reviewer's, or the agent's when
 * Generate is pressed — following the instruction in Settings, and arriving
 * here to be read and edited before anything is opened. */
export function CreatePullRequest({ pull, branch, agent, github }: CreatePullRequestProps) {
  const [open, setOpen] = useState(false);
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [asked, setAsked] = useState(false);
  const [creating, setCreating] = useState(false);
  // Pushed means the host has the branch; without it, Create pushes first.
  const pushed = pull.openAt !== null;

  // The agent's draft lands while the form is open, and replaces what is in
  // it only when the reviewer asked for one.
  const draft = pull.draft;
  useEffect(() => {
    if (!draft || !asked) return;
    setTitle(draft.title);
    setBody(draft.body);
    setAsked(false);
  }, [draft, asked]);

  async function create() {
    setCreating(true);
    const opened = await github.openPull(title, body);
    setCreating(false);
    if (opened) setOpen(false);
  }

  return (
    <>
      <Button
        size="xs"
        className="create-pull bg-comment-ink text-surface hover:bg-comment-ink/90"
        onClick={() => {
          if (draft && !title && !body) {
            setTitle(draft.title);
            setBody(draft.body);
          }
          setOpen(true);
        }}
      >
        Create pull request…
      </Button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="border-rule bg-surface sm:max-w-lg">
          <DialogHeader>
            <DialogTitle className="font-sans text-base text-ink">Create pull request</DialogTitle>
            <DialogDescription className="text-ink-muted">
              {pushed ? (
                <>
                  From <code className="font-mono text-ink">{branch}</code>.
                </>
              ) : (
                <>
                  <code className="font-mono text-ink">{branch}</code> is not on GitHub yet: farol pushes it to origin
                  first.
                </>
              )}
            </DialogDescription>
          </DialogHeader>

          {agent && (
            <div>
              <Button
                size="xs"
                variant="outline"
                className="generate-pull gap-1"
                disabled={asked}
                title={`Ask ${agent} for the title and description, following the instruction in Settings`}
                onClick={async () => setAsked(await github.generatePullDraft())}
              >
                <Sparkles className="size-3.5" aria-hidden="true" />
                {asked ? `Asked ${agent}…` : "Generate"}
              </Button>
            </div>
          )}

          <label className="flex flex-col gap-1 font-sans text-xs text-ink-muted">
            Title
            <Input className="border-rule-strong bg-ground font-sans text-sm text-ink" value={title} onChange={(e) => setTitle(e.target.value)} />
          </label>
          <label className="flex flex-col gap-1 font-sans text-xs text-ink-muted">
            Description
            <Textarea
              className="min-h-40 resize-y border-rule-strong bg-ground font-sans text-sm text-ink"
              value={body}
              onChange={(e) => setBody(e.target.value)}
            />
          </label>

          <DialogFooter>
            <Button size="xs" variant="ghost" className="text-ink-muted" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button
              size="xs"
              className="bg-comment-ink text-surface hover:bg-comment-ink/90"
              disabled={!title.trim() || creating}
              onClick={() => void create()}
            >
              {pushed ? "Create pull request" : "Push and create"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

type CreatePullRequestProps = {
  pull: PullRequestView;
  branch: string;
  /** The harness Generate reaches, or null — and then there is no Generate. */
  agent: string | null;
  github: GitHubState;
};
