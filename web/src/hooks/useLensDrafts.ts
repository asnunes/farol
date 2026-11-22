import { useState } from "react";
import type { LensChange, LensGroup, LensRow, NewLens } from "@/api";

/** The lens catalog as it is being edited: words changed, lenses added and
 * lenses deleted, all held until the section's Save. The switch that turns a
 * lens on or off is not here: it saves as it is flipped, like every switch. */
export function useLensDrafts(lenses: LensRow[], store: Store) {
  const [edits, setEdits] = useState<Record<string, Words>>({});
  const [removed, setRemoved] = useState<string[]>([]);
  const [added, setAdded] = useState<Added[]>([]);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const changed = lenses.filter((lens) => {
    const edit = edits[lens.id];
    return edit && (edit.title !== lens.title || edit.prompt !== lens.prompt);
  });

  return {
    words: (lens: LensRow): Words => edits[lens.id] ?? { title: lens.title, prompt: lens.prompt },
    edit: (id: string, words: Words) => setEdits((was) => ({ ...was, [id]: words })),
    unsaved: (lens: LensRow) => changed.includes(lens),
    removed: (id: string) => removed.includes(id),
    remove: (id: string) => setRemoved((was) => [...was, id]),
    keep: (id: string) => setRemoved((was) => was.filter((one) => one !== id)),
    added: (group: LensGroup) => added.filter((one) => one.group === group),
    add: (group: LensGroup) =>
      setAdded((was) => [...was, { draft: `new-${Date.now()}-${was.length}`, group, title: "", prompt: "" }]),
    editAdded: (draft: string, words: Words) =>
      setAdded((was) => was.map((one) => (one.draft === draft ? { ...one, ...words } : one))),
    drop: (draft: string) => setAdded((was) => was.filter((one) => one.draft !== draft)),
    error: (id: string): string | undefined => errors[id],
    count: changed.length + removed.length + added.length,
    discard: () => {
      setEdits({});
      setRemoved([]);
      setAdded([]);
      setErrors({});
    },
    /** Deletions, then changed words, then new lenses, one at a time. What was
     * kept leaves the drafts; what failed stays, with its reason. */
    save: async () => {
      const failed: Record<string, string> = {};
      for (const id of removed) {
        const why = await store.remove(id);
        if (why) failed[id] = why;
      }
      for (const lens of changed) {
        const why = await store.change(lens.id, edits[lens.id]);
        if (why) failed[lens.id] = why;
      }
      for (const one of added) {
        const why = await store.add({ group: one.group, title: one.title, prompt: one.prompt });
        if (why) failed[one.draft] = why;
      }
      setRemoved((was) => was.filter((id) => id in failed));
      setEdits((was) => Object.fromEntries(Object.entries(was).filter(([id]) => id in failed)));
      setAdded((was) => was.filter((one) => one.draft in failed));
      setErrors(failed);
    },
  };
}

export type LensDrafts = ReturnType<typeof useLensDrafts>;

export type Words = { title: string; prompt: string };

export type Added = Words & { draft: string; group: LensGroup };

type Store = {
  add: (lens: NewLens) => Promise<string | null>;
  change: (id: string, change: LensChange) => Promise<string | null>;
  remove: (id: string) => Promise<string | null>;
};
