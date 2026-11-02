import { useCallback, useRef } from "react";
import { lensAt, readingOrder, type CommentView, type LensView, type ReviewView } from "@/api";
import { unreadAfter } from "@/hooks/useCurrentFile";
import type { CommentActions } from "@/hooks/useComments";
import type { OpenFiles } from "@/hooks/useOpenFiles";
import { scrollToFile, scrollToLens, scrollToThread } from "@/lib/scroll";
import { unreachable } from "@/lib/utils";

/** How the reader moves through the review: where they are in the reading
 * order, going to a file or a lens, the next unread, and what marking
 * something read does to where they stand.
 *
 * Out of `App`, which composes the screen and should not also decide how the
 * reader moves through it; and testable on its own, where it used to be
 * reachable only through the whole page. */
export function useNavigation({ review, current, setCurrent, toggleViewed, toggleSeen, files, comments }: NavigationInputs) {
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
      //
      // From the lens that was marked, which is not always where the reader
      // is: a tick can be clicked on a lens that has not reached the line yet.
      const from = placeOf(lens) ?? current;
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
    } else if (about.kind === "lens" && about.file !== null) {
      goTo(about.file);
    }
    scrollToThread(thread.id);
  };

  return { order, index, goTo, goToPlace, nextUnread, mark, markLens, jumpToThread };
}

export type Navigation = ReturnType<typeof useNavigation>;

/** The place a pull request or block lens holds on the page, as `lens:<key>`. */
function placeOf(lens: LensView): string | null {
  const target = lens.target;
  if (!target) return null;
  switch (target.kind) {
    case "pr":
    case "block":
      return `lens:${lens.key}`;
    // A file's lens sits inside the file, which is the place to go to.
    case "file":
      return null;
    default:
      return unreachable(target);
  }
}

type NavigationInputs = {
  review: ReviewView | null;
  /** A file path, or `lens:<key>` for a pull request or block lens. */
  current: string | null;
  setCurrent: (place: string) => void;
  toggleViewed: (path: string, viewed: boolean) => Promise<void>;
  toggleSeen: (lens: LensView, seen: boolean) => Promise<void>;
  files: OpenFiles;
  comments: Pick<CommentActions, "expand" | "drifted">;
};
