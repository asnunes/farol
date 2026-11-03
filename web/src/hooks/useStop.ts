import { useState } from "react";
import { api } from "@/api";

/** Stopping the server from the page, and whether it has stopped: once it
 * has, nothing on the page will answer any more, so the page says so instead
 * of looking alive. */
export function useStop() {
  const [stopped, setStopped] = useState(false);

  return {
    stopped,
    /** Rejects with the server's reason when it would not stop. */
    stop: () => api.stop().then(() => setStopped(true)),
  };
}
