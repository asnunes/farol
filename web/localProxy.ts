import type { ProxyOptions } from "vite";

/** Preserve the dev page's same-origin calls without trusting other origins. */
export function localProxy(target: string): ProxyOptions {
  return {
    target,
    changeOrigin: true,
    configure(proxy) {
      proxy.on("proxyReq", (outgoing, incoming) => {
        const host = incoming.headers.host;
        const origin = incoming.headers.origin;
        if (host && /^(localhost|127\.0\.0\.1):\d+$/.test(host) && origin === `http://${host}`) {
          outgoing.setHeader("origin", target);
        }
      });
    },
  };
}
