import { ChevronDown } from "lucide-react";
import { Button } from "@/components/ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import type { PullRequestView, ReviewMode as Mode } from "@/api";

/** Which review this is, why farol thinks so, and the way to say otherwise.
 *
 * The reason is one hover away: a wrong guess is expected — a branch two
 * people share, a pull request opened on someone's behalf — and the reader
 * should see what it was based on before deciding it is wrong. */
export function ReviewMode({ pull, onChoose }: ReviewModeProps) {
  const current = pull.mode.mode;

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size="xs"
          className="review-mode h-auto gap-1 px-1 font-mono text-xs font-normal text-ink-muted hover:text-ink"
          title={`${NAMES[current]} — ${pull.mode.reason}`}
        >
          Mode: <span className="font-semibold text-ink-soft">{NAMES[current]}</span>
          <ChevronDown className="size-3" aria-hidden="true" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-72">
        <p className="px-2 py-1.5 font-sans text-xs text-ink-muted">
          {NAMES[current]}: {pull.mode.reason}.
        </p>
        {(["self", "peer"] as const).map((mode) => (
          <DropdownMenuItem key={mode} disabled={mode === current} onSelect={() => void onChoose(mode)}>
            <span className="font-sans text-sm">{NAMES[mode]}</span>
            <span className="font-sans text-xs text-ink-muted">{WHAT[mode]}</span>
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

const NAMES: Record<Mode, string> = { self: "Self review", peer: "Peer review" };

const WHAT: Record<Mode, string> = {
  self: "Your branch: the agent may change the code.",
  peer: "Someone else's pull request: the agent does not touch the code, and threads can become pending comments.",
};

type ReviewModeProps = {
  pull: PullRequestView;
  onChoose: (mode: Mode) => Promise<void>;
};
