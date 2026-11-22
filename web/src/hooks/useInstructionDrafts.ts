import { useState } from "react";
import type { TextSetting } from "@/api";

/** The instructions as they are being edited: a draft per field, written
 * together by the section's Save and dropped together by Discard.
 *
 * Restoring a default is an edit like any other: the default goes into the
 * draft, and Save is what sends it, as a reset so the file stops carrying it. */
export function useInstructionDrafts(settings: TextSetting[], store: Store) {
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [errors, setErrors] = useState<Record<string, string>>({});
  const pending = settings.filter((s) => s.key in drafts && drafts[s.key] !== s.value);

  return {
    value: (s: TextSetting) => drafts[s.key] ?? s.value,
    edit: (key: string, value: string) => setDrafts((was) => ({ ...was, [key]: value })),
    restore: (s: TextSetting) => setDrafts((was) => ({ ...was, [s.key]: s.default })),
    unsaved: (s: TextSetting) => pending.includes(s),
    error: (key: string): string | undefined => errors[key],
    count: pending.length,
    discard: () => {
      setDrafts({});
      setErrors({});
    },
    /** One field at a time: what was kept leaves the drafts, what failed stays
     * with its reason under it. */
    save: async () => {
      const failed: Record<string, string> = {};
      for (const s of pending) {
        const value = drafts[s.key];
        const why = value === s.default ? await store.reset(s.key) : await store.set(s.key, value);
        if (why) failed[s.key] = why;
      }
      setDrafts((was) => Object.fromEntries(Object.entries(was).filter(([key]) => key in failed)));
      setErrors(failed);
    },
  };
}

export type InstructionDrafts = ReturnType<typeof useInstructionDrafts>;

type Store = {
  set: (key: string, value: string) => Promise<string | null>;
  reset: (key: string) => Promise<string | null>;
};
