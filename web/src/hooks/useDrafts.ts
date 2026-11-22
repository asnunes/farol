import { useState } from "react";

/** What a section of Settings holds before its Save: a list of pending
 * changes, one per key, each written by `write` when the section saves.
 *
 * One list rather than a collection per kind of change, so setting, undoing,
 * counting, discarding and saving are each written once, whatever the change
 * is. Undoing is always `unset`. */
export function useDrafts<C extends Draft>(write: (change: C) => Promise<string | null>) {
  const [changes, setChanges] = useState<C[]>([]);
  const [errors, setErrors] = useState<Record<string, string>>({});

  return {
    get: (key: string): C | undefined => changes.find((change) => change.key === key),
    all: changes,
    /** Add a change, or replace the one under the same key. */
    set: (change: C) =>
      setChanges((was) =>
        was.some((one) => one.key === change.key)
          ? was.map((one) => (one.key === change.key ? change : one))
          : [...was, change],
      ),
    unset: (key: string) => setChanges((was) => was.filter((one) => one.key !== key)),
    error: (key: string): string | undefined => errors[key],
    count: changes.length,
    discard: () => {
      setChanges([]);
      setErrors({});
    },
    /** One change at a time, in the order they were made. What was kept
     * leaves the list; what failed stays, with its reason. */
    save: async () => {
      const failed: Record<string, string> = {};
      for (const change of changes) {
        const why = await write(change);
        if (why) failed[change.key] = why;
      }
      setChanges((was) => was.filter((one) => one.key in failed));
      setErrors(failed);
    },
  };
}

export type Drafts<C extends Draft> = ReturnType<typeof useDrafts<C>>;

/** Anything pending: told apart from the rest by its key. */
export type Draft = { key: string };
