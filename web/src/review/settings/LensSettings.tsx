import { useState } from "react";
import { Button } from "@/components/ui/button";
import { LensEditor } from "@/review/settings/LensEditor";
import { useLenses } from "@/hooks/useLenses";
import type { LensGroup } from "@/api";

/** The lenses, in their three groups: switch each on or off, change its title
 * and prompt, delete it, or add one at the end of a group. */
export function LensSettings({ open }: LensSettingsProps) {
  const lenses = useLenses(open);
  // The lens being edited, or the group a new one is being added to.
  const [editing, setEditing] = useState<string | null>(null);
  const done = (kept: boolean) => {
    if (kept) setEditing(null);
    return kept;
  };

  return (
    <section className="lens-settings flex flex-col gap-3 border-t border-rule pt-4">
      <div>
        <h3 className="font-sans text-sm font-semibold text-ink">Lenses</h3>
        <p className="mt-1 text-xs leading-relaxed text-ink-muted">
          What the session writes about each change, read before the code.
        </p>
      </div>
      {GROUPS.map(({ group, name }) => (
        <div key={group} className="flex flex-col">
          <h4 className="font-mono text-[0.6875rem] tracking-wide text-ink-muted uppercase">{name}</h4>
          <ul className="flex flex-col divide-y divide-rule">
            {lenses.lenses
              .filter((lens) => lens.group === group)
              .map((lens) => (
                <li key={lens.id} className="py-1.5">
                  <div className="flex items-center gap-3 text-sm text-ink">
                    <input
                      type="checkbox"
                      className="size-4 accent-current"
                      checked={lens.enabled}
                      disabled={lenses.saving}
                      aria-label={`${lens.title} is on`}
                      onChange={(e) => void lenses.change(lens.id, { enabled: e.target.checked })}
                    />
                    <span className={`min-w-0 flex-1 truncate ${lens.enabled ? "" : "text-ink-muted"}`}>{lens.title}</span>
                    <Button variant="ghost" size="xs" disabled={lenses.saving} onClick={() => setEditing(editing === lens.id ? null : lens.id)}>
                      Edit
                    </Button>
                    <Button variant="ghost" size="xs" disabled={lenses.saving} onClick={() => void lenses.remove(lens.id)}>
                      Delete
                    </Button>
                  </div>
                  {editing === lens.id && (
                    <LensEditor
                      title={lens.title}
                      prompt={lens.prompt}
                      saving={lenses.saving}
                      onSave={async (title, prompt) => done(await lenses.change(lens.id, { title, prompt }))}
                      onCancel={() => setEditing(null)}
                    />
                  )}
                </li>
              ))}
          </ul>
          {editing === `new:${group}` ? (
            <LensEditor
              title=""
              prompt=""
              saving={lenses.saving}
              onSave={async (title, prompt) => done(await lenses.add({ title, group, prompt }))}
              onCancel={() => setEditing(null)}
            />
          ) : (
            <Button variant="outline" size="xs" className="mt-1 self-start" onClick={() => setEditing(`new:${group}`)}>
              Add lens
            </Button>
          )}
        </div>
      ))}
      {lenses.error && <p role="alert" className="text-sm text-ink">{lenses.error}</p>}
    </section>
  );
}

const GROUPS: { group: LensGroup; name: string }[] = [
  { group: "pr", name: "Pull request" },
  { group: "block", name: "Block" },
  { group: "file", name: "File" },
];

type LensSettingsProps = { open: boolean };
