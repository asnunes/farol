import { useEffect, useState } from "react";
import { api, type FileDiff } from "@/api";

/** The diff of the file being read, fetched as the reader moves. */
export function useFileDiff(path: string | null, onError: (e: string) => void) {
  const [diff, setDiff] = useState<FileDiff | null>(null);

  useEffect(() => {
    if (!path) return;
    let cancelled = false;
    api
      .file(path)
      .then((d) => !cancelled && setDiff(d))
      .catch((e) => !cancelled && onError(String(e)));
    return () => {
      // The reader moved on before this arrived; showing it would replace the
      // diff they are looking at now.
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [path]);

  return diff;
}
