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
  seeLens: (lens: string, target: LensTarget, seen: boolean) =>
    send("/api/lenses/seen", "POST", { lens, target, seen }),

  /** A stretch of the file the diff did not print, for a reader opening a gap. */
  lines: (path: string, from: number, to: number) =>
    fetch(`/api/lines?path=${encodeURIComponent(path)}&from=${from}&to=${to}`)
      .then(json<{ lines: string[] }>)
      .then((answer) => answer.lines),

  comments: () => fetch("/api/comments").then(json<CommentsView>),
  /** Answers with the comment as stored, whose id is what a Send needs. */
  addComment: (path: string, side: Side, from: number, to: number, body: string) =>
    fetch("/api/comments", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ path, side, from, to, body }),
    }).then(json<CommentView>),
  /** With `discardPending`, the thread's pending comment is taken off the pull
   * request too; without it, a thread that still has one is refused. */
  closeComment: (id: string, discardPending = false) =>
    send(`/api/comments/${encodeURIComponent(id)}${discardPending ? "?discardPending=true" : ""}`, "DELETE"),
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

/** The whole review, as one answer. Read this first: the shapes below are the
 * pieces it is built from, and this is the one the screen is drawn against. */
export type ReviewView = {
  branch: string;
  base: string;
  generatedAt: string;
  commitsBehind: number;
  blocks: BlockView[];
  unmapped: string[];
  totalFiles: number;
  viewedFiles: number;
  /** The first file in reading order nobody has read yet, and null once they
   * all have been. Where the reader is put when the map arrives, and where the
   * key that walks to the next unread file wraps round to. */
  firstUnread: string | null;
  /** The pull request's lenses, in order, drawn before the first block. */
  lenses: LensView[];
};

export type BlockView = {
  slug: string;
  title: string;
  context: string;
  /** The block's lenses, between its context and its code. */
  lenses: LensView[];
  files: FileView[];
  /** What the block holds, which is more than it renders: a file it shares with
   * an earlier block is read there and listed in neither list. Counted by the
   * server, which is the only side that knows the membership. Never zero. */
  totalFiles: number;
  viewedFiles: number;
};

export type FileView = {
  path: string;
  status: string;
  additions: number;
  deletions: number;
  viewed: boolean;
  /** The note the block wrote for this file, if it wrote one. */
  note: string | null;
  lineNotes: LineNoteView[];
  /** Why the map says this file is worth a closer look, when it does. */
  look: string | null;
  /** The file's own lenses, drawn under its header. */
  lenses: LensView[];
  /** The session wrote lenses on this review and the file is not worth a
   * closer look, so its diff starts folded. */
  folded: boolean;
};

/** A note pinned to a span of lines on the new side. */
export type LineNoteView = {
  from: number;
  to: number;
  text: string;
};

/** One file's diff, fetched when the reader opens it. */
export type FileDiff = {
  path: string;
  status: string;
  hunks: Hunk[];
  /** git will not diff this file: binary content, or `-diff` in .gitattributes. */
  binary: boolean;
  additions: number;
  deletions: number;
  /** How long the file is after the change, which is where the last gap ends. */
  lineCount: number;
};

export type Hunk = {
  oldStart: number;
  oldLines: number;
  newStart: number;
  newLines: number;
  lines: DiffLine[];
};

/** Which side of the diff a line number is counted on.
 *
 * Both sides number from one, so the same number names two different lines —
 * the file as it was, and the file as it now reads. Anything anchored to a line
 * carries this with it, all the way to the host, where it is spelled LEFT and
 * RIGHT. */
export type Side = "old" | "new";

export type DiffLine = {
  kind: "context" | "added" | "removed";
  oldNumber: number | null;
  newNumber: number | null;
  content: string;
};

/** The comments, and the files the store could not read.
 *
 * The unreadable ones travel with the list because the page is the only place
 * their absence shows: a comment whose markdown got broken by hand stops
 * rendering, and silence there reads as never having written it. */
export type CommentsView = {
  /** The comments of one file, under its path, oldest first. Shaped by the
   * server like every other answer, so the browser looks a file up instead of
   * grouping the list again wherever it needs the cut. */
  files: Record<string, CommentView[]>;
  /** Threads whose lines the diff no longer shows, oldest first. The screen
   * puts them at the end of their file's block, or in a block of their own. */
  drifted: CommentView[];
  unreadable: Unreadable[];
  /** The harness a Send reaches — "Claude Code", "Codex" — or null, and then
   * there is no Send to offer. */
  agent: string | null;
};

/** A comment the store could not read.
 *
 * What broke is the header, which is the part saying where the comment belongs
 * — so `about` is there only when the header still names a file. When it does
 * not, `excerpt` is what the reviewer recognises it by. */
export type Unreadable = {
  /** The file to open to fix it, from the root of the worktree. */
  file: string;
  about: string | null;
  excerpt: string | null;
  why: string;
};

/** A question the reviewer left over a span of lines they were reading.
 *
 * There is no answered state: closing one removes it, so every comment the
 * page holds is still waiting. */
export type CommentView = {
  id: string;
  path: string;
  /** Which side of the diff `from` and `to` are counted on. */
  side: Side;
  from: number;
  to: number;
  /** Markdown, as it was typed. */
  body: string;
  /** Who wrote the thread's comment for the pull request, when it has one.
   * What it says, and whether it went, is in `PullRequestView.pending`. */
  pending: PendingByView | null;
  /** The turns after the question, oldest first. */
  replies: ReplyView[];
  /** Set while an agent owes an answer. */
  waiting: WaitingView | null;
};

/** One turn of a comment's conversation. */
export type ReplyView = {
  from: "reviewer" | "agent";
  /** "You", or the harness that answered. */
  name: string;
  body: string;
  /** Seconds since the epoch, as text. */
  at: string;
};

/** Something owed — an answer, or a pending comment: handed over, or
 * acknowledged by the agent as being worked on. */
export type WaitingView = {
  on: string;
  stage: "sent" | "working";
  purpose: "answer" | "pending";
  /** Seconds since the epoch, as text. */
  since: string;
};

export type PendingByView = {
  /** GitHub's id for the comment: a regenerated one has a new id. */
  id: string;
  by: "reviewer" | "agent";
  name: string;
};

/** The pull request as the screen needs it, asked of GitHub apart from the
 * threads so they never wait on the network. */
export type PullRequestView = {
  /** Fewer states than reasons: everything a login fixes reads as "connect". */
  connection: "ready" | "connect" | "no-pull-request" | "ambiguous" | "no-remote";
  number: number | null;
  mine: boolean;
  author: string | null;
  /** The pull request's page, when there is one. */
  url: string | null;
  openAt: string | null;
  /** The command that connects, for the host the repository is on. */
  login: string;
  numbers: number[];
  mode: { mode: ReviewMode; reason: string };
  /** By thread id. */
  pending: Record<string, RemotePendingView>;
  pendingFailed: string | null;
};

export type ReviewMode = "self" | "peer";

export type RemotePendingView = {
  state: "pending" | "published" | "gone";
  body: string | null;
  url: string | null;
};

export type Verdict = "comment" | "request-changes" | "approve";

export type SentView = {
  url: string;
  comments: number;
  read: number;
  readFailed: string | null;
};

/** Where a file sits: the block it is read under and how far down the map that
 * block is, which is what the band above the diff counts off. */
export type FileHome = { block: BlockView; index: number };

/** One lens on one target, and what it said. */
export type LensView = {
  id: string;
  title: string;
  state: "written" | "outdated" | "missing";
  text: string;
  /** The short commit it was written at, or "uncommitted work". */
  writtenAt: string | null;
  /** Files changed since it was written; zero when only its prompt did. */
  changed: number;
  /** The citations that can be followed, as they appear in the text. */
  citations: CitationView[];
  /** What it is written about; null while nothing is written. */
  target: LensTarget | null;
  /** Marked as seen, for this very answer. */
  seen: boolean;
};

export type LensTarget = { kind: "pr" } | { kind: "block"; slug: string } | { kind: "file"; path: string };

/** A place in the code a lens points at. */
export type CitationView = { text: string; path: string; side: Side; from: number; to: number };

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
