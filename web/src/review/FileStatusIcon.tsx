import { FileDiff, FileMinus, FilePlus, FileSymlink } from "lucide-react";
import { cn, unreachable } from "@/lib/utils";
import type { FileStatus } from "@/api";

/** What the change did to the file, drawn where the file is named: a document
 * with the change's sign in it, the way GitHub's file tree draws it. New and
 * deleted take the diff's colours, so a file the change took away is plain to
 * see while it sits folded; a modified one, the common case, stays quiet. */
export function FileStatusIcon({ status, className }: FileStatusIconProps) {
  const { Icon, tone } = look(status);
  return (
    <Icon
      className={cn("status shrink-0", tone, className)}
      data-status={status}
      role="img"
      aria-label={statusName(status)}
    />
  );
}

/** The status in words, for a tooltip and for a screen reader. */
export function statusName(status: FileStatus): string {
  switch (status) {
    case "added":
      return "New file";
    case "modified":
      return "Modified";
    case "deleted":
      return "Deleted";
    case "renamed":
      return "Renamed";
    default:
      return unreachable(status);
  }
}

function look(status: FileStatus): Look {
  switch (status) {
    case "added":
      return { Icon: FilePlus, tone: "text-add-ink" };
    case "modified":
      return { Icon: FileDiff, tone: "text-faint" };
    case "deleted":
      return { Icon: FileMinus, tone: "text-del-ink" };
    case "renamed":
      return { Icon: FileSymlink, tone: "text-faint" };
    default:
      return unreachable(status);
  }
}

type FileStatusIconProps = { status: FileStatus; className?: string };

type Look = { Icon: typeof FileDiff; tone: string };
