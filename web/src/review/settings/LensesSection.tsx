import { useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import { Unsaved } from "./Unsaved";
import type { Drafts } from "@/hooks/useDrafts";
import type { useLenses } from "@/hooks/useLenses";
import type { LensGroup, LensRow } from "@/api";

/** The lens catalog in its three groups. A lens's switch saves as it is
 * flipped; its words, a new lens and a deletion wait for the section's Save. */
export function LensesSection({ lenses, drafts }: LensesSectionProps) {
  // Which lenses have their words open for editing: a page of every prompt
  // open at once is a page nobody can find a lens in.
  const [open, setOpen] = useState<string[]>([]);
  const toggle = (id: string) => setOpen((was) => (was.includes(id) ? was.filter((one) => one !== id) : [...was, id]));
  const added = useRef(0);

  return (
    <div className="lens-settings flex flex-col gap-8">
      <p className="text-sm leading-relaxed text-ink-muted">What the session writes about each change, read before the code.</p>
      {GROUPS.map(({ group, name }) => (
        <section key={group} className="flex flex-col gap-2">
          <h3 className="font-mono text-[0.6875rem] tracking-wide text-ink-muted uppercase">{name}</h3>
          <ul className="flex flex-col divide-y divide-rule rounded-md border border-rule bg-surface">
            {lenses.lenses
              .filter((lens) => lens.group === group)
              .map((lens) => (
                <Existing
                  key={lens.id}
                  lens={lens}
                  drafts={drafts}
                  saving={lenses.saving}
                  open={open.includes(lens.id)}
                  onToggle={() => toggle(lens.id)}
                  onSwitch={(on) => void lenses.change(lens.id, { enabled: on })}
                />
              ))}
            {drafts.all.map((draft) =>
              draft.kind === "add" && draft.group === group ? (
                <li key={draft.key} className="px-3 py-2">
                  <div className="flex items-center gap-3 text-sm text-ink">
                    <span className="min-w-0 flex-1 truncate text-ink-muted">{draft.title || "New lens"}</span>
                    <Unsaved />
                    <Button variant="ghost" size="xs" onClick={() => drafts.unset(draft.key)}>
                      Remove
                    </Button>
                  </div>
                  <Fields words={draft} onChange={(words) => drafts.set({ ...draft, ...words })} />
                  {drafts.error(draft.key) && <p role="alert" className="mt-1 text-xs text-del-ink">{drafts.error(draft.key)}</p>}
                </li>
              ) : null,
            )}
          </ul>
          <Button
            variant="outline"
            size="xs"
            className="self-start"
            onClick={() => drafts.set({ key: `new:${++added.current}`, kind: "add", group, title: "", prompt: "" })}
          >
            Add lens
          </Button>
        </section>
      ))}
      {lenses.error && <p role="alert" className="text-sm text-del-ink">{lenses.error}</p>}
    </div>
  );
}

/** One lens of the catalog, with what is pending on it: new words, or its
 * deletion. One pending change per lens, so deleting drops an edit. */
function Existing({ lens, drafts, saving, open, onToggle, onSwitch }: ExistingProps) {
  const key = `lens:${lens.id}`;
  const draft = drafts.get(key);
  const gone = draft?.kind === "remove";
  const words = draft?.kind === "edit" ? draft : lens;
  // Back to what is stored is no change at all.
  const edit = (next: Words) =>
    next.title === lens.title && next.prompt === lens.prompt
      ? drafts.unset(key)
      : drafts.set({ key, kind: "edit", id: lens.id, ...next });

  return (
    <li className="px-3 py-2">
      <div className="flex items-center gap-3 text-sm text-ink">
        <Switch checked={lens.enabled} disabled={saving || gone} aria-label={`${lens.title} is on`} onCheckedChange={onSwitch} />
        <span className={cn("min-w-0 flex-1 truncate", !lens.enabled && "text-ink-muted", gone && "line-through")}>
          {words.title}
        </span>
        {draft?.kind === "edit" && <Unsaved />}
        {gone ? (
          <>
            <span className="unsaved font-sans text-xs text-highlight">Deleted on save</span>
            <Button variant="ghost" size="xs" onClick={() => drafts.unset(key)}>
              Undo
            </Button>
          </>
        ) : (
          <>
            <Button variant="ghost" size="xs" aria-expanded={open} onClick={onToggle}>
              {open ? "Close" : "Edit"}
            </Button>
            <Button variant="ghost" size="xs" onClick={() => drafts.set({ key, kind: "remove", id: lens.id })}>
              Delete
            </Button>
          </>
        )}
      </div>
      {open && !gone && <Fields words={words} onChange={edit} />}
      {drafts.error(key) && <p role="alert" className="mt-1 text-xs text-del-ink">{drafts.error(key)}</p>}
    </li>
  );
}

/** A lens's title and prompt, written into the section's drafts as typed. */
function Fields({ words, onChange }: FieldsProps) {
  return (
    <div className="lens-editor mt-2 flex flex-col gap-2 text-sm text-ink">
      <label className="flex flex-col gap-1">
        <span className="text-xs text-ink-muted">Title</span>
        <Input value={words.title} onChange={(e) => onChange({ title: e.target.value, prompt: words.prompt })} />
      </label>
      <label className="flex flex-col gap-1">
        <span className="text-xs text-ink-muted">Prompt</span>
        <Textarea rows={5} value={words.prompt} onChange={(e) => onChange({ title: words.title, prompt: e.target.value })} />
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

/** What is pending on the catalog: new words for a lens, a new lens, or a
 * lens deleted. */
export type LensDraft =
  | ({ key: string; kind: "edit"; id: string } & Words)
  | { key: string; kind: "remove"; id: string }
  | ({ key: string; kind: "add"; group: LensGroup } & Words);

type Words = { title: string; prompt: string };

type LensesSectionProps = {
  lenses: ReturnType<typeof useLenses>;
  drafts: Drafts<LensDraft>;
};

type ExistingProps = {
  lens: LensRow;
  drafts: Drafts<LensDraft>;
  saving: boolean;
  open: boolean;
  onToggle: () => void;
  onSwitch: (on: boolean) => void;
};

type FieldsProps = { words: Words; onChange: (words: Words) => void };
