import { createHash } from "node:crypto";
import { lstat, open, readFile, readdir, realpath } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";
import { isDeepStrictEqual } from "node:util";

import {
  VerificationError,
  deriveOutputs,
  probeCandidateEvidence,
  sha256,
} from "./engine-verify.mjs";

const DOSSIER_SCHEMA_VERSION = 1;
const DOSSIER_KIND = "morflo-engine-candidate-dossier";
const CAPTURE_POLICY = "morflo-offline-probe-v1";
const ASSURANCE_STATEMENT =
  "Observed technical evidence only; not approved for bundling, licensing, or redistribution.";
const MAX_ENTRIES = 256;
const MAX_DEPTH = 8;
const MAX_FILE_BYTES = 1024 * 1024 * 1024;
const MAX_TOTAL_BYTES = 2 * 1024 * 1024 * 1024;
const MAX_DOSSIER_BYTES = 2 * 1024 * 1024;

export async function createCandidateDossier({ directory, allowExecution, probe }) {
  requireExecutionConsent(allowExecution);
  const root = await candidateRoot(directory);
  const before = await inventoryCandidate(root);
  const ffmpegName = executableName("ffmpeg");
  const ffprobeName = executableName("ffprobe");
  const ffmpeg = requiredExecutable(before, ffmpegName);
  const ffprobe = requiredExecutable(before, ffprobeName);
  const observed = probe
    ? await resolveProbeAdapter(probe, {
        ffmpeg: path.join(root, ffmpegName),
        ffprobe: path.join(root, ffprobeName),
      })
    : await probeCandidateEvidence(path.join(root, ffmpegName), path.join(root, ffprobeName));
  validateProbe(observed);

  const after = await inventoryCandidate(root);
  if (!isDeepStrictEqual(before, after)) {
    throw new VerificationError(
      "CANDIDATE_MUTATED_DURING_PROBE",
      "candidate files changed while the engine pair was being probed",
    );
  }

  const dossier = {
    schemaVersion: DOSSIER_SCHEMA_VERSION,
    kind: DOSSIER_KIND,
    capturePolicy: CAPTURE_POLICY,
    assurance: {
      status: "unreviewed",
      statement: ASSURANCE_STATEMENT,
    },
    engine: {
      name: "FFmpeg",
      version: observed.version,
    },
    target: hostTarget(),
    buildConfiguration: observed.buildConfiguration,
    compilerIdentification: observed.compilerIdentification,
    observedLibraryVersions: observed.libraryVersions.map((library) => ({ ...library })),
    executables: {
      ffmpeg: executableRecord(ffmpeg),
      ffprobe: executableRecord(ffprobe),
    },
    additionalFiles: before
      .filter(
        (record) =>
          ![pathIdentity(ffmpegName), pathIdentity(ffprobeName)].includes(
            pathIdentity(record.path),
          ),
      )
      .map((record) => ({ ...record })),
    observedCapabilities: {
      decoders: [...observed.decoders],
      encoders: [...observed.encoders],
      demuxers: [...observed.demuxers],
      muxers: [...observed.muxers],
      filters: [...observed.filters],
      outputs: deriveOutputs(observed),
    },
  };
  const bytes = canonicalBytes(dossier);
  if (bytes.length > MAX_DOSSIER_BYTES) {
    throw new VerificationError(
      "DOSSIER_SIZE_INVALID",
      "the canonical dossier exceeds the 2 MiB evidence limit",
    );
  }
  return { dossier, bytes, dossierSha256: sha256(bytes), root };
}

export async function writeCandidateDossier({ directory, output, allowExecution, probe }) {
  requireAbsolute(output, "--out");
  const preflightRoot = await candidateRoot(directory);
  await validateExternalEvidencePath(output, preflightRoot, { mustExist: false });
  const captured = await createCandidateDossier({ directory, allowExecution, probe });
  let handle;
  try {
    handle = await open(output, "wx", 0o600);
    await handle.writeFile(captured.bytes);
    await handle.sync();
  } catch (error) {
    if (error?.code === "EEXIST") {
      throw new VerificationError(
        "DOSSIER_OUTPUT_EXISTS",
        "the explicit dossier output already exists and will not be overwritten",
      );
    }
    if (error instanceof VerificationError) throw error;
    throw filesystemError("DOSSIER_WRITE_FAILED", error);
  } finally {
    await handle?.close();
  }
  return captured;
}

export async function checkCandidateDossier({
  directory,
  dossier: dossierPath,
  allowExecution,
  probe,
}) {
  requireAbsolute(dossierPath, "--check");
  const preflightRoot = await candidateRoot(directory);
  await validateExternalEvidencePath(dossierPath, preflightRoot, { mustExist: true });
  const captured = await createCandidateDossier({ directory, allowExecution, probe });
  let bytes;
  try {
    const metadata = await lstat(dossierPath);
    if (!metadata.isFile() || metadata.isSymbolicLink()) {
      throw new VerificationError(
        "DOSSIER_TYPE_REJECTED",
        "the dossier must be a regular non-reparse file",
      );
    }
    if (metadata.size < 1 || metadata.size > MAX_DOSSIER_BYTES) {
      throw new VerificationError(
        "DOSSIER_SIZE_INVALID",
        "the dossier must be between 1 byte and 2 MiB",
      );
    }
    bytes = await readFile(dossierPath);
  } catch (error) {
    if (error instanceof VerificationError) throw error;
    throw filesystemError("DOSSIER_READ_FAILED", error);
  }
  let actual;
  try {
    actual = JSON.parse(bytes.toString("utf8"));
  } catch {
    throw new VerificationError("DOSSIER_SCHEMA_INVALID", "the dossier is not valid JSON");
  }
  validateBoundaryMarker(actual);
  if (!isDeepStrictEqual(actual, captured.dossier)) {
    throw new VerificationError(
      "DOSSIER_EVIDENCE_MISMATCH",
      "the dossier no longer matches the candidate files or observed engine evidence",
    );
  }
  if (!bytes.equals(captured.bytes)) {
    throw new VerificationError(
      "DOSSIER_NONCANONICAL",
      "the dossier is not in Morflo's canonical deterministic form",
    );
  }
  return { ...captured, dossierSha256: sha256(bytes) };
}

async function candidateRoot(directory) {
  requireAbsolute(directory, "--dir");
  let metadata;
  try {
    metadata = await lstat(directory);
  } catch (error) {
    throw filesystemError("CANDIDATE_UNAVAILABLE", error);
  }
  if (!metadata.isDirectory() || metadata.isSymbolicLink()) {
    throw new VerificationError(
      "CANDIDATE_ROOT_REJECTED",
      "the explicit candidate root must be a regular directory, not a symlink or reparse point",
    );
  }
  let canonical;
  try {
    canonical = await realpath(directory);
  } catch (error) {
    throw filesystemError("CANDIDATE_UNAVAILABLE", error);
  }
  if (pathIdentity(canonical) !== pathIdentity(path.resolve(directory))) {
    throw new VerificationError(
      "CANDIDATE_ROOT_REJECTED",
      "the explicit candidate root cannot resolve through a symlink or reparse point",
    );
  }
  return canonical;
}

async function inventoryCandidate(root) {
  const records = [];
  const limits = { entries: 0, bytes: 0 };
  await inventoryDirectory(root, "", records, limits, 0);
  records.sort((left, right) => compareText(left.path, right.path));
  const identities = new Set();
  for (const record of records) {
    const identity = pathIdentity(record.path);
    if (identities.has(identity)) {
      throw new VerificationError(
        "DUPLICATE_CANDIDATE_PATH",
        "candidate paths must remain unique under host path semantics",
      );
    }
    identities.add(identity);
  }
  return records;
}

async function inventoryDirectory(root, relative, records, limits, depth) {
  if (depth > MAX_DEPTH) {
    throw new VerificationError(
      "CANDIDATE_TOO_DEEP",
      "the candidate directory exceeds the nesting limit",
    );
  }
  let entries;
  try {
    entries = await readdir(resolveRelative(root, relative), { withFileTypes: true });
  } catch (error) {
    throw filesystemError("CANDIDATE_READ_FAILED", error);
  }
  entries.sort((left, right) => compareText(left.name, right.name));
  if (relative && entries.length === 0) {
    throw new VerificationError(
      "EMPTY_CANDIDATE_DIRECTORY",
      "empty directories are not artifact evidence",
    );
  }
  for (const entry of entries) {
    limits.entries += 1;
    if (limits.entries > MAX_ENTRIES) {
      throw new VerificationError(
        "CANDIDATE_ENTRY_LIMIT_EXCEEDED",
        "the candidate directory contains too many entries",
      );
    }
    validatePathSegment(entry.name);
    const child = relative ? `${relative}/${entry.name}` : entry.name;
    if ([...child].length > 512) {
      throw new VerificationError(
        "CANDIDATE_PATH_INVALID",
        "candidate evidence paths cannot exceed 512 characters",
      );
    }
    const absolute = resolveRelative(root, child);
    let metadata;
    try {
      metadata = await lstat(absolute);
    } catch (error) {
      throw filesystemError("CANDIDATE_READ_FAILED", error);
    }
    if (metadata.isSymbolicLink()) {
      throw new VerificationError(
        "REPARSE_POINT_REJECTED",
        `${bounded(child, 180)} is a symlink or reparse point`,
      );
    }
    if (metadata.isDirectory()) {
      await assertCanonicalEntry(root, absolute, child);
      await inventoryDirectory(root, child, records, limits, depth + 1);
      continue;
    }
    if (!metadata.isFile()) {
      throw new VerificationError(
        "CANDIDATE_ENTRY_TYPE_REJECTED",
        `${bounded(child, 180)} is not a regular file`,
      );
    }
    await assertCanonicalEntry(root, absolute, child);
    const record = await stableFileEvidence(root, absolute, child);
    if (record.sizeBytes < 1 || record.sizeBytes > MAX_FILE_BYTES) {
      throw new VerificationError(
        "CANDIDATE_FILE_SIZE_INVALID",
        `${bounded(child, 180)} must be between 1 byte and 1 GiB`,
      );
    }
    limits.bytes += record.sizeBytes;
    if (limits.bytes > MAX_TOTAL_BYTES) {
      throw new VerificationError(
        "CANDIDATE_TOTAL_SIZE_INVALID",
        "the candidate directory exceeds the 2 GiB evidence limit",
      );
    }
    records.push(record);
  }
}

async function stableFileEvidence(root, absolute, relative) {
  let handle;
  try {
    const pathBefore = await lstat(absolute, { bigint: true });
    handle = await open(absolute, "r");
    const handleBefore = await handle.stat({ bigint: true });
    if (!sameFileIdentity(pathBefore, handleBefore)) {
      throw new VerificationError(
        "CANDIDATE_MUTATED_DURING_HASH",
        `${bounded(relative, 180)} changed before it could be hashed`,
      );
    }
    const hash = createHash("sha256");
    const buffer = Buffer.allocUnsafe(64 * 1024);
    for (;;) {
      const { bytesRead } = await handle.read(buffer, 0, buffer.length, null);
      if (bytesRead === 0) break;
      hash.update(buffer.subarray(0, bytesRead));
    }
    const handleAfter = await handle.stat({ bigint: true });
    const pathAfter = await lstat(absolute, { bigint: true });
    if (
      !sameFileIdentity(handleBefore, handleAfter) ||
      !sameFileIdentity(handleAfter, pathAfter) ||
      !sameFileState(handleBefore, handleAfter) ||
      !sameFileState(handleAfter, pathAfter)
    ) {
      throw new VerificationError(
        "CANDIDATE_MUTATED_DURING_HASH",
        `${bounded(relative, 180)} changed while it was being hashed`,
      );
    }
    await assertCanonicalEntry(root, absolute, relative);
    return {
      path: relative,
      sizeBytes: Number(handleAfter.size),
      sha256: hash.digest("hex"),
    };
  } catch (error) {
    if (error instanceof VerificationError) throw error;
    throw filesystemError("CANDIDATE_READ_FAILED", error);
  } finally {
    await handle?.close();
  }
}

function sameFileIdentity(left, right) {
  return left.dev === right.dev && left.ino === right.ino;
}

function sameFileState(left, right) {
  return (
    left.size === right.size && left.mtimeNs === right.mtimeNs && left.ctimeNs === right.ctimeNs
  );
}

async function assertCanonicalEntry(root, absolute, relative) {
  let canonical;
  try {
    canonical = await realpath(absolute);
  } catch (error) {
    throw filesystemError("CANDIDATE_READ_FAILED", error);
  }
  if (
    pathIdentity(canonical) !== pathIdentity(path.resolve(absolute)) ||
    !isWithin(root, canonical)
  ) {
    throw new VerificationError(
      "CANDIDATE_PATH_ESCAPE",
      `${bounded(relative, 180)} resolves outside the explicit candidate root`,
    );
  }
}

function requiredExecutable(records, filename) {
  const record = records.find(
    (candidate) => pathIdentity(candidate.path) === pathIdentity(filename),
  );
  if (!record || record.path.includes("/")) {
    throw new VerificationError(
      "CANDIDATE_EXECUTABLE_MISSING",
      `the candidate root must contain exact ${filename}`,
    );
  }
  return record;
}

function executableRecord(record) {
  return { filename: record.path, sizeBytes: record.sizeBytes, sha256: record.sha256 };
}

async function resolveProbeAdapter(probe, paths) {
  return typeof probe === "function" ? probe(paths) : probe;
}

function validateProbe(probe) {
  if (!probe || typeof probe !== "object" || Array.isArray(probe)) {
    throw new VerificationError("ENGINE_PROBE_FAILED", "the engine probe returned no evidence");
  }
  if (!validText(probe.version, 128)) {
    throw new VerificationError(
      "ENGINE_PROBE_FAILED",
      "the engine probe returned an invalid version",
    );
  }
  if (
    !validText(probe.buildConfiguration, 65_536) ||
    !probe.buildConfiguration.startsWith("--")
  ) {
    throw new VerificationError(
      "ENGINE_PROBE_FAILED",
      "the engine probe returned an invalid buildConfiguration",
    );
  }
  if (!validText(probe.compilerIdentification, 512)) {
    throw new VerificationError(
      "ENGINE_PROBE_FAILED",
      "the engine probe returned an invalid compilerIdentification",
    );
  }
  if (
    !Array.isArray(probe.libraryVersions) ||
    probe.libraryVersions.length < 1 ||
    probe.libraryVersions.length > 32 ||
    probe.libraryVersions.some(
      (library, index) =>
        !library ||
        typeof library.name !== "string" ||
        !/^[A-Za-z0-9_.-]+$/.test(library.name) ||
        ![library.major, library.minor, library.micro, library.version].every(
          (value) => Number.isSafeInteger(value) && value >= 0,
        ) ||
        !validText(library.ident, 128) ||
        (index > 0 && probe.libraryVersions[index - 1].name >= library.name),
    )
  ) {
    throw new VerificationError(
      "ENGINE_PROBE_FAILED",
      "the engine probe returned a noncanonical library version inventory",
    );
  }
  for (const field of ["decoders", "encoders", "demuxers", "muxers", "filters"]) {
    const values = probe[field];
    if (
      !Array.isArray(values) ||
      values.length < 1 ||
      values.length > 4096 ||
      values.some(
        (value, index) =>
          typeof value !== "string" ||
          value.length > 128 ||
          !/^[A-Za-z0-9_.-]+(?:,[A-Za-z0-9_.-]+)*$/.test(value) ||
          (index > 0 && values[index - 1] >= value),
      )
    ) {
      throw new VerificationError(
        "ENGINE_PROBE_FAILED",
        `the engine probe returned a noncanonical ${field} inventory`,
      );
    }
  }
}

function validText(value, maximum) {
  return (
    typeof value === "string" &&
    value.length >= 1 &&
    Buffer.byteLength(value, "utf8") <= maximum &&
    ![...value].some((character) => {
      const code = character.codePointAt(0);
      return code < 0x20 || code === 0x7f;
    })
  );
}

async function validateExternalEvidencePath(file, root, { mustExist }) {
  if (isWithin(root, path.resolve(file))) {
    throw new VerificationError(
      "DOSSIER_INSIDE_CANDIDATE",
      "the dossier must remain outside the candidate directory so it cannot change the inventory",
    );
  }
  const parent = path.dirname(file);
  let canonicalParent;
  try {
    canonicalParent = await realpath(parent);
  } catch (error) {
    throw filesystemError("DOSSIER_PARENT_INVALID", error);
  }
  if (pathIdentity(canonicalParent) !== pathIdentity(path.resolve(parent))) {
    throw new VerificationError(
      "DOSSIER_PARENT_REJECTED",
      "the dossier parent cannot resolve through a symlink or reparse point",
    );
  }
  if (mustExist) {
    let canonicalFile;
    try {
      canonicalFile = await realpath(file);
    } catch (error) {
      throw filesystemError("DOSSIER_UNAVAILABLE", error);
    }
    if (pathIdentity(canonicalFile) !== pathIdentity(path.resolve(file))) {
      throw new VerificationError(
        "DOSSIER_TYPE_REJECTED",
        "the dossier cannot resolve through a symlink or reparse point",
      );
    }
  } else {
    try {
      await lstat(file);
      throw new VerificationError(
        "DOSSIER_OUTPUT_EXISTS",
        "the explicit dossier output already exists and will not be overwritten",
      );
    } catch (error) {
      if (error instanceof VerificationError) throw error;
      if (error?.code !== "ENOENT") throw filesystemError("DOSSIER_OUTPUT_INVALID", error);
    }
  }
}

function validateBoundaryMarker(dossier) {
  if (
    !dossier ||
    dossier.schemaVersion !== DOSSIER_SCHEMA_VERSION ||
    dossier.kind !== DOSSIER_KIND ||
    dossier.capturePolicy !== CAPTURE_POLICY ||
    dossier.assurance?.status !== "unreviewed" ||
    dossier.assurance?.statement !== ASSURANCE_STATEMENT
  ) {
    throw new VerificationError(
      "DOSSIER_SCHEMA_INVALID",
      "the file is not a versioned unreviewed Morflo candidate dossier",
    );
  }
}

function canonicalBytes(dossier) {
  return Buffer.from(`${JSON.stringify(dossier, null, 2)}\n`, "utf8");
}

function hostTarget() {
  const platform = { win32: "windows", darwin: "macos", linux: "linux" }[process.platform];
  const architecture = { x64: "x86_64", arm64: "aarch64" }[process.arch];
  if (!platform || !architecture) {
    throw new VerificationError(
      "HOST_TARGET_UNSUPPORTED",
      "this host cannot capture a media-engine dossier",
    );
  }
  return { platform, architecture };
}

function executableName(stem) {
  return process.platform === "win32" ? `${stem}.exe` : stem;
}

function validatePathSegment(segment) {
  if (
    !segment ||
    segment === "." ||
    segment === ".." ||
    segment.includes("/") ||
    segment.includes("\\") ||
    [...segment].some((character) => character.codePointAt(0) < 0x20 || character === "\u007f")
  ) {
    throw new VerificationError(
      "CANDIDATE_PATH_INVALID",
      "candidate paths must be portable relative evidence paths",
    );
  }
}

function resolveRelative(root, relative) {
  return relative ? path.join(root, ...relative.split("/")) : root;
}

function isWithin(root, candidate) {
  const relative = path.relative(root, candidate);
  return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative));
}

function pathIdentity(value) {
  const normalized = path.normalize(value);
  return process.platform === "win32" ? normalized.toLowerCase() : normalized;
}

function compareText(left, right) {
  return left < right ? -1 : left > right ? 1 : 0;
}

function requireExecutionConsent(value) {
  if (value !== true) {
    throw new VerificationError(
      "EXECUTION_CONSENT_REQUIRED",
      "--allow-execution is required because probing runs the candidate's exact ffmpeg and ffprobe files without claiming they are sandboxed",
    );
  }
}

function requireAbsolute(value, argument) {
  if (typeof value !== "string" || !path.isAbsolute(value)) {
    throw new VerificationError(
      "EXPLICIT_ABSOLUTE_PATH_REQUIRED",
      `${argument} must be an explicit absolute path`,
    );
  }
}

function filesystemError(code, error) {
  return new VerificationError(
    code,
    `filesystem operation failed (${error?.code ?? "I/O error"})`,
  );
}

function bounded(value, limit) {
  const characters = [...String(value)];
  return characters.length > limit ? `${characters.slice(0, limit).join("")}…` : String(value);
}

function parseArguments(arguments_) {
  const values = { allowExecution: false };
  for (let index = 0; index < arguments_.length; index += 1) {
    const argument = arguments_[index];
    if (argument === "--help" || argument === "-h") return { help: true };
    if (argument === "--allow-execution") {
      if (values.allowExecution) {
        throw new VerificationError("ARGUMENT_INVALID", "duplicate --allow-execution");
      }
      values.allowExecution = true;
      continue;
    }
    const key = { "--dir": "directory", "--out": "output", "--check": "dossier" }[argument];
    if (!key) {
      throw new VerificationError("ARGUMENT_INVALID", "an unknown argument was provided");
    }
    if (values[key] !== undefined) {
      throw new VerificationError("ARGUMENT_INVALID", `duplicate argument: ${argument}`);
    }
    const value = arguments_[index + 1];
    if (!value || value.startsWith("--")) {
      throw new VerificationError("ARGUMENT_INVALID", `missing value for ${argument}`);
    }
    values[key] = value;
    index += 1;
  }
  if (!values.directory || Boolean(values.output) === Boolean(values.dossier)) {
    throw new VerificationError(
      "ARGUMENT_INVALID",
      "--dir and exactly one of --out or --check are required",
    );
  }
  requireAbsolute(values.directory, "--dir");
  requireAbsolute(values.output ?? values.dossier, values.output ? "--out" : "--check");
  requireExecutionConsent(values.allowExecution);
  return values;
}

function printUsage() {
  console.error(
    "USAGE: pnpm engine:dossier --dir <absolute-candidate-directory> (--out <absolute-new-json> | --check <absolute-json>) --allow-execution",
  );
}

async function main() {
  let arguments_;
  try {
    arguments_ = parseArguments(process.argv.slice(2));
  } catch (error) {
    console.error("ENGINE CANDIDATE DOSSIER: rejected");
    console.error(`REASON: ${error.message}`);
    printUsage();
    process.exitCode = 1;
    return;
  }
  if (arguments_.help) {
    printUsage();
    return;
  }
  try {
    const result = arguments_.output
      ? await writeCandidateDossier(arguments_)
      : await checkCandidateDossier(arguments_);
    console.log(
      arguments_.output
        ? "ENGINE CANDIDATE DOSSIER: unreviewed evidence captured"
        : "ENGINE CANDIDATE DOSSIER: current evidence matched",
    );
    console.log(`DOSSIER SHA-256: ${result.dossierSha256}`);
    console.log(`ENGINE: ${result.dossier.engine.name} ${result.dossier.engine.version}`);
    console.log(
      `TARGET: ${result.dossier.target.platform} ${result.dossier.target.architecture}`,
    );
    console.log(
      `ARTIFACTS: ${result.dossier.additionalFiles.length + 2} regular files hashed before and after probing`,
    );
    console.log(
      `CAPABILITIES: ${result.dossier.observedCapabilities.decoders.length} decoders, ${result.dossier.observedCapabilities.encoders.length} encoders, ${result.dossier.observedCapabilities.demuxers.length} demuxers, ${result.dossier.observedCapabilities.muxers.length} muxers, ${result.dossier.observedCapabilities.filters.length} filters, ${result.dossier.observedCapabilities.outputs.length} observed Morflo outputs`,
    );
    console.log(`ASSURANCE: ${ASSURANCE_STATEMENT}`);
    console.log(
      "EXECUTION: exact candidate ffmpeg/ffprobe names were probed with a sanitized environment; no sandbox claim is made",
    );
  } catch (error) {
    console.error("ENGINE CANDIDATE DOSSIER: rejected");
    console.error(
      `REASON: ${
        error instanceof VerificationError
          ? error.message
          : "UNEXPECTED_FAILURE: the dossier command encountered a bounded local error"
      }`,
    );
    process.exitCode = 1;
  }
}

if (process.argv[1] && pathToFileURL(process.argv[1]).href === import.meta.url) {
  await main();
}
