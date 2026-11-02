import { useState } from "react";

/** Which threads the reader folded to one line.
 *
 * Held above the threads rather than in each, because the activity box has to
 * open one from the other end of the page. */
export function useThreadFolds() {
  const [minimized, setMinimized] = useState<ReadonlySet<string>>(new Set());

  return {
    isMinimized: (id: string) => minimized.has(id),
    toggleMinimized: (id: string) =>
      setMinimized((was) => {
        const next = new Set(was);
        if (!next.delete(id)) next.add(id);
        return next;
      }),
    expand: (id: string) =>
      setMinimized((was) => {
        if (!was.has(id)) return was;
        const next = new Set(was);
        next.delete(id);
        return next;
      }),
  };
}

export type ThreadFolds = ReturnType<typeof useThreadFolds>;
