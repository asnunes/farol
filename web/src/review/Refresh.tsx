import { Button } from "@/components/ui/button";

/** Says a newer map is on disk, and takes it when clicked.
 *
 * Announced rather than applied on its own: a map that changes while somebody
 * is halfway through moves the blocks and the file they are reading. */
export function Refresh({ onRefresh }: RefreshProps) {
  return (
    <Button
      variant="ghost"
      size="xs"
      className="refresh rounded-full bg-highlight-dim font-mono text-highlight hover:bg-highlight-dim hover:opacity-80"
      onClick={onRefresh}
      title="A newer map was written. Click to read it."
    >
      new map · refresh
    </Button>
  );
}

type RefreshProps = { onRefresh: () => void };
