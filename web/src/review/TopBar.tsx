import { PanelLeft, PanelLeftClose, TriangleAlert } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import type { RefreshReason } from "@/hooks/useReview";
import { Refresh } from "@/review/Refresh";
import { Publish } from "@/review/publish/Publish";
import { ViewToggle } from "@/review/ViewToggle";
import { MOD } from "@/hooks/useShortcuts";
import type { DiffView } from "@/hooks/useDiffView";
import type { Publishing } from "@/hooks/usePublishing";
import type { CommentView, ReviewView, Unreadable as UnreadableView } from "@/api";

/** Where you are and how far through you are. */
export function TopBar({
  review,
  view,
  onView,
  stale,
  onRefresh,
  unreadable,
  publishing,
  comments,
  onError,
  sidebarOpen,
  onToggleSidebar,
  toggleRef,
}: TopBarProps) {
  const done = review.totalFiles > 0 && review.viewedFiles === review.totalFiles;

  return (
    // Wrapping and tightening rather than dropping anything: every control here
    // is one a reviewer needs to finish, and a narrow screen is still a screen
    // a review gets read on.
    <header className="top col-span-full flex flex-wrap items-center justify-between gap-x-4 gap-y-2 border-b border-rule bg-surface px-4 py-2 md:px-5 md:py-2.5">
      <div className="left flex min-w-0 items-center gap-3 md:gap-3">
        {/* Ahead of the sidebar it opens and closes, which is where every
            editor puts it and the only place it cannot be mistaken for chrome
            belonging to the diff. */}
        <SidebarToggle open={sidebarOpen} onToggle={onToggleSidebar} toggleRef={toggleRef} />

        {/* The refs give way first: a branch named after a ticket and three
            words is the one thing here with no natural width, and truncating it
            costs less than pushing everything else off the bar. */}
        <div className="refs flex min-w-0 items-baseline gap-2 font-mono text-[0.8125rem]">
          <span className="head truncate font-semibold">{review.branch}</span>
          <span className="shrink-0 text-rule-strong">→</span>
          <span className="base truncate text-ink-muted">{review.base}</span>
          {/* Beside the refs it qualifies: what is behind is this branch's map,
              not anything on the right-hand side of the bar. */}
          {review.commitsBehind > 0 && (
            <Badge className="stale-chip shrink-0 rounded-full border-transparent bg-highlight-dim font-mono text-xs font-normal text-highlight">
              map {review.commitsBehind} commit{review.commitsBehind === 1 ? "" : "s"} behind
            </Badge>
          )}
        </div>
      </div>

      <div className="flex flex-wrap items-center justify-end gap-3 gap-y-2 md:gap-4">
        {unreadable.length > 0 && <Unreadable broken={unreadable} />}
        {stale && <Refresh reason={stale} onRefresh={onRefresh} />}
        <ViewToggle view={view} onChange={onView} />
        <div className="progress flex items-center gap-2 font-mono text-xs text-ink-muted">
          {done ? (
            // The only celebration in the app, and only once there is nothing
            // left to read.
            <span className="done" title="Everything read">
              🎉
            </span>
          ) : (
            <span>
              {review.viewedFiles} / {review.totalFiles}
            </span>
          )}
          <Progress
            className="meter h-1.5 w-16 bg-sunken md:w-28"
            value={review.totalFiles ? (review.viewedFiles / review.totalFiles) * 100 : 0}
            aria-label={`${review.viewedFiles} of ${review.totalFiles} read`}
          />
        </div>
        {/* Last on the bar, because it is the last thing done: everything to
            its left is the reading, and this is what closes it. */}
        <Publish
          publishing={publishing}
          comments={comments}
          read={review.viewedFiles}
          onError={onError}
        />
      </div>
    </header>
  );
}

/** Show or hide the sidebar, for the reader who wants the width back. */
function SidebarToggle({ open, onToggle, toggleRef }: SidebarToggleProps) {
  const what = `${open ? "Hide" : "Show"} the sidebar — ${MOD}B`;

  return (
    <Button
      ref={toggleRef}
      variant="ghost"
      size="icon-xs"
      className="sidebartoggle shrink-0 text-faint hover:bg-transparent hover:text-ink max-md:size-9"
      aria-expanded={open}
      aria-label={what}
      title={what}
      onClick={onToggle}
    >
      {open ? (
        <PanelLeftClose className="size-4" aria-hidden="true" />
      ) : (
        <PanelLeft className="size-4" aria-hidden="true" />
      )}
    </Button>
  );
}

/** Comment files the store could not read.
 *
 * Up here rather than beside the code, because a comment whose header is broken
 * has no line left to sit next to — that is exactly what is wrong with it. The
 * chip names the files in its tooltip, since fixing one means opening it. */
function Unreadable({ broken }: { broken: UnreadableView[] }) {
  return (
    <Tooltip delayDuration={0}>
      <TooltipTrigger asChild>
        <Badge className="unreadable cursor-default gap-1.5 rounded-full border-transparent bg-del-bg font-mono text-xs font-normal text-del-ink">
          <TriangleAlert className="size-3.5" aria-hidden="true" />
          {broken.length} comment{broken.length === 1 ? "" : "s"} unreadable
        </Badge>
      </TooltipTrigger>
      {/* A real tooltip rather than `title`, and only here: this is the one
          piece of chrome whose message is three lines per comment, which the
          browser's own tooltip crams into a strip nobody can read. */}
      <TooltipContent className="max-w-[36rem]">
        {broken.map((one) => (
          <div key={one.file} className="mb-2 last:mb-0">
            <div className="font-sans">{headline(one)}</div>
            <div className="font-sans opacity-80">{one.why}</div>
            <div className="font-mono text-[0.6875rem] opacity-70">{one.file}</div>
          </div>
        ))}
      </TooltipContent>
    </Tooltip>
  );
}

/** What the reviewer knows it by: the file it was about, or their own words. */
function headline(one: UnreadableView): string {
  return one.about ?? (one.excerpt === null ? "an empty comment" : `"${one.excerpt}"`);
}

type TopBarProps = {
  review: ReviewView;
  view: DiffView;
  onView: (view: DiffView) => void;
  stale: RefreshReason | null;
  onRefresh: () => void;
  /** Comment files that could not be parsed, in the reviewer's terms. */
  unreadable: UnreadableView[];
  publishing: Publishing;
  comments: CommentView[];
  onError: (message: string) => void;
  sidebarOpen: boolean;
  onToggleSidebar: () => void;
  /** Focus comes back here when the sidebar closes under whoever was in it. */
  toggleRef: React.Ref<HTMLButtonElement>;
};

type SidebarToggleProps = {
  open: boolean;
  onToggle: () => void;
  toggleRef: React.Ref<HTMLButtonElement>;
};
