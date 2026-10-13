import {
  ArrowRight,
  Check,
  CheckCircle2,
  CircleEllipsis,
  FileImage,
  FileVideo,
  FolderOpen,
  Plus,
  RefreshCw,
  Settings2,
  Square,
  TriangleAlert,
  X,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import {
  getImageThumbnail,
  getOutputPreview,
  getPosterFrame,
  hasDesktopRuntime,
} from "./lib/backend";
import { extensionLabel, formatBytes, formatDimensions } from "./lib/format";
import {
  matchesLedgerView,
  measuredBatch,
  measuredChange,
  needsAttention,
  plannedSettings,
  type LedgerView,
} from "./lib/ledger";
import { copy } from "./i18n/en";
import type { ConversionSettings, MediaFile } from "./types";

// Bound decoder concurrency across the visible ledger. Offscreen requests are skipped.
let previewChain: Promise<unknown> = Promise.resolve();
function Thumbnail({ file, output = false }: { file: MediaFile; output?: boolean }) {
  const ref = useRef<HTMLSpanElement>(null);
  const [url, setUrl] = useState<string>();
  const [visible, setVisible] = useState(false);
  const ready = !output || (file.status === "succeeded" && Boolean(file.output));
  useEffect(() => {
    const node = ref.current;
    if (!node || !hasDesktopRuntime()) return;
    const observer = new IntersectionObserver(([entry]) =>
      setVisible(Boolean(entry?.isIntersecting)),
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    setUrl(undefined);
    if (!visible || !ready || !hasDesktopRuntime() || file.kind === "unsupported") return;
    const request = { disposed: false };
    const isCurrent = () => !request.disposed;
    previewChain = previewChain
      .catch(() => undefined)
      .then(async () => {
        if (!isCurrent()) return;
        try {
          const result = output
            ? await getOutputPreview(file.id)
            : file.kind === "video"
              ? await getPosterFrame(file.id, 0)
              : await getImageThumbnail(file.id);
          if (isCurrent()) setUrl(result);
        } catch {
          /* A thumbnail is optional; the verified file remains revealable. */
        }
      });
    return () => {
      request.disposed = true;
    };
  }, [file.id, file.kind, output, ready, visible]);
  return (
    <span
      ref={ref}
      className={`ledger-thumbnail${!ready ? " is-planned" : ""}${(output ? file.output?.hasAlpha : file.hasAlpha) ? " has-alpha" : ""}`}
    >
      {url ? (
        <img src={url} alt={`${output ? "Actual output" : "Source"} preview of ${file.name}`} />
      ) : !ready ? (
        <span className="planned-format">{file.kind === "video" ? "VIDEO" : "IMAGE"}</span>
      ) : file.kind === "video" ? (
        <FileVideo size={24} aria-hidden="true" />
      ) : (
        <FileImage size={24} aria-hidden="true" />
      )}
      {file.animated && !output ? <small>Animated</small> : null}
    </span>
  );
}

interface Props {
  files: MediaFile[];
  settingsById: Record<string, ConversionSettings>;
  selectedIds: Set<string>;
  onToggleSelected: (id: string) => void;
  onSelectOnly: (id: string) => void;
  onRemove: (id: string) => void;
  onChoose: () => void;
  onClear: () => void;
  onReveal: (id: string) => void;
  onRetry: (id: string) => void;
  onCancel: (id: string) => void;
  view: LedgerView;
  onViewChange: (view: LedgerView) => void;
  onPrepareRetries: (ids: string[]) => void;
  preparingRetries: boolean;
  inspectorOpen: boolean;
  onInspectorToggle: () => void;
}

export function ConversionLedger(props: Props) {
  const { files, selectedIds, settingsById } = props;
  const batch = measuredBatch(files);
  const visibleFiles = files.filter((file) => matchesLedgerView(file, props.view));
  const attention = files.filter(needsAttention);
  const completedCount = files.filter((file) => file.status === "succeeded").length;
  const running = files.some((file) => ["running", "queued", "probing"].includes(file.status));
  return (
    <section
      className={`queue-panel conversion-ledger${files.length === 1 ? " single-session small-session" : files.length === 2 ? " small-session" : ""}`}
      aria-labelledby="queue-title"
    >
      <div className="section-heading">
        <div>
          <p className="eyebrow">Originals in. Possibilities out.</p>
          <h1 id="queue-title">Conversion queue</h1>
        </div>
        <div className="section-tools">
          <button className="secondary-button compact" onClick={props.onChoose}>
            <Plus size={16} />
            Add files
          </button>
          <button
            className="icon-button"
            aria-label={props.inspectorOpen ? "Hide file settings" : "Show file settings"}
            aria-expanded={props.inspectorOpen}
            onClick={props.onInspectorToggle}
          >
            <Settings2 size={18} />
          </button>
        </div>
      </div>
      <div className={`ledger-total ${batch.change.tone}`}>
        <div>
          <span className="ledger-kicker">
            {batch.count ? "Measured results" : "Your conversion ledger"}
          </span>
          <strong>
            {batch.count ? (
              <>
                {formatBytes(batch.input)} <span aria-hidden="true">→</span>{" "}
                {formatBytes(batch.output)}
              </>
            ) : (
              <>
                Source <span aria-hidden="true">→</span> Output
              </>
            )}
          </strong>
        </div>
        <p>
          {batch.count ? (
            <>
              <b>
                {batch.change.bytes} ({batch.change.percent})
              </b>
              <span>
                {batch.count} of {files.length} files measured · successful outputs only
              </span>
            </>
          ) : (
            <>
              See what changes.
              <span>Exact sizes appear after conversion. Originals stay untouched.</span>
            </>
          )}
        </p>
      </div>
      <div className="ledger-toolbar">
        <div className="ledger-navigation">
          <div className="ledger-views" role="group" aria-label="Filter conversion queue">
            {(
              [
                ["all", "All files", files.length],
                ["attention", "Needs attention", attention.length],
                ["completed", "Completed", completedCount],
              ] as const
            ).map(([view, label, count]) => (
              <button
                key={view}
                type="button"
                aria-pressed={props.view === view}
                onClick={() => props.onViewChange(view)}
                disabled={props.preparingRetries}
              >
                {label} <span>{count}</span>
              </button>
            ))}
          </div>
        </div>
        <div className="queue-summary">
          <span>{selectedIds.size} selected for settings</span>
          <button
            className="text-button select-all"
            onClick={() => {
              for (const file of visibleFiles)
                if (!selectedIds.has(file.id)) props.onToggleSelected(file.id);
            }}
            disabled={visibleFiles.length === 0 || props.preparingRetries}
          >
            {props.view === "all" ? "Select all" : "Select shown"}
          </button>
          <button
            className="text-button"
            onClick={props.onClear}
            disabled={running || props.preparingRetries}
          >
            Clear queue
          </button>
        </div>
      </div>
      {props.view === "attention" && attention.length > 0 ? (
        <button
          className="text-button prepare-retries"
          title="Keep the current settings. Review them before choosing Convert."
          disabled={running || props.preparingRetries}
          onClick={() => props.onPrepareRetries(attention.map((file) => file.id))}
        >
          <RefreshCw size={13} aria-hidden="true" />
          {props.preparingRetries
            ? "Preparing…"
            : `Prepare ${attention.length} ${attention.length === 1 ? "retry" : "retries"}`}
        </button>
      ) : null}
      <div className="ledger-columns" aria-hidden="true">
        <span>Source / unchanged</span>
        <span>Target / actual output</span>
        <span>Change / status</span>
      </div>
      <div className="file-list" role="list" aria-label={copy.accessibility.filesToConvert}>
        {visibleFiles.map((file) => {
          const settings = settingsById[file.id];
          const actual = file.status === "succeeded" ? file.output : undefined;
          const format =
            actual?.format ??
            (file.kind === "image"
              ? settings?.image.outputFormat
              : file.kind === "video"
                ? settings?.video.outputFormat
                : undefined);
          const active = ["running", "queued", "probing"].includes(file.status);
          const change = actual ? measuredChange(file.sizeBytes, actual.sizeBytes) : undefined;
          return (
            <article
              className={`file-row status-${file.status}${selectedIds.has(file.id) ? " is-selected" : ""}`}
              key={file.id}
              data-file-id={file.id}
              role="listitem"
              aria-current={selectedIds.has(file.id) ? "true" : undefined}
            >
              <label className="select-check">
                <input
                  type="checkbox"
                  checked={selectedIds.has(file.id)}
                  onChange={() => props.onToggleSelected(file.id)}
                  aria-label={`Select ${file.name}`}
                />
                <span>
                  <Check size={12} />
                </span>
              </label>
              <button
                className="file-row-main"
                onClick={() => props.onSelectOnly(file.id)}
                aria-label={`Inspect ${file.name}`}
              >
                <span className="ledger-source">
                  <Thumbnail file={file} />
                  <span className="file-identity">
                    <strong title={file.name}>{file.name}</strong>
                    <span className="ledger-format">
                      {extensionLabel(file.extension)} <b>{formatBytes(file.sizeBytes)}</b>
                    </span>
                    <span>
                      {file.width && file.height
                        ? formatDimensions(file.width, file.height)
                        : file.kind === "unsupported"
                          ? "Inspection needed"
                          : "Dimensions unavailable"}
                    </span>
                    <span>
                      {file.hasAlpha
                        ? "Transparency"
                        : file.kind === "video"
                          ? "Video source"
                          : "Original file"}
                    </span>
                  </span>
                </span>
                <ArrowRight className="conversion-arrow" size={20} aria-hidden="true" />
                <span className="ledger-target">
                  <Thumbnail file={file} output />
                  <span className="file-identity">
                    <span className="ledger-kicker">
                      {actual ? "Actual output" : "Planned output"}
                    </span>
                    <strong className="target-format">
                      {format ? copy.formats[format] : "—"}
                      {actual ? <span>{formatBytes(actual.sizeBytes)}</span> : null}
                    </strong>
                    <span>
                      {actual
                        ? actual.width && actual.height
                          ? formatDimensions(actual.width, actual.height)
                          : "Output verified"
                        : plannedSettings(file, settings)}
                    </span>
                    <span
                      title={actual?.name ?? file.error?.title}
                      className={file.error ? "ledger-error" : undefined}
                    >
                      {actual
                        ? actual.name
                        : (file.error?.title ??
                          (file.status === "canceled"
                            ? "Ready to retry when you are"
                            : "Size measured after conversion"))}
                    </span>
                  </span>
                </span>
              </button>
              <div className={`row-state ${change?.tone ?? ""}`}>
                <strong>
                  {change
                    ? change.percent
                    : file.status === "running" && file.progress !== undefined
                      ? `${Math.round(file.progress * 100)}%`
                      : "—"}
                </strong>
                <span>
                  {change?.bytes ??
                    (file.status === "failed"
                      ? "No output created"
                      : file.status === "canceled"
                        ? "Original kept"
                        : "Not measured")}
                </span>
                <span className="ledger-status">
                  {actual ? (
                    <CheckCircle2 size={13} />
                  ) : file.status === "failed" ? (
                    <TriangleAlert size={13} />
                  ) : active ? (
                    <CircleEllipsis size={13} />
                  ) : null}
                  {copy.phases[file.phase]}
                </span>
              </div>
              <div className="row-actions">
                {actual ? (
                  <button
                    className="text-button ledger-reveal"
                    onClick={() => props.onReveal(file.id)}
                    aria-label={`Reveal output for ${file.name}`}
                  >
                    <FolderOpen size={15} />
                    Reveal
                  </button>
                ) : null}
                {["failed", "canceled"].includes(file.status) ? (
                  <button
                    className="icon-button small"
                    onClick={() => props.onRetry(file.id)}
                    aria-label={`Retry ${file.name}`}
                    title="Try again"
                    disabled={props.preparingRetries}
                  >
                    <RefreshCw size={15} />
                  </button>
                ) : null}
                {file.status !== "probing" ? (
                  <button
                    className="icon-button small"
                    onClick={() => (active ? props.onCancel(file.id) : props.onRemove(file.id))}
                    aria-label={`${active ? "Cancel" : "Remove"} ${file.name}`}
                    title={active ? "Cancel conversion" : "Remove from queue"}
                    disabled={props.preparingRetries}
                  >
                    {active ? <Square size={14} /> : <X size={15} />}
                  </button>
                ) : null}
              </div>
              {file.status === "running" ? (
                <progress
                  className="row-progress"
                  max={1}
                  value={file.progress}
                  aria-label={`Conversion progress for ${file.name}`}
                />
              ) : null}
            </article>
          );
        })}
        {visibleFiles.length === 0 ? (
          <div className="ledger-empty-view">
            <CheckCircle2 size={24} aria-hidden="true" />
            <strong>
              {props.view === "attention"
                ? "No files need attention"
                : "No completed files yet"}
            </strong>
            <p>
              {props.view === "attention"
                ? "Failed and canceled files appear here. Your other files are still in the queue."
                : "Verified outputs appear here after conversion."}
            </p>
            <button
              className="secondary-button compact"
              onClick={() => props.onViewChange("all")}
            >
              Show all files
            </button>
          </div>
        ) : null}
      </div>
      <div className="ledger-footnote">
        <CheckCircle2 size={14} />
        Originals stay untouched
        <span>Settings apply to selected files · Convert processes all ready files</span>
      </div>
    </section>
  );
}
