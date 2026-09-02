import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  MANIFEST_FILENAME,
  VerificationError,
  parseCandidateProgramEvidence,
  sha256,
  stageVerifiedBundle,
  verifyBundle,
} from "./engine-verify.mjs";

const TEST_DAY = Math.floor(Date.UTC(2025, 7, 31) / 86_400_000);

function executableName(stem) {
  return process.platform === "win32" ? `${stem}.exe` : stem;
}

function target() {
  return {
    platform: { win32: "windows", darwin: "macos", linux: "linux" }[process.platform],
    architecture: { x64: "x86_64", arm64: "aarch64" }[process.arch],
  };
}

function fakeProbe() {
  return {
    version: "8.1.2-test",
    buildConfiguration: "--disable-network --enable-small",
    decoders: ["png"],
    encoders: ["mjpeg", "png"],
    demuxers: ["image2"],
    muxers: ["image2"],
    filters: ["scale"],
  };
}

async function fixtureFile(root, relative, bytes) {
  const destination = path.join(root, ...relative.split("/"));
  await mkdir(path.dirname(destination), { recursive: true });
  await writeFile(destination, bytes);
  return {
    path: relative,
    sizeBytes: bytes.length,
    sha256: createHash("sha256").update(bytes).digest("hex"),
  };
}

async function fixtureManifest(root, probe = fakeProbe()) {
  const ffmpeg = await fixtureFile(
    root,
    executableName("ffmpeg"),
    Buffer.from("synthetic ffmpeg adapter"),
  );
  const ffprobe = await fixtureFile(
    root,
    executableName("ffprobe"),
    Buffer.from("synthetic ffprobe adapter"),
  );
  const notice = await fixtureFile(root, "notices/NOTICE.txt", Buffer.from("notice evidence"));
  const source = await fixtureFile(
    root,
    "source/SOURCE_OFFER.txt",
    Buffer.from("source-offer evidence"),
  );
  return {
    schemaVersion: 1,
    engine: { name: "FFmpeg", version: probe.version },
    target: target(),
    provenance: { sourceUrlOrIdentifier: "synthetic:test-adapter:v1" },
    buildConfiguration: probe.buildConfiguration,
    declaredLicense: "LicenseRef-Synthetic-Test-Only",
    executables: {
      ffmpeg: { filename: ffmpeg.path, sizeBytes: ffmpeg.sizeBytes, sha256: ffmpeg.sha256 },
      ffprobe: {
        filename: ffprobe.path,
        sizeBytes: ffprobe.sizeBytes,
        sha256: ffprobe.sha256,
      },
    },
    supportingFiles: [],
    reviewedCapabilities: {
      decoders: probe.decoders,
      encoders: probe.encoders,
      demuxers: probe.demuxers,
      muxers: probe.muxers,
      filters: probe.filters,
      outputs: ["png", "jpeg"],
    },
    noticeReferences: [notice],
    sourceOfferReferences: [source],
    review: {
      status: "approved",
      reviewedBy: "Morflo synthetic test adapter",
      reviewedAt: "2025-08-01",
      validUntil: "2026-08-01",
    },
  };
}

async function writeManifest(root, manifest) {
  const bytes = Buffer.from(JSON.stringify(manifest, null, 2));
  await writeFile(path.join(root, MANIFEST_FILENAME), bytes);
  return sha256(bytes);
}

async function withBundle(name, callback) {
  const parent = await mkdtemp(path.join(os.tmpdir(), "morflo-engine-test-"));
  const root = path.join(parent, name);
  await mkdir(root);
  try {
    await callback(root);
  } finally {
    await rm(parent, { recursive: true, force: true });
  }
}

async function expectCode(promise, code) {
  await assert.rejects(promise, (error) => {
    assert.ok(error instanceof VerificationError);
    assert.equal(error.code, code);
    return true;
  });
}

test("valid synthetic adapter matches an independent manifest pin", async () => {
  await withBundle("reviewed synthetic", async (root) => {
    const probe = fakeProbe();
    const manifest = await fixtureManifest(root, probe);
    const digest = await writeManifest(root, manifest);
    const verified = await verifyBundle({
      directory: root,
      expectedManifestSha256: digest,
      probe,
      today: TEST_DAY,
    });
    assert.equal(verified.manifestSha256, digest);
  });
});

test("structured candidate evidence matches the pair and canonicalizes library versions", () => {
  const pair = fakeProbe();
  const evidence = parseCandidateProgramEvidence(
    JSON.stringify({
      program_version: {
        version: pair.version,
        configuration: pair.buildConfiguration,
        compiler_ident: "synthetic compiler",
      },
      library_versions: [
        { name: "libavutil", major: 60, minor: 1, micro: 2, version: 1, ident: "Lavu" },
        { name: "libavcodec", major: 62, minor: 1, micro: 2, version: 2, ident: "Lavc" },
      ],
    }),
    pair,
  );
  assert.equal(evidence.compilerIdentification, "synthetic compiler");
  assert.deepEqual(
    evidence.libraryVersions.map((library) => library.name),
    ["libavcodec", "libavutil"],
  );
  assert.throws(
    () =>
      parseCandidateProgramEvidence(
        JSON.stringify({
          program_version: {
            version: "mismatched",
            configuration: pair.buildConfiguration,
            compiler_ident: "synthetic compiler",
          },
          library_versions: [],
        }),
        pair,
      ),
    (error) => error instanceof VerificationError && error.code === "ENGINE_PROBE_FAILED",
  );
});

test("hash mismatch is rejected", async () => {
  await withBundle("hash mismatch", async (root) => {
    const probe = fakeProbe();
    const manifest = await fixtureManifest(root, probe);
    await writeManifest(root, manifest);
    await writeFile(
      path.join(root, executableName("ffmpeg")),
      Buffer.from("synthetic ffmpeg replace"),
    );
    await expectCode(
      verifyBundle({ directory: root, probe, today: TEST_DAY }),
      "ARTIFACT_HASH_MISMATCH",
    );
  });
});

test("version and capability mismatches are rejected", async () => {
  await withBundle("probe mismatches", async (root) => {
    const probe = fakeProbe();
    const manifest = await fixtureManifest(root, probe);
    await writeManifest(root, manifest);
    await expectCode(
      verifyBundle({
        directory: root,
        probe: { ...probe, version: "8.1.3-test" },
        today: TEST_DAY,
      }),
      "ENGINE_VERSION_MISMATCH",
    );
    await expectCode(
      verifyBundle({
        directory: root,
        probe: { ...probe, encoders: [...probe.encoders, "webp"] },
        today: TEST_DAY,
      }),
      "CAPABILITY_MISMATCH",
    );
  });
});

test("missing notice and traversal evidence are rejected", async () => {
  await withBundle("evidence failures", async (root) => {
    const probe = fakeProbe();
    const manifest = await fixtureManifest(root, probe);
    manifest.noticeReferences = [];
    await writeManifest(root, manifest);
    await expectCode(
      verifyBundle({ directory: root, probe, today: TEST_DAY }),
      "NOTICE_EVIDENCE_MISSING",
    );
    manifest.noticeReferences = [
      { path: "../NOTICE.txt", sizeBytes: 1, sha256: "0".repeat(64) },
    ];
    await writeManifest(root, manifest);
    await expectCode(
      verifyBundle({ directory: root, probe, today: TEST_DAY }),
      "ARTIFACT_PATH_INVALID",
    );
  });
});

test("Unicode directories and evidence paths are supported", async () => {
  await withBundle("審核済み 京都 🧳", async (root) => {
    const probe = fakeProbe();
    const manifest = await fixtureManifest(root, probe);
    await rm(path.join(root, "notices", "NOTICE.txt"));
    manifest.noticeReferences = [
      await fixtureFile(root, "notices/京都 注意事項.txt", Buffer.from("Unicode notice")),
    ];
    await writeManifest(root, manifest);
    await verifyBundle({ directory: root, probe, today: TEST_DAY });
  });
});

test("unknown schema fields and stale review status are rejected", async () => {
  await withBundle("schema failures", async (root) => {
    const probe = fakeProbe();
    const manifest = await fixtureManifest(root, probe);
    manifest.inventedApproval = true;
    await writeManifest(root, manifest);
    await expectCode(
      verifyBundle({ directory: root, probe, today: TEST_DAY }),
      "MANIFEST_SCHEMA_INVALID",
    );
    delete manifest.inventedApproval;
    manifest.review.validUntil = "2025-08-02";
    await writeManifest(root, manifest);
    await expectCode(
      verifyBundle({ directory: root, probe, today: TEST_DAY }),
      "REVIEW_EXPIRED",
    );
  });
});

test("staging copies only a pinned verified bundle and rejects overlap", async () => {
  const parent = await mkdtemp(path.join(os.tmpdir(), "morflo-engine-stage-test-"));
  const source = path.join(parent, "source");
  const destination = path.join(parent, "stage");
  await mkdir(source);
  try {
    const probe = fakeProbe();
    const manifest = await fixtureManifest(source, probe);
    const digest = await writeManifest(source, manifest);
    const staged = await stageVerifiedBundle({
      directory: source,
      destination,
      expectedManifestSha256: digest,
      probe,
      today: TEST_DAY,
    });
    assert.equal(staged.manifestSha256, digest);
    await expectCode(
      stageVerifiedBundle({
        directory: source,
        destination: path.join(source, "nested-stage"),
        expectedManifestSha256: digest,
        probe,
        today: TEST_DAY,
      }),
      "STAGE_OVERLAP_REJECTED",
    );
  } finally {
    await rm(parent, { recursive: true, force: true });
  }
});
