/** Says a newer map is on disk, and takes it when clicked.
 *
 * Announced rather than applied on its own: a map that changes while somebody
 * is halfway through moves the blocks and the file they are reading. */
export function Refresh({ onRefresh }: RefreshProps) {
  return (
    <button
      className="refresh cursor-pointer rounded-full bg-accent-dim px-2.5 py-1 font-mono text-xs text-accent transition-opacity hover:opacity-80"
      onClick={onRefresh}
      title="A newer map was written. Click to read it."
    >
      new map · refresh
    </button>
  );
}

type RefreshProps = { onRefresh: () => void };
