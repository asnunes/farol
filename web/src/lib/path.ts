/** Split once: the directory keeps its trailing slash so the two halves
 * concatenate back to the original path. */
export function splitPath(path: string): SplitPath {
  const i = path.lastIndexOf("/");
  return i === -1
    ? { dir: "", name: path }
    : { dir: path.slice(0, i + 1), name: path.slice(i + 1) };
}

/** A path in two halves, for a header that greys the directory and keeps the
 * file name in front. */
export type SplitPath = { dir: string; name: string };
