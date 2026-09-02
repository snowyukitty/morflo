import { createHash } from "node:crypto";
import { readFile, stat } from "node:fs/promises";
import { createServer } from "node:net";
import { delimiter, join } from "node:path";
import { spawnSync } from "node:child_process";

export async function reserveLoopbackPort() {
  const server = createServer();
  await new Promise((resolveListen, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolveListen);
  });
  const address = server.address();
  if (!address || typeof address === "string") throw new Error("Could not reserve a TCP port.");
  await new Promise((resolveClose, reject) =>
    server.close((error) => (error ? reject(error) : resolveClose())),
  );
  return address.port;
}

export async function environmentWithoutInheritedEngine() {
  const environment = { ...process.env };
  delete environment.MORFLO_FFMPEG_PATH;
  delete environment.MORFLO_FFPROBE_PATH;
  const pathKey =
    Object.keys(environment).find((key) => key.toLowerCase() === "path") ?? "Path";
  const entries = (environment[pathKey] ?? "").split(delimiter).filter(Boolean);
  const retained = [];
  const removedEngineDirectories = [];
  for (const entry of entries) {
    const containsEngine = await Promise.all([
      stat(join(entry, "ffmpeg.exe")).catch(() => undefined),
      stat(join(entry, "ffprobe.exe")).catch(() => undefined),
    ]).then((results) => results.some((metadata) => metadata?.isFile()));
    if (containsEngine) removedEngineDirectories.push(entry);
    else retained.push(entry);
  }
  if (removedEngineDirectories.length === 0) {
    throw new Error("Could not remove an inherited FFmpeg PATH entry for the stale-PATH test.");
  }
  environment[pathKey] = retained.join(delimiter);
  return { environment, removedEngineDirectoryCount: removedEngineDirectories.length };
}

export async function waitForWebViewTarget(endpoint, child) {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    if (child.exitCode !== null) {
      throw new Error(`Morflo exited before WebView2 became available (${child.exitCode}).`);
    }
    try {
      const response = await fetch(`${endpoint}/json/list`);
      if (response.ok) {
        const targets = await response.json();
        const target = targets.find((candidate) => candidate.url?.includes("tauri.localhost"));
        if (target) return target;
      }
    } catch {
      // WebView2 has not opened its loopback debugging endpoint yet.
    }
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 100));
  }
  throw new Error("Timed out waiting for Morflo's WebView2 debugging target.");
}

export async function waitForFile(path, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const metadata = await stat(path).catch(() => undefined);
    if (metadata?.isFile() && metadata.size > 0) return;
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 100));
  }
  throw new Error(`Timed out waiting for a complete output: ${path}`);
}

export async function sha256(path) {
  return createHash("sha256")
    .update(await readFile(path))
    .digest("hex");
}

export function runPowerShellJson(script) {
  const encoded = Buffer.from(script, "utf16le").toString("base64");
  const result = spawnSync(
    "powershell.exe",
    ["-NoLogo", "-NoProfile", "-NonInteractive", "-EncodedCommand", encoded],
    {
      encoding: "utf8",
      shell: false,
      windowsHide: true,
    },
  );
  if (result.error || result.status !== 0) {
    throw new Error(
      `PowerShell evidence command failed: ${result.error?.message ?? result.stderr.trim()}`,
    );
  }
  const output = result.stdout.trim();
  return output ? JSON.parse(output) : undefined;
}

export function processIdsForExecutable(executablePath) {
  const pathLiteral = executablePath.replaceAll("'", "''");
  const result = runPowerShellJson(`
$target = [IO.Path]::GetFullPath('${pathLiteral}')
$ids = @(
  Get-CimInstance Win32_Process -Filter "Name = 'morflo.exe'" |
    Where-Object {
      $_.ExecutablePath -and
      [IO.Path]::GetFullPath($_.ExecutablePath).Equals($target, [StringComparison]::OrdinalIgnoreCase)
    } |
    Select-Object -ExpandProperty ProcessId
)
@($ids) | ConvertTo-Json -Compress
`);
  if (result === undefined) return [];
  return Array.isArray(result) ? result : [result];
}

export async function waitForChildExit(child, timeoutMs, label) {
  if (child.exitCode !== null) return child.exitCode;
  return Promise.race([
    new Promise((resolveExit, reject) => {
      child.once("error", reject);
      child.once("exit", (code) => resolveExit(code));
    }),
    new Promise((_, reject) =>
      setTimeout(() => reject(new Error(`Timed out waiting for ${label} to exit.`)), timeoutMs),
    ),
  ]);
}
