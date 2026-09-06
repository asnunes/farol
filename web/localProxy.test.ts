// @vitest-environment node
import { createServer as httpServer } from "node:http";
import type { AddressInfo } from "node:net";
import { afterAll, beforeAll, expect, it } from "vitest";
import { createServer, type ViteDevServer } from "vite";
import { localProxy } from "./localProxy";

let vite: ViteDevServer;
let origin: string;
let target: string;
const backend = httpServer((request, response) => {
  response.setHeader("Content-Type", "application/json");
  response.end(JSON.stringify({ host: request.headers.host, origin: request.headers.origin }));
});

beforeAll(async () => {
  await new Promise<void>((resolve) => backend.listen(0, "127.0.0.1", resolve));
  target = `http://127.0.0.1:${(backend.address() as AddressInfo).port}`;
  vite = await createServer({
    configFile: false,
    server: { host: "127.0.0.1", port: 0, proxy: { "/api": localProxy(target) } },
    logLevel: "silent",
  });
  await vite.listen();
  origin = `http://127.0.0.1:${(vite.httpServer!.address() as AddressInfo).port}`;
});

afterAll(async () => {
  await vite?.close();
  await new Promise<void>((resolve) => backend.close(() => resolve()));
});

it("forwards a dev page's own origin as the backend origin", async () => {
  const response = await fetch(`${origin}/api/review`, { headers: { Origin: origin } });
  expect(await response.json()).toEqual({ host: new URL(target).host, origin: target });
});

it("does not authorize an unrelated origin through the dev proxy", async () => {
  const response = await fetch(`${origin}/api/review`, { headers: { Origin: "http://foreign.example" } });
  expect((await response.json()).origin).toBe("http://foreign.example");
});
