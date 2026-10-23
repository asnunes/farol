import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import type { TextSetting } from "@/api";

/** A text preference, edited as a draft and written only on Save.
 *
 * Not saved on every key like the switches are: each keystroke would rewrite
 * the file and flash "Saved". Closing the dialog drops the draft; a failed save
 * keeps it, so nothing typed is lost to an error. The caller keys it on the
 * saved value, so the server's answer to a save or a reset starts a fresh
 * draft from what is now stored. */
export function TextField({ setting, saving, onSave, onReset }: TextFieldProps) {
  const [draft, setDraft] = useState(setting.value);

  return (
    <div className="text-setting flex flex-col gap-2 text-sm text-ink">
      <label htmlFor={`setting-${setting.key}`}>{setting.title}</label>
      {setting.description && (
        <span id={`setting-${setting.key}-description`} className="text-xs leading-relaxed text-ink-muted">
          {setting.description}
        </span>
      )}
      <Textarea
        id={`setting-${setting.key}`}
        value={draft}
        rows={4}
        disabled={saving}
        aria-describedby={setting.description ? `setting-${setting.key}-description` : undefined}
        onChange={(event) => setDraft(event.target.value)}
      />
      <div className="flex justify-end gap-2">
        <Button variant="ghost" size="xs" disabled={saving} onClick={() => void onReset()}>
          Restore default
        </Button>
        <Button size="xs" disabled={saving || draft === setting.value} onClick={() => void onSave(draft)}>
          Save
        </Button>
      </div>
    </div>
  );
}

type TextFieldProps = {
  setting: TextSetting;
  saving: boolean;
  onSave: (value: string) => Promise<boolean>;
  onReset: () => Promise<boolean>;
};
