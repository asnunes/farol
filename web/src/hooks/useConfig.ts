import { useEffect, useState } from "react";
import { api, type Config, type SettingValue } from "@/api";

export function useConfig(open: boolean) {
  const [config, setConfig] = useState<Config | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    let active = true;
    setError(null);
    setSaved(false);
    api.config().then(
      (value) => { if (active) setConfig(value); },
      (reason: Error) => { if (active) setError(reason.message); },
    );
    return () => { active = false; };
  }, [open]);

  /** True once the server kept it, so a text box knows its draft is now the value. */
  async function save(job: Promise<Config>): Promise<boolean> {
    setSaving(true);
    setSaved(false);
    setError(null);
    try {
      setConfig(await job);
      setSaved(true);
      return true;
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Could not save settings.");
      return false;
    } finally {
      setSaving(false);
    }
  }

  const setSetting = (key: string, value: SettingValue) => save(api.setSetting(key, value));
  const resetSetting = (key: string) => save(api.resetSetting(key));

  return { config, error, saving, saved, setSetting, resetSetting };
}
