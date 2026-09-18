import { useSyncExternalStore } from "react";

/** Background tabs keep review state, but do not start more visual work. */
export function usePageVisible() {
  return useSyncExternalStore(subscribe, visible, () => true);
}

function subscribe(changed: () => void) {
  document.addEventListener("visibilitychange", changed);
  return () => document.removeEventListener("visibilitychange", changed);
}

function visible() {
  return document.visibilityState !== "hidden";
}
