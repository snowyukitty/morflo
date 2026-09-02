import { chromium } from "@playwright/test";
import { copyFile, mkdir, stat, writeFile } from "node:fs/promises";
import { basename, join, resolve } from "node:path";
import { spawn } from "node:child_process";

import {
  environmentWithoutInheritedEngine,
  processIdsForExecutable,
  reserveLoopbackPort,
  sha256,
  waitForChildExit,
  waitForWebViewTarget,
} from "./lib/native-windows.mjs";

const repositoryRoot = resolve(import.meta.dirname, "..");
const defaultApp = resolve(process.env.LOCALAPPDATA ?? "", "Morflo", "morflo.exe");
const appPath = resolve(process.env.MORFLO_INSTALLED_APP ?? defaultApp);
const imageFixture = resolve(repositoryRoot, "fixtures", "generated", "transparent-grid.png");
const videoFixture = resolve(repositoryRoot, "fixtures", "generated", "portrait-phone.mov");
const relativeFixture = resolve(
  repositoryRoot,
  "fixtures",
  "generated",
  "transparent-grid.webp",
);
const evidenceRoot = resolve(repositoryRoot, "work", "morflo", "native-open-with-windows");
const runId = new Date().toISOString().replaceAll(":", "-").replaceAll(".", "-");
const runRoot = join(evidenceRoot, runId);
const imageName = "Open with 京都 🧳 O'Reilly.png";
const videoName = "Open with 日本語 🚲.mov";
const relativeName = "Relative launch 高雄 🌿.webp";
const imagePath = join(runRoot, imageName);
const videoPath = join(runRoot, videoName);
const relativePath = join(runRoot, relativeName);

if (process.platform !== "win32") {
  throw new Error("The native Open with test is Windows-only.");
}
for (const required of [appPath, imageFixture, videoFixture, relativeFixture]) {
  if (!(await stat(required).catch(() => undefined))?.isFile()) {
    throw new Error(`Required file is unavailable: ${required}`);
  }
}
if (processIdsForExecutable(appPath).length > 0) {
  throw new Error("Close Morflo before running the native single-window intake test.");
}

await mkdir(runRoot, { recursive: true });
await copyFile(imageFixture, imagePath);
await copyFile(videoFixture, videoPath);
await copyFile(relativeFixture, relativePath);
const hashesBefore = {
  [imageName]: await sha256(imagePath),
  [videoName]: await sha256(videoPath),
  [relativeName]: await sha256(relativePath),
};

let app;
let browser;
try {
  const port = await reserveLoopbackPort();
  const debugArgument = `--remote-debugging-port=${port}`;
  const existingArguments = process.env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS?.trim();
  const { environment: appEnvironment, removedEngineDirectoryCount } =
    await environmentWithoutInheritedEngine();
  const environment = {
    ...appEnvironment,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: existingArguments
      ? `${existingArguments} ${debugArgument}`
      : debugArgument,
  };
  app = spawn(appPath, [], {
    env: environment,
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

  await page.getByRole("heading", { name: "Files in. Better formats out." }).waitFor();
  await page.screenshot({ path: join(runRoot, "01-native-empty.png") });

  const firstArrivalStarted = performance.now();
  const secondInstance = spawn(appPath, [imagePath, videoPath], {
    cwd: runRoot,
    env: environment,
    shell: false,
    stdio: "ignore",
    windowsHide: true,
  });
  const secondExitCode = await waitForChildExit(
    secondInstance,
    15_000,
    "second Morflo instance",
  );
  if (secondExitCode !== 0) {
    throw new Error(`The second Morflo instance exited with code ${secondExitCode}.`);
  }
  await page.getByRole("heading", { name: "Conversion queue" }).waitFor({ timeout: 30_000 });
  await page.locator(".file-identity strong", { hasText: imageName }).waitFor();
  await page.locator(".file-identity strong", { hasText: videoName }).waitFor();
  await page.getByText("2 files", { exact: true }).waitFor();
  const firstArrivalMs = Math.round(performance.now() - firstArrivalStarted);
  await page.screenshot({ path: join(runRoot, "02-native-open-with-joined.png") });

  const relativeInstance = spawn(appPath, [basename(relativePath)], {
    cwd: runRoot,
    env: environment,
    shell: false,
    stdio: "ignore",
    windowsHide: true,
  });
  const relativeExitCode = await waitForChildExit(
    relativeInstance,
    15_000,
    "relative-path Morflo instance",
  );
  if (relativeExitCode !== 0) {
    throw new Error(`The relative-path Morflo instance exited with code ${relativeExitCode}.`);
  }
  const relativeRow = page.locator(".file-row", { hasText: relativeName });
  await relativeRow.locator(".file-identity strong").waitFor();
  await page.getByText("3 files", { exact: true }).waitFor();
  const relativeOutput = (await relativeRow.locator(".output-chip").textContent())?.trim();
  if (relativeOutput !== "PNG") {
    throw new Error(`Transparent WebP should recommend PNG, received: ${relativeOutput}`);
  }
  await page.screenshot({ path: join(runRoot, "03-native-relative-joined.png") });

  const appProcessIds = processIdsForExecutable(appPath);
  if (appProcessIds.length !== 1 || appProcessIds[0] !== app.pid) {
    throw new Error(
      `Expected one installed Morflo process, found: ${appProcessIds.join(", ")}`,
    );
  }
  const appTargets = (
    await fetch(`${endpoint}/json/list`).then((response) => response.json())
  ).filter((candidate) => candidate.url?.includes("tauri.localhost"));
  if (appTargets.length !== 1) {
    throw new Error(`Expected one Morflo WebView target, found ${appTargets.length}.`);
  }

  const hashesAfter = {
    [imageName]: await sha256(imagePath),
    [videoName]: await sha256(videoPath),
    [relativeName]: await sha256(relativePath),
  };
  if (JSON.stringify(hashesBefore) !== JSON.stringify(hashesAfter)) {
    throw new Error("An Open with source file changed during intake.");
  }

  const result = {
    environment: "Windows 11 x64; installed Tauri WebView2; real second-process handoff",
    appPath,
    inheritedEnginePathEntriesRemoved: removedEngineDirectoryCount,
    journey:
      "empty Morflo -> second process with PNG + MOV -> same queue -> relative WebP -> same queue",
    files: [imageName, videoName, relativeName],
    firstArrivalMs,
    secondProcessesExitedCleanly: true,
    oneMorfloProcess: true,
    oneMorfloWebView: true,
    relativePathResolvedAgainstActivationDirectory: true,
    transparentWebpRecommendedPng: true,
    sourceHashesPreserved: true,
    screenshots: [
      join(runRoot, "01-native-empty.png"),
      join(runRoot, "02-native-open-with-joined.png"),
      join(runRoot, "03-native-relative-joined.png"),
    ],
    webviewTarget: { title: target.title, url: target.url },
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
