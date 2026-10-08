import { mkdir, readFile, stat, unlink, writeFile } from "node:fs/promises";
import { resolve, join } from "node:path";
import { spawnSync } from "node:child_process";

import { processIdsForExecutable, runPowerShellJson } from "./lib/native-windows.mjs";

const repositoryRoot = resolve(import.meta.dirname, "..");
const packageJson = JSON.parse(await readFile(resolve(repositoryRoot, "package.json"), "utf8"));
const defaultInstaller = resolve(
  repositoryRoot,
  "src-tauri",
  "target",
  "release",
  "bundle",
  "nsis",
  `Morflo_${packageJson.version}_x64-setup.exe`,
);
const defaultInstalledApp = resolve(process.env.LOCALAPPDATA ?? "", "Morflo", "morflo.exe");
const installerPath = resolve(process.env.MORFLO_NATIVE_INSTALLER ?? defaultInstaller);
const installedAppPath = resolve(process.env.MORFLO_INSTALLED_APP ?? defaultInstalledApp);
const uninstallPath = join(resolve(installedAppPath, ".."), "uninstall.exe");
const manifestPath = resolve(repositoryRoot, "src-tauri", "windows", "open-with.json");
const evidenceRoot = resolve(repositoryRoot, "work", "morflo", "open-with-installer");
const runId = new Date().toISOString().replaceAll(":", "-").replaceAll(".", "-");
const runRoot = join(evidenceRoot, runId);

if (process.platform !== "win32") {
  throw new Error("The Open with installer test is Windows-only.");
}

const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
if (!Array.isArray(manifest.extensions) || manifest.extensions.length === 0) {
  throw new Error("The Open with manifest is invalid.");
}
if (!(await stat(installerPath).catch(() => undefined))?.isFile()) {
  throw new Error(`The current Morflo installer is unavailable: ${installerPath}`);
}
if (processIdsForExecutable(installedAppPath).length > 0) {
  throw new Error("Close Morflo before running the reversible installer registration test.");
}

await mkdir(runRoot, { recursive: true });

const desktopDirectory = runPowerShellJson(
  "[Environment]::GetFolderPath('Desktop') | ConvertTo-Json -Compress",
);
if (typeof desktopDirectory !== "string" || desktopDirectory.length === 0) {
  throw new Error("Windows did not return a usable Desktop directory.");
}
const desktopShortcutPath = join(desktopDirectory, "Morflo.lnk");
const desktopShortcutBaseline = await readFile(desktopShortcutPath).catch(() => undefined);

function registrySnapshot() {
  const extensions = manifest.extensions.map((extension) => `'${extension}'`).join(", ");
  const progId = manifest.progId.replaceAll("'", "''");
  const applicationName = manifest.applicationName.replaceAll("'", "''");
  const executable = manifest.executable.replaceAll("'", "''");
  return runPowerShellJson(`
function Read-RegistryValue([string]$path, [string]$name) {
  $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($path)
  if ($null -eq $key) { return $null }
  try { return $key.GetValue($name, $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames) }
  finally { $key.Dispose() }
}
function Has-RegistryValue([string]$path, [string]$name) {
  $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($path)
  if ($null -eq $key) { return $false }
  try { return @($key.GetValueNames()) -contains $name }
  finally { $key.Dispose() }
}
$rows = foreach ($extension in @(${extensions})) {
  [pscustomobject]@{
    extension = $extension
    classesDefault = Read-RegistryValue "Software\\Classes\\.$extension" ""
    userChoice = Read-RegistryValue "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\FileExts\\.$extension\\UserChoice" "ProgId"
    openWith = Has-RegistryValue "Software\\Classes\\.$extension\\OpenWithProgids" '${progId}'
    capability = Has-RegistryValue "Software\\Morflo\\Capabilities\\FileAssociations" ".$extension"
    supportedType = Has-RegistryValue "Software\\Classes\\Applications\\${executable}\\SupportedTypes" ".$extension"
  }
}
[pscustomobject]@{
  extensions = @($rows)
  registeredApplication = Read-RegistryValue "Software\\RegisteredApplications" '${applicationName}'
  progIdCommand = Read-RegistryValue "Software\\Classes\\${progId}\\shell\\open\\command" ""
  applicationCommand = Read-RegistryValue "Software\\Classes\\Applications\\${executable}\\shell\\open\\command" ""
  friendlyAppName = Read-RegistryValue "Software\\Classes\\Applications\\${executable}" "FriendlyAppName"
  allowSilentDefaultTakeOver = Has-RegistryValue "Software\\Classes\\${progId}" "AllowSilentDefaultTakeOver"
} | ConvertTo-Json -Depth 5 -Compress
`);
}

function runInstaller(executable, label, arguments_) {
  const result = spawnSync(executable, arguments_, {
    encoding: "utf8",
    shell: false,
    windowsHide: true,
    timeout: 120_000,
  });
  if (result.error || result.status !== 0) {
    throw new Error(
      `${label} failed: ${result.error?.message ?? result.stderr.trim() ?? `exit ${result.status}`}`,
    );
  }
}

async function restoreDesktopShortcut() {
  if (desktopShortcutBaseline) {
    await writeFile(desktopShortcutPath, desktopShortcutBaseline);
    return;
  }
  await unlink(desktopShortcutPath).catch((error) => {
    if (error?.code !== "ENOENT") throw error;
  });
}

function defaults(snapshot) {
  return snapshot.extensions.map(({ extension, classesDefault, userChoice }) => ({
    extension,
    classesDefault: classesDefault ?? null,
    userChoice: userChoice ?? null,
  }));
}

function assertDefaultsUnchanged(before, after, label) {
  if (JSON.stringify(defaults(before)) !== JSON.stringify(defaults(after))) {
    throw new Error(`${label} changed an existing Windows default or UserChoice.`);
  }
}

function assertRegistered(snapshot) {
  for (const entry of snapshot.extensions) {
    if (!entry.openWith || !entry.capability || !entry.supportedType) {
      throw new Error(`Incomplete Open with registration for .${entry.extension}.`);
    }
  }
  const expectedCommand = `"${installedAppPath}" "%1"`;
  if (
    snapshot.progIdCommand !== expectedCommand ||
    snapshot.applicationCommand !== expectedCommand
  ) {
    throw new Error(
      `The registered handler command is not safely quoted: ${snapshot.progIdCommand}`,
    );
  }
  if (snapshot.registeredApplication !== "Software\\Morflo\\Capabilities") {
    throw new Error("Morflo is missing from RegisteredApplications.");
  }
  if (snapshot.friendlyAppName !== manifest.applicationName) {
    throw new Error("The registered handler does not expose Morflo's friendly name.");
  }
  if (!snapshot.allowSilentDefaultTakeOver) {
    throw new Error("The ProgID is missing the silent-default-takeover guard.");
  }
}

function assertUnregistered(snapshot) {
  for (const entry of snapshot.extensions) {
    if (entry.openWith || entry.capability || entry.supportedType) {
      throw new Error(`Uninstall left Morflo registration behind for .${entry.extension}.`);
    }
  }
  if (
    snapshot.registeredApplication !== null ||
    snapshot.progIdCommand !== null ||
    snapshot.applicationCommand !== null ||
    snapshot.friendlyAppName !== null ||
    snapshot.allowSilentDefaultTakeOver
  ) {
    throw new Error("Uninstall left a Morflo-owned association key behind.");
  }
}

let result;
try {
  const baseline = registrySnapshot();
  runInstaller(installerPath, "Silent Morflo install", ["/S", "/NS"]);
  const installed = registrySnapshot();
  assertDefaultsUnchanged(baseline, installed, "Install");
  assertRegistered(installed);

  if (!(await stat(uninstallPath).catch(() => undefined))?.isFile()) {
    throw new Error(`The installed uninstaller is unavailable: ${uninstallPath}`);
  }
  runInstaller(uninstallPath, "Silent Morflo uninstall", ["/S"]);
  const uninstalled = registrySnapshot();
  assertDefaultsUnchanged(baseline, uninstalled, "Uninstall");
  assertUnregistered(uninstalled);

  runInstaller(installerPath, "Final silent Morflo reinstall", ["/S", "/NS"]);
  const reinstalled = registrySnapshot();
  assertDefaultsUnchanged(baseline, reinstalled, "Reinstall");
  assertRegistered(reinstalled);
  if (!(await stat(installedAppPath).catch(() => undefined))?.isFile()) {
    throw new Error("The final installed Morflo executable is unavailable.");
  }

  result = {
    environment: "Windows 11 x64; current-user NSIS install/uninstall/reinstall",
    installerPath,
    installedAppPath,
    progId: manifest.progId,
    registeredExtensions: manifest.extensions,
    existingDefaultsPreservedAcrossInstallUninstallReinstall: true,
    handlerCommandsSafelyQuoted: true,
    uninstallRemovedOnlyMorfloOwnedRegistration: true,
    finalState: "installed and registered as an alternate Open with handler",
    snapshots: { baseline, installed, uninstalled, reinstalled },
  };
} finally {
  await restoreDesktopShortcut();
}

const restoredDesktopShortcut = await readFile(desktopShortcutPath).catch(() => undefined);
if (
  (desktopShortcutBaseline === undefined) !== (restoredDesktopShortcut === undefined) ||
  (desktopShortcutBaseline && !desktopShortcutBaseline.equals(restoredDesktopShortcut))
) {
  throw new Error("The installer test did not restore the pre-existing Desktop shortcut.");
}
result.desktopShortcutRestoredByteForByte = true;
await writeFile(join(runRoot, "result.json"), `${JSON.stringify(result, null, 2)}\n`, "utf8");
process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
