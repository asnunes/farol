import type { AboutView } from "./wire/AboutView";
import type { BlockView } from "./wire/BlockView";
import type { CommentView } from "./wire/CommentView";
import type { FileView } from "./wire/FileView";
import type { LensTarget } from "./wire/LensTarget";
import type { LensView } from "./wire/LensView";
import type { ReviewView } from "./wire/ReviewView";
import type { Side } from "./wire/Side";
import type { CommentsView } from "./wire/CommentsView";
import type { FileDiffView } from "./wire/FileDiffView";
import type { OpenedPullView } from "./wire/OpenedPullView";
import type { PullRequestView } from "./wire/PullRequestView";
import type { SentView } from "./wire/SentView";
import type { SyncedView } from "./wire/SyncedView";
import type { ReviewMode } from "./wire/ReviewMode";

/** Everything the browser asks the server for. */
export const api = {
  config: () => fetch("/api/config").then(json<Config>),
  setSetting: (key: string, value: SettingValue) =>
    fetch("/api/config", {
      method: "PUT",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ key, value }),
    }).then(json<Config>),
  /** Back to the default, answered with the configuration as it now stands. */
  resetSetting: (key: string) =>
    fetch(`/api/config/${encodeURIComponent(key)}`, { method: "DELETE" }).then(json<Config>),
  lenses: () => fetch("/api/lenses").then(json<LensCatalogView>),
  addLens: (lens: NewLens) =>
    fetch("/api/lenses", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(lens),
    }).then(json<LensCatalogView>),
  /** Only what is sent changes: the words, the switch, or both. */
  saveLens: (id: string, change: LensChange) =>
    fetch(`/api/lenses/${encodeURIComponent(id)}`, {
      method: "PUT",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(change),
    }).then(json<LensCatalogView>),
  /** Gone for good, a lens farol ships included. */
  deleteLens: (id: string) =>
    fetch(`/api/lenses/${encodeURIComponent(id)}`, { method: "DELETE" }).then(json<LensCatalogView>),
  review: () => fetch("/api/review").then(json<ReviewView>),
  file: (path: string) =>
    fetch(`/api/file?path=${encodeURIComponent(path)}`).then(json<FileDiff>),
  setViewed: (path: string, viewed: boolean) =>
    send("/api/viewed", "POST", { path, viewed }),
  /** Mark a lens as seen, or take the mark back; it is pinned to the answer. */
  /** Stop the server; it answers before it goes. */
  stop: () => send("/api/stop", "POST"),
  seeLens: (lens: string, target: LensTarget, seen: boolean) =>
    send("/api/lenses/seen", "POST", { lens, target, seen }),

  /** A stretch of the file the diff did not print, for a reader opening a gap. */
  lines: (path: string, from: number, to: number) =>
    fetch(`/api/lines?path=${encodeURIComponent(path)}&from=${from}&to=${to}`)
      .then(json<{ lines: string[] }>)
      .then((answer) => answer.lines),

  comments: () => fetch("/api/comments").then(json<CommentsView>),
  /** Answers with the comment as stored, whose id is what a Send needs. */
  addComment: (about: NewAbout, body: string) =>
    fetch("/api/comments", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ ...about, body }),
    }).then(json<CommentView>),
  /** The thread only: its pending comment stays on the pull request. */
  closeComment: (id: string) => send(`/api/comments/${encodeURIComponent(id)}`, "DELETE"),
  /** The reviewer writing back under a comment; sending it on is `sendComment`. */
  replyComment: (id: string, body: string) =>
    fetch(`/api/comments/${encodeURIComponent(id)}/replies`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ body }),
    }).then(json<CommentView>),
  /** Only the id goes: the server builds the message from the comment it holds. */
  sendComment: (id: string) =>
    send(`/api/comments/${encodeURIComponent(id)}/send`, "POST"),

  /** The pull request, the review mode and each thread's pending comment. */
  github: () => fetch("/api/github").then(json<PullRequestView>),
  chooseMode: (mode: ReviewMode) => send("/api/mode", "PUT", { mode }),
  /** The reviewer's own words, into their pending review on the pull request. */
  writePending: (id: string, body: string) =>
    send(`/api/comments/${encodeURIComponent(id)}/pending`, "POST", { body }),
  editPending: (id: string, body: string) =>
    send(`/api/comments/${encodeURIComponent(id)}/pending`, "PUT", { body }),
  removePending: (id: string) =>
    send(`/api/comments/${encodeURIComponent(id)}/pending`, "DELETE"),
  /** Ask the agent to write it; `instruction` is the note for this one. */
  generatePending: (id: string, instruction = "") =>
    send(`/api/comments/${encodeURIComponent(id)}/generate`, "POST", { instruction }),
  /** Only the verdict and summary go: what is submitted is what is pending. */
  submitReview: (verdict: Verdict, summary: string) =>
    fetch("/api/github/review", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ verdict, summary }),
    }).then(json<SentView>),
  /** Push the reviewer's own branch, so the pull request shows what was read. */
  pushBranch: () => send("/api/github/push", "POST"),
  /** The pull request's read marks made to match the ones here. */
  syncRead: () => fetch("/api/github/read", { method: "POST" }).then(json<SyncedView>),
  /** Pushes the branch first when the host has not seen it. */
  openPull: (title: string, body: string) =>
    fetch("/api/github/pull", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ title, body }),
    }).then(json<OpenedPullView>),
  /** The agent writes the draft back; it arrives with `PullRequestView.draft`. */
  generatePullDraft: () => send("/api/github/pull/draft", "POST"),
};

/** The page, top to bottom: each block announced, then the files it holds, then
 * the files that belong to no block at all.
 *
 * The one place the browser says what that order is. It was said twice — here
 * for the keys and again in the pane for the rows — and nothing held the two
 * together: a group added to one and not the other leaves `j` walking to a file
 * that is not the one below, and the scroll naming a third.
 *
 * The server states it once as well, to answer where the reading resumes. The
 * two meet at `firstUnread`, and `a review resumes at the file the page puts
 * first` is the test that keeps them meeting. */
export function readingRows(review: ReviewView): ReadingRow[] {
  return [
    ...review.blocks.flatMap((block, i): ReadingRow[] => [
      { block, number: i + 1 },
      ...block.files.map((file) => ({ file })),
    ]),
  ];
}

/** The files alone, in that same order — what j/k and "next unread" walk.
 *
 * Taken off the rows rather than assembled again, so the list the keys move
 * through is the list on the screen by construction. */
export function readingOrder(review: ReviewView): FileView[] {
  return readingRows(review).flatMap((row) => ("file" in row ? [row.file] : []));
}

/** A lens the reader is on, and the block it opens when it has one. */
export type LensHere = { lens: LensView; block: BlockView | null };

/** One line of the page: a block's band, or a file under it. */
export type ReadingRow = { block: BlockView; number: number } | { file: FileView };

/** The block a file is rendered under, for the band above the diff. */
export function blockOf(review: ReviewView, path: string): FileHome | null {
  for (let i = 0; i < review.blocks.length; i++) {
    if (review.blocks[i].files.some((f) => f.path === path)) {
      return { block: review.blocks[i], index: i };
    }
  }
  return null;
}

/** Where a lens is in the reading, as `current` names it: its key with `lens:`
 * in front, so it can never be taken for a path. The one place that spelling is
 * written; `lensKeyOf` reads it back. */
export function lensPlace(key: string): string {
  return `lens:${key}`;
}

/** The lens key a place names, or null when it names a file. */
export function lensKeyOf(place: string): string | null {
  return place.startsWith("lens:") ? place.slice("lens:".length) : null;
}

/** The pull request or block lens the reader is on, when `current` names one
 * rather than a file: the lens's key with `lens:` in front, so it can never be
 * taken for a path. */
export function lensAt(review: ReviewView, current: string | null): LensHere | null {
  const pr = review.lenses.find((lens) => current === lensPlace(lens.key));
  if (pr) return { lens: pr, block: null };
  for (const block of review.blocks) {
    const lens = block.lenses.find((l) => current === lensPlace(l.key));
    if (lens) return { lens, block };
  }
  return null;
}

// The shapes the server sends are generated from the Rust views by ts-rs into
// `wire/`: written once, on the side that sends them. Re-exported here under
// the names the screen already reads them by.
export type { AboutView, BlockView, CommentView, CommentsView, FileView, LensTarget, LensView, ReviewView, Side };
export type { OpenedPullView, PullRequestView, ReviewMode, SentView, SyncedView };
export type { CitationView } from "./wire/CitationView";
export type { DriftView } from "./wire/DriftView";
export type { FileStatus } from "./wire/FileStatus";
export type { LineNoteView } from "./wire/LineNoteView";
export type { LoosePendingView } from "./wire/LoosePendingView";
export type { PendingByView } from "./wire/PendingByView";
export type { PullDraftView } from "./wire/PullDraftView";
export type { RemotePendingView } from "./wire/RemotePendingView";
export type { ReplyView } from "./wire/ReplyView";
export type { WaitingView } from "./wire/WaitingView";
export type FileDiff = FileDiffView;
export type { HunkView as Hunk } from "./wire/HunkView";
export type { LineView as DiffLine } from "./wire/LineView";
export type { UnreadableView as Unreadable } from "./wire/UnreadableView";

/** A thread on a span of lines, the one kind that can become a comment for
 * the pull request. */
export type LinesAbout = Extract<AboutView, { kind: "lines" }>;

/** What a new thread is about, as the server takes it. */
export type NewAbout =
  | { path: string; side: Side; from: number; to: number }
  | { lens: string; target: LensTarget }
  | { block: string };

/** The lines a thread is about, or null for one on a lens or a block. */
export function linesOf(thread: CommentView): LinesAbout | null {
  return thread.about.kind === "lines" ? thread.about : null;
}

export type Verdict = "comment" | "request-changes" | "approve";

/** Where a file sits: the block it is read under and how far down the map that
 * block is, which is what the band above the diff counts off. */
export type FileHome = { block: BlockView; index: number };

export type LensGroup = "pr" | "block" | "file";

export type LensCatalogView = { lenses: LensRow[] };

/** A lens as Settings lists it. */
export type LensRow = { id: string; title: string; group: LensGroup; prompt: string; enabled: boolean };

export type NewLens = { title: string; group: LensGroup; prompt: string };

export type LensChange = { title?: string; prompt?: string; enabled?: boolean };

export type Config = { settings: Setting[] };

/** One preference, told apart by `type`: the screen draws a switch or a text
 * box from it without knowing which preference it is. */
export type Setting = BooleanSetting | TextSetting;

export type SettingValue = boolean | string;

export type BooleanSetting = {
  key: string;
  title: string;
  description: string | null;
  type: "boolean";
  default: boolean;
  value: boolean;
};

export type TextSetting = {
  key: string;
  title: string;
  description: string | null;
  type: "text";
  default: string;
  value: string;
};

async function json<T>(res: Response): Promise<T> {
  if (!res.ok) throw new Error(await res.text());
  return res.json() as Promise<T>;
}


async function send(url: string, method: string, body?: unknown) {
  const res = await fetch(url, {
    method,
    ...(body === undefined
      ? {}
      : { headers: { "content-type": "application/json" }, body: JSON.stringify(body) }),
  });
  if (!res.ok) throw new Error(await res.text());
}
