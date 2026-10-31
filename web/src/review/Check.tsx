import { Button } from "@/components/ui/button";

/** The mark at the start of a sidebar row, as a button of its own.
 *
 * Laid over the row's left padding rather than inside the row, because the row
 * is a button already and one cannot hold another. */
export function Check({ done, label, onClick, children }: CheckProps) {
  return (
    <Button
      variant="ghost"
      size="icon-xs"
      className="check absolute top-1/2 left-1.5 z-10 size-5 -translate-y-1/2 hover:bg-sunken"
      aria-label={label}
      aria-pressed={done}
      title={label}
      onClick={onClick}
    >
      {children}
    </Button>
  );
}

type CheckProps = {
  done: boolean;
  /** What pressing it does, and to which row. */
  label: string;
  onClick: () => void;
  children: React.ReactNode;
};
