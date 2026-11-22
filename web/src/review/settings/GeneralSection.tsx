import { Switch } from "@/components/ui/switch";
import type { BooleanSetting } from "@/api";

/** The switches, each saved as it is flipped: there is no draft to keep. */
export function GeneralSection({ switches, saving, onSet }: GeneralSectionProps) {
  return (
    <div className="flex flex-col gap-4">
      {switches.map((setting) => (
        <label key={setting.key} className="flex items-start gap-3 text-sm text-ink">
          <Switch
            className="mt-px"
            checked={setting.value}
            disabled={saving}
            onCheckedChange={(on) => onSet(setting.key, on)}
            aria-labelledby={`setting-${setting.key}-title`}
            aria-describedby={setting.description ? `setting-${setting.key}-description` : undefined}
          />
          <span>
            <span id={`setting-${setting.key}-title`}>{setting.title}</span>
            {setting.description && (
              <span id={`setting-${setting.key}-description`} className="mt-1 block text-xs leading-relaxed text-ink-muted">
                {setting.description}
              </span>
            )}
          </span>
        </label>
      ))}
    </div>
  );
}

type GeneralSectionProps = {
  switches: BooleanSetting[];
  saving: boolean;
  onSet: (key: string, value: boolean) => void;
};
