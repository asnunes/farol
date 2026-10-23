import { Check, Copy as CopyIcon, ExternalLink } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useCopy } from "@/hooks/useCopy";
import type { PullRequestView } from "@/api";

/** What stands between a peer review and the pull request, said once, in the
 * bar, instead of on every thread whose pending buttons it takes away.
 *
 * Fewer cases than reasons: no gh, no login and a refused token are all fixed
 * by logging in, so they all read as Connect GitHub. */
export function Connection({ pull }: ConnectionProps) {
  const { copied, copy } = useCopy();

  switch (pull.connection) {
    case "ready":
    case "no-remote":
      return null;
    case "connect":
      return (
        <Button
          variant="outline"
          size="xs"
          className="connect font-sans text-xs"
          title={`Run ${pull.login} in a terminal, then reload`}
          onClick={() => void copy(pull.login)}
        >
          {copied ? <Check className="size-3.5 text-add-ink" /> : <CopyIcon className="size-3.5" />}
          Connect GitHub: <code className="font-mono">{pull.login}</code>
        </Button>
      );
    case "no-pull-request":
      return pull.openAt ? (
        <Button asChild variant="outline" size="xs" className="no-pull font-sans text-xs">
          <a href={pull.openAt} target="_blank" rel="noreferrer">
            No pull request for this branch <ExternalLink className="size-3" aria-hidden="true" />
          </a>
        </Button>
      ) : (
        <Badge variant="outline" className="no-pull font-sans text-xs font-normal">
          No pull request for this branch: push it first
        </Badge>
      );
    case "ambiguous":
      return (
        <Badge variant="outline" className="ambiguous font-sans text-xs font-normal">
          More than one pull request shows this commit: {pull.numbers.map((n) => `#${n}`).join(", ")}
        </Badge>
      );
  }
}

type ConnectionProps = { pull: PullRequestView };
