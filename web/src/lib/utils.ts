import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

/** Merge class lists so a caller's utility wins over a component's default. */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/** What went wrong, in the words the server used.
 *
 * `String(err)` prefixes "Error:", which the server never wrote and the reader
 * has no use for: they are looking at a red box with a warning sign in it and
 * already know it is an error. */
export function said(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}
