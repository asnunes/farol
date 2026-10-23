import { useState } from "react";
import { ChevronDown, Sparkles } from "lucide-react";
import { Button } from "@/components/ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Textarea } from "@/components/ui/textarea";

/** Ask the agent for the thread's pending comment — or for a new one.
 *
 * One click generates with the instruction from Settings, which is what most
 * presses want. The arrow beside it has a single entry, for the press that
 * wants a word added just this once: the click already is the plain case, so
 * a menu offering it again would be a second way to do the same thing. */
export function Generate({ label, onGenerate }: GenerateProps) {
  const [asking, setAsking] = useState(false);
  const [note, setNote] = useState("");
  const [busy, setBusy] = useState(false);

  async function generate(instruction: string) {
    setBusy(true);
    const took = await onGenerate(instruction);
    setBusy(false);
    if (took) {
      setAsking(false);
      setNote("");
    }
  }

  if (asking) {
    return (
      <div className="generate-with flex flex-col gap-2">
        <Textarea
          autoFocus
          aria-label="Instructions for this pending comment"
          className="resize-y border-rule-strong bg-surface font-sans text-sm text-ink"
          rows={2}
          placeholder="e.g. “shorter”, “suggest the env var version”"
          value={note}
          disabled={busy}
          onChange={(e) => setNote(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
              e.preventDefault();
              if (note.trim()) void generate(note);
            }
            if (e.key === "Escape") {
              e.preventDefault();
              setAsking(false);
            }
          }}
        />
        <span className="font-sans text-xs text-ink-muted">
          Added to your instruction from Settings · ⌘↵ generates
        </span>
        <div className="flex gap-2">
          <Button
            size="xs"
            className="bg-comment-ink text-surface hover:bg-comment-ink/90"
            disabled={!note.trim() || busy}
            onClick={() => void generate(note)}
          >
            {label}
          </Button>
          <Button size="xs" variant="ghost" className="text-ink-muted" onClick={() => setAsking(false)}>
            Cancel
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="generate inline-flex items-center">
      <Button
        variant="ghost"
        size="xs"
        className="text-comment-faint hover:bg-sunken hover:text-comment-ink"
        disabled={busy}
        onClick={() => void generate("")}
      >
        <Sparkles className="size-3.5" aria-hidden="true" />
        {label}
      </Button>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            variant="ghost"
            size="icon-xs"
            className="text-comment-faint hover:bg-sunken hover:text-comment-ink"
            aria-label={`${label} with instructions`}
            disabled={busy}
          >
            <ChevronDown className="size-3.5" aria-hidden="true" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start">
          <DropdownMenuItem onSelect={() => setAsking(true)}>
            <span className="font-sans text-sm">{label} with instructions…</span>
            <span className="font-sans text-xs text-ink-muted">
              adds a note to your instruction from Settings, just for this one
            </span>
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}

type GenerateProps = {
  /** "Generate pending comment", or "Regenerate" once there is one. */
  label: string;
  /** True once the agent took the request. */
  onGenerate: (instruction: string) => Promise<boolean>;
};
