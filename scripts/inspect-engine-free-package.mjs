import { lstat, readdir, realpath } from "node:fs/promises";
import path from "node:path";
import process from "node:process";

const MAX_ENTRIES = 256;
const MAX_DEPTH = 8;

function parseDirectory(arguments_) {
  if (arguments_.length !== 2 || arguments_[0] !== "--dir") {
    throw new Error("usage: --dir <explicit-absolute-installed-directory>");
  }
  if (!path.isAbsolute(arguments_[1])) {
    throw new Error("the installed directory must be an explicit absolute path");
  }
  return arguments_[1];
}

async function inspect(directory) {
  const rootMetadata = await lstat(directory);
  if (!rootMetadata.isDirectory() || rootMetadata.isSymbolicLink()) {
    throw new Error("the installed root must be a regular directory");
  }
  const root = await realpath(directory);
  const files = [];
  const counter = { value: 0 };

  async function walk(relative, depth) {
    if (depth > MAX_DEPTH) throw new Error("the installed tree exceeds the depth limit");
    const entries = await readdir(path.join(root, relative), { withFileTypes: true });
    for (const entry of entries) {
      counter.value += 1;
      if (counter.value > MAX_ENTRIES)
        throw new Error("the installed tree exceeds the entry limit");
      const child = path.join(relative, entry.name);
      const metadata = await lstat(path.join(root, child));
      if (metadata.isSymbolicLink()) throw new Error("the installed tree contains a link");
      if (metadata.isDirectory()) {
        await walk(child, depth + 1);
      } else if (metadata.isFile()) {
        files.push(child.split(path.sep).join("/"));
      } else {
        throw new Error("the installed tree contains an unsupported entry type");
      }
    }
  }

  await walk("", 0);
  const forbidden = files.filter((file) => {
    const normalized = file.toLowerCase();
    const basename = path.posix.basename(normalized);
    return (
      normalized.startsWith("engines/") ||
      basename === "morflo-engine-bundle.json" ||
      basename === "ffmpeg" ||
      basename === "ffmpeg.exe" ||
      basename === "ffprobe" ||
      basename === "ffprobe.exe"
    );
  });
  if (forbidden.length > 0) {
    throw new Error(`forbidden media-engine entries found: ${forbidden.join(", ")}`);
  }
  return files.sort();
}

try {
  const directory = parseDirectory(process.argv.slice(2));
  const files = await inspect(directory);
  console.log("ENGINE-FREE PACKAGE INSPECTION: passed");
  console.log(`INSTALLED FILES: ${files.length}`);
  for (const file of files) console.log(`- ${file}`);
} catch (error) {
  console.error("ENGINE-FREE PACKAGE INSPECTION: failed");
  const reason =
    error && typeof error === "object" && "code" in error
      ? `filesystem operation failed (${error.code})`
      : error instanceof Error
        ? error.message
        : "bounded local error";
  console.error(`REASON: ${reason}`);
  process.exitCode = 1;
}
