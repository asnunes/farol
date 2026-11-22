import { useEffect, useState } from "react";
import { api, type LensCatalogView, type LensChange, type NewLens } from "@/api";

/** The lens catalog as Settings edits it, read afresh each time the panel opens. */
export function useLenses(open: boolean) {
  const [catalog, setCatalog] = useState<LensCatalogView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    let active = true;
    setError(null);
    api.lenses().then(
      (value) => { if (active) setCatalog(value); },
      (reason: Error) => { if (active) setError(reason.message); },
    );
    return () => { active = false; };
  }, [open]);

  /** Null once the server kept it; otherwise why it did not. */
  async function save(job: Promise<LensCatalogView>): Promise<string | null> {
    setSaving(true);
    setError(null);
    try {
      setCatalog(await job);
      return null;
    } catch (reason) {
      const why = reason instanceof Error ? reason.message : "Could not save the lens.";
      setError(why);
      return why;
    } finally {
      setSaving(false);
    }
  }

  return {
    lenses: catalog?.lenses ?? [],
    error,
    saving,
    add: (lens: NewLens) => save(api.addLens(lens)),
    change: (id: string, change: LensChange) => save(api.saveLens(id, change)),
    remove: (id: string) => save(api.deleteLens(id)),
  };
}
