import { createHash } from "node:crypto";
import {
  chmod,
  copyFile,
  lstat,
  mkdir,
  open,
  readFile,
  readdir,
  realpath,
  stat,
} from "node:fs/promises";
import { constants as fsConstants } from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawn } from "node:child_process";
import { pathToFileURL } from "node:url";

export const MANIFEST_FILENAME = "morflo-engine-bundle.json";
const SCHEMA_VERSION = 1;
const MAX_MANIFEST_BYTES = 1024 * 1024;
const MAX_CAPTURE_BYTES = 2 * 1024 * 1024;
const MAX_BUNDLE_ENTRIES = 256;
const MAX_BUNDLE_DEPTH = 8;
const ENGINE_TIMEOUT_MS = 12_000;
const MAX_REVIEW_DAYS = 366;
const OUTPUTS = ["png", "jpeg", "webp", "avif", "ico", "mp4", "webm", "gif"];

export class VerificationError extends Error {
  constructor(code, detail) {
    super(`${code}: ${bounded(String(detail), 360)}`);
    this.name = "VerificationError";
    this.code = code;
  }
}

export async function verifyBundle({ directory, expectedManifestSha256, probe, today }) {
  const prepared = await prepareBundle({
    directory,
    expectedManifestSha256,
    today: today ?? Math.floor(Date.now() / 86_400_000),
  });
  const actualProbe = probe ?? (await probePair(prepared.ffmpeg, prepared.ffprobe));
  compareProbe(prepared.manifest, actualProbe);
  await verifyTrackedFiles(prepared.root, prepared.manifest);
  return { ...prepared, probe: actualProbe };
}

export async function stageVerifiedBundle({
  directory,
  destination,
  expectedManifestSha256,
  probe,
  today,
}) {
  if (!path.isAbsolute(directory) || !path.isAbsolute(destination)) {
    throw new VerificationError(
      "EXPLICIT_ABSOLUTE_PATH_REQUIRED",
      "source and staging directories must be explicit absolute paths",
    );
  }
  try {
    await lstat(destination);
    throw new VerificationError(
      "STAGE_ALREADY_EXISTS",
      "the explicit staging directory already exists",
    );
  } catch (error) {
    if (error instanceof VerificationError) throw error;
    if (error?.code !== "ENOENT") throw filesystemError("STAGE_CHECK_FAILED", error);
  }

  const verified = await verifyBundle({
    directory,
    expectedManifestSha256,
    probe,
    today,
  });
  const destinationParent = path.dirname(destination);
  let canonicalParent;
  try {
    canonicalParent = await realpath(destinationParent);
  } catch (error) {
    throw filesystemError("STAGE_PARENT_INVALID", error);
  }
  if (pathIdentity(canonicalParent) !== pathIdentity(path.resolve(destinationParent))) {
    throw new VerificationError(
      "STAGE_PARENT_REJECTED",
      "the staging parent cannot resolve through a symlink or reparse point",
    );
  }
  const canonicalDestination = path.join(canonicalParent, path.basename(destination));
  if (
    isWithin(verified.root, canonicalDestination) ||
    isWithin(canonicalDestination, verified.root)
  ) {
    throw new VerificationError(
      "STAGE_OVERLAP_REJECTED",
      "source and staging directories cannot contain one another",
    );
  }
  await mkdir(destination);
  for (const relative of verified.relativeFiles) {
    const source = resolveRelative(verified.root, relative);
    const target = resolveRelative(destination, relative);
    await mkdir(path.dirname(target), { recursive: true });
    await copyFile(source, target, fsConstants.COPYFILE_EXCL);
    if (process.platform !== "win32") {
      const sourceStat = await stat(source);
      await chmod(target, sourceStat.mode);
    }
  }
  return verifyBundle({
    directory: destination,
    expectedManifestSha256,
    probe,
    today,
  });
}

async function prepareBundle({ directory, expectedManifestSha256, today }) {
  await validateBundleRoot(directory);
  const root = await realpath(directory);
  const manifestPath = path.join(root, MANIFEST_FILENAME);
  await validateRegularFile(manifestPath, MANIFEST_FILENAME);
  const manifestStat = await stat(manifestPath);
  if (manifestStat.size < 1 || manifestStat.size > MAX_MANIFEST_BYTES) {
    throw new VerificationError(
      "MANIFEST_SIZE_INVALID",
      "the manifest must be between 1 byte and 1 MiB",
    );
  }
  const bytes = await readFile(manifestPath);
  const manifestSha256 = sha256(bytes);
  if (expectedManifestSha256 !== undefined) {
    validateSha256(expectedManifestSha256, "expected manifest digest");
    if (manifestSha256 !== expectedManifestSha256) {
      throw new VerificationError(
        "MANIFEST_TRUST_PIN_MISMATCH",
        "the manifest does not match the separately reviewed digest",
      );
    }
  }

  let manifest;
  try {
    manifest = JSON.parse(bytes.toString("utf8"));
  } catch {
    throw new VerificationError("MANIFEST_SCHEMA_INVALID", "manifest is not valid JSON");
  }
  validateManifest(manifest, today);
  await verifyTrackedFiles(root, manifest);
  const relativeFiles = trackedRelativeFiles(manifest);
  await verifyInventory(root, relativeFiles);
  return {
    manifest,
    manifestSha256,
    root,
    relativeFiles,
    ffmpeg: resolveRelative(root, manifest.executables.ffmpeg.filename),
    ffprobe: resolveRelative(root, manifest.executables.ffprobe.filename),
  };
}

function validateManifest(manifest, today) {
  assertObject(manifest, "manifest");
  assertKeys(
    manifest,
    [
      "schemaVersion",
      "engine",
      "target",
      "provenance",
      "buildConfiguration",
      "declaredLicense",
      "executables",
      "reviewedCapabilities",
      "noticeReferences",
      "sourceOfferReferences",
      "review",
    ],
    ["supportingFiles"],
    "manifest",
  );
  if (manifest.schemaVersion !== SCHEMA_VERSION) {
    throw new VerificationError(
      "MANIFEST_SCHEMA_UNSUPPORTED",
      `schemaVersion must be ${SCHEMA_VERSION}`,
    );
  }

  assertObject(manifest.engine, "engine");
  assertKeys(manifest.engine, ["name", "version"], [], "engine");
  if (manifest.engine.name !== "FFmpeg") {
    throw new VerificationError("ENGINE_NAME_INVALID", "engine.name must be FFmpeg");
  }
  validateText(manifest.engine.version, 128, "engine.version");

  assertObject(manifest.target, "target");
  assertKeys(manifest.target, ["platform", "architecture"], [], "target");
  const expectedPlatform = { win32: "windows", darwin: "macos", linux: "linux" }[
    process.platform
  ];
  const expectedArchitecture = { x64: "x86_64", arm64: "aarch64" }[process.arch];
  if (!expectedPlatform || !expectedArchitecture) {
    throw new VerificationError(
      "HOST_TARGET_UNSUPPORTED",
      "this host cannot verify a reviewed sidecar",
    );
  }
  if (
    manifest.target.platform !== expectedPlatform ||
    manifest.target.architecture !== expectedArchitecture
  ) {
    throw new VerificationError(
      "TARGET_MISMATCH",
      "the declared platform or architecture does not match this host",
    );
  }

  assertObject(manifest.provenance, "provenance");
  assertKeys(manifest.provenance, ["sourceUrlOrIdentifier"], [], "provenance");
  validateText(
    manifest.provenance.sourceUrlOrIdentifier,
    2048,
    "provenance.sourceUrlOrIdentifier",
  );
  validateText(manifest.buildConfiguration, 65_536, "buildConfiguration");
  if (!manifest.buildConfiguration.startsWith("--")) {
    throw new VerificationError(
      "BUILD_CONFIGURATION_INVALID",
      "buildConfiguration must contain the exact configure argument line",
    );
  }
  validateText(manifest.declaredLicense, 256, "declaredLicense");

  assertObject(manifest.executables, "executables");
  assertKeys(manifest.executables, ["ffmpeg", "ffprobe"], [], "executables");
  validateExecutable(manifest.executables.ffmpeg, "ffmpeg");
  validateExecutable(manifest.executables.ffprobe, "ffprobe");

  const supportingFiles = manifest.supportingFiles ?? [];
  assertArray(supportingFiles, "supportingFiles", 0, 128);
  for (const file of supportingFiles) {
    validateFileRecord(file, "supporting file", ["path", "role", "sizeBytes", "sha256"]);
    if (!new Set(["runtime_library", "data"]).has(file.role)) {
      throw new VerificationError(
        "SUPPORTING_FILE_ROLE_INVALID",
        "supporting file role must be runtime_library or data",
      );
    }
  }
  manifest.supportingFiles = supportingFiles;

  validateCapabilities(manifest.reviewedCapabilities);
  validateEvidenceList(manifest.noticeReferences, "notice", "NOTICE_EVIDENCE_MISSING");
  validateEvidenceList(
    manifest.sourceOfferReferences,
    "source offer",
    "SOURCE_OFFER_EVIDENCE_MISSING",
  );
  validateReview(manifest.review, today);
}

function validateExecutable(record, stem) {
  validateFileRecord(record, stem, ["filename", "sizeBytes", "sha256"]);
  const expected = process.platform === "win32" ? `${stem}.exe` : stem;
  if (record.filename.toLowerCase() !== expected.toLowerCase()) {
    throw new VerificationError(
      "EXECUTABLE_NAME_INVALID",
      `the reviewed ${stem} filename must be ${expected}`,
    );
  }
}

function validateFileRecord(record, label, keys) {
  assertObject(record, label);
  assertKeys(record, keys, [], label);
  const relative = "path" in record ? record.path : record.filename;
  validateRelativePath(relative);
  if (!Number.isSafeInteger(record.sizeBytes) || record.sizeBytes < 1) {
    throw new VerificationError(
      "ARTIFACT_SIZE_INVALID",
      `${label} sizeBytes must be a positive safe integer`,
    );
  }
  validateSha256(record.sha256, label);
}

function validateEvidenceList(list, label, missingCode) {
  assertArray(list, `${label} references`, 1, 32, missingCode);
  for (const evidence of list) {
    validateFileRecord(evidence, label, ["path", "sizeBytes", "sha256"]);
  }
}

function validateCapabilities(capabilities) {
  assertObject(capabilities, "reviewedCapabilities");
  assertKeys(
    capabilities,
    ["decoders", "encoders", "demuxers", "muxers", "filters", "outputs"],
    [],
    "reviewedCapabilities",
  );
  for (const name of ["decoders", "encoders", "demuxers", "muxers", "filters"]) {
    const values = capabilities[name];
    assertArray(values, `reviewedCapabilities.${name}`, 1, 4096);
    if (
      values.some(
        (value) =>
          typeof value !== "string" ||
          value.length < 1 ||
          value.length > 128 ||
          !/^[A-Za-z0-9_.-]+(?:,[A-Za-z0-9_.-]+)*$/.test(value),
      )
    ) {
      throw new VerificationError(
        "CAPABILITY_EVIDENCE_INVALID",
        `reviewedCapabilities.${name} contains an invalid token`,
      );
    }
    if (values.some((value, index) => index > 0 && values[index - 1] >= value)) {
      throw new VerificationError(
        "CAPABILITY_EVIDENCE_NONCANONICAL",
        `reviewedCapabilities.${name} must be sorted and unique`,
      );
    }
  }
  assertArray(capabilities.outputs, "reviewedCapabilities.outputs", 1, OUTPUTS.length);
  const unique = new Set(capabilities.outputs);
  const canonical = OUTPUTS.filter((output) => unique.has(output));
  if (
    unique.size !== capabilities.outputs.length ||
    canonical.length !== capabilities.outputs.length ||
    !canonical.every((output, index) => capabilities.outputs[index] === output)
  ) {
    throw new VerificationError(
      "OUTPUT_EVIDENCE_NONCANONICAL",
      "reviewedCapabilities.outputs must be unique and use Morflo's canonical order",
    );
  }
}

function validateReview(review, today) {
  assertObject(review, "review");
  assertKeys(review, ["status", "reviewedBy", "reviewedAt", "validUntil"], [], "review");
  if (review.status !== "approved") {
    throw new VerificationError("REVIEW_NOT_APPROVED", "review.status must be approved");
  }
  validateText(review.reviewedBy, 256, "review.reviewedBy");
  const reviewedAt = parseDate(review.reviewedAt);
  const validUntil = parseDate(review.validUntil);
  if (reviewedAt > today) {
    throw new VerificationError(
      "REVIEW_DATE_INVALID",
      "review.reviewedAt cannot be in the future",
    );
  }
  if (validUntil < reviewedAt || validUntil - reviewedAt > MAX_REVIEW_DAYS) {
    throw new VerificationError(
      "REVIEW_WINDOW_INVALID",
      "review.validUntil must be within 366 days of review.reviewedAt",
    );
  }
  if (today > validUntil) {
    throw new VerificationError("REVIEW_EXPIRED", "the reviewed bundle approval has expired");
  }
}

async function verifyTrackedFiles(root, manifest) {
  const records = [
    [manifest.executables.ffmpeg.filename, manifest.executables.ffmpeg, "ffmpeg"],
    [manifest.executables.ffprobe.filename, manifest.executables.ffprobe, "ffprobe"],
    ...manifest.noticeReferences.map((record) => [record.path, record, "notice"]),
    ...manifest.sourceOfferReferences.map((record) => [record.path, record, "source offer"]),
    ...manifest.supportingFiles.map((record) => [record.path, record, "supporting file"]),
  ];
  for (const [relative, record, label] of records) {
    await verifyFileRecord(root, relative, record, label);
  }
}

async function verifyFileRecord(root, relative, record, label) {
  validateRelativePath(relative);
  const file = resolveRelative(root, relative);
  await validateRegularFile(file, label);
  const resolved = await realpath(file);
  if (!isWithin(root, resolved)) {
    throw new VerificationError(
      "ARTIFACT_ESCAPES_BUNDLE",
      `${label} resolves outside the reviewed directory`,
    );
  }
  const metadata = await stat(resolved);
  if (metadata.size !== record.sizeBytes) {
    throw new VerificationError(
      "ARTIFACT_SIZE_MISMATCH",
      `${label} size does not match the manifest`,
    );
  }
  const actualHash = await sha256File(resolved);
  if (actualHash !== record.sha256) {
    throw new VerificationError(
      "ARTIFACT_HASH_MISMATCH",
      `${label} SHA-256 does not match the manifest`,
    );
  }
}

function trackedRelativeFiles(manifest) {
  const files = [
    MANIFEST_FILENAME,
    manifest.executables.ffmpeg.filename,
    manifest.executables.ffprobe.filename,
    ...manifest.noticeReferences.map((record) => record.path),
    ...manifest.sourceOfferReferences.map((record) => record.path),
    ...manifest.supportingFiles.map((record) => record.path),
  ];
  const identities = new Set();
  for (const file of files) {
    validateRelativePath(file);
    const identity = pathIdentity(file);
    if (identities.has(identity)) {
      throw new VerificationError(
        "DUPLICATE_ARTIFACT_PATH",
        "each manifest artifact path must be unique",
      );
    }
    identities.add(identity);
  }
  return files;
}

async function verifyInventory(root, trackedFiles) {
  const expectedFiles = new Set(trackedFiles.map(pathIdentity));
  const expectedDirectories = new Set();
  for (const file of trackedFiles) {
    const segments = file.split("/");
    segments.pop();
    while (segments.length > 0) {
      expectedDirectories.add(pathIdentity(segments.join("/")));
      segments.pop();
    }
  }
  const counter = { value: 0 };
  await inspectDirectory(root, "", expectedFiles, expectedDirectories, counter, 0);
}

async function inspectDirectory(
  root,
  relative,
  expectedFiles,
  expectedDirectories,
  counter,
  depth,
) {
  if (depth > MAX_BUNDLE_DEPTH) {
    throw new VerificationError(
      "BUNDLE_TOO_DEEP",
      "the reviewed directory exceeds the nesting limit",
    );
  }
  let entries;
  try {
    entries = await readdir(resolveRelative(root, relative), { withFileTypes: true });
  } catch (error) {
    throw filesystemError("BUNDLE_READ_FAILED", error);
  }
  for (const entry of entries) {
    counter.value += 1;
    if (counter.value > MAX_BUNDLE_ENTRIES) {
      throw new VerificationError(
        "BUNDLE_FILE_LIMIT_EXCEEDED",
        "the reviewed directory contains too many entries",
      );
    }
    const child = relative ? `${relative}/${entry.name}` : entry.name;
    const identity = pathIdentity(child);
    const metadata = await lstat(resolveRelative(root, child));
    if (metadata.isSymbolicLink()) {
      throw new VerificationError(
        "REPARSE_POINT_REJECTED",
        `${bounded(child, 180)} is a symlink or reparse point`,
      );
    }
    if (metadata.isDirectory()) {
      if (!expectedDirectories.has(identity)) {
        throw new VerificationError(
          "UNTRACKED_BUNDLE_ENTRY",
          `${bounded(child, 180)} is not declared by the manifest`,
        );
      }
      await inspectDirectory(
        root,
        child,
        expectedFiles,
        expectedDirectories,
        counter,
        depth + 1,
      );
    } else if (!metadata.isFile() || !expectedFiles.has(identity)) {
      throw new VerificationError(
        "UNTRACKED_BUNDLE_ENTRY",
        `${bounded(child, 180)} is not declared by the manifest`,
      );
    }
  }
}

async function validateBundleRoot(directory) {
  let metadata;
  try {
    metadata = await lstat(directory);
  } catch (error) {
    throw filesystemError("BUNDLE_UNAVAILABLE", error);
  }
  if (!metadata.isDirectory() || metadata.isSymbolicLink()) {
    throw new VerificationError(
      "BUNDLE_ROOT_REJECTED",
      "the explicit bundle root must be a regular directory, not a symlink or reparse point",
    );
  }
}

async function validateRegularFile(file, label) {
  let metadata;
  try {
    metadata = await lstat(file);
  } catch {
    throw new VerificationError("ARTIFACT_UNAVAILABLE", `${label} is missing`);
  }
  if (!metadata.isFile() || metadata.isSymbolicLink()) {
    throw new VerificationError(
      "ARTIFACT_TYPE_REJECTED",
      `${label} must be a regular non-reparse file`,
    );
  }
}

function compareProbe(manifest, probe) {
  assertObject(probe, "engine probe adapter");
  if (manifest.engine.version !== probe.version) {
    throw new VerificationError(
      "ENGINE_VERSION_MISMATCH",
      "the runtime engine version does not match the manifest",
    );
  }
  if (manifest.buildConfiguration !== probe.buildConfiguration) {
    throw new VerificationError(
      "BUILD_CONFIGURATION_MISMATCH",
      "the runtime build configuration does not match the manifest",
    );
  }
  for (const name of ["decoders", "encoders", "demuxers", "muxers", "filters"]) {
    if (!arraysEqual(manifest.reviewedCapabilities[name], probe[name])) {
      throw new VerificationError(
        "CAPABILITY_MISMATCH",
        `the runtime ${name} inventory does not match the manifest`,
      );
    }
  }
  const actualOutputs = deriveOutputs(probe);
  if (!arraysEqual(manifest.reviewedCapabilities.outputs, actualOutputs)) {
    throw new VerificationError(
      "OUTPUT_CAPABILITY_MISMATCH",
      "the derived Morflo output capabilities do not match the manifest",
    );
  }
}

async function probePair(ffmpeg, ffprobe) {
  const ffmpegVersion = await capture(ffmpeg, ["-version"], "ffmpeg version");
  const ffprobeVersion = await capture(ffprobe, ["-version"], "ffprobe version");
  const version = parseVersion(ffmpegVersion, "ffmpeg");
  const probeVersion = parseVersion(ffprobeVersion, "ffprobe");
  const buildConfiguration = parseBuildConfiguration(ffmpegVersion);
  const probeConfiguration = parseBuildConfiguration(ffprobeVersion);
  if (!version || !probeVersion) {
    throw new VerificationError(
      "ENGINE_PROBE_FAILED",
      "ffmpeg or ffprobe did not report a compatible version",
    );
  }
  if (version !== probeVersion) {
    throw new VerificationError(
      "ENGINE_PROBE_FAILED",
      "ffmpeg and ffprobe reported different versions",
    );
  }
  if (!buildConfiguration || buildConfiguration !== probeConfiguration) {
    throw new VerificationError(
      "ENGINE_PROBE_FAILED",
      "ffmpeg and ffprobe reported missing or different build configurations",
    );
  }
  const [decoders, encoders, demuxers, muxers, filters] = await Promise.all([
    probeListing(ffmpeg, "-decoders", "decoder"),
    probeListing(ffmpeg, "-encoders", "encoder"),
    probeListing(ffmpeg, "-demuxers", "demuxer"),
    probeListing(ffmpeg, "-muxers", "muxer"),
    probeListing(ffmpeg, "-filters", "filter"),
  ]);
  return {
    version,
    buildConfiguration,
    decoders,
    encoders,
    demuxers,
    muxers,
    filters,
  };
}

export async function probeCandidateEvidence(ffmpeg, ffprobe) {
  const pair = await probePair(ffmpeg, ffprobe);
  const output = await capture(
    ffprobe,
    ["-v", "error", "-show_program_version", "-show_library_versions", "-of", "json"],
    "ffprobe library version",
  );
  return parseCandidateProgramEvidence(output, pair);
}

export function parseCandidateProgramEvidence(output, pair) {
  let structured;
  try {
    structured = JSON.parse(output);
  } catch {
    throw new VerificationError(
      "ENGINE_PROBE_FAILED",
      "ffprobe library version evidence was not valid JSON",
    );
  }
  const program = structured?.program_version;
  if (
    program?.version !== pair.version ||
    program?.configuration !== pair.buildConfiguration ||
    typeof program?.compiler_ident !== "string" ||
    program.compiler_ident.length < 1
  ) {
    throw new VerificationError(
      "ENGINE_PROBE_FAILED",
      "ffprobe structured program evidence did not match the engine pair",
    );
  }
  const libraries = structured?.library_versions;
  if (!Array.isArray(libraries) || libraries.length < 1 || libraries.length > 32) {
    throw new VerificationError(
      "ENGINE_PROBE_FAILED",
      "ffprobe returned an invalid library version inventory",
    );
  }
  const libraryVersions = libraries
    .map((library) => {
      if (
        typeof library?.name !== "string" ||
        !/^[A-Za-z0-9_.-]+$/.test(library.name) ||
        ![library.major, library.minor, library.micro, library.version].every(
          (value) => Number.isSafeInteger(value) && value >= 0,
        ) ||
        typeof library.ident !== "string" ||
        library.ident.length < 1 ||
        library.ident.length > 128
      ) {
        throw new VerificationError(
          "ENGINE_PROBE_FAILED",
          "ffprobe returned a malformed library version record",
        );
      }
      return {
        name: library.name,
        major: library.major,
        minor: library.minor,
        micro: library.micro,
        version: library.version,
        ident: library.ident,
      };
    })
    .sort((left, right) => (left.name < right.name ? -1 : left.name > right.name ? 1 : 0));
  if (
    libraryVersions.some(
      (value, index) => index > 0 && libraryVersions[index - 1].name === value.name,
    )
  ) {
    throw new VerificationError(
      "ENGINE_PROBE_FAILED",
      "ffprobe returned duplicate library version records",
    );
  }
  return {
    ...pair,
    compilerIdentification: program.compiler_ident,
    libraryVersions,
  };
}

async function probeListing(ffmpeg, argument, label) {
  const output = await capture(ffmpeg, ["-hide_banner", argument], label);
  const listing = parseListing(output);
  if (listing.length === 0) {
    throw new VerificationError(
      "ENGINE_PROBE_FAILED",
      `ffmpeg returned an empty ${label} inventory`,
    );
  }
  return listing;
}

function capture(executable, arguments_, label) {
  return new Promise((resolve, reject) => {
    const child = spawn(executable, arguments_, {
      env: engineProcessEnvironment(),
      shell: false,
      windowsHide: true,
      stdio: ["ignore", "pipe", "pipe"],
    });
    const stdout = [];
    const stderr = [];
    let captured = 0;
    let finished = false;
    const finish = (callback) => {
      if (finished) return;
      finished = true;
      clearTimeout(timer);
      callback();
    };
    const collect = (target) => (chunk) => {
      captured += chunk.length;
      if (captured > MAX_CAPTURE_BYTES) {
        child.kill();
        finish(() =>
          reject(
            new VerificationError(
              "ENGINE_PROBE_FAILED",
              `${label} probe exceeded the output limit`,
            ),
          ),
        );
        return;
      }
      target.push(chunk);
    };
    child.stdout.on("data", collect(stdout));
    child.stderr.on("data", collect(stderr));
    child.on("error", (error) =>
      finish(() =>
        reject(
          new VerificationError(
            "ENGINE_PROBE_FAILED",
            `${label} probe could not start (${error.code ?? "process error"})`,
          ),
        ),
      ),
    );
    child.on("close", (code) =>
      finish(() => {
        if (code !== 0) {
          reject(
            new VerificationError(
              "ENGINE_PROBE_FAILED",
              `${label} probe failed with exit code ${code}`,
            ),
          );
          return;
        }
        resolve(Buffer.concat(stdout).toString("utf8"));
      }),
    );
    const timer = setTimeout(() => {
      child.kill();
      finish(() =>
        reject(new VerificationError("ENGINE_PROBE_FAILED", `${label} probe timed out`)),
      );
    }, ENGINE_TIMEOUT_MS);
  });
}

function parseVersion(output, tool) {
  const fields = output.split(/\r?\n/, 1)[0]?.trim().split(/\s+/) ?? [];
  return fields[0] === tool && fields[1] === "version" ? fields[2] : undefined;
}

function parseBuildConfiguration(output) {
  const line = output
    .split(/\r?\n/)
    .find((candidate) => candidate.startsWith("configuration: "));
  return line?.slice("configuration: ".length);
}

function parseListing(output) {
  const names = new Set();
  for (const line of output.split(/\r?\n/)) {
    const [flags, name] = line.trim().split(/\s+/);
    if (flags && name && flags.length <= 7 && /^[A-Za-z.]+$/.test(flags) && name !== "=") {
      names.add(name);
    }
  }
  return [...names].sort();
}

export function deriveOutputs(probe) {
  const encoders = new Set(probe.encoders);
  const muxers = new Set(probe.muxers);
  const filters = new Set(probe.filters);
  return OUTPUTS.filter((format) => {
    switch (format) {
      case "png":
        return encoders.has("png");
      case "jpeg":
        return encoders.has("mjpeg");
      case "webp":
        return encoders.has("libwebp") || encoders.has("libwebp_anim");
      case "avif":
        return muxers.has("avif") && encoders.has("libaom-av1");
      case "ico":
        return muxers.has("ico") && encoders.has("png") && filters.has("scale");
      case "mp4":
        return (
          muxers.has("mp4") &&
          encoders.has("libx264") &&
          encoders.has("aac") &&
          filters.has("scale")
        );
      case "webm":
        return (
          muxers.has("webm") &&
          encoders.has("libvpx-vp9") &&
          encoders.has("libopus") &&
          filters.has("scale")
        );
      case "gif":
        return (
          encoders.has("gif") &&
          encoders.has("png") &&
          filters.has("palettegen") &&
          filters.has("paletteuse") &&
          filters.has("scale") &&
          filters.has("fps")
        );
      default:
        return false;
    }
  });
}

function validateRelativePath(value) {
  if (
    typeof value !== "string" ||
    value.length < 1 ||
    value.length > 512 ||
    value.includes("\\") ||
    value.includes("\0") ||
    value.startsWith("/") ||
    value.split("/").some((segment) => segment === "" || segment === "." || segment === "..")
  ) {
    throw new VerificationError(
      "ARTIFACT_PATH_INVALID",
      "artifact paths must be bounded slash-separated relative paths without traversal",
    );
  }
}

function resolveRelative(root, relative) {
  if (!relative) return root;
  return path.join(root, ...relative.split("/"));
}

function isWithin(root, candidate) {
  const relative = path.relative(root, candidate);
  return (
    relative === "" ||
    (!relative.startsWith(`..${path.sep}`) && relative !== ".." && !path.isAbsolute(relative))
  );
}

function pathIdentity(value) {
  const normalized = String(value).split(path.sep).join("/");
  return process.platform === "win32" ? normalized.toLowerCase() : normalized;
}

function validateSha256(value, label) {
  if (typeof value !== "string" || !/^[0-9a-f]{64}$/.test(value)) {
    throw new VerificationError(
      "SHA256_INVALID",
      `${label} SHA-256 must be 64 lowercase hexadecimal characters`,
    );
  }
}

function validateText(value, maxBytes, label) {
  if (
    typeof value !== "string" ||
    value.trim() !== value ||
    value.length < 1 ||
    Buffer.byteLength(value) > maxBytes ||
    hasDisallowedControl(value)
  ) {
    throw new VerificationError(
      "MANIFEST_TEXT_INVALID",
      `${label} is missing, unbounded, or contains control characters`,
    );
  }
}

function hasDisallowedControl(value) {
  return [...value].some((character) => {
    const code = character.codePointAt(0);
    return code === 127 || (code < 32 && code !== 9);
  });
}

function parseDate(value) {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
  if (!match) {
    throw new VerificationError("REVIEW_DATE_INVALID", "review dates must use YYYY-MM-DD");
  }
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const timestamp = Date.UTC(year, month - 1, day);
  const date = new Date(timestamp);
  if (
    year < 2000 ||
    date.getUTCFullYear() !== year ||
    date.getUTCMonth() !== month - 1 ||
    date.getUTCDate() !== day
  ) {
    throw new VerificationError(
      "REVIEW_DATE_INVALID",
      "review date is not a valid calendar day",
    );
  }
  return Math.floor(timestamp / 86_400_000);
}

function assertObject(value, label) {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new VerificationError("MANIFEST_SCHEMA_INVALID", `${label} must be an object`);
  }
}

function assertKeys(value, required, optional, label) {
  const allowed = new Set([...required, ...optional]);
  const missing = required.filter((key) => !(key in value));
  const unknown = Object.keys(value).filter((key) => !allowed.has(key));
  if (missing.length > 0 || unknown.length > 0) {
    throw new VerificationError(
      "MANIFEST_SCHEMA_INVALID",
      `${label} has missing or unknown fields`,
    );
  }
}

function assertArray(value, label, minimum, maximum, missingCode = "MANIFEST_SCHEMA_INVALID") {
  if (!Array.isArray(value) || value.length < minimum || value.length > maximum) {
    throw new VerificationError(
      minimum > 0 && Array.isArray(value) && value.length === 0
        ? missingCode
        : "MANIFEST_SCHEMA_INVALID",
      `${label} must contain between ${minimum} and ${maximum} items`,
    );
  }
}

async function sha256File(file) {
  const handle = await open(file, "r");
  const hash = createHash("sha256");
  const buffer = Buffer.allocUnsafe(64 * 1024);
  try {
    for (;;) {
      const { bytesRead } = await handle.read(buffer, 0, buffer.length, null);
      if (bytesRead === 0) break;
      hash.update(buffer.subarray(0, bytesRead));
    }
  } finally {
    await handle.close();
  }
  return hash.digest("hex");
}

export function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

export function engineProcessEnvironment(
  environment = process.env,
  platform = process.platform,
) {
  const sanitized = { LC_ALL: "C", LANG: "C" };
  if (platform !== "win32") return sanitized;

  for (const required of ["SystemRoot", "WINDIR", "TEMP", "TMP"]) {
    const match = Object.entries(environment).find(
      ([name, value]) => name.toLowerCase() === required.toLowerCase() && value,
    );
    if (match) sanitized[required] = match[1];
  }
  return sanitized;
}

function arraysEqual(left, right) {
  return (
    Array.isArray(left) &&
    Array.isArray(right) &&
    left.length === right.length &&
    left.every((value, index) => value === right[index])
  );
}

function filesystemError(code, error) {
  return new VerificationError(
    code,
    `filesystem operation failed (${error?.code ?? "I/O error"})`,
  );
}

function bounded(value, limit) {
  const characters = [...value];
  return characters.length > limit ? `${characters.slice(0, limit).join("")}…` : value;
}

function parseArguments(arguments_) {
  const values = {};
  for (let index = 0; index < arguments_.length; index += 2) {
    const argument = arguments_[index];
    if (argument === "--help" || argument === "-h") return { help: true };
    const key = {
      "--dir": "directory",
      "--expected-manifest-sha256": "expectedManifestSha256",
      "--stage": "destination",
    }[argument];
    if (!key)
      throw new VerificationError("ARGUMENT_INVALID", "an unknown argument was provided");
    if (values[key] !== undefined) {
      throw new VerificationError("ARGUMENT_INVALID", `duplicate argument: ${argument}`);
    }
    const value = arguments_[index + 1];
    if (!value) {
      throw new VerificationError("ARGUMENT_INVALID", `missing value for ${argument}`);
    }
    values[key] = value;
  }
  if (!values.directory) {
    throw new VerificationError("ARGUMENT_INVALID", "--dir is required");
  }
  if (!path.isAbsolute(values.directory)) {
    throw new VerificationError(
      "EXPLICIT_ABSOLUTE_PATH_REQUIRED",
      "--dir must be an explicit absolute directory",
    );
  }
  if (values.destination && !path.isAbsolute(values.destination)) {
    throw new VerificationError(
      "EXPLICIT_ABSOLUTE_PATH_REQUIRED",
      "--stage must be an explicit absolute directory",
    );
  }
  if (values.destination && !values.expectedManifestSha256) {
    throw new VerificationError(
      "ARGUMENT_INVALID",
      "--stage requires --expected-manifest-sha256",
    );
  }
  return values;
}

function printUsage() {
  console.error(
    "USAGE: pnpm engine:verify --dir <absolute-directory> [--expected-manifest-sha256 <sha256> --stage <absolute-directory>]",
  );
}

async function main() {
  let arguments_;
  try {
    arguments_ = parseArguments(process.argv.slice(2));
  } catch (error) {
    console.error("ENGINE BUNDLE: rejected");
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
    const verified = arguments_.destination
      ? await stageVerifiedBundle(arguments_)
      : await verifyBundle(arguments_);
    console.log("ENGINE BUNDLE: integrity verified offline");
    console.log(`MANIFEST SHA-256: ${verified.manifestSha256}`);
    console.log(`ENGINE: ${verified.manifest.engine.name} ${verified.manifest.engine.version}`);
    console.log(
      `TARGET: ${verified.manifest.target.platform} ${verified.manifest.target.architecture}`,
    );
    console.log(`REVIEW: approved through ${verified.manifest.review.validUntil}`);
    console.log(
      `CAPABILITIES: ${verified.probe.decoders.length} decoders, ${verified.probe.encoders.length} encoders, ${verified.probe.demuxers.length} demuxers, ${verified.probe.muxers.length} muxers, ${verified.probe.filters.length} filters, ${verified.manifest.reviewedCapabilities.outputs.length} Morflo outputs`,
    );
    if (arguments_.expectedManifestSha256) {
      console.log("TRUST PIN: exact reviewed digest matched");
    } else {
      console.log(
        "TRUST: integrity evidence only; reviewed bundled status requires packaging with this exact manifest digest as an independent pin",
      );
    }
    if (arguments_.destination) console.log("STAGING: verified copy completed");
  } catch (error) {
    console.error("ENGINE BUNDLE: rejected");
    console.error(
      `REASON: ${
        error instanceof VerificationError
          ? error.message
          : "UNEXPECTED_FAILURE: the verifier encountered a bounded local error"
      }`,
    );
    process.exitCode = 1;
  }
}

if (process.argv[1] && pathToFileURL(process.argv[1]).href === import.meta.url) {
  await main();
}
