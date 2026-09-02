import { readFile, readdir } from "node:fs/promises";
import { extname, join, relative, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const packageJson = JSON.parse(await readFile(join(root, "package.json"), "utf8"));
const allowedRuntimePackages = new Set([
  "@tauri-apps/api",
  "@tauri-apps/plugin-dialog",
  "lucide-react",
  "react",
  "react-dom",
]);
const unexpectedPackages = Object.keys(packageJson.dependencies ?? {}).filter(
  (name) => !allowedRuntimePackages.has(name),
);
if (unexpectedPackages.length > 0) {
  throw new Error(`Review unexpected runtime packages: ${unexpectedPackages.join(", ")}`);
}

const productionFiles = [
  ...(await sourceFiles(join(root, "src"), new Set([".ts", ".tsx"]))),
  ...(await sourceFiles(join(root, "src-tauri", "src"), new Set([".rs"]))),
  join(root, "index.html"),
];
const networkPatterns = [
  /\bfetch\s*\(/u,
  /\bXMLHttpRequest\b/u,
  /\bWebSocket\b/u,
  /\bEventSource\b/u,
  /\bsendBeacon\b/u,
  /\bTcpStream\b/u,
  /\bUdpSocket\b/u,
  /\breqwest\b/u,
  /\bhyper::/u,
];
const findings = [];
for (const file of productionFiles) {
  const text = await readFile(file, "utf8");
  for (const pattern of networkPatterns) {
    if (pattern.test(text)) findings.push(`${relative(root, file)} matched ${pattern}`);
  }
}
if (findings.length > 0) {
  throw new Error(`Production network API audit failed:\n${findings.join("\n")}`);
}

const cargoToml = await readFile(join(root, "src-tauri", "Cargo.toml"), "utf8");
for (const disallowed of ["reqwest", "hyper", "ureq", "sentry", "opentelemetry"]) {
  if (new RegExp(`^${disallowed}\\s*=`, "mu").test(cargoToml)) {
    throw new Error(`Review disallowed Rust runtime dependency: ${disallowed}`);
  }
}

const tauriConfig = JSON.parse(
  await readFile(join(root, "src-tauri", "tauri.conf.json"), "utf8"),
);
const csp = tauriConfig.app?.security?.csp ?? "";
const connectDirective = csp
  .split(";")
  .map((directive) => directive.trim())
  .find((directive) => directive.startsWith("connect-src "));
if (connectDirective !== "connect-src ipc: http://ipc.localhost") {
  throw new Error(`CSP connect-src needs review: ${connectDirective ?? "missing"}`);
}

process.stdout.write(
  `${JSON.stringify(
    {
      result: "passed",
      productionFilesScanned: productionFiles.length,
      runtimePackages: [...allowedRuntimePackages],
      connectSrc: connectDirective,
      note: "Normal conversion uses typed Tauri IPC and local child processes; no production network API was found.",
    },
    null,
    2,
  )}\n`,
);

async function sourceFiles(directory, extensions) {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) files.push(...(await sourceFiles(path, extensions)));
    else if (extensions.has(extname(entry.name))) files.push(path);
  }
  return files;
}
