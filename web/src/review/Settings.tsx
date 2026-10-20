import { useState } from "react";
import { Settings as SettingsIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { useConfig } from "@/hooks/useConfig";
import { TextField } from "./TextField";

export function Settings() {
  const [open, setOpen] = useState(false);
  const { config, error, saving, saved, setSetting, resetSetting } = useConfig(open);

  if (!config?.settings.length && !error) return null;

  return (
    <>
      <Button
        variant="ghost"
        size="icon-xs"
        className="text-ink-muted hover:bg-transparent hover:text-ink max-md:size-9"
        onClick={() => setOpen(true)}
        aria-label="Settings"
        title="Settings"
      >
        <SettingsIcon className="size-3.5" aria-hidden="true" />
      </Button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="border-rule bg-surface sm:max-w-md">
          <DialogHeader>
            <DialogTitle className="font-sans text-base text-ink">Settings</DialogTitle>
            <DialogDescription className="text-ink-muted">
              For all repositories on this machine, under your user account.
            </DialogDescription>
          </DialogHeader>
          {config?.settings.map((setting) =>
            setting.type === "text" ? (
              <TextField
                key={`${setting.key}:${setting.value}`}
                setting={setting}
                saving={saving}
                onSave={(value) => setSetting(setting.key, value)}
                onReset={() => resetSetting(setting.key)}
              />
            ) : (
              <label key={setting.key} className="flex items-start gap-3 text-sm text-ink">
                <input
                  type="checkbox"
                  className="mt-0.5 size-4 accent-current"
                  checked={setting.value}
                  disabled={saving}
                  onChange={(event) => void setSetting(setting.key, event.target.checked)}
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
            ),
          )}
          {error && <p role="alert" className="text-sm text-ink">{error}</p>}
          <p role="status" className="min-h-4 text-xs text-ink-muted">{saving ? "Saving…" : saved ? "Saved" : ""}</p>
        </DialogContent>
      </Dialog>
    </>
  );
}
