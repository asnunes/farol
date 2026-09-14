import { useCallback, useEffect, useRef, useState } from "react";
import { api, type FileDiff } from "@/api";
import { said } from "@/lib/utils";

/** The diffs the reader has reached, fetched once each and kept, and what went
 * wrong for the ones that did not arrive.
 *
 * The review is one long page, so a diff arrives as the reader gets near it
 * rather than all of them at the start: a branch of a hundred files would ask
 * for a hundred diffs before showing anything. Once fetched it stays, because
 * scrolling back up is the commonest thing a reviewer does. */
export function useDiffs() {
  const [diffs, setDiffs] = useState<Record<string, FileDiff>>({});
  const [errors, setErrors] = useState<Record<string, string>>({});
  const asked = useRef(new Set<string>());
  const active = useRef(true);

  // A refreshed reading owns a new cache. Requests from the previous reading
  // must not report errors or deliver code after that reading has unmounted.
  useEffect(() => {
    active.current = true;
    return () => { active.current = false; };
  }, []);

  const request = useCallback((path: string) => {
    if (asked.current.has(path)) return;
    asked.current.add(path);

    api
      .file(path)
      .then((diff) => {
        if (active.current) setDiffs((all) => ({ ...all, [path]: diff }));
      })
      .catch((e) => {
        if (!active.current) return;
        // The path stays asked for. A failure is answered by the reader
        // pressing the button, not by the next render: the section is watched
        // again every time the page draws, and a path let go of here is one
        // that is fetched, and fails, on every unrelated keystroke.
        setErrors((all) => ({ ...all, [path]: said(e) }));
      });
  }, []);

  const retry = useCallback(
    (path: string) => {
      asked.current.delete(path);
      // Cleared before the request, not after it: the section falls back to
      // its loading line while the second attempt is in the air.
      setErrors((all) => {
        const rest = { ...all };
        delete rest[path];
        return rest;
      });
      request(path);
    },
    [request],
  );

  return { diffs, errors, request, retry };
}
