import { useSyncExternalStore } from "react";

/** Whether the screen is too narrow to put two things side by side.
 *
 * The one query the app asks about width, and it is asked in two places: here,
 * where a component has to draw something different rather than merely style it
 * differently, and in the `max-md:` classes that do the rest. The string is what
 * Tailwind's `max-md:` compiles to, spelled out so the two cannot drift apart.
 *
 * Styling alone would be enough if the difference were only a matter of size.
 * It is not: a split row that has the same line on both sides prints it once on
 * a narrow screen, and no stylesheet can tell that two cells hold the same
 * line. */
export function useNarrow(): boolean {
  return useSyncExternalStore(watch, () => query()?.matches ?? false, () => false);
}

const NARROW = "not all and (min-width: 48rem)";

/** jsdom has no `matchMedia`, and a test that is not about the width should not
 * have to stand one up. Missing, it reads as the wide screen the app was
 * written for. */
function query(): MediaQueryList | null {
  return typeof window !== "undefined" && window.matchMedia
    ? window.matchMedia(NARROW)
    : null;
}

function watch(onChange: () => void): () => void {
  const media = query();
  media?.addEventListener("change", onChange);
  return () => media?.removeEventListener("change", onChange);
}
