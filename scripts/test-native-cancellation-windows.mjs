import { chromium } from "@playwright/test";
import { mkdir, readdir, stat, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { spawn, spawnSync } from "node:child_process";

import {
  environmentWithoutInheritedEngine,
  reserveLoopbackPort,
  sha256,
  waitForWebViewTarget,
} from "./lib/native-windows.mjs";

const repositoryRoot = resolve(import.meta.dirname, "..");
const defaultApp = resolve(repositoryRoot, "src-tauri", "target", "release", "morflo.exe");
const appPath = resolve(process.env.MORFLO_NATIVE_APP ?? defaultApp);
const evidenceRoot = resolve(repositoryRoot, "work", "morflo", "native-cancellation-windows");
const runId = new Date().toISOString().replaceAll(":", "-").replaceAll(".", "-");
const runRoot = join(evidenceRoot, runId);
const sourceName = "Cancel me 京都 🧳 O'Reilly.mp4";
const sourcePath = join(runRoot, sourceName);
const finalOutput = join(runRoot, "Cancel me 京都 🧳 O'Reilly.webm");

if (process.platform !== "win32") {
  throw new Error("The packaged-native Morflo cancellation test is Windows-only.");
}
if (!(await stat(appPath).catch(() => undefined))?.isFile()) {
  throw new Error(`Required app is unavailable: ${appPath}`);
}

await mkdir(runRoot, { recursive: true });
const generated = spawnSync(
  "ffmpeg",
  [
    "-hide_banner",
    "-loglevel",
    "error",
    "-nostdin",
    "-y",
    "-f",
    "lavfi",
    "-i",
    "testsrc2=s=1280x720:r=30:d=30",
    "-f",
    "lavfi",
    "-i",
    "sine=frequency=880:sample_rate=48000:d=30",
    "-shortest",
    "-c:v",
    "libx264",
    "-preset",
    "ultrafast",
    "-crf",
    "17",
    "-pix_fmt",
    "yuv420p",
    "-c:a",
    "aac",
    "-b:a",
    "128k",
    sourcePath,
  ],
  { encoding: "utf8", shell: false, windowsHide: true },
);
if (generated.status !== 0) {
  throw new Error(`Could not generate the cancellation fixture: ${generated.stderr.trim()}`);
}
const sourceHashBefore = await sha256(sourcePath);
const startedAt = performance.now();

let app;
let browser;
try {
  const port = await reserveLoopbackPort();
  const debugArgument = `--remote-debugging-port=${port}`;
  const existingArguments = process.env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS?.trim();
  const { environment: appEnvironment, removedEngineDirectoryCount } =
    await environmentWithoutInheritedEngine();
  app = spawn(appPath, [sourcePath], {
    env: {
      ...appEnvironment,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: existingArguments
        ? `${existingArguments} ${debugArgument}`
        : debugArgument,
    },
    shell: false,
    stdio: "ignore",
    windowsHide: true,
  });

  const endpoint = `http://127.0.0.1:${port}`;
  const target = await waitForWebViewTarget(endpoint, app);
  browser = await chromium.connectOverCDP(endpoint);
  const page = browser
    .contexts()
    .flatMap((context) => context.pages())
    .find((candidate) => candidate.url().includes("tauri.localhost"));
  if (!page) throw new Error("Connected to WebView2, but Morflo's app target was not present.");

  await page.getByRole("heading", { name: "Conversion queue" }).waitFor({ timeout: 30_000 });
  await page.locator(".file-identity strong", { hasText: sourceName }).waitFor();
  await page.getByRole("button", { name: /Web-friendly WebM/u }).click();
  await page.getByRole("button", { name: "Convert", exact: true }).click();
  const cancel = page.getByRole("button", { name: `Cancel ${sourceName}` });
  await cancel.waitFor({ timeout: 30_000 });
  const cancelStartedAt = performance.now();
  await cancel.click();
  const row = page.locator(".file-row", { hasText: sourceName });
  await row.getByText("Canceled", { exact: true }).waitFor({ timeout: 5_000 });
  const cancellationMs = Math.round(performance.now() - cancelStartedAt);
  if (cancellationMs >= 2_500) {
    throw new Error(`Native cancellation took ${cancellationMs} ms.`);
  }
  await page.waitForTimeout(500);
  await page.screenshot({ path: join(runRoot, "01-native-canceled.png") });

  if (await stat(finalOutput).catch(() => undefined)) {
    throw new Error("A canceled conversion published a final output.");
  }
  const leftovers = (await readdir(runRoot)).filter((name) => name.includes(".morflo-part"));
  if (leftovers.length > 0) {
    throw new Error(`Recognized partial outputs remain: ${leftovers.join(", ")}`);
  }
  if ((await sha256(sourcePath)) !== sourceHashBefore)
    throw new Error("The source video changed.");
  const survivingMediaChildren = listDirectFfmpegChildren(app.pid);
  if (survivingMediaChildren.length > 0) {
    throw new Error(`FFmpeg child processes survived cancellation: ${survivingMediaChildren}`);
  }

  const result = {
    environment: "Windows 11 x64; installed release Tauri WebView2",
    appPath,
    webviewTarget: { title: target.title, url: target.url },
    inheritedEnginePathEntriesRemoved: removedEngineDirectoryCount,
    journey: "startup 30 sec MP4 -> Web-friendly WebM -> real encode -> cancel one job",
    sourceName,
    sourceBytes: (await stat(sourcePath)).size,
    sourceHashPreserved: true,
    cancellationMs,
    finalOutputPublished: false,
    partialOutputsRemaining: leftovers.length,
    survivingFfmpegChildren: survivingMediaChildren.length,
    elapsedMs: Math.round(performance.now() - startedAt),
    screenshot: join(runRoot, "01-native-canceled.png"),
  };
  await writeFile(join(runRoot, "result.json"), `${JSON.stringify(result, null, 2)}\n`, "utf8");
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
} finally {
  await browser?.close().catch(() => undefined);
  if (app && app.exitCode === null) {
    app.kill();
    await Promise.race([
      new Promise((resolveExit) => app.once("exit", resolveExit)),
      new Promise((resolveTimeout) => setTimeout(resolveTimeout, 3_000)),
    ]);
  }
}

function listDirectFfmpegChildren(parentProcessId) {
  const command =
    "$parent = [uint32]$args[0]; @(Get-CimInstance Win32_Process -Filter \"Name = 'ffmpeg.exe'\" | Where-Object { $_.ParentProcessId -eq $parent } | Select-Object -ExpandProperty ProcessId) | ConvertTo-Json -Compress";
  const result = spawnSync(
    "powershell",
    ["-NoProfile", "-Command", command, String(parentProcessId)],
    { encoding: "utf8", shell: false, windowsHide: true },
  );
  if (result.status !== 0) {
    throw new Error(`Could not inspect the native process tree: ${result.stderr.trim()}`);
  }
  const text = result.stdout.trim();
  if (!text) return [];
  const parsed = JSON.parse(text);
  return Array.isArray(parsed) ? parsed : [parsed];
}
