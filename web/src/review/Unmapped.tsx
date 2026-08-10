/** Files the branch changed after the map was written. They are in the diff and
 * nowhere in the map, so the screen would otherwise never mention them. */
export function Unmapped({ paths }: { paths: string[] }) {
  return (
    <div className="unmapped m-6 rounded-md border border-note-rule bg-note-bg p-4">
      <strong className="font-sans text-sm text-ink">
        {paths.length} file(s) changed after this map was made
      </strong>
      <p className="mt-1 font-serif text-sm text-ink-soft">
        Run the review-map skill again to fold them in.
      </p>
      <ul className="mt-2 font-mono text-xs text-muted">
        {paths.map((p) => (
          <li key={p}>{p}</li>
        ))}
      </ul>
    </div>
  );
}
