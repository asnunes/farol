import { Sparkles } from "lucide-react";
import { Button } from "@/components/ui/button";

/** Ask the agent for the pull request's title and description, following the
 * instruction in Settings. Shows that it was asked until the draft lands. */
export function GenerateDraft({ agent, asked, onGenerate }: GenerateDraftProps) {
  return (
    <div>
      <Button
        size="xs"
        variant="outline"
        className="generate-pull gap-1"
        disabled={asked}
        title={`Ask ${agent} for the title and description, following the instruction in Settings`}
        onClick={onGenerate}
      >
        <Sparkles className="size-3.5" aria-hidden="true" />
        {asked ? `Asked ${agent}…` : "Generate"}
      </Button>
    </div>
  );
}

type GenerateDraftProps = {
  agent: string;
  asked: boolean;
  onGenerate: () => void;
};
