import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Unsaved } from "./Unsaved";
import type { InstructionDrafts } from "@/hooks/useInstructionDrafts";
import type { TextSetting } from "@/api";

/** What the agent is told with each message farol sends it, one field per
 * message, each a draft until the section's Save. */
export function InstructionsSection({ settings, drafts, saving }: InstructionsSectionProps) {
  return (
    <div className="flex flex-col gap-8">
      {settings.map((setting) => {
        const value = drafts.value(setting);
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
              onChange={(event) => drafts.edit(setting.key, event.target.value)}
            />
            <div className="flex items-center gap-3">
              <Button
                variant="ghost"
                size="xs"
                className="text-ink-muted"
                disabled={saving || value === setting.default}
                onClick={() => drafts.restore(setting)}
              >
                Reset to default
              </Button>
              {drafts.unsaved(setting) && <Unsaved />}
            </div>
            {drafts.error(setting.key) && <p role="alert" className="text-xs text-del-ink">{drafts.error(setting.key)}</p>}
          </div>
        );
      })}
    </div>
  );
}

type InstructionsSectionProps = {
  settings: TextSetting[];
  drafts: InstructionDrafts;
  saving: boolean;
};
