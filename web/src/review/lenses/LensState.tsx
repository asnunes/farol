import type { LensView } from "@/api";

/** When the lens was written, and whether the code moved since.
 *
 * Never "current": a file that did not change does not prove the answer still
 * holds, so the page only says when it knows something changed. */
export function LensState({ lens }: LensStateProps) {
  if (lens.state === "missing") return null;
  return (
    <>
      <span className="written text-xs text-faint">written at {lens.writtenAt}</span>
      {lens.state === "outdated" && (
        <span className="outdated rounded-full bg-highlight-dim px-2 text-[0.6875rem] text-highlight">
          {lens.changed === 0
            ? "Outdated · its prompt changed"
            : `Outdated · ${lens.changed} ${lens.changed === 1 ? "file" : "files"} changed since`}
        </span>
      )}
    </>
  );
}

type LensStateProps = { lens: LensView };
