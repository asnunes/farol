/** Everything the browser asks the server for. */
export const api = {
  review: () => fetch("/api/review").then(json<ReviewView>),
  file: (path: string) =>
    fetch(`/api/file?path=${encodeURIComponent(path)}`).then(json<FileDiff>),
  setViewed: (path: string, viewed: boolean) =>
    send("/api/viewed", "POST", { path, viewed }),

  /** A stretch of the file the diff did not print, for a reader opening a gap. */
  lines: (path: string, from: number, to: number) =>
    fetch(`/api/lines?path=${encodeURIComponent(path)}&from=${from}&to=${to}`)
      .then(json<{ lines: string[] }>)
      .then((answer) => answer.lines),

  comments: () => fetch("/api/comments").then(json<CommentsView>),
  addComment: (path: string, from: number, to: number, body: string) =>
    send("/api/comments", "POST", { path, from, to, body }),
  closeComment: (id: string) => send(`/api/comments/${encodeURIComponent(id)}`, "DELETE"),

  readiness: () => fetch("/api/publish").then(json<ReadinessView>),
  publish: (verdict: Verdict, summary: string) =>
    write<SentView>("/api/publish", "POST", { verdict, summary }),
  /** One way. Nothing reads it back, here or on the server. */
  saveToken: (token: string) => send("/api/token", "PUT", { token }),
};

/** Flat reading order across blocks — what j/k and "next unread" walk. */
export function readingOrder(review: ReviewView): FileView[] {
  return [...review.blocks.flatMap((b) => b.files), ...review.looseSkim];
}

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
  looseSkim: FileView[];
  unmapped: string[];
  totalFiles: number;
  viewedFiles: number;
};

export type BlockView = {
  slug: string;
  title: string;
  context: string;
  files: FileView[];
};

export type FileView = {
  path: string;
  status: string;
  additions: number;
  deletions: number;
  viewed: boolean;
  notes: TaggedNote[];
  lineNotes: TaggedLineNote[];
  tags: string[];
  skim: boolean;
  skimReason: string | null;
};

export type TaggedNote = { block: string; text: string };

export type TaggedLineNote = {
  block: string;
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
  comments: CommentView[];
  unreadable: Unreadable[];
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
  from: number;
  to: number;
  /** Markdown, as it was typed. */
  body: string;
  /** Where it can be read on the pull request, once it has gone. Publishing is
   * not answering: a published comment is still waiting for one. */
  published: string | null;
};

/** Whether the review can be sent, and when it cannot, what is in the way.
 *
 * A reason rather than a flag: each state is a different thing for the reader
 * to go and do, and the panel that explains it is written per state. */
export type ReadinessView = {
  state:
    | "ready"
    | "noRemote"
    | "noToken"
    | "tokenRefused"
    | "branchNotPushed"
    | "noPullRequest";
  branch: string;
  pullRequest?: number;
  /** The page that opens a pull request, on the one state that has one. */
  openAt?: string;
};

/** What the review says about the change as a whole. */
export type Verdict = "comment" | "requestChanges" | "approve";

/** What a sent review left behind. */
export type SentView = { url: string; comments: number };

/** Where a file sits: the block it is read under and how far down the map that
 * block is, which is what the band above the diff counts off. */
export type FileHome = { block: BlockView; index: number };

async function json<T>(res: Response): Promise<T> {
  if (!res.ok) throw new Error(await res.text());
  return res.json() as Promise<T>;
}

/** A write whose answer matters. Publishing is the only one: what comes back
 * is where the review can now be read, which the page has no way to work out
 * for itself. */
async function write<T>(url: string, method: string, body: unknown): Promise<T> {
  const res = await fetch(url, {
    method,
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json() as Promise<T>;
}

/** A write. The server answers with the thing it wrote, but the callers reload
 * rather than trust a copy, so what matters here is that a refusal is raised
 * instead of passing for success. */
async function send(url: string, method: string, body?: unknown) {
  const res = await fetch(url, {
    method,
    ...(body === undefined
      ? {}
      : { headers: { "content-type": "application/json" }, body: JSON.stringify(body) }),
  });
  if (!res.ok) throw new Error(await res.text());
}
