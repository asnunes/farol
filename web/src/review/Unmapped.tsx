import { TriangleAlert } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";

/** Files the branch changed after the map was written. They are in the diff and
 * nowhere in the map, so the screen would otherwise never mention them. */
export function Unmapped({ paths }: UnmappedProps) {
  return (
    <Alert className="unmapped m-4 w-auto border-note-rule bg-note-bg md:m-6">
      <TriangleAlert className="text-highlight" />
      <AlertTitle className="font-sans text-ink">
        {paths.length} file(s) changed after this map was made
      </AlertTitle>
      <AlertDescription className="text-ink-soft">
        <p className="font-serif text-sm">Run the farol skill again to fold them in.</p>
        {/* A path has no spaces to break at, so it is broken anywhere rather than
            pushing the alert — and the page under it — sideways. */}
        <ul className="font-mono text-xs break-all text-ink-muted">
          {paths.map((p) => (
            <li key={p}>{p}</li>
          ))}
        </ul>
      </AlertDescription>
    </Alert>
  );
}

type UnmappedProps = { paths: string[] };
