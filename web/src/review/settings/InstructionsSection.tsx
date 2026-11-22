import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Unsaved } from "./Unsaved";
import type { Drafts } from "@/hooks/useDrafts";
import type { TextSetting } from "@/api";

/** What the agent is told with each message farol sends it, one field per
 * message, each a draft until the section's Save.
 *
 * Restoring a default is an edit like any other: the default goes into the
 * draft, and Save is what sends it. */
export function InstructionsSection({ settings, drafts, saving }: InstructionsSectionProps) {
  return (
    <div className="flex flex-col gap-8">
      {settings.map((setting) => {
        const value = drafts.get(setting.key)?.value ?? setting.value;
        // Back to what is stored is no change at all.
        const edit = (next: string) =>
          next === setting.value ? drafts.unset(setting.key) : drafts.set({ key: setting.key, value: next });
        return (
          <div key={setting.key} className="text-setting flex flex-col gap-2 text-sm text-ink">
            <label htmlFor={`setting-${setting.key}`} className="font-medium">{setting.title}</label>
            {setting.description && (
              <span id={`setting-${setting.key}-description`} className="text-xs leading-relaxed text-ink-muted">
                {setting.description}
              </span>
            )}
            <Textarea
              id={`setting-${setting.key}`}
              value={value}
              rows={6}
              disabled={saving}
              aria-describedby={setting.description ? `setting-${setting.key}-description` : undefined}
              onChange={(event) => edit(event.target.value)}
            />
            <div className="flex items-center gap-3">
              <Button
                variant="ghost"
                size="xs"
                className="text-ink-muted"
                disabled={saving || value === setting.default}
                onClick={() => edit(setting.default)}
              >
                Reset to default
              </Button>
              {drafts.get(setting.key) && <Unsaved />}
            </div>
            {drafts.error(setting.key) && <p role="alert" className="text-xs text-del-ink">{drafts.error(setting.key)}</p>}
          </div>
        );
      })}
    </div>
  );
}

/** A new value for one instruction, under the setting's key. */
export type InstructionDraft = { key: string; value: string };

type InstructionsSectionProps = {
  settings: TextSetting[];
  drafts: Drafts<InstructionDraft>;
  saving: boolean;
};
