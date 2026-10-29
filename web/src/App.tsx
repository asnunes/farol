import { useCallback, useRef, useState } from "react";
import { TriangleAlert, X } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { TooltipProvider } from "@/components/ui/tooltip";
import { lensAt, readingOrder, type CommentView, type LensView } from "@/api";
import { cn } from "@/lib/utils";
import { useComments } from "@/hooks/useComments";
import { useActivity } from "@/hooks/useActivity";
import { useGitHub } from "@/hooks/useGitHub";
import { useDiffView } from "@/hooks/useDiffView";
import { useOpenFiles } from "@/hooks/useOpenFiles";
import { useTheme } from "@/hooks/useTheme";
import { useReview } from "@/hooks/useReview";
import { useShortcuts } from "@/hooks/useShortcuts";
import { useSidebar } from "@/hooks/useSidebar";
import { Activity } from "@/review/activity/Activity";
import { HelpDialog } from "@/review/HelpDialog";
import { KeyBar } from "@/review/KeyBar";
import { scrollToFile, scrollToLens, scrollToThread } from "@/lib/scroll";
import { unreadAfter } from "@/hooks/useCurrentFile";
import { Reading } from "@/review/Reading";
import { Sidebar } from "@/review/Sidebar";
import { TopBar } from "@/review/TopBar";

/** Composition only: what is on screen and in what order. Everything that
 * decides how a thing looks lives in the piece that draws it. */
export default function App() {
  const {
    review,
    current,
    setCurrent,
    error,
    toggleViewed,
    toggleSeen,
    stale,
    refresh,
    generation,
  } = useReview();
  const [helpOpen, setHelpOpen] = useState(false);
  const { open: sidebarOpen, toggle, dismiss } = useSidebar();
  // Where focus goes when the sidebar it opens is taken out from under it.
  const sidebarToggle = useRef<HTMLButtonElement>(null);
  // What the reader just tried and did not get: a comment that would not save,
  // a review the host turned down. Apart from the load failure above, because
  // the review is still on the screen and still worth reading, and blanking it
  // would take away the very thing the message is about.
  const [failed, setFailed] = useState<string | null>(null);
  const [view, setView] = useDiffView();
  const files = useOpenFiles();
  const [theme, setTheme] = useTheme();
  const github = useGitHub(setFailed, generation);
  const comments = useComments(setFailed, generation, github);
  const activity = useActivity(comments.all, github.pull, review?.branch ?? "", comments.loaded);
  const [activityOpen, setActivityOpen] = useState(false);

  // The sidebar can go while the reader is standing in it, and focus has to be
  // put somewhere before it does — dropped on the document, the next Tab
  // starts the page over. Only from inside it: ⌘B is deliberately live while a
  // comment is being typed, and that box keeps the cursor.
  const toggleSidebar = useCallback(() => {
    if (document.querySelector(".sidebar")?.contains(document.activeElement)) {
      sidebarToggle.current?.focus();
    }
    toggle();
  }, [toggle]);

  const order = review ? readingOrder(review) : [];
  // On a lens, the keys count from just before what it opens: the block's
  // first file, or the first file of all for the pull request's.
  const onLens = review ? lensAt(review, current) : null;
  const index = onLens
    ? order.findIndex((f) => f.path === onLens.block?.files[0]?.path)
      - (onLens.block ? 1 : 0)
    : order.findIndex((f) => f.path === current);
  const reading = useRef(order);
  reading.current = order;

  // Moving names the file *and* scrolls to it. Naming it only through the
  // scroll would mean waiting for the observer to answer, and two presses in a
  // row would both count from the file the reader had already left.
  const goTo = useCallback(
    (path: string) => {
      setCurrent(path);
      // Going to a file opens it. Being taken to one that stayed folded away
      // because it had been read would look like arriving nowhere.
      //
      // Through the ref because this function is handed to the keyboard effect
      // and has to keep its identity across renders, while the order it looks
      // the file up in is a new array on every one of them.
      //
      // Not one whose lenses stand in for it: its lenses are on screen, and
      // the code stays behind Show code unless the map puts it in focus.
      const file = reading.current.find((f) => f.path === path);
      if (file && !(file.folded && !file.viewed)) files.set(file, true);
      scrollToFile(path);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [setCurrent],
  );

  // Where the cursor is, is the browser's to know, so the search from it
  // happens here. Wrapping round is not: the last unread file may be behind
  // you after marking things read, and the one to wrap to is the same
  // first-unread the server hands over for the landing.
  const nextUnread = () => {
    const ahead = order.slice(index + 1).find((f) => !f.viewed)?.path;
    const next = ahead ?? review?.firstUnread;
    if (next) goTo(next);
  };

  // Marking a file read folds it away, and unmarking opens it again — both by
  // the mark itself, without a choice being recorded here. Folding ahead of the
  // server would be this side deciding a file is read before it is, and the tick
  // box and the block counts already wait for the answer; a fold that did not
  // would be the one thing on the row moving early.
  const mark = useCallback(
    (path: string, viewed: boolean) => {
      // Marking one read folds it away, which leaves the reader looking at
      // whatever was underneath. Take them to what is still to be read.
      if (viewed) goToPlace(unreadAfter(path));
      return toggleViewed(path, viewed);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [toggleViewed],
  );

  // The same for a lens: seen folds it away, so the reader is taken to what
  // comes after it rather than left on it for the next press to unfold.
  const markLens = useCallback(
    (lens: LensView, seen: boolean) => {
      // After the server answers, not before: seeing a block's lenses can
      // make its files read, and the next unread place is only known then.
      const from = current;
      return toggleSeen(lens, seen).then(() => {
        if (seen) requestAnimationFrame(() => goToPlace(unreadAfter(from)));
      });
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [current, toggleSeen],
  );

  // A place is a file path or `lens:<key>`; going to a lens names it and
  // scrolls, going to a file goes the way the keys do.
  function goToPlace(place: string | null) {
    if (place?.startsWith("lens:")) {
      setCurrent(place);
      scrollToLens(place.slice("lens:".length));
    } else if (place) {
      goTo(place);
    }
  }

  // From the activity box to a thread: its file opened if reading had folded
  // it, the thread opened if the reader had minimised it, and the thread
  // brought to the middle of the pane.
  //
  // A thread off the diff is not in its file but after its block, or at the
  // end of the page, and is always drawn: there is no file to open first.
  // A thread on a lens or a block is drawn where its text is: the lens is
  // reached the way the sidebar reaches it, a file's lens by opening its file.
  const jumpToThread = (thread: CommentView) => {
    comments.expand(thread.id);
    const about = thread.about;
    if (about.kind === "lines") {
      if (!comments.drifted.some((t) => t.id === thread.id)) goTo(about.path);
    } else if (about.kind === "lens" && about.key.startsWith("file:")) {
      goTo(about.key.slice("file:".length, about.key.lastIndexOf(":")));
    }
    scrollToThread(thread.id);
  };

  useShortcuts({
    review,
    order,
    index,
    current,
    goTo,
    nextUnread,
    toggleViewed: mark,
    toggleSeen: markLens,
    setHelpOpen,
    toggleSidebar,
    toggleActivity: () => setActivityOpen((open) => !open),
  });

  if (error) {
    return (
      <Alert
        variant="destructive"
        className="fatal m-4 w-auto border-rule bg-surface md:m-8"
      >
        <TriangleAlert />
        <AlertTitle className="font-sans">
          The review could not be loaded
        </AlertTitle>
        <AlertDescription className="font-mono text-sm whitespace-pre-wrap">
          {error}
        </AlertDescription>
      </Alert>
    );
  }
  if (!review) {
    return (
      <div className="fatal p-8 font-mono text-sm text-ink-muted">Loading…</div>
    );
  }

  return (
    <TooltipProvider>
      <div
        className={cn(
          // `dvh` and not `vh`: a phone's browser chrome slides in and out over
          // the bottom of the viewport, and `vh` measures it as if it were
          // never there — which puts the key bar under it.
          "app grid h-dvh",
          sidebarOpen
            ? "grid-cols-[minmax(0,1fr)] grid-rows-[auto_auto_minmax(0,1fr)] md:grid-cols-[19rem_minmax(0,1fr)] md:grid-rows-[auto_minmax(0,1fr)]"
            : "grid-cols-[minmax(0,1fr)] grid-rows-[auto_minmax(0,1fr)]",
        )}
      >
        <TopBar
          review={review}
          view={view}
          onView={setView}
          stale={stale}
          onRefresh={() => void refresh()}
          unreadable={comments.unreadable}
          sidebarOpen={sidebarOpen}
          onToggleSidebar={toggleSidebar}
          toggleRef={sidebarToggle}
          onNextUnread={nextUnread}
          github={github}
          threads={comments.all}
        />
        {sidebarOpen && (
          <Sidebar
            review={review}
            comments={comments.on}
            current={current}
            onPick={(place) => {
              goToPlace(place);
              // Over the code, picking is the end of what the sidebar was for,
              // so it goes — and the row that was clicked goes with it.
              if (dismiss()) sidebarToggle.current?.focus();
            }}
          />
        )}

        <Reading
          key={generation}
          review={review}
          view={view}
          current={current}
          onCurrent={setCurrent}
          onToggleViewed={(path, viewed) => void mark(path, viewed)}
          files={files}
          comments={comments}
          onSeen={(lens, seen) => void toggleSeen(lens, seen)}
        />

        <KeyBar
          theme={theme}
          onTheme={setTheme}
          sidebarOpen={sidebarOpen}
          onHelp={() => setHelpOpen(true)}
        />
        <HelpDialog open={helpOpen} onOpenChange={setHelpOpen} />
        {/* Only where there is an agent to hear from, or something it already
            did: on a review with neither, the corner stays empty. */}
        {(comments.agent || activity.updates.length > 0 || activity.working.length > 0) && (
          <Activity
            activity={activity}
            open={activityOpen}
            onOpen={setActivityOpen}
            onJump={jumpToThread}
          />
        )}
        {failed && <Failed what={failed} onClose={() => setFailed(null)} />}
      </div>
    </TooltipProvider>
  );
}

/** Something the reader tried that did not happen.
 *
 * Over the review rather than in it: the page keeps its shape, nothing below
 * jumps, and what they were reading when it failed is still where they left it.
 * It sits above the key bar and waits to be dismissed, because a message that
 * fades on its own is one they can miss while looking at the code. */
function Failed({ what, onClose }: { what: string; onClose: () => void }) {
  return (
    <Alert
      variant="destructive"
      role="alert"
      className="failed fixed right-4 bottom-24 left-4 z-50 flex w-auto max-w-[34rem] items-start gap-3 border-rule bg-surface shadow-lg sm:left-auto"
    >
      <TriangleAlert />
      <AlertDescription className="min-w-0 flex-1 font-mono text-sm whitespace-pre-wrap">
        {what}
      </AlertDescription>
      <Button
        size="icon-xs"
        variant="ghost"
        className="text-faint hover:text-ink"
        aria-label="Dismiss"
        onClick={onClose}
      >
        <X className="size-3.5" aria-hidden="true" />
      </Button>
    </Alert>
  );
}
