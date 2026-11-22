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

  /** Null once the server kept it; otherwise why it did not, for the field
   * that asked to say under itself. */
  async function save(job: Promise<Config>): Promise<string | null> {
    setSaving(true);
    setSaved(false);
    setError(null);
    try {
      setConfig(await job);
      setSaved(true);
      return null;
    } catch (reason) {
      const why = reason instanceof Error ? reason.message : "Could not save settings.";
      setError(why);
      return why;
    } finally {
      setSaving(false);
    }
  }

  const setSetting = (key: string, value: SettingValue) => save(api.setSetting(key, value));
  const resetSetting = (key: string) => save(api.resetSetting(key));

  return { config, error, saving, saved, setSetting, resetSetting };
}
