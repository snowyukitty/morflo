/* global document, HTMLImageElement, HTMLInputElement, HTMLVideoElement */

import { chromium } from "@playwright/test";
import { copyFile, mkdir, readdir, stat, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { spawn, spawnSync } from "node:child_process";

import {
  environmentWithoutInheritedEngine,
  reserveLoopbackPort,
  sha256,
  waitForFile,
  waitForWebViewTarget,
} from "./lib/native-windows.mjs";

const repositoryRoot = resolve(import.meta.dirname, "..");
const defaultApp = resolve(repositoryRoot, "src-tauri", "target", "release", "morflo.exe");
const defaultFixture = resolve(
  repositoryRoot,
  "fixtures",
  "generated",
  "high-motion-gif-source.mp4",
);
const appPath = resolve(process.env.MORFLO_NATIVE_APP ?? defaultApp);
const fixturePath = resolve(process.env.MORFLO_NATIVE_GIF_FIXTURE ?? defaultFixture);
const evidenceRoot = resolve(repositoryRoot, "work", "morflo", "native-gif-windows");
const runId = new Date().toISOString().replaceAll(":", "-").replaceAll(".", "-");
const runRoot = join(evidenceRoot, runId);
const sourceName = "Native moment 京都 🧳 O'Reilly.mp4";
const sourcePath = join(runRoot, sourceName);
const convertedOutput = join(runRoot, "Native moment 京都 🧳 O'Reilly.gif");

if (process.platform !== "win32") {
  throw new Error("The packaged-native Morflo GIF smoke test is Windows-only.");
}

for (const required of [appPath, fixturePath]) {
  const metadata = await stat(required).catch(() => undefined);
  if (!metadata?.isFile()) throw new Error(`Required file is unavailable: ${required}`);
}

await mkdir(runRoot, { recursive: true });
await copyFile(fixturePath, sourcePath);
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
  const momentStartedAt = performance.now();
  await page.getByRole("button", { name: /Animated GIF/u }).click();
  await page.getByRole("group", { name: "GIF start and end range" }).waitFor();
  const momentFrames = page.locator(".moment-strip.has-frames img");
  await momentFrames.first().waitFor({ timeout: 30_000 });
  const momentStripMs = Math.round(performance.now() - momentStartedAt);
  if ((await momentFrames.count()) !== 7) {
    throw new Error(`Expected seven real moment frames, found ${await momentFrames.count()}.`);
  }
  const momentFramesDecoded = await momentFrames.evaluateAll((images) =>
    images.every((image) => image instanceof HTMLImageElement && image.naturalWidth === 160),
  );
  if (!momentFramesDecoded)
    throw new Error("One or more moment frames did not decode at 160 px.");

  const start = page.getByRole("slider", { name: "GIF start time" });
  const end = page.getByRole("slider", { name: "GIF end time" });
  await setRangeValue(start, "0.4");
  await setRangeValue(end, "2.4");
  await page.getByText("2.0 sec selected", { exact: true }).waitFor();

  await page.getByRole("button", { name: "Play selected preview" }).click();
  const localPreview = page.locator('video[aria-label="Selected video range preview"]');
  await localPreview.waitFor({ timeout: 30_000 });
  await page.waitForFunction(() => {
    const video = document.querySelector(".video-poster video");
    return video instanceof HTMLVideoElement && video.readyState >= 1 && video.videoWidth > 0;
  });
  await page.screenshot({ path: join(runRoot, "01-native-moments-and-preview.png") });

  await page.getByRole("button", { name: "Convert", exact: true }).click();
  const completed = page.getByRole("button", { name: `Reveal output for ${sourceName}` });
  const failed = page.locator(".error-panel");
  const outcome = await Promise.race([
    completed.waitFor({ timeout: 120_000 }).then(() => "completed"),
    failed.waitFor({ timeout: 120_000 }).then(() => "failed"),
  ]);
  if (outcome === "failed") {
    await page.screenshot({ path: join(runRoot, "02-native-gif-failed.png") });
    const message = await failed.innerText();
    const technical = await failed.locator(".error-code code").textContent();
    throw new Error(`Native GIF conversion failed: ${message}\n${technical ?? ""}`);
  }
  await waitForFile(convertedOutput, 15_000);
  const receiptHeading = page.getByRole("heading", { name: "Your GIF is ready" });
  await receiptHeading.waitFor();
  const outputPreview = page.locator(".result-preview.has-preview img");
  await outputPreview.waitFor({ timeout: 30_000 });
  const outputPreviewDecoded = await outputPreview.evaluate(
    (image) => image instanceof HTMLImageElement && image.naturalWidth > 0,
  );
  if (!outputPreviewDecoded) throw new Error("The completed GIF preview did not decode.");
  const receiptText = await page.locator(".completion-panel").innerText();
  await page.screenshot({ path: join(runRoot, "02-native-gif-converted.png") });

  const sourceHashAfter = await sha256(sourcePath);
  if (sourceHashBefore !== sourceHashAfter) throw new Error("The source video changed.");
  const leftovers = (await readdir(runRoot)).filter((name) => name.includes(".morflo-part"));
  if (leftovers.length > 0) {
    throw new Error(`Recognized partial outputs remain: ${leftovers.join(", ")}`);
  }

  const probe = spawnSync(
    "ffprobe",
    [
      "-v",
      "error",
      "-count_frames",
      "-select_streams",
      "v:0",
      "-show_entries",
      "stream=codec_name,width,height,nb_read_frames:format=format_name,duration",
      "-of",
      "json",
      convertedOutput,
    ],
    { encoding: "utf8", shell: false, windowsHide: true },
  );
  if (probe.status !== 0) {
    throw new Error(`ffprobe rejected Morflo's GIF output: ${probe.stderr.trim()}`);
  }
  const outputProbe = JSON.parse(probe.stdout);
  const videoStream = outputProbe.streams?.[0];
  if (videoStream?.codec_name !== "gif" || Number(videoStream.nb_read_frames) <= 1) {
    throw new Error("The native GIF output was not a multi-frame GIF.");
  }

  const result = {
    environment: "Windows 11 x64; release Tauri WebView2 attached over process-local CDP",
    appPath,
    webviewTarget: { title: target.title, url: target.url },
    inheritedEnginePathEntriesRemoved: removedEngineDirectoryCount,
    journey:
      "startup high-motion MP4 -> seven real moments -> 0.4–2.4 sec local preview -> palette GIF -> decoded receipt",
    sourceName,
    sourceHashPreserved: true,
    momentFrameCount: 7,
    momentFramesDecoded,
    momentStripMs,
    selectedRangeSeconds: [0.4, 2.4],
    localPreviewDecoded: true,
    convertedOutput,
    outputBytes: (await stat(convertedOutput)).size,
    outputProbe,
    receiptHeading: await receiptHeading.textContent(),
    receiptText,
    outputPreviewDecoded,
    partialOutputsRemaining: leftovers.length,
    elapsedMs: Math.round(performance.now() - startedAt),
    screenshots: [
      join(runRoot, "01-native-moments-and-preview.png"),
      join(runRoot, "02-native-gif-converted.png"),
    ],
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

async function setRangeValue(locator, value) {
  await locator.evaluate((input, nextValue) => {
    if (!(input instanceof HTMLInputElement)) throw new Error("Expected a range input.");
    const valueSetter = Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      "value",
    )?.set;
    valueSetter?.call(input, nextValue);
    input.dispatchEvent(new Event("input", { bubbles: true }));
    input.dispatchEvent(new Event("change", { bubbles: true }));
  }, value);
}
