import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

import { createServer } from "vite";

const server = await createServer({
  server: { host: "127.0.0.1", port: 0, strictPort: true },
});

await server.listen();
const devUrl = server.resolvedUrls?.local[0]?.replace(/\/$/u, "");

if (!devUrl) {
  await server.close();
  throw new Error("Vite did not provide a loopback development URL.");
}

const tauriCli = fileURLToPath(
  new URL("../node_modules/@tauri-apps/cli/tauri.js", import.meta.url),
);
const override = JSON.stringify({ build: { devUrl } });
const child = spawn(process.execPath, [tauriCli, "dev", "--config", override], {
  cwd: process.cwd(),
  env: process.env,
  stdio: "inherit",
});

const stop = async (signal) => {
  if (!child.killed) child.kill(signal);
  await server.close();
};

process.once("SIGINT", () => void stop("SIGINT"));
process.once("SIGTERM", () => void stop("SIGTERM"));

const exitCode = await new Promise((resolve) => {
  child.once("exit", (code) => resolve(code ?? 1));
  child.once("error", () => resolve(1));
});

await server.close();
process.exitCode = exitCode;
