/** Everything the browser asks the server for. */
export const api = {
  review: () => fetch("/api/review").then(json<ReviewView>),
  file: (path: string) =>
    fetch(`/api/file?path=${encodeURIComponent(path)}`).then(json<FileDiff>),
  setViewed: (path: string, viewed: boolean) =>
    send("/api/viewed", "POST", { path, viewed }),

  comments: () => fetch("/api/comments").then(json<CommentView[]>),
  addComment: (path: string, from: number, to: number, body: string) =>
    send("/api/comments", "POST", { path, from, to, body }),
  resolveComment: (id: string, resolved: boolean) =>
    send(`/api/comments/${encodeURIComponent(id)}/resolve`, "POST", { resolved }),
  removeComment: (id: string) => send(`/api/comments/${encodeURIComponent(id)}`, "DELETE"),
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
};

export type Hunk = {
  old_start: number;
  old_lines: number;
  new_start: number;
  new_lines: number;
  lines: DiffLine[];
};

export type DiffLine = {
  kind: "context" | "added" | "removed";
  old_number: number | null;
  new_number: number | null;
  content: string;
};

/** What the reviewer wrote back, over a span of lines they were reading. */
export type CommentView = {
  id: string;
  path: string;
  from: number;
  to: number;
  /** Markdown, as it was typed. */
  body: string;
  resolved: boolean;
};

/** Where a file sits: the block it is read under and how far down the map that
 * block is, which is what the band above the diff counts off. */
export type FileHome = { block: BlockView; index: number };

async function json<T>(res: Response): Promise<T> {
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
