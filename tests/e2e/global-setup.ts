import { resolve } from "node:path";

import { createServer } from "vite";

export default async function globalSetup(): Promise<() => Promise<void>> {
  const server = await createServer({
    configFile: resolve("vite.config.ts"),
    logLevel: "error",
    server: {
      host: "127.0.0.1",
      port: 0,
      strictPort: false,
    },
  });
  await server.listen();

  const address = server.httpServer?.address();
  if (!address || typeof address === "string") {
    await server.close();
    throw new Error("Vite did not expose an ephemeral test port");
  }
  process.env.MORFLO_E2E_BASE_URL = `http://127.0.0.1:${address.port}`;

  return async () => {
    await server.close();
  };
}
