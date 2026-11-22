import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import { Unsaved } from "./Unsaved";
import type { LensDrafts, Words } from "@/hooks/useLensDrafts";
import type { useLenses } from "@/hooks/useLenses";
import type { LensGroup } from "@/api";

/** The lens catalog in its three groups. A lens's switch saves as it is
 * flipped; its words, a new lens and a deletion wait for the section's Save. */
export function LensesSection({ lenses, drafts }: LensesSectionProps) {
  // Which lenses have their words open for editing: a page of every prompt
  // open at once is a page nobody can find a lens in.
  const [open, setOpen] = useState<string[]>([]);
  const toggle = (id: string) => setOpen((was) => (was.includes(id) ? was.filter((one) => one !== id) : [...was, id]));

  return (
    <div className="lens-settings flex flex-col gap-8">
      <p className="text-sm leading-relaxed text-ink-muted">What the session writes about each change, read before the code.</p>
      {GROUPS.map(({ group, name }) => (
        <section key={group} className="flex flex-col gap-2">
          <h3 className="font-mono text-[0.6875rem] tracking-wide text-ink-muted uppercase">{name}</h3>
          <ul className="flex flex-col divide-y divide-rule rounded-md border border-rule bg-surface">
            {lenses.lenses
              .filter((lens) => lens.group === group)
              .map((lens) => {
                const gone = drafts.removed(lens.id);
                const words = drafts.words(lens);
                return (
                  <li key={lens.id} className="px-3 py-2">
                    <div className="flex items-center gap-3 text-sm text-ink">
                      <Switch
                        checked={lens.enabled}
                        disabled={lenses.saving || gone}
                        aria-label={`${lens.title} is on`}
                        onCheckedChange={(on) => void lenses.change(lens.id, { enabled: on })}
                      />
                      <span className={cn("min-w-0 flex-1 truncate", !lens.enabled && "text-ink-muted", gone && "line-through")}>
                        {words.title}
                      </span>
                      {drafts.unsaved(lens) && <Unsaved />}
                      {gone ? (
                        <>
                          <span className="unsaved font-sans text-xs text-highlight">Deleted on save</span>
                          <Button variant="ghost" size="xs" onClick={() => drafts.keep(lens.id)}>
                            Undo
                          </Button>
                        </>
                      ) : (
                        <>
                          <Button variant="ghost" size="xs" aria-expanded={open.includes(lens.id)} onClick={() => toggle(lens.id)}>
                            {open.includes(lens.id) ? "Close" : "Edit"}
                          </Button>
                          <Button variant="ghost" size="xs" onClick={() => drafts.remove(lens.id)}>
                            Delete
                          </Button>
                        </>
                      )}
                    </div>
                    {open.includes(lens.id) && !gone && <Fields words={words} onChange={(next) => drafts.edit(lens.id, next)} />}
                    {drafts.error(lens.id) && <p role="alert" className="mt-1 text-xs text-del-ink">{drafts.error(lens.id)}</p>}
                  </li>
                );
              })}
            {drafts.added(group).map((one) => (
              <li key={one.draft} className="px-3 py-2">
                <div className="flex items-center gap-3 text-sm text-ink">
                  <span className="min-w-0 flex-1 truncate text-ink-muted">{one.title || "New lens"}</span>
                  <Unsaved />
                  <Button variant="ghost" size="xs" onClick={() => drafts.drop(one.draft)}>
                    Remove
                  </Button>
                </div>
                <Fields words={one} onChange={(next) => drafts.editAdded(one.draft, next)} />
                {drafts.error(one.draft) && <p role="alert" className="mt-1 text-xs text-del-ink">{drafts.error(one.draft)}</p>}
              </li>
            ))}
          </ul>
          <Button variant="outline" size="xs" className="self-start" onClick={() => drafts.add(group)}>
            Add lens
          </Button>
        </section>
      ))}
      {lenses.error && <p role="alert" className="text-sm text-del-ink">{lenses.error}</p>}
    </div>
  );
}

/** A lens's title and prompt, written into the section's drafts as typed. */
function Fields({ words, onChange }: FieldsProps) {
  return (
    <div className="lens-editor mt-2 flex flex-col gap-2 text-sm text-ink">
      <label className="flex flex-col gap-1">
        <span className="text-xs text-ink-muted">Title</span>
        <Input value={words.title} onChange={(e) => onChange({ ...words, title: e.target.value })} />
      </label>
      <label className="flex flex-col gap-1">
        <span className="text-xs text-ink-muted">Prompt</span>
        <Textarea rows={5} value={words.prompt} onChange={(e) => onChange({ ...words, prompt: e.target.value })} />
        <span className="text-xs text-ink-muted">What the agent follows to write it. It can be just "Use the X skill".</span>
      </label>
    </div>
  );
}

const GROUPS: { group: LensGroup; name: string }[] = [
  { group: "pr", name: "Pull request" },
  { group: "block", name: "Block" },
  { group: "file", name: "File" },
];

type LensesSectionProps = {
  lenses: ReturnType<typeof useLenses>;
  drafts: LensDrafts;
};

type FieldsProps = { words: Words; onChange: (words: Words) => void };
