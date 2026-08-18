import { useCallback, useRef, useState } from "react";
import { api, type FileDiff } from "@/api";
import { said } from "@/lib/utils";

/** The diffs the reader has reached, fetched once each and kept.
 *
 * The review is one long page, so a diff arrives as the reader gets near it
 * rather than all of them at the start: a branch of a hundred files would ask
 * for a hundred diffs before showing anything. Once fetched it stays, because
 * scrolling back up is the commonest thing a reviewer does. */
export function useDiffs(onError: (message: string) => void) {
  const [diffs, setDiffs] = useState<Record<string, FileDiff>>({});
  const asked = useRef(new Set<string>());

  const request = useCallback(
    (path: string) => {
      if (asked.current.has(path)) return;
      asked.current.add(path);

      api
        .file(path)
        .then((diff) => setDiffs((all) => ({ ...all, [path]: diff })))
        .catch((e) => {
          // Let it be asked for again: the reader will scroll past it twice.
          asked.current.delete(path);
          onError(said(e));
        });
    },
    [onError],
  );

  return { diffs, request };
}
