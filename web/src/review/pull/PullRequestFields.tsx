import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import type { PullRequestForm } from "@/hooks/usePullRequestForm";

/** The title and description the pull request is opened with. */
export function PullRequestFields({ form }: PullRequestFieldsProps) {
  return (
    <>
      <label className="flex flex-col gap-1 font-sans text-xs text-ink-muted">
        Title
        <Input
          className="border-rule-strong bg-ground font-sans text-sm text-ink"
          value={form.title}
          onChange={(e) => form.setTitle(e.target.value)}
        />
      </label>
      <label className="flex flex-col gap-1 font-sans text-xs text-ink-muted">
        Description
        <Textarea
          className="min-h-40 resize-y border-rule-strong bg-ground font-sans text-sm text-ink"
          value={form.body}
          onChange={(e) => form.setBody(e.target.value)}
        />
      </label>
    </>
  );
}

type PullRequestFieldsProps = { form: PullRequestForm };
