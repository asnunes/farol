import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

/** Merge class lists so a caller's utility wins over a component's default. */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
