import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { usePullRequestForm } from "@/hooks/usePullRequestForm";
import { GenerateDraft } from "@/review/pull/GenerateDraft";
import { Opened } from "@/review/pull/Opened";
import { PullRequestFields } from "@/review/pull/PullRequestFields";
import type { GitHubState } from "@/hooks/useGitHub";
import type { PullRequestView } from "@/api";

/** Open the pull request from the review, where Submit review sits once there
 * is one: the button, and the dialog it opens. What the form holds lives in
 * `usePullRequestForm`; the pieces of the form are drawn by their own
 * components. */
export function CreatePullRequest({ pull, branch, agent, github }: CreatePullRequestProps) {
  const [open, setOpen] = useState(false);
  const form = usePullRequestForm(pull.draft, github);
  // Pushed means the host has the branch; without it, Create pushes first.
  const pushed = pull.openAt !== null;

  return (
    <>
      <Button
        size="xs"
        className="create-pull bg-comment-ink text-surface hover:bg-comment-ink/90"
        onClick={() => {
          form.begin();
          setOpen(true);
        }}
      >
        Create pull request…
      </Button>
      <Dialog
        open={open}
        onOpenChange={(next) => {
          setOpen(next);
          // Only now: the page learns there is a pull request, and this button
          // goes. Earlier, it would have taken the dialog saying where with it.
          if (!next && form.opened) void github.reload();
        }}
      >
        <DialogContent className="border-rule bg-surface sm:max-w-lg">
          {form.opened ? (
            <Opened opened={form.opened} />
          ) : (
            <>
              <DialogHeader>
                <DialogTitle className="font-sans text-base text-ink">Create pull request</DialogTitle>
                <DialogDescription className="text-ink-muted">
                  {pushed ? (
                    <>
                      From <code className="font-mono text-ink">{branch}</code>.
                    </>
                  ) : (
                    <>
                      <code className="font-mono text-ink">{branch}</code> is not on GitHub yet: farol pushes it to
                      origin first.
                    </>
                  )}
                </DialogDescription>
              </DialogHeader>
              {agent && <GenerateDraft agent={agent} asked={form.asked} onGenerate={() => void form.generate()} />}
              <PullRequestFields form={form} />
              <DialogFooter>
                <Button size="xs" variant="ghost" className="text-ink-muted" onClick={() => setOpen(false)}>
                  Cancel
                </Button>
                <Button
                  size="xs"
                  className="bg-comment-ink text-surface hover:bg-comment-ink/90"
                  disabled={!form.title.trim() || form.creating}
                  onClick={() => void form.create()}
                >
                  {pushed ? "Create pull request" : "Push and create"}
                </Button>
              </DialogFooter>
            </>
          )}
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
