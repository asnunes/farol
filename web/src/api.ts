export type TaggedNote = { block: string; text: string };
export type TaggedLineNote = {
  block: string;
  from: number;
  to: number;
  text: string;
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

export type BlockView = {
  slug: string;
  title: string;
  context: string;
  files: FileView[];
};

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

export type DiffLine = {
  kind: "context" | "added" | "removed";
  old_number: number | null;
  new_number: number | null;
  content: string;
};

export type Hunk = {
  old_start: number;
  old_lines: number;
  new_start: number;
  new_lines: number;
  lines: DiffLine[];
};

export type FileDiff = {
  path: string;
  status: string;
  hunks: Hunk[];
  /** git will not diff this file: binary content, or `-diff` in .gitattributes. */
  binary: boolean;
  additions: number;
  deletions: number;
};

async function json<T>(res: Response): Promise<T> {
  if (!res.ok) throw new Error(await res.text());
  return res.json() as Promise<T>;
}

export const api = {
  review: () => fetch("/api/review").then(json<ReviewView>),
  file: (path: string) =>
    fetch(`/api/file?path=${encodeURIComponent(path)}`).then(json<FileDiff>),
  setViewed: (path: string, viewed: boolean) =>
    fetch("/api/viewed", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ path, viewed }),
    }),
};

/** Flat reading order across blocks — what j/k and "next unread" walk. */
export function readingOrder(review: ReviewView): FileView[] {
  return [...review.blocks.flatMap((b) => b.files), ...review.looseSkim];
}

/** The block a file is rendered under, for the band above the diff. */
export function blockOf(
  review: ReviewView,
  path: string,
): { block: BlockView; index: number } | null {
  for (let i = 0; i < review.blocks.length; i++) {
    if (review.blocks[i].files.some((f) => f.path === path)) {
      return { block: review.blocks[i], index: i };
    }
  }
  return null;
}
