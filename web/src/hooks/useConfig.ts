import { useEffect, useState } from "react";
import { api, type Config } from "@/api";

export function useConfig(open: boolean) {
  const [config, setConfig] = useState<Config | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (!open) return;
    let active = true;
    setConfig(null);
    setError(null);
    setSaved(false);
    api.config().then(
      (value) => { if (active) setConfig(value); },
      (reason: Error) => { if (active) setError(reason.message); },
    );
    return () => { active = false; };
  }, [open]);

  async function setOpenBrowser(value: boolean) {
    setSaving(true);
    setSaved(false);
    setError(null);
    try {
      await api.setOpenBrowser(value);
      setConfig({ open_browser: value });
      setSaved(true);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Could not save settings.");
    } finally {
      setSaving(false);
    }
  }

  return { config, error, saving, saved, setOpenBrowser };
}
