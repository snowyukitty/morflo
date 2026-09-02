import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, rename, rm, symlink, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  checkCandidateDossier,
  createCandidateDossier,
  writeCandidateDossier,
} from "./engine-dossier.mjs";
import {
  MANIFEST_FILENAME,
  VerificationError,
  engineProcessEnvironment,
  verifyBundle,
} from "./engine-verify.mjs";

function executableName(stem) {
  return process.platform === "win32" ? `${stem}.exe` : stem;
}

function fakeProbe() {
  return {
    version: "8.1.2-dossier-test",
    buildConfiguration: "--disable-network --enable-small",
    compilerIdentification: "synthetic compiler 1.0",
    libraryVersions: [
      {
        name: "libavcodec",
        major: 62,
        minor: 1,
        micro: 2,
        version: 4063490,
        ident: "Lavc62.1.2",
      },
      {
        name: "libavutil",
        major: 60,
        minor: 1,
        micro: 2,
        version: 3932418,
        ident: "Lavu60.1.2",
      },
    ],
    decoders: ["png"],
    encoders: ["mjpeg", "png"],
    demuxers: ["image2"],
    muxers: ["image2"],
    filters: ["scale"],
  };
}

async function withCandidate(name, callback) {
  const parent = await mkdtemp(path.join(os.tmpdir(), "morflo-dossier-test-"));
  const root = path.join(parent, name);
  await mkdir(root);
  await writeFile(path.join(root, executableName("ffmpeg")), "synthetic ffmpeg");
  await writeFile(path.join(root, executableName("ffprobe")), "synthetic ffprobe");
  try {
    await callback({ parent, root });
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

test("dossier capture requires explicit consent before any candidate execution", async () => {
  await withCandidate("consent", async ({ root }) => {
    let probed = false;
    await expectCode(
      createCandidateDossier({
        directory: root,
        allowExecution: false,
        probe: () => {
          probed = true;
          return fakeProbe();
        },
      }),
      "EXECUTION_CONSENT_REQUIRED",
    );
    assert.equal(probed, false);
  });
});

test("dossier is deterministic, path-neutral, unreviewed, and Unicode-safe", async () => {
  await withCandidate("候補 京都 🧳", async ({ root }) => {
    await mkdir(path.join(root, "notices"));
    await writeFile(path.join(root, "notices", "京都 注意事項.txt"), "unreviewed evidence");
    const first = await createCandidateDossier({
      directory: root,
      allowExecution: true,
      probe: fakeProbe(),
    });
    const second = await createCandidateDossier({
      directory: root,
      allowExecution: true,
      probe: fakeProbe(),
    });

    assert.deepEqual(first.dossier, second.dossier);
    assert.equal(first.dossierSha256, second.dossierSha256);
    assert.equal(first.dossier.assurance.status, "unreviewed");
    assert.match(first.dossier.assurance.statement, /not approved/);
    assert.equal(first.dossier.additionalFiles[0].path, "notices/京都 注意事項.txt");
    assert.equal(first.bytes.includes(Buffer.from(root)), false);

    const schema = JSON.parse(
      await readFile(
        new URL("../src-tauri/engine-candidate-dossier.schema.json", import.meta.url),
        "utf8",
      ),
    );
    assert.equal(schema.properties.schemaVersion.const, 1);
    assert.equal(schema.properties.assurance.properties.status.const, "unreviewed");
    assert.equal(schema.required.includes("observedLibraryVersions"), true);
    assert.equal(schema.$defs.libraryVersion.additionalProperties, false);
    assert.equal(schema.additionalProperties, false);
  });
});

test("write is exclusive and check detects stale candidate evidence", async () => {
  await withCandidate("exclusive", async ({ parent, root }) => {
    const output = path.join(parent, "candidate-dossier.json");
    await writeCandidateDossier({
      directory: root,
      output,
      allowExecution: true,
      probe: fakeProbe(),
    });
    await checkCandidateDossier({
      directory: root,
      dossier: output,
      allowExecution: true,
      probe: fakeProbe(),
    });
    await expectCode(
      writeCandidateDossier({
        directory: root,
        output,
        allowExecution: true,
        probe: fakeProbe(),
      }),
      "DOSSIER_OUTPUT_EXISTS",
    );

    const canonical = await readFile(output);
    const parsed = JSON.parse(canonical.toString("utf8"));
    const reordered = { kind: parsed.kind, ...parsed };
    await writeFile(output, `${JSON.stringify(reordered, null, 2)}\n`);
    await expectCode(
      checkCandidateDossier({
        directory: root,
        dossier: output,
        allowExecution: true,
        probe: fakeProbe(),
      }),
      "DOSSIER_NONCANONICAL",
    );
    await writeFile(output, canonical);

    await writeFile(path.join(root, executableName("ffmpeg")), "substituted ffmpeg");
    await expectCode(
      checkCandidateDossier({
        directory: root,
        dossier: output,
        allowExecution: true,
        probe: fakeProbe(),
      }),
      "DOSSIER_EVIDENCE_MISMATCH",
    );
  });
});

test("only exact root executable names and schema-bounded probe evidence are accepted", async () => {
  await withCandidate("strict-input", async ({ root }) => {
    await rename(
      path.join(root, executableName("ffmpeg")),
      path.join(root, executableName("renamed-ffmpeg")),
    );
    await expectCode(
      createCandidateDossier({
        directory: root,
        allowExecution: true,
        probe: fakeProbe(),
      }),
      "CANDIDATE_EXECUTABLE_MISSING",
    );
    await rename(
      path.join(root, executableName("renamed-ffmpeg")),
      path.join(root, executableName("ffmpeg")),
    );
    const invalid = fakeProbe();
    invalid.buildConfiguration = "--disable-network\nforged-field";
    await expectCode(
      createCandidateDossier({
        directory: root,
        allowExecution: true,
        probe: invalid,
      }),
      "ENGINE_PROBE_FAILED",
    );
  });
});

test("invalid output is rejected before the probe adapter can run", async () => {
  await withCandidate("preflight", async ({ parent, root }) => {
    const output = path.join(parent, "already-there.json");
    await writeFile(output, "sentinel");
    let probed = false;
    await expectCode(
      writeCandidateDossier({
        directory: root,
        output,
        allowExecution: true,
        probe: () => {
          probed = true;
          return fakeProbe();
        },
      }),
      "DOSSIER_OUTPUT_EXISTS",
    );
    assert.equal(probed, false);
    assert.equal(await readFile(output, "utf8"), "sentinel");
  });
});

test("probe-time file mutation fails closed", async () => {
  await withCandidate("mutation", async ({ root }) => {
    const evidence = path.join(root, "NOTICE.txt");
    await writeFile(evidence, "before");
    await expectCode(
      createCandidateDossier({
        directory: root,
        allowExecution: true,
        probe: async () => {
          await writeFile(evidence, "after");
          return fakeProbe();
        },
      }),
      "CANDIDATE_MUTATED_DURING_PROBE",
    );
  });
});

test("dossier remains outside the candidate and cannot masquerade as a reviewed manifest", async () => {
  await withCandidate("boundary", async ({ root }) => {
    await expectCode(
      writeCandidateDossier({
        directory: root,
        output: path.join(root, "candidate-dossier.json"),
        allowExecution: true,
        probe: fakeProbe(),
      }),
      "DOSSIER_INSIDE_CANDIDATE",
    );

    const captured = await createCandidateDossier({
      directory: root,
      allowExecution: true,
      probe: fakeProbe(),
    });
    await writeFile(path.join(root, MANIFEST_FILENAME), captured.bytes);
    await expectCode(
      verifyBundle({ directory: root, probe: fakeProbe() }),
      "MANIFEST_SCHEMA_INVALID",
    );
  });
});

test("candidate symlinks or reparse points are rejected when the host permits creating one", async (context) => {
  await withCandidate("links", async ({ parent, root }) => {
    const outside = path.join(parent, "outside.txt");
    await writeFile(outside, "outside");
    try {
      await symlink(outside, path.join(root, "linked.txt"), "file");
    } catch (error) {
      if (["EPERM", "EACCES", "ENOTSUP"].includes(error?.code)) {
        context.skip("host cannot create an unprivileged test symlink");
        return;
      }
      throw error;
    }
    await expectCode(
      createCandidateDossier({
        directory: root,
        allowExecution: true,
        probe: fakeProbe(),
      }),
      "REPARSE_POINT_REJECTED",
    );
  });
});

test("media-engine child environment excludes parent credentials and search paths", () => {
  const sanitized = engineProcessEnvironment(
    {
      PATH: "C:\\untrusted-search-path",
      HOME: "C:\\private-home",
      GITHUB_TOKEN: "secret",
      SystemRoot: "C:\\Windows",
      TEMP: "C:\\Temp",
    },
    "win32",
  );
  assert.deepEqual(sanitized, {
    LC_ALL: "C",
    LANG: "C",
    SystemRoot: "C:\\Windows",
    TEMP: "C:\\Temp",
  });
  assert.equal("PATH" in sanitized, false);
  assert.equal("HOME" in sanitized, false);
  assert.equal("GITHUB_TOKEN" in sanitized, false);
});
