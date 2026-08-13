import { useState } from "react";
import type { FileView } from "@/api";

/** Which files are open on the page.
 *
 * A file that has been read is closed: it is done, and the ones still to read
 * should not be buried under it. Anything the reader says overrides that, so a
 * file they opened stays open however it is marked, until they close it or
 * mark it again. */
export function useOpenFiles() {
  const [choice, setChoice] = useState<Record<string, boolean>>({});

  return {
    isOpen: (file: FileView) => choice[file.path] ?? !file.viewed,
    set: (path: string, open: boolean) => setChoice((all) => ({ ...all, [path]: open })),
  };
}

export type OpenFiles = ReturnType<typeof useOpenFiles>;
