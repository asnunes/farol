import { useEffect, useRef } from "react";
import { scrollToFile } from "@/lib/scroll";

/** Take the reader to where they stopped, once, when the review arrives.
 *
 * The page opens at the top otherwise, and the first unread file is halfway
 * down a review that has been read before. */
export function useResumeAt(path: string | null) {
  const landed = useRef(false);

  useEffect(() => {
    if (landed.current || !path) return;
    landed.current = true;
    scrollToFile(path);
  }, [path]);
}
