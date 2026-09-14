/** Split once: the directory keeps its trailing slash so the two halves
 * concatenate back to the original path. */
export function splitPath(path: string): SplitPath {
  const i = path.lastIndexOf("/");
  return i === -1
    ? { dir: "", name: path }
    : { dir: path.slice(0, i + 1), name: path.slice(i + 1) };
}

/** What each path is called in a list that holds all of them: the file name
 * alone while it is unique, and the shortest tail of its directory once
 * something else in the list shares the name.
 *
 * Computed over the whole list rather than per block, because two `mod.rs` in
 * different blocks are still two rows the reader has to tell apart. */
export function fileLabels(paths: string[]): FileLabels {
  const homonyms = new Map<string, string[]>();
  for (const path of paths) {
    const { name } = splitPath(path);
    const group = homonyms.get(name) ?? [];
    group.push(path);
    homonyms.set(name, group);
  }

  const labels = new Map<string, FileLabel>();
  for (const [name, group] of homonyms) {
    const depth = separatingDepth(group);
    for (const path of group) labels.set(path, { name, where: tail(path, depth) });
  }
  return (path) => labels.get(path) ?? { name: splitPath(path).name, where: "" };
}

/** A path in two halves, for a header that greys the directory and keeps the
 * file name in front. */
export type SplitPath = { dir: string; name: string };

/** How the list names one of its paths. A lookup rather than a map so a caller
 * asking about a path that was not in the list gets the bare name instead of a
 * hole. */
export type FileLabels = (path: string) => FileLabel;

/** A path as the sidebar prints it: `where` is empty while the name stands on
 * its own, and holds just enough directory to separate it otherwise. */
export type FileLabel = { name: string; where: string };

/** The fewest trailing directory segments that give every path in the group a
 * tail of its own. One segment settles nearly everything; a deeper tree needs
 * more, and paths that never separate fall back to the whole directory. */
function separatingDepth(group: string[]): number {
  if (group.length < 2) return 0;
  const deepest = Math.max(...group.map((p) => segments(p).length));
  for (let depth = 1; depth < deepest; depth++) {
    if (new Set(group.map((p) => tail(p, depth))).size === group.length) return depth;
  }
  return deepest;
}

function tail(path: string, depth: number): string {
  return depth === 0 ? "" : segments(path).slice(-depth).join("/");
}

function segments(path: string): string[] {
  return splitPath(path).dir.split("/").filter(Boolean);
}
