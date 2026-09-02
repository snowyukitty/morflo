import {
  AlertCircle,
  Aperture,
  Check,
  CheckCircle2,
  ChevronDown,
  CircleEllipsis,
  Clock3,
  Copy,
  FileImage,
  FileVideo,
  FolderOpen,
  Image as ImageIcon,
  Info,
  Layers3,
  Moon,
  MoreHorizontal,
  Play,
  Plus,
  RefreshCw,
  Settings2,
  ShieldCheck,
  Square,
  Sun,
  TriangleAlert,
  X,
} from "lucide-react";
import {
  type CSSProperties,
  type DragEvent,
  type PointerEvent as ReactPointerEvent,
  type ReactNode,
  type RefObject,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";

import morfloMarkUrl from "../master.svg?url";
import { copy } from "./i18n/en";
import {
  cancelJob,
  clearSession,
  chooseEngineFolder,
  chooseFiles,
  chooseOutputFolder,
  forgetSource,
  getCapabilities,
  getImageThumbnail,
  getOutputPreview,
  getPosterFrame,
  getPreviewClip,
  getStartupReport,
  getPendingFiles,
  getVideoStoryboard,
  hasDesktopRuntime,
  inspectFiles,
  listenForExternalFiles,
  listenForJobProgress,
  normalizeCommandError,
  refreshCapabilities,
  revealOutput,
  retryInspection,
  selectEngineDirectory,
  startJob,
} from "./lib/backend";
import {
  demoCapabilities,
  getDemoFiles,
  missingDemoCapabilities,
  nativeDemoCapabilities,
} from "./lib/demo";
import {
  extensionLabel,
  formatBytes,
  formatDimensions,
  formatDuration,
  formatMomentTime,
} from "./lib/format";
import { loadPreferences, savePreferences, type PreferencesV1 } from "./lib/preferences";
import { recommendedImageOutput } from "./lib/recommendations";
import type {
  CapabilityRegistry,
  CollisionPolicy,
  ConversionError,
  ConversionSettings,
  GifPreset,
  ImageSettings,
  MediaFile,
  MetadataPolicy,
  OutputFormat,
  OutputSummary,
  Quality,
  ResizeMode,
  Resolution,
  StartupReport,
  VideoSettings,
} from "./types";

const outputLabels: Record<OutputFormat, string> = { ...copy.formats };
const DEMO_MOMENT_FRAME_COUNT = 7;

const defaultSettings: ConversionSettings = {
  image: {
    outputFormat: "jpeg",
    quality: "balanced",
    resizeMode: "original",
    background: "#F5F1E8",
    metadata: "remove",
    animation: "ask",
  },
  video: {
    outputFormat: "mp4",
    resolution: "original",
    quality: "balanced",
    metadata: "preserve",
  },
  gif: {
    preset: "web",
    startSeconds: 1.8,
    endSeconds: 6.2,
    width: 540,
    fps: 12,
    quality: "balanced",
    loop: "forever",
  },
  destination: "same",
  collisionPolicy: "suffix",
};

type Theme = "light" | "dark";

interface TimedPreview {
  key: string;
  url: string;
}

interface TimedStoryboard {
  key: string;
  urls: string[];
}

function settingsFor(
  file: MediaFile,
  demoState: string | null,
  preferences: PreferencesV1,
): ConversionSettings {
  const settings = structuredClone(defaultSettings);
  settings.collisionPolicy = preferences.collisionPolicy;
  settings.image.metadata = preferences.imageMetadata;
  settings.video.metadata = preferences.videoMetadata;
  if (file.kind === "image") {
    settings.image.outputFormat = recommendedImageOutput(file);
  }
  if (file.kind === "video" && file.durationSeconds !== undefined) {
    settings.gif.startSeconds = 0;
    settings.gif.endSeconds = Math.min(6, file.durationSeconds);
  }
  if (demoState === "gif") settings.video.outputFormat = "gif";
  return settings;
}

function updateFile(files: MediaFile[], id: string, update: Partial<MediaFile>): MediaFile[] {
  return files.map((file) => (file.id === id ? { ...file, ...update } : file));
}

export function App() {
  const query = useMemo(() => new URLSearchParams(window.location.search), []);
  const demoState = query.get("demo");
  const isDirectionStudy = query.get("study") === "design";
  const initialPreferences = useMemo(() => loadPreferences(), []);
  const [preferences, setPreferences] = useState<PreferencesV1>(initialPreferences);
  const initialFiles = useMemo(() => getDemoFiles(demoState), [demoState]);
  const [files, setFiles] = useState<MediaFile[]>(initialFiles);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(
    () =>
      new Set(
        demoState && query.get("selection") === "all"
          ? initialFiles.map((file) => file.id)
          : initialFiles[0]
            ? [initialFiles[0].id]
            : [],
      ),
  );
  const [settingsById, setSettingsById] = useState<Record<string, ConversionSettings>>(() =>
    Object.fromEntries(
      initialFiles.map((file) => [file.id, settingsFor(file, demoState, initialPreferences)]),
    ),
  );
  const [capabilities, setCapabilities] = useState<CapabilityRegistry>(() =>
    demoState
      ? demoState === "engine-missing"
        ? missingDemoCapabilities
        : demoState === "engine-native"
          ? nativeDemoCapabilities
          : demoCapabilities
      : { engine: { available: false, name: copy.engineName, source: "missing" }, outputs: [] },
  );
  const [theme, setTheme] = useState<Theme>(() => {
    const explicit = query.get("theme");
    if (explicit === "light" || explicit === "dark") return explicit;
    if (initialPreferences.theme === "light" || initialPreferences.theme === "dark") {
      return initialPreferences.theme;
    }
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  });
  const [dragActive, setDragActive] = useState(false);
  const [busyAdding, setBusyAdding] = useState(false);
  const [liveMessage, setLiveMessage] = useState("");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [diagnosticsOpen, setDiagnosticsOpen] = useState(
    () => query.get("panel") === "diagnostics",
  );
  const [engineBusy, setEngineBusy] = useState(() => !demoState && hasDesktopRuntime());
  const [engineError, setEngineError] = useState<ConversionError>();
  const [posterPreview, setPosterPreview] = useState<TimedPreview>();
  const [storyboardPreview, setStoryboardPreview] = useState<TimedStoryboard>();
  const [imagePreview, setImagePreview] = useState<TimedPreview>();
  const [clipPreview, setClipPreview] = useState<TimedPreview>();
  const [outputPreview, setOutputPreview] = useState<TimedPreview>();
  const [previewLoadingId, setPreviewLoadingId] = useState<string>();
  const [storyboardLoadingId, setStoryboardLoadingId] = useState<string>();
  const [startupReport, setStartupReport] = useState<StartupReport>();
  const chooseButtonRef = useRef<HTMLButtonElement>(null);
  const settingsButtonRef = useRef<HTMLButtonElement>(null);
  const diagnosticsReturnFocusRef = useRef<HTMLButtonElement | null>(null);

  const closeSettings = useCallback(() => {
    setSettingsOpen(false);
    window.setTimeout(() => settingsButtonRef.current?.focus(), 0);
  }, []);

  const openDiagnostics = useCallback((trigger: HTMLButtonElement) => {
    diagnosticsReturnFocusRef.current = trigger;
    setSettingsOpen(false);
    setDiagnosticsOpen(true);
  }, []);

  const closeDiagnostics = useCallback(() => {
    setDiagnosticsOpen(false);
    window.setTimeout(() => diagnosticsReturnFocusRef.current?.focus(), 0);
  }, []);

  const handleRefreshEngine = useCallback(async () => {
    if (demoState || !hasDesktopRuntime()) {
      setLiveMessage(
        capabilities.engine.available
          ? copy.diagnostics.checkedReady
          : copy.diagnostics.checkedMissing,
      );
      return;
    }
    setEngineBusy(true);
    setEngineError(undefined);
    try {
      const registry = await refreshCapabilities();
      setCapabilities(registry);
      setLiveMessage(
        registry.engine.available
          ? copy.diagnostics.checkedReady
          : copy.diagnostics.checkedMissing,
      );
    } catch (error) {
      const mapped = normalizeCommandError(error, {
        code: "engine_failed",
        title: copy.diagnostics.checkFailedTitle,
        message: copy.diagnostics.checkFailedMessage,
      });
      setEngineError(mapped);
      setLiveMessage(mapped.title);
    } finally {
      setEngineBusy(false);
    }
  }, [capabilities.engine.available, demoState]);

  const handleChooseEngine = useCallback(async () => {
    if (demoState || !hasDesktopRuntime()) return;
    const directory = await chooseEngineFolder();
    if (!directory) return;
    setEngineBusy(true);
    setEngineError(undefined);
    try {
      const registry = await selectEngineDirectory(directory);
      setCapabilities(registry);
      setLiveMessage(copy.diagnostics.selectedReady);
    } catch (error) {
      const mapped = normalizeCommandError(error, {
        code: "engine_failed",
        title: copy.diagnostics.checkFailedTitle,
        message: copy.diagnostics.checkFailedMessage,
      });
      setEngineError(mapped);
      setLiveMessage(mapped.title);
    } finally {
      setEngineBusy(false);
    }
  }, [demoState]);

  const addPaths = useCallback(
    async (paths: string[]) => {
      if (paths.length === 0) return;
      setBusyAdding(true);
      setLiveMessage(copy.live.inspectingFiles(paths.length));
      try {
        const inspected = await inspectFiles(paths);
        setFiles((current) => [...current, ...inspected]);
        setSettingsById((current) => ({
          ...current,
          ...Object.fromEntries(
            inspected.map((file) => [file.id, settingsFor(file, null, preferences)]),
          ),
        }));
        setSelectedIds(new Set(inspected[0] ? [inspected[0].id] : []));
        setLiveMessage(copy.live.filesReady(inspected.length));
      } catch (error) {
        setLiveMessage(error instanceof Error ? error.message : copy.live.inspectFailed);
      } finally {
        setBusyAdding(false);
      }
    },
    [preferences],
  );

  const handleChooseFiles = useCallback(async () => {
    if (demoState) return;
    const paths = await chooseFiles();
    await addPaths(paths);
  }, [addPaths, demoState]);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    document.documentElement.style.colorScheme = theme;
  }, [theme]);

  useEffect(() => savePreferences(preferences), [preferences]);

  useEffect(() => {
    if (demoState || !hasDesktopRuntime()) return;
    let disposed = false;
    let stopExternalFiles: (() => void) | undefined;
    let stopProgress: (() => void) | undefined;
    let stopDrop: (() => void) | undefined;
    let pendingIntake = Promise.resolve();

    const drainPendingFiles = () => {
      pendingIntake = pendingIntake
        .then(async () => {
          const paths = await getPendingFiles();
          if (!disposed && paths.length > 0) await addPaths(paths);
        })
        .catch(() => {
          if (!disposed) setLiveMessage(copy.live.inspectFailed);
        });
    };

    void getCapabilities()
      .then((registry) => {
        if (!disposed) setCapabilities(registry);
      })
      .catch((error: unknown) => {
        if (disposed) return;
        setEngineError(
          normalizeCommandError(error, {
            code: "engine_failed",
            title: copy.diagnostics.checkFailedTitle,
            message: copy.diagnostics.checkFailedMessage,
          }),
        );
      })
      .finally(() => {
        if (!disposed) setEngineBusy(false);
      });
    void getStartupReport().then((report) => {
      if (!disposed) setStartupReport(report);
    });
    void listenForExternalFiles(() => drainPendingFiles())
      .then((unlisten) => {
        if (disposed) unlisten();
        else {
          stopExternalFiles = unlisten;
          drainPendingFiles();
        }
      })
      .catch(() => drainPendingFiles());
    void listenForJobProgress((event) => {
      if (disposed) return;
      setFiles((current) => updateFile(current, event.jobId, event));
      setLiveMessage(
        `${copy.phases[event.phase]}${event.progress === undefined ? "" : ` ${Math.round(event.progress * 100)}%`}`,
      );
    }).then((unlisten) => {
      stopProgress = unlisten;
    });
    void import("@tauri-apps/api/webviewWindow")
      .then(({ getCurrentWebviewWindow }) =>
        getCurrentWebviewWindow().onDragDropEvent((event) => {
          if (event.payload.type === "enter" || event.payload.type === "over") {
            setDragActive(true);
          } else if (event.payload.type === "leave") {
            setDragActive(false);
          } else {
            setDragActive(false);
            void addPaths(event.payload.paths);
          }
        }),
      )
      .then((unlisten) => {
        stopDrop = unlisten;
      });

    return () => {
      disposed = true;
      stopExternalFiles?.();
      stopProgress?.();
      stopDrop?.();
    };
  }, [addPaths, demoState]);

  const readyFiles = files.filter((file) => file.status === "ready");
  const runningFiles = files.filter(
    (file) => file.status === "running" || file.status === "queued",
  );
  const selectedFiles = files.filter((file) => selectedIds.has(file.id));
  const activeFile = selectedFiles[0] ?? files[0];
  const activeSettings = activeFile ? settingsById[activeFile.id] : undefined;
  const activePosterKey =
    activeFile?.kind === "video" && activeSettings?.video.outputFormat === "gif"
      ? `${activeFile.id}:${activeSettings.gif.startSeconds.toFixed(1)}`
      : undefined;
  const activeClipKey =
    activeFile?.kind === "video" && activeSettings?.video.outputFormat === "gif"
      ? `${activeFile.id}:${activeSettings.gif.startSeconds.toFixed(1)}:${activeSettings.gif.endSeconds.toFixed(1)}`
      : undefined;
  const activeStoryboardKey =
    activeFile?.kind === "video" && activeSettings?.video.outputFormat === "gif"
      ? activeFile.id
      : undefined;

  useEffect(() => {
    if (demoState || !hasDesktopRuntime() || activeFile?.kind !== "image") return;
    const jobId = activeFile.id;
    if (imagePreview?.key === jobId) return;
    let disposed = false;
    const timer = window.setTimeout(() => {
      void getImageThumbnail(jobId)
        .then((url) => {
          if (!disposed) setImagePreview({ key: jobId, url });
        })
        .catch(() => {
          // Conversion remains available when a bounded source thumbnail cannot be decoded.
        });
    }, 80);
    return () => {
      disposed = true;
      window.clearTimeout(timer);
    };
  }, [activeFile, demoState, imagePreview]);

  useEffect(() => {
    if (
      demoState ||
      !hasDesktopRuntime() ||
      activeFile?.kind !== "video" ||
      activeSettings?.video.outputFormat !== "gif"
    ) {
      return;
    }
    const jobId = activeFile.id;
    const atSeconds = activeSettings.gif.startSeconds;
    const key = `${jobId}:${atSeconds.toFixed(1)}`;
    if (posterPreview?.key === key) return;
    let disposed = false;
    const timer = window.setTimeout(() => {
      void getPosterFrame(jobId, atSeconds)
        .then((url) => {
          if (!disposed) setPosterPreview({ key, url });
        })
        .catch(() => {
          // The workbench keeps its calm fallback artwork if a poster cannot be decoded.
        });
    }, 180);
    return () => {
      disposed = true;
      window.clearTimeout(timer);
    };
  }, [activeFile, activeSettings, demoState, posterPreview]);

  useEffect(() => {
    if (demoState || !hasDesktopRuntime() || !activeStoryboardKey) return;
    const jobId = activeStoryboardKey;
    if (storyboardPreview?.key === jobId) return;
    let disposed = false;
    setStoryboardLoadingId(jobId);
    const timer = window.setTimeout(() => {
      void getVideoStoryboard(jobId)
        .then((urls) => {
          if (!disposed) setStoryboardPreview({ key: jobId, urls });
        })
        .catch(() => {
          // The range remains fully operable when bounded moment frames cannot be decoded.
        })
        .finally(() => {
          if (!disposed) {
            setStoryboardLoadingId((current) => (current === jobId ? undefined : current));
          }
        });
    }, 120);
    return () => {
      disposed = true;
      window.clearTimeout(timer);
      setStoryboardLoadingId((current) => (current === jobId ? undefined : current));
    };
  }, [activeStoryboardKey, demoState, storyboardPreview]);

  useEffect(() => {
    if (
      demoState ||
      !hasDesktopRuntime() ||
      activeFile?.status !== "succeeded" ||
      !activeFile.output ||
      outputPreview?.key === activeFile.id
    ) {
      return;
    }
    const jobId = activeFile.id;
    let disposed = false;
    void getOutputPreview(jobId)
      .then((url) => {
        if (!disposed) setOutputPreview({ key: jobId, url });
      })
      .catch(() => {
        // Every completed output remains revealable when its bounded preview is unavailable.
      });
    return () => {
      disposed = true;
    };
  }, [activeFile, demoState, outputPreview]);

  const patchSelectedSettings = useCallback(
    (patcher: (settings: ConversionSettings, file: MediaFile) => ConversionSettings) => {
      setSettingsById((current) => {
        const next = { ...current };
        for (const file of files) {
          if (!selectedIds.has(file.id)) continue;
          const settings = current[file.id];
          if (settings) next[file.id] = patcher(structuredClone(settings), file);
        }
        return next;
      });
    },
    [files, selectedIds],
  );

  const removeFile = useCallback((id: string) => {
    void forgetSource(id);
    setFiles((current) => current.filter((file) => file.id !== id));
    setSelectedIds((current) => {
      const next = new Set(current);
      next.delete(id);
      return next;
    });
    setSettingsById((current) => {
      return Object.fromEntries(Object.entries(current).filter(([key]) => key !== id));
    });
    setImagePreview((current) => (current?.key === id ? undefined : current));
    setPosterPreview((current) => (current?.key.startsWith(`${id}:`) ? undefined : current));
    setClipPreview((current) => (current?.key.startsWith(`${id}:`) ? undefined : current));
    setOutputPreview((current) => (current?.key === id ? undefined : current));
    window.setTimeout(() => {
      const nextRow = document.querySelector<HTMLButtonElement>(".file-row-main");
      (nextRow ?? chooseButtonRef.current)?.focus();
    }, 0);
  }, []);

  const clearQueue = useCallback(() => {
    if (runningFiles.length > 0) return;
    void clearSession();
    setFiles([]);
    setSelectedIds(new Set());
    setSettingsById({});
    setImagePreview(undefined);
    setPosterPreview(undefined);
    setClipPreview(undefined);
    setOutputPreview(undefined);
    setLiveMessage(copy.live.queueCleared);
    window.setTimeout(() => chooseButtonRef.current?.focus(), 0);
  }, [runningFiles.length]);

  const startConversions = useCallback(async () => {
    const targets = readyFiles;
    if (targets.length === 0 || !capabilities.engine.available) return;
    setLiveMessage(copy.live.startingConversions(targets.length));
    setFiles((current) =>
      current.map((file) =>
        targets.some((target) => target.id === file.id)
          ? { ...file, status: "queued", phase: "Waiting", error: undefined }
          : file,
      ),
    );
    if (demoState) return;
    await Promise.all(
      targets.map(async (file) => {
        const settings = settingsById[file.id];
        if (!settings) return;
        try {
          await startJob(file, settings);
        } catch (error) {
          const mapped = normalizeCommandError(error, {
            code: "engine_failed",
            title: copy.live.conversionStartTitle,
            message: copy.live.conversionStartMessage,
          });
          setFiles((current) =>
            updateFile(current, file.id, {
              status: "failed",
              phase: "Needs attention",
              error: mapped,
            }),
          );
        }
      }),
    );
  }, [capabilities.engine.available, demoState, readyFiles, settingsById]);

  const cancelAll = useCallback(async () => {
    await Promise.all(
      runningFiles.map(async (file) => {
        if (demoState) {
          setFiles((current) =>
            updateFile(current, file.id, { status: "canceled", phase: "Canceled" }),
          );
        } else {
          await cancelJob(file.id);
        }
      }),
    );
  }, [demoState, runningFiles]);

  const cancelOne = useCallback(
    async (id: string) => {
      if (demoState) {
        setFiles((current) =>
          updateFile(current, id, {
            status: "canceled",
            phase: "Canceled",
            progress: undefined,
          }),
        );
      } else {
        await cancelJob(id);
      }
    },
    [demoState],
  );

  const previewGifRange = useCallback(
    async (file: MediaFile, settings: ConversionSettings["gif"]) => {
      if (demoState || !hasDesktopRuntime()) {
        setLiveMessage(copy.live.desktopPreview);
        return;
      }
      const key = `${file.id}:${settings.startSeconds.toFixed(1)}:${settings.endSeconds.toFixed(1)}`;
      setPreviewLoadingId(file.id);
      setLiveMessage(copy.live.preparingPreview);
      try {
        const url = await getPreviewClip(file.id, settings.startSeconds, settings.endSeconds);
        setClipPreview({ key, url });
        setLiveMessage(copy.live.playingPreview);
      } catch (error) {
        const mapped = normalizeCommandError(error, {
          code: "engine_failed",
          title: copy.live.previewUnavailable,
          message: copy.live.previewStillConvertible,
        });
        setLiveMessage(`${mapped.title}. ${mapped.message}`);
      } finally {
        setPreviewLoadingId(undefined);
      }
    },
    [demoState],
  );

  const retryFile = useCallback(
    async (id: string) => {
      const file = files.find((candidate) => candidate.id === id);
      if (!file) return;
      if (file.kind !== "unsupported" || demoState) {
        setFiles((current) =>
          updateFile(current, id, {
            status: "ready",
            phase: "Waiting",
            error: undefined,
          }),
        );
        setLiveMessage(copy.live.retryReady(file.name));
        return;
      }

      setFiles((current) =>
        updateFile(current, id, { status: "probing", phase: "Inspecting", error: undefined }),
      );
      setLiveMessage(copy.live.inspectingAgain(file.name));
      try {
        const inspected = await retryInspection(id);
        setFiles((current) =>
          current.map((candidate) => (candidate.id === id ? inspected : candidate)),
        );
        if (inspected.status === "ready") {
          setSettingsById((current) => ({
            ...current,
            [id]: settingsFor(inspected, null, preferences),
          }));
          setLiveMessage(copy.live.fileReady(inspected.name));
        } else {
          setLiveMessage(copy.live.stillNeedsAttention(inspected.name));
        }
      } catch (error) {
        const mapped = normalizeCommandError(error, {
          code: "engine_failed",
          title: copy.live.inspectionRestartTitle,
          message: copy.live.inspectionRestartMessage,
        });
        setFiles((current) =>
          updateFile(current, id, {
            status: "failed",
            phase: "Needs attention",
            error: mapped,
          }),
        );
        setLiveMessage(mapped.title);
      }
    },
    [demoState, files, preferences],
  );

  useEffect(() => {
    const onKeyDown = (event: globalThis.KeyboardEvent) => {
      if (diagnosticsOpen) return;
      const target = event.target;
      const editing =
        target instanceof HTMLInputElement ||
        target instanceof HTMLSelectElement ||
        target instanceof HTMLTextAreaElement;
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "o") {
        event.preventDefault();
        void handleChooseFiles();
      }
      if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
        event.preventDefault();
        void startConversions();
      }
      if ((event.ctrlKey || event.metaKey) && event.key === ",") {
        event.preventDefault();
        setSettingsOpen(true);
      }
      if (event.key === "Escape" && settingsOpen) closeSettings();
      if (!editing && (event.key === "Delete" || event.key === "Backspace")) {
        const removable = selectedFiles.filter(
          (file) => !["running", "queued", "probing"].includes(file.status),
        );
        if (removable.length > 0) {
          event.preventDefault();
          for (const file of removable) removeFile(file.id);
        }
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [
    closeSettings,
    diagnosticsOpen,
    handleChooseFiles,
    removeFile,
    selectedFiles,
    settingsOpen,
    startConversions,
  ]);

  const toggleSelected = (id: string) => {
    setSelectedIds((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const selectOnly = (id: string) => setSelectedIds(new Set([id]));

  const onBrowserDrop = (event: DragEvent<HTMLElement>) => {
    event.preventDefault();
    setDragActive(false);
    if (!hasDesktopRuntime()) setLiveMessage(copy.live.desktopDropOnly);
  };

  if (isDirectionStudy) return <DirectionStudy />;

  return (
    <div
      className={`app-shell${dragActive ? " is-dragging" : ""}`}
      onDragEnter={(event) => {
        event.preventDefault();
        setDragActive(true);
      }}
      onDragOver={(event) => event.preventDefault()}
      onDragLeave={(event) => {
        if (event.currentTarget === event.target) setDragActive(false);
      }}
      onDrop={onBrowserDrop}
    >
      <a className="skip-link" href="#main-content">
        {copy.accessibility.skipToWorkspace}
      </a>
      <AppHeader
        theme={theme}
        onThemeChange={() =>
          setTheme((current) => {
            const next = current === "light" ? "dark" : "light";
            setPreferences((value) => ({ ...value, theme: next }));
            return next;
          })
        }
        settingsOpen={settingsOpen}
        onSettings={() => setSettingsOpen((current) => !current)}
        settingsButtonRef={settingsButtonRef}
      />

      {settingsOpen ? (
        <PreferencesPanel
          collisionPolicy={preferences.collisionPolicy}
          onCollisionPolicy={(collisionPolicy) => {
            setPreferences((value) => ({ ...value, collisionPolicy }));
            patchSelectedSettings((settings) => ({ ...settings, collisionPolicy }));
          }}
          startupReport={startupReport}
          onClose={closeSettings}
        />
      ) : null}

      <main id="main-content" className={files.length === 0 ? "empty-main" : "workspace-main"}>
        {files.length === 0 ? (
          <EmptyState
            engineAvailable={
              capabilities.engine.available || (!demoState && !hasDesktopRuntime())
            }
            engineChecking={engineBusy}
            engineIsBuiltIn={capabilities.engine.source === "native"}
            busy={busyAdding}
            onChoose={() => void handleChooseFiles()}
            onDiagnostics={openDiagnostics}
            chooseButtonRef={chooseButtonRef}
          />
        ) : (
          <>
            <QueuePanel
              files={files}
              settingsById={settingsById}
              selectedIds={selectedIds}
              onToggleSelected={toggleSelected}
              onSelectOnly={selectOnly}
              onRemove={removeFile}
              onChoose={() => void handleChooseFiles()}
              onClear={clearQueue}
              onReveal={(id) => void revealOutput(id)}
              onRetry={(id) => void retryFile(id)}
              onCancel={(id) => void cancelOne(id)}
            />
            <Inspector
              file={activeFile}
              selectedFiles={selectedFiles}
              selectedCount={selectedFiles.length}
              settings={activeSettings}
              capabilities={capabilities}
              onSettings={patchSelectedSettings}
              onReveal={(id) => void revealOutput(id)}
              imagePreview={
                activeFile && imagePreview?.key === activeFile.id ? imagePreview.url : undefined
              }
              posterPreview={
                activePosterKey && posterPreview?.key === activePosterKey
                  ? posterPreview.url
                  : undefined
              }
              storyboardFrames={
                activeStoryboardKey && storyboardPreview?.key === activeStoryboardKey
                  ? storyboardPreview.urls
                  : undefined
              }
              storyboardLoading={activeFile?.id === storyboardLoadingId}
              storyboardDemo={Boolean(demoState)}
              clipPreview={
                activeClipKey && clipPreview?.key === activeClipKey
                  ? clipPreview.url
                  : undefined
              }
              outputPreview={
                activeFile && outputPreview?.key === activeFile.id
                  ? outputPreview.url
                  : undefined
              }
              previewLoading={activeFile?.id === previewLoadingId}
              onPreview={(file, gif) => void previewGifRange(file, gif)}
              onRetry={(id) => void retryFile(id)}
            />
          </>
        )}
      </main>

      {files.length > 0 ? (
        <ActionBar
          files={files}
          engine={capabilities.engine}
          onConvert={() => void startConversions()}
          onCancelAll={() => void cancelAll()}
          onDiagnostics={openDiagnostics}
        />
      ) : null}

      {diagnosticsOpen ? (
        <EngineDiagnostics
          capabilities={capabilities}
          busy={engineBusy}
          error={engineError}
          onRefresh={() => void handleRefreshEngine()}
          onChoose={() => void handleChooseEngine()}
          onClose={closeDiagnostics}
        />
      ) : null}

      <div className="sr-only" aria-live="polite" aria-atomic="true">
        {liveMessage}
      </div>
      {dragActive ? (
        <div className="drop-overlay" aria-hidden="true">
          <div className="drop-overlay-card">
            <Plus size={24} />
            <span>{copy.dropFiles}</span>
          </div>
        </div>
      ) : null}
    </div>
  );
}

interface AppHeaderProps {
  theme: Theme;
  onThemeChange: () => void;
  settingsOpen: boolean;
  onSettings: () => void;
  settingsButtonRef: RefObject<HTMLButtonElement | null>;
}

function AppHeader({
  theme,
  onThemeChange,
  settingsOpen,
  onSettings,
  settingsButtonRef,
}: AppHeaderProps) {
  return (
    <header className="app-header">
      <div className="brand-lockup" aria-label={copy.accessibility.appName}>
        <img className="brand-mark" src={morfloMarkUrl} alt="" />
        <span className="wordmark">{copy.wordmark}</span>
        <span className="version-mark">0.1</span>
      </div>
      <div className="privacy-note">
        <ShieldCheck size={15} aria-hidden="true" />
        <span>{copy.privacy}</span>
      </div>
      <div className="header-actions">
        <button
          className="icon-button"
          type="button"
          onClick={onThemeChange}
          aria-label={copy.accessibility.useTheme(theme === "light" ? "dark" : "light")}
          title={copy.accessibility.useTheme(theme === "light" ? "dark" : "light")}
        >
          {theme === "light" ? <Moon size={18} /> : <Sun size={18} />}
        </button>
        <button
          ref={settingsButtonRef}
          className={`icon-button${settingsOpen ? " is-active" : ""}`}
          type="button"
          onClick={onSettings}
          aria-label={copy.accessibility.openSettings}
          aria-expanded={settingsOpen}
          title={copy.accessibility.settingsShortcut}
        >
          <Settings2 size={18} />
        </button>
      </div>
    </header>
  );
}

interface EmptyStateProps {
  engineAvailable: boolean;
  engineChecking: boolean;
  engineIsBuiltIn: boolean;
  busy: boolean;
  onChoose: () => void;
  onDiagnostics: (trigger: HTMLButtonElement) => void;
  chooseButtonRef: RefObject<HTMLButtonElement | null>;
}

function EmptyState({
  engineAvailable,
  engineChecking,
  engineIsBuiltIn,
  busy,
  onChoose,
  onDiagnostics,
  chooseButtonRef,
}: EmptyStateProps) {
  return (
    <section className="empty-state" aria-labelledby="empty-title">
      <div className="empty-copy">
        <p className="eyebrow">{copy.empty.eyebrow}</p>
        <h1 id="empty-title">{copy.emptyTitle}</h1>
        <p>{copy.emptyBody}</p>
      </div>
      <div className="drop-landing" data-testid="drop-landing">
        <div className="landing-orbit" aria-hidden="true">
          <span className="orbit-tile tile-image">
            <ImageIcon size={20} />
          </span>
          <span className="orbit-path" />
          <span className="orbit-core">
            <img className="orbit-mark" src={morfloMarkUrl} alt="" />
          </span>
          <span className="orbit-path path-out" />
          <span className="orbit-tile tile-output">
            <Check size={20} />
          </span>
        </div>
        <div className="landing-action">
          <strong>{copy.dropFiles}</strong>
          <span>{engineIsBuiltIn ? copy.empty.builtInFormats : copy.empty.formats}</span>
          <button
            ref={chooseButtonRef}
            className="primary-button"
            type="button"
            onClick={onChoose}
            disabled={busy}
          >
            {busy ? <CircleEllipsis className="spin" size={17} /> : <Plus size={17} />}
            {busy ? copy.live.inspecting : copy.chooseFiles}
          </button>
        </div>
      </div>
      {/* Only advertise journeys the active engine can actually finish. On the
          built-in engine the video and WebP examples would promise work that
          cannot happen, which is exactly what the format list must never do. */}
      <div className="journey-strip" aria-label={copy.accessibility.commonConversions}>
        <Journey from={copy.formats.png} to={copy.formats.jpeg} />
        {engineIsBuiltIn ? (
          <>
            <Journey from={copy.formats.webp} to={copy.formats.png} />
            <Journey from={copy.empty.icon} to={copy.formats.ico} />
          </>
        ) : (
          <>
            <Journey from={copy.empty.photos} to={copy.formats.webp} />
            <Journey from={copy.formats.mov} to={copy.formats.mp4} />
            <Journey from={copy.empty.video} to={copy.formats.gif} />
          </>
        )}
      </div>
      {engineChecking ? (
        <div className="inline-alert info engine-alert" role="status">
          <CircleEllipsis className="spin" size={17} />
          <div>
            <strong>{copy.diagnostics.checking}</strong>
            <span>{copy.diagnostics.checkingBody}</span>
          </div>
        </div>
      ) : !engineAvailable ? (
        <div className="inline-alert danger" role="alert">
          <AlertCircle size={17} />
          <div>
            <strong>{copy.engineMissing}</strong>
            <span>{copy.empty.diagnostics}</span>
          </div>
          <button
            className="secondary-button compact alert-action"
            type="button"
            onClick={(event) => onDiagnostics(event.currentTarget)}
          >
            {copy.action.diagnostics}
          </button>
        </div>
      ) : engineIsBuiltIn ? (
        <div className="inline-alert info engine-alert" role="status">
          <Info size={17} />
          <div>
            <strong>{copy.diagnostics.nativeTitle}</strong>
            <span>{copy.diagnostics.nativeBody}</span>
          </div>
          <button
            className="secondary-button compact alert-action"
            type="button"
            onClick={(event) => onDiagnostics(event.currentTarget)}
          >
            {copy.action.diagnostics}
          </button>
        </div>
      ) : null}
    </section>
  );
}

function Journey({ from, to }: { from: string; to: string }) {
  return (
    <span className="journey">
      <span>{from}</span>
      <span aria-hidden="true">→</span>
      <strong>{to}</strong>
    </span>
  );
}

interface QueuePanelProps {
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
}

function QueuePanel({
  files,
  settingsById,
  selectedIds,
  onToggleSelected,
  onSelectOnly,
  onRemove,
  onChoose,
  onClear,
  onReveal,
  onRetry,
  onCancel,
}: QueuePanelProps) {
  const running = files.some((file) => ["running", "queued", "probing"].includes(file.status));
  return (
    <section className="queue-panel" aria-labelledby="queue-title">
      <div className="section-heading">
        <div>
          <p className="eyebrow">{copy.queue.eyebrow}</p>
          <h1 id="queue-title">{copy.conversionQueue}</h1>
        </div>
        <div className="section-tools">
          <button className="secondary-button compact" type="button" onClick={onChoose}>
            <Plus size={16} />
            {copy.addFiles}
          </button>
          <button
            className="text-button"
            type="button"
            onClick={onClear}
            disabled={running}
            title={running ? copy.accessibility.cancelBeforeClear : undefined}
          >
            {copy.clearQueue}
          </button>
        </div>
      </div>
      <div className="queue-summary">
        <span>{copy.queue.count(files.length)}</span>
        <span className="summary-dot" aria-hidden="true" />
        <span>{copy.queue.selected(selectedIds.size)}</span>
        <button
          className="text-button select-all"
          type="button"
          onClick={() => {
            for (const file of files) if (!selectedIds.has(file.id)) onToggleSelected(file.id);
          }}
        >
          {copy.selectAll}
        </button>
      </div>
      <div className="file-list" role="list" aria-label={copy.accessibility.filesToConvert}>
        {files.map((file) => (
          <QueueRow
            key={file.id}
            file={file}
            settings={settingsById[file.id]}
            selected={selectedIds.has(file.id)}
            onToggleSelected={() => onToggleSelected(file.id)}
            onSelect={() => onSelectOnly(file.id)}
            onRemove={() => onRemove(file.id)}
            onReveal={() => onReveal(file.id)}
            onRetry={() => onRetry(file.id)}
            onCancel={() => onCancel(file.id)}
          />
        ))}
      </div>
    </section>
  );
}

interface QueueRowProps {
  file: MediaFile;
  settings?: ConversionSettings | undefined;
  selected: boolean;
  onToggleSelected: () => void;
  onSelect: () => void;
  onRemove: () => void;
  onReveal: () => void;
  onRetry: () => void;
  onCancel: () => void;
}

function QueueRow({
  file,
  settings,
  selected,
  onToggleSelected,
  onSelect,
  onRemove,
  onReveal,
  onRetry,
  onCancel,
}: QueueRowProps) {
  const output =
    file.output?.format ??
    (file.kind === "image" ? settings?.image.outputFormat : settings?.video.outputFormat);
  const outputName = output ? outputLabels[output] : "—";
  const isTerminal =
    file.status === "succeeded" || file.status === "failed" || file.status === "canceled";
  const isActive =
    file.status === "running" || file.status === "queued" || file.status === "probing";
  return (
    <article
      className={`file-row status-${file.status}${selected ? " is-selected" : ""}`}
      role="listitem"
      aria-current={selected ? "true" : undefined}
    >
      <label className="select-check" title={copy.selectFile}>
        <input
          type="checkbox"
          checked={selected}
          onChange={onToggleSelected}
          aria-label={copy.accessibility.select(file.name)}
        />
        <span>
          <Check size={12} />
        </span>
      </label>
      <button className="file-row-main" type="button" onClick={onSelect}>
        <span className={`file-kind kind-${file.kind}`} aria-hidden="true">
          {file.kind === "video" ? <FileVideo size={21} /> : <FileImage size={21} />}
        </span>
        <span className="file-identity">
          <strong title={file.name}>{file.name}</strong>
          <span>
            {extensionLabel(file.extension)}
            {file.width && file.height ? ` · ${formatDimensions(file.width, file.height)}` : ""}
            {file.durationSeconds ? ` · ${formatDuration(file.durationSeconds)}` : ""}
            {` · ${formatBytes(file.sizeBytes)}`}
          </span>
        </span>
        <span
          className="conversion-arrow"
          aria-label={copy.accessibility.convertTo(outputName)}
        >
          →
        </span>
        <span className="output-chip">{outputName}</span>
      </button>
      <div className="row-state">
        <StatusGlyph status={file.status} />
        <span>{copy.phases[file.phase]}</span>
        {file.status === "succeeded" && file.output ? (
          <span className="result-size">
            {copy.queue.outputSize(formatBytes(file.output.sizeBytes))}
          </span>
        ) : null}
        {file.progress !== undefined && file.status === "running" ? (
          <span className="progress-number">{Math.round(file.progress * 100)}%</span>
        ) : null}
      </div>
      {file.status === "running" && file.progress !== undefined ? (
        <progress
          className="row-progress"
          max={1}
          value={file.progress}
          aria-label={copy.accessibility.progress(file.name)}
        />
      ) : null}
      <div className="row-actions">
        {file.status === "succeeded" ? (
          <button
            className="icon-button small"
            type="button"
            onClick={onReveal}
            aria-label={copy.accessibility.reveal(file.name)}
            title={copy.reveal}
          >
            <FolderOpen size={16} />
          </button>
        ) : null}
        {file.status === "failed" ? (
          <button
            className="icon-button small"
            type="button"
            onClick={onRetry}
            aria-label={copy.accessibility.retry(file.name)}
            title={copy.tryAgain}
          >
            <RefreshCw size={16} />
          </button>
        ) : null}
        {file.status === "probing" ? null : isActive ? (
          <button
            className="icon-button small"
            type="button"
            onClick={onCancel}
            aria-label={copy.accessibility.cancel(file.name)}
            title={copy.accessibility.cancelConversion}
          >
            <Square size={14} />
          </button>
        ) : (
          <button
            className="icon-button small"
            type="button"
            onClick={onRemove}
            aria-label={copy.accessibility.remove(file.name)}
            title={copy.remove}
          >
            <X size={16} />
          </button>
        )}
        {!isTerminal && !isActive ? <MoreHorizontal size={16} aria-hidden="true" /> : null}
      </div>
    </article>
  );
}

function StatusGlyph({ status }: { status: MediaFile["status"] }) {
  if (status === "succeeded")
    return <CheckCircle2 className="status-icon success" size={16} aria-hidden="true" />;
  if (status === "failed")
    return <AlertCircle className="status-icon danger" size={16} aria-hidden="true" />;
  if (status === "running" || status === "probing")
    return <CircleEllipsis className="status-icon accent spin" size={16} aria-hidden="true" />;
  if (status === "canceled")
    return <Square className="status-icon muted" size={14} aria-hidden="true" />;
  return <Clock3 className="status-icon muted" size={15} aria-hidden="true" />;
}

interface InspectorProps {
  file?: MediaFile | undefined;
  selectedFiles: MediaFile[];
  selectedCount: number;
  settings?: ConversionSettings | undefined;
  capabilities: CapabilityRegistry;
  onSettings: (
    patcher: (settings: ConversionSettings, file: MediaFile) => ConversionSettings,
  ) => void;
  onReveal: (id: string) => void;
  onRetry: (id: string) => void;
  imagePreview?: string | undefined;
  posterPreview?: string | undefined;
  storyboardFrames?: string[] | undefined;
  storyboardLoading: boolean;
  storyboardDemo: boolean;
  clipPreview?: string | undefined;
  outputPreview?: string | undefined;
  previewLoading: boolean;
  onPreview: (file: MediaFile, settings: ConversionSettings["gif"]) => void;
}

function Inspector({
  file,
  selectedFiles,
  selectedCount,
  settings,
  capabilities,
  onSettings,
  onReveal,
  onRetry,
  imagePreview,
  posterPreview,
  storyboardFrames,
  storyboardLoading,
  storyboardDemo,
  clipPreview,
  outputPreview,
  previewLoading,
  onPreview,
}: InspectorProps) {
  if (!file || !settings) {
    return (
      <aside className="inspector empty-inspector">
        <Layers3 size={28} />
        <p>{copy.inspector.empty}</p>
      </aside>
    );
  }
  const headerTitle = selectedCount > 1 ? `${selectedCount} selected files` : file.name;
  const terminalSelection =
    selectedCount > 1 &&
    selectedFiles.every(
      (selected) =>
        selected.status === "succeeded" ||
        selected.status === "failed" ||
        selected.status === "canceled",
    );
  return (
    <aside
      className={`inspector${file.kind === "video" && settings.video.outputFormat === "gif" ? " gif-inspector" : ""}`}
      aria-labelledby="inspector-title"
    >
      <div className="inspector-header">
        <div>
          <p className="eyebrow">
            {terminalSelection
              ? copy.inspector.batchResult
              : selectedCount > 1
                ? copy.inspector.groupSettings
                : copy.inspector.selectedFile}
          </p>
          <h2 id="inspector-title" title={headerTitle}>
            {headerTitle}
          </h2>
        </div>
        <span
          className={`kind-pill${selectedCount > 1 ? " kind-batch" : ` kind-${file.kind}`}`}
        >
          {selectedCount > 1
            ? copy.inspector.batch
            : file.kind === "video"
              ? copy.inspector.video
              : copy.inspector.image}
        </span>
      </div>

      {terminalSelection ? (
        <BatchCompletionPanel files={selectedFiles} />
      ) : file.status === "succeeded" ? (
        <CompletionPanel
          file={file}
          previewUrl={outputPreview}
          destination={settings.destination}
          onReveal={() => onReveal(file.id)}
        />
      ) : file.status === "failed" && file.error ? (
        <ErrorPanel file={file} onRetry={() => onRetry(file.id)} />
      ) : file.kind === "image" ? (
        <ImageInspector
          file={file}
          settings={settings.image}
          capabilities={capabilities}
          previewUrl={imagePreview}
          onSettings={onSettings}
        />
      ) : (
        <VideoInspector
          file={file}
          settings={settings.video}
          gif={settings.gif}
          capabilities={capabilities}
          onSettings={onSettings}
          posterPreview={posterPreview}
          storyboardFrames={storyboardFrames}
          storyboardLoading={storyboardLoading}
          storyboardDemo={storyboardDemo}
          clipPreview={clipPreview}
          previewLoading={previewLoading}
          onPreview={onPreview}
        />
      )}

      {!terminalSelection && file.status !== "succeeded" && file.status !== "failed" ? (
        <DestinationPanel settings={settings} onSettings={onSettings} />
      ) : null}
    </aside>
  );
}

function ImageInspector({
  file,
  settings,
  capabilities,
  previewUrl,
  onSettings,
}: {
  file: MediaFile;
  settings: ImageSettings;
  capabilities: CapabilityRegistry;
  previewUrl?: string | undefined;
  onSettings: InspectorProps["onSettings"];
}) {
  const formats: ImageSettings["outputFormat"][] = ["jpeg", "png", "webp", "avif", "ico"];
  const alphaFlatten = Boolean(file.hasAlpha && settings.outputFormat === "jpeg");
  return (
    <div className="inspector-content">
      {previewUrl ? (
        <div className="image-source-preview">
          <img src={previewUrl} alt={copy.accessibility.imagePreview(file.name)} />
          <span>{copy.image.localPreview}</span>
        </div>
      ) : null}
      <SettingGroup title={copy.image.outputFormat} hint={copy.image.recommended}>
        <div className="format-grid">
          {formats.map((format) => {
            const capability = capabilities.outputs.find((item) => item.format === format);
            const available = capability?.available ?? false;
            const description =
              format === "jpeg"
                ? copy.image.jpegDescription
                : format === "png" && file.hasAlpha
                  ? copy.image.pngAlphaDescription
                  : format === "webp"
                    ? copy.image.webpDescription
                    : format === "ico"
                      ? copy.image.icoDescription
                      : undefined;
            return (
              <button
                key={format}
                className={`format-choice${settings.outputFormat === format ? " is-selected" : ""}`}
                type="button"
                disabled={!available}
                title={!available ? capability?.reason : undefined}
                aria-label={
                  description ? `${outputLabels[format]}. ${description}` : outputLabels[format]
                }
                onClick={() =>
                  onSettings((all, current) => {
                    if (current.kind === "image") all.image.outputFormat = format;
                    return all;
                  })
                }
              >
                <span>{outputLabels[format]}</span>
                {description ? <small>{description}</small> : null}
              </button>
            );
          })}
        </div>
      </SettingGroup>

      {alphaFlatten ? (
        <div className="warning-card" role="note">
          <div className="warning-heading">
            <TriangleAlert size={17} />
            <strong>{copy.image.alphaTitle}</strong>
          </div>
          <p>{copy.image.alphaBody}</p>
          <label className="color-field">
            <span>{copy.image.background}</span>
            <input
              type="color"
              value={settings.background}
              onChange={(event) => {
                const background = event.currentTarget.value.toUpperCase();
                onSettings((all) => {
                  all.image.background = background;
                  return all;
                });
              }}
            />
            <code>{settings.background}</code>
          </label>
        </div>
      ) : null}

      {file.animated ? (
        <div className="warning-card" role="note">
          <div className="warning-heading">
            <Layers3 size={17} />
            <strong>{copy.image.animatedTitle}</strong>
          </div>
          <p>{copy.image.animatedBody}</p>
          <label className="animation-choice">
            <input
              type="checkbox"
              checked={settings.animation === "first"}
              onChange={(event) => {
                const useFirstFrame = event.currentTarget.checked;
                onSettings((all) => {
                  all.image.animation = useFirstFrame ? "first" : "ask";
                  return all;
                });
              }}
            />
            <span>{copy.image.firstFrame}</span>
          </label>
        </div>
      ) : null}

      {settings.outputFormat === "ico" ? (
        <div className="info-card">
          <Aperture size={18} />
          <div>
            <strong>{copy.image.icoTitle}</strong>
            <span>{copy.image.icoBody}</span>
          </div>
        </div>
      ) : null}

      {settings.outputFormat !== "png" && settings.outputFormat !== "ico" ? (
        <QualitySelector
          value={settings.quality}
          onChange={(quality) =>
            onSettings((all) => {
              all.image.quality = quality;
              return all;
            })
          }
        />
      ) : null}

      {settings.outputFormat !== "ico" ? (
        <SettingGroup title={copy.image.dimensions}>
          <label className="select-field">
            <span className="sr-only">{copy.image.resizeMode}</span>
            <select
              value={settings.resizeMode}
              onChange={(event) => {
                const resizeMode = event.currentTarget.value as ResizeMode;
                onSettings((all) => {
                  all.image.resizeMode = resizeMode;
                  if (
                    (resizeMode === "width" ||
                      resizeMode === "contain" ||
                      resizeMode === "cover") &&
                    all.image.width === undefined
                  ) {
                    all.image.width = file.width ?? 1280;
                  }
                  if (
                    (resizeMode === "height" ||
                      resizeMode === "contain" ||
                      resizeMode === "cover") &&
                    all.image.height === undefined
                  ) {
                    all.image.height = file.height ?? 720;
                  }
                  if (resizeMode === "percentage" && all.image.percentage === undefined) {
                    all.image.percentage = 75;
                  }
                  return all;
                });
              }}
            >
              <option value="original">{copy.image.original}</option>
              <option value="width">{copy.image.setWidth}</option>
              <option value="height">{copy.image.setHeight}</option>
              <option value="percentage">{copy.image.percentage}</option>
              <option value="contain">{copy.image.contain}</option>
              <option value="cover">{copy.image.cover}</option>
            </select>
            <ChevronDown size={16} aria-hidden="true" />
          </label>
          {settings.resizeMode !== "original" ? (
            <div className="dimension-fields">
              {settings.resizeMode === "width" ||
              settings.resizeMode === "contain" ||
              settings.resizeMode === "cover" ? (
                <label>
                  <span>{copy.image.width}</span>
                  <input
                    type="number"
                    min={1}
                    max={32768}
                    step={1}
                    value={settings.width ?? ""}
                    onChange={(event) => {
                      const width = Number(event.currentTarget.value);
                      onSettings((all) => {
                        all.image.width = Number.isFinite(width) ? width : undefined;
                        return all;
                      });
                    }}
                  />
                  <small>{copy.units.pixels}</small>
                </label>
              ) : null}
              {settings.resizeMode === "height" ||
              settings.resizeMode === "contain" ||
              settings.resizeMode === "cover" ? (
                <label>
                  <span>{copy.image.height}</span>
                  <input
                    type="number"
                    min={1}
                    max={32768}
                    step={1}
                    value={settings.height ?? ""}
                    onChange={(event) => {
                      const height = Number(event.currentTarget.value);
                      onSettings((all) => {
                        all.image.height = Number.isFinite(height) ? height : undefined;
                        return all;
                      });
                    }}
                  />
                  <small>{copy.units.pixels}</small>
                </label>
              ) : null}
              {settings.resizeMode === "percentage" ? (
                <label>
                  <span>{copy.image.scale}</span>
                  <input
                    type="number"
                    min={1}
                    max={400}
                    step={1}
                    value={settings.percentage ?? ""}
                    onChange={(event) => {
                      const percentage = Number(event.currentTarget.value);
                      onSettings((all) => {
                        all.image.percentage = Number.isFinite(percentage)
                          ? percentage
                          : undefined;
                        return all;
                      });
                    }}
                  />
                  <small>{copy.units.percent}</small>
                </label>
              ) : null}
            </div>
          ) : null}
          <p className="field-note">{copy.image.aspectNote}</p>
        </SettingGroup>
      ) : null}

      {settings.outputFormat !== "ico" ? (
        <MetadataSelector
          value={settings.metadata}
          onChange={(metadata) =>
            onSettings((all) => {
              all.image.metadata = metadata;
              return all;
            })
          }
        />
      ) : null}

      <Warnings
        warnings={file.warnings.filter((warning) => warning.code !== "alpha-flatten")}
      />
      <TechnicalDetails file={file} output={outputLabels[settings.outputFormat]} />
    </div>
  );
}

function VideoInspector({
  file,
  settings,
  gif,
  capabilities,
  onSettings,
  posterPreview,
  storyboardFrames,
  storyboardLoading,
  storyboardDemo,
  clipPreview,
  previewLoading,
  onPreview,
}: {
  file: MediaFile;
  settings: VideoSettings;
  gif: ConversionSettings["gif"];
  capabilities: CapabilityRegistry;
  onSettings: InspectorProps["onSettings"];
  posterPreview?: string | undefined;
  storyboardFrames?: string[] | undefined;
  storyboardLoading: boolean;
  storyboardDemo: boolean;
  clipPreview?: string | undefined;
  previewLoading: boolean;
  onPreview: InspectorProps["onPreview"];
}) {
  const formats: VideoSettings["outputFormat"][] = ["mp4", "webm", "gif"];
  const compactOutcomes = settings.outputFormat === "gif";
  return (
    <div className="inspector-content">
      <SettingGroup title={copy.video.result} hint={copy.video.outcomeHint}>
        <div className={`outcome-list${compactOutcomes ? " is-compact" : ""}`}>
          {formats.map((format) => {
            const capability = capabilities.outputs.find((item) => item.format === format);
            const available = capability?.available ?? false;
            const description =
              format === "mp4"
                ? copy.video.mp4Description
                : format === "webm"
                  ? copy.video.webmDescription
                  : copy.video.gifDescription;
            return (
              <button
                key={format}
                className={`outcome-choice${settings.outputFormat === format ? " is-selected" : ""}`}
                type="button"
                disabled={!available}
                title={!available ? capability?.reason : undefined}
                onClick={() =>
                  onSettings((all, current) => {
                    if (current.kind === "video") all.video.outputFormat = format;
                    return all;
                  })
                }
              >
                <span className="outcome-icon">
                  {format === "gif" ? <Aperture size={18} /> : <FileVideo size={18} />}
                </span>
                <span className="outcome-copy">
                  <strong>
                    {compactOutcomes
                      ? format === "gif"
                        ? copy.formats.gif
                        : format.toUpperCase()
                      : format === "mp4"
                        ? copy.video.universalMp4
                        : format === "webm"
                          ? copy.video.webFriendlyWebm
                          : copy.video.animatedGif}
                  </strong>
                  <small>{description}</small>
                </span>
                <span className="radio-mark">
                  <Check size={12} />
                </span>
              </button>
            );
          })}
        </div>
      </SettingGroup>

      {settings.outputFormat === "gif" ? (
        <GifInspector
          file={file}
          settings={gif}
          onSettings={onSettings}
          posterPreview={posterPreview}
          storyboardFrames={storyboardFrames}
          storyboardLoading={storyboardLoading}
          storyboardDemo={storyboardDemo}
          clipPreview={clipPreview}
          previewLoading={previewLoading}
          onPreview={onPreview}
        />
      ) : (
        <>
          <SettingGroup title={copy.video.resolution}>
            <SegmentedControl
              label={copy.video.outputResolution}
              value={settings.resolution}
              options={[
                ["original", copy.video.original],
                ["1080p", copy.video.hd1080],
                ["720p", copy.video.hd720],
              ]}
              onChange={(resolution) =>
                onSettings((all) => {
                  all.video.resolution = resolution as Resolution;
                  return all;
                })
              }
            />
            <p className="field-note">{copy.video.noUpscale}</p>
          </SettingGroup>
          <QualitySelector
            value={settings.quality}
            onChange={(quality) =>
              onSettings((all) => {
                all.video.quality = quality;
                return all;
              })
            }
          />
          <MetadataSelector
            value={settings.metadata}
            onChange={(metadata) =>
              onSettings((all) => {
                all.video.metadata = metadata;
                return all;
              })
            }
          />
        </>
      )}
      <Warnings warnings={file.warnings} />
      <TechnicalDetails file={file} output={outputLabels[settings.outputFormat]} />
    </div>
  );
}

function GifInspector({
  file,
  settings,
  onSettings,
  posterPreview,
  storyboardFrames,
  storyboardLoading,
  storyboardDemo,
  clipPreview,
  previewLoading,
  onPreview,
}: {
  file: MediaFile;
  settings: ConversionSettings["gif"];
  onSettings: InspectorProps["onSettings"];
  posterPreview?: string | undefined;
  storyboardFrames?: string[] | undefined;
  storyboardLoading: boolean;
  storyboardDemo: boolean;
  clipPreview?: string | undefined;
  previewLoading: boolean;
  onPreview: InspectorProps["onPreview"];
}) {
  const duration = Math.max(file.durationSeconds ?? 10, 0.1);
  const minimumGap = Math.min(0.5, duration);
  const selectedDuration = Math.max(0, settings.endSeconds - settings.startSeconds);
  const complexity = selectedDuration * settings.fps * (settings.width / 540) ** 2;
  const sizeSignal =
    complexity > 300
      ? copy.gif.veryLarge
      : complexity > 120
        ? copy.gif.likelyLarge
        : copy.gif.comfortable;
  const setGif = (patch: Partial<ConversionSettings["gif"]>) =>
    onSettings((all) => {
      all.gif = { ...all.gif, ...patch };
      return all;
    });
  const startRangeRef = useRef<HTMLInputElement>(null);
  const endRangeRef = useRef<HTMLInputElement>(null);
  const startPercent = Math.min(100, Math.max(0, (settings.startSeconds / duration) * 100));
  const endPercent = Math.min(100, Math.max(0, (settings.endSeconds / duration) * 100));
  const handleMomentPointerDown = (event: ReactPointerEvent<HTMLDivElement>) => {
    if ((event.target as HTMLElement).tagName === "INPUT") return;
    const bounds = event.currentTarget.getBoundingClientRect();
    if (bounds.width <= 0) return;
    const ratio = Math.min(1, Math.max(0, (event.clientX - bounds.left) / bounds.width));
    const nextTime = ratio * duration;
    if (
      Math.abs(nextTime - settings.startSeconds) <= Math.abs(nextTime - settings.endSeconds)
    ) {
      setGif({ startSeconds: Math.min(nextTime, settings.endSeconds - minimumGap) });
      startRangeRef.current?.focus();
    } else {
      setGif({ endSeconds: Math.max(nextTime, settings.startSeconds + minimumGap) });
      endRangeRef.current?.focus();
    }
  };
  const posterStyle = {
    "--poster-accent": `${Math.round(startPercent)}%`,
    ...(posterPreview ? { backgroundImage: `url(${posterPreview})` } : {}),
  } as CSSProperties;
  return (
    <section className="gif-workbench" aria-labelledby="gif-heading">
      <div className="gif-heading-row">
        <div>
          <p className="eyebrow">{copy.gif.eyebrow}</p>
          <h3 id="gif-heading">{copy.gif.title}</h3>
        </div>
        <span>{copy.gif.selectedSeconds(selectedDuration)}</span>
      </div>
      <div
        className={`video-poster${posterPreview ? " has-poster" : ""}${clipPreview ? " is-playing" : ""}`}
        style={posterStyle}
        aria-label={copy.accessibility.posterPreview}
      >
        {clipPreview ? (
          <video
            src={clipPreview}
            controls
            autoPlay
            muted
            loop
            aria-label={copy.accessibility.selectedPreview}
          />
        ) : (
          <>
            {!posterPreview ? (
              <>
                <div className="poster-sky" />
                <div className="poster-horizon" />
              </>
            ) : null}
            <button
              type="button"
              className="poster-play"
              aria-label={
                previewLoading
                  ? copy.accessibility.preparingSelectedPreview
                  : copy.accessibility.playSelectedPreview
              }
              disabled={previewLoading}
              onClick={() => onPreview(file, settings)}
            >
              {previewLoading ? (
                <CircleEllipsis className="spin" size={18} />
              ) : (
                <Play size={18} fill="currentColor" />
              )}
            </button>
          </>
        )}
        <span className="poster-time">
          {formatDuration(settings.startSeconds)} — {formatDuration(settings.endSeconds)}
        </span>
      </div>
      <div
        className="moment-control"
        role="group"
        aria-label={copy.accessibility.gifRange}
        aria-busy={storyboardLoading}
      >
        <div
          className={`moment-strip${storyboardFrames?.length ? " has-frames" : ""}${storyboardDemo ? " is-demo" : ""}${storyboardLoading ? " is-loading" : ""}`}
          onPointerDown={handleMomentPointerDown}
        >
          <div
            className="moment-frames"
            style={
              {
                "--moment-columns": storyboardFrames?.length ?? DEMO_MOMENT_FRAME_COUNT,
              } as CSSProperties
            }
            aria-hidden="true"
          >
            {storyboardFrames?.length
              ? storyboardFrames.map((url, index) => (
                  <img key={index} src={url} alt="" draggable={false} />
                ))
              : Array.from({ length: DEMO_MOMENT_FRAME_COUNT }, (_, index) => (
                  <span
                    key={index}
                    style={
                      {
                        "--moment-x": `${18 + index * 10}%`,
                      } as CSSProperties
                    }
                  />
                ))}
          </div>
          {!storyboardDemo && !storyboardFrames?.length ? (
            <span className="moment-status" aria-hidden="true">
              {storyboardLoading ? copy.gif.preparingMoments : copy.gif.momentsUnavailable}
            </span>
          ) : null}
          <span
            className="moment-shade is-before"
            style={{ width: `${startPercent}%` }}
            aria-hidden="true"
          />
          <span
            className="moment-shade is-after"
            style={{ left: `${endPercent}%` }}
            aria-hidden="true"
          />
          <span
            className="moment-selection"
            style={{
              left: `${startPercent}%`,
              width: `${Math.max(0, endPercent - startPercent)}%`,
            }}
            aria-hidden="true"
          />
          <input
            ref={startRangeRef}
            id={`gif-start-${file.id}`}
            className="moment-range-input is-start"
            type="range"
            min={0}
            max={Math.max(0, settings.endSeconds - minimumGap)}
            step={0.1}
            value={settings.startSeconds}
            aria-label={copy.accessibility.gifStart}
            aria-valuetext={copy.accessibility.gifTime(formatMomentTime(settings.startSeconds))}
            onChange={(event) =>
              setGif({
                startSeconds: Math.min(
                  Number(event.currentTarget.value),
                  settings.endSeconds - minimumGap,
                ),
              })
            }
          />
          <input
            ref={endRangeRef}
            id={`gif-end-${file.id}`}
            className="moment-range-input is-end"
            type="range"
            min={Math.min(duration, settings.startSeconds + minimumGap)}
            max={duration}
            step={0.1}
            value={settings.endSeconds}
            aria-label={copy.accessibility.gifEnd}
            aria-valuetext={copy.accessibility.gifTime(formatMomentTime(settings.endSeconds))}
            onChange={(event) =>
              setGif({
                endSeconds: Math.max(
                  Number(event.currentTarget.value),
                  settings.startSeconds + minimumGap,
                ),
              })
            }
          />
        </div>
        <div className="moment-readouts">
          <output htmlFor={`gif-start-${file.id}`}>
            <span>{copy.gif.start}</span>
            <strong>{formatMomentTime(settings.startSeconds)}</strong>
          </output>
          <span className="moment-source">
            {copy.gif.sourceDuration(formatMomentTime(duration))}
          </span>
          <output htmlFor={`gif-end-${file.id}`}>
            <span>{copy.gif.end}</span>
            <strong>{formatMomentTime(settings.endSeconds)}</strong>
          </output>
        </div>
      </div>
      <SettingGroup title={copy.gif.preset}>
        <SegmentedControl
          label={copy.gif.preset}
          value={settings.preset}
          options={[
            ["chat", copy.gif.chat],
            ["web", copy.gif.web],
            ["high", copy.gif.high],
          ]}
          onChange={(value) => {
            const preset = value as GifPreset;
            const values =
              preset === "chat"
                ? { width: 360, fps: 10, quality: "smaller" as Quality }
                : preset === "high"
                  ? { width: 720, fps: 18, quality: "best" as Quality }
                  : { width: 540, fps: 12, quality: "balanced" as Quality };
            setGif({ preset, ...values });
          }}
        />
      </SettingGroup>
      <div
        className={`size-guidance ${sizeSignal === copy.gif.comfortable ? "calm" : "warning"}`}
      >
        {sizeSignal === copy.gif.comfortable ? <Info size={16} /> : <TriangleAlert size={16} />}
        <div>
          <strong>{sizeSignal}</strong>
          <span>
            {sizeSignal === copy.gif.comfortable
              ? copy.gif.comfortableBody
              : copy.gif.largeBody}
          </span>
        </div>
      </div>
      <details className="advanced-details">
        <summary>
          {copy.technicalDetails}
          <ChevronDown size={15} />
        </summary>
        <div className="advanced-grid">
          <label>
            {copy.gif.width}
            <input
              type="number"
              min={160}
              max={1280}
              step={2}
              value={settings.width}
              onChange={(event) => setGif({ width: Number(event.currentTarget.value) })}
            />
          </label>
          <label>
            {copy.gif.frameRate}
            <input
              type="number"
              min={5}
              max={30}
              value={settings.fps}
              onChange={(event) => setGif({ fps: Number(event.currentTarget.value) })}
            />
          </label>
          <label>
            {copy.gif.loop}
            <select
              value={settings.loop}
              onChange={(event) =>
                setGif({ loop: event.currentTarget.value as "forever" | "once" })
              }
            >
              <option value="forever">{copy.gif.forever}</option>
              <option value="once">{copy.gif.once}</option>
            </select>
          </label>
        </div>
      </details>
    </section>
  );
}

function SettingGroup({
  title,
  hint,
  children,
}: {
  title: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <section className="setting-group">
      <div className="setting-title">
        <h3>{title}</h3>
        {hint ? <span>{hint}</span> : null}
      </div>
      {children}
    </section>
  );
}

function QualitySelector({
  value,
  onChange,
}: {
  value: Quality;
  onChange: (value: Quality) => void;
}) {
  return (
    <SettingGroup title={copy.quality.title}>
      <SegmentedControl
        label={copy.quality.label}
        value={value}
        options={[
          ["smaller", copy.quality.smaller],
          ["balanced", copy.quality.balanced],
          ["best", copy.quality.best],
        ]}
        onChange={(next) => onChange(next as Quality)}
      />
    </SettingGroup>
  );
}

function SegmentedControl({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: string;
  options: readonly (readonly [string, string])[];
  onChange: (value: string) => void;
}) {
  return (
    <div className="segmented" role="group" aria-label={label}>
      {options.map(([optionValue, optionLabel]) => (
        <button
          key={optionValue}
          className={value === optionValue ? "is-selected" : ""}
          type="button"
          aria-pressed={value === optionValue}
          onClick={() => onChange(optionValue)}
        >
          {optionLabel}
        </button>
      ))}
    </div>
  );
}

function MetadataSelector({
  value,
  onChange,
}: {
  value: MetadataPolicy;
  onChange: (value: MetadataPolicy) => void;
}) {
  return (
    <SettingGroup title={copy.metadata.title}>
      <div className="metadata-choice">
        <label htmlFor="metadata-remove" aria-label={copy.metadata.remove}>
          <input
            id="metadata-remove"
            type="radio"
            name="metadata"
            checked={value === "remove"}
            onChange={() => onChange("remove")}
          />
          <span>
            <strong>{copy.metadata.remove}</strong>
            <small>{copy.metadata.removeNote}</small>
          </span>
        </label>
        <label htmlFor="metadata-preserve" aria-label={copy.metadata.preserve}>
          <input
            id="metadata-preserve"
            type="radio"
            name="metadata"
            checked={value === "preserve"}
            onChange={() => onChange("preserve")}
          />
          <span>
            <strong>{copy.metadata.preserve}</strong>
            <small>{copy.metadata.preserveNote}</small>
          </span>
        </label>
      </div>
    </SettingGroup>
  );
}

function Warnings({ warnings }: { warnings: MediaFile["warnings"] }) {
  if (warnings.length === 0) return null;
  return (
    <div className="warnings-list">
      {warnings.map((warning) => (
        <div className={`inline-alert ${warning.severity}`} key={warning.code} role="note">
          {warning.severity === "warning" ? <TriangleAlert size={17} /> : <Info size={17} />}
          <div>
            <strong>{warning.title}</strong>
            <span>{warning.message}</span>
          </div>
        </div>
      ))}
    </div>
  );
}

function TechnicalDetails({ file, output }: { file: MediaFile; output: string }) {
  return (
    <details className="technical-details">
      <summary>
        {copy.technicalDetails}
        <ChevronDown size={15} />
      </summary>
      <dl>
        <div>
          <dt>{copy.technical.input}</dt>
          <dd>{extensionLabel(file.extension)}</dd>
        </div>
        <div>
          <dt>{copy.technical.output}</dt>
          <dd>{output}</dd>
        </div>
        {file.videoCodec ? (
          <div>
            <dt>{copy.technical.video}</dt>
            <dd>{file.videoCodec}</dd>
          </div>
        ) : null}
        {file.videoTracks !== undefined ? (
          <div>
            <dt>{copy.technical.videoTracks}</dt>
            <dd>{file.videoTracks}</dd>
          </div>
        ) : null}
        {file.audioTracks !== undefined ? (
          <div>
            <dt>{copy.technical.audioTracks}</dt>
            <dd>{file.audioTracks}</dd>
          </div>
        ) : null}
        {file.subtitleTracks !== undefined ? (
          <div>
            <dt>{copy.technical.subtitles}</dt>
            <dd>{file.subtitleTracks}</dd>
          </div>
        ) : null}
        {file.chapterCount !== undefined ? (
          <div>
            <dt>{copy.technical.chapters}</dt>
            <dd>{file.chapterCount}</dd>
          </div>
        ) : null}
        {(file.attachmentTracks ?? 0) + (file.dataTracks ?? 0) > 0 ? (
          <div>
            <dt>{copy.technical.containerExtras}</dt>
            <dd>{(file.attachmentTracks ?? 0) + (file.dataTracks ?? 0)}</dd>
          </div>
        ) : null}
      </dl>
    </details>
  );
}

function DestinationPanel({
  settings,
  onSettings,
}: {
  settings: ConversionSettings;
  onSettings: InspectorProps["onSettings"];
}) {
  const customName = settings.destinationPath?.split(/[\\/]/u).filter(Boolean).pop();
  return (
    <section className="destination-panel">
      <div className="setting-title">
        <h3>{copy.destination}</h3>
      </div>
      <button
        className="destination-choice"
        type="button"
        onClick={() => {
          void chooseOutputFolder().then((destinationPath) => {
            if (!destinationPath) return;
            onSettings((all) => ({ ...all, destination: "custom", destinationPath }));
          });
        }}
      >
        <FolderOpen size={18} />
        <span>
          <strong>{settings.destination === "custom" ? customName : copy.sameFolder}</strong>
          <small>
            {settings.destination === "custom"
              ? copy.destinationCopy.customFolder
              : copy.collisionSafe}
          </small>
        </span>
        <ChevronDown size={16} />
      </button>
      {settings.destination === "custom" ? (
        <button
          className="text-button destination-reset"
          type="button"
          onClick={() =>
            onSettings((all) => ({
              ...all,
              destination: "same",
              destinationPath: undefined,
            }))
          }
        >
          {copy.destinationCopy.useSource}
        </button>
      ) : null}
      <label className="collision-line">
        <span>{copy.destinationCopy.collisionLabel}</span>
        <select
          value={settings.collisionPolicy}
          onChange={(event) => {
            const collisionPolicy = event.currentTarget.value as CollisionPolicy;
            onSettings((all) => ({ ...all, collisionPolicy }));
          }}
        >
          <option value="suffix">{copy.destinationCopy.suffix}</option>
          <option value="skip">{copy.destinationCopy.skipFile}</option>
          <option value="replace">{copy.destinationCopy.replace}</option>
        </select>
      </label>
      {settings.collisionPolicy === "replace" ? (
        <p className="popover-warning collision-warning" role="note">
          <TriangleAlert size={15} />
          {copy.destinationCopy.replaceWarning}
        </p>
      ) : null}
    </section>
  );
}

function CompletionPanel({
  file,
  previewUrl,
  destination,
  onReveal,
}: {
  file: MediaFile;
  previewUrl?: string | undefined;
  destination: ConversionSettings["destination"];
  onReveal: () => void;
}) {
  const output = file.output;
  if (!output) return null;
  const sizeOutcome = describeSizeOutcome(file.sizeBytes, output.sizeBytes);
  const outputMeasure = formatOutputMeasure(output);
  return (
    <div className="completion-panel">
      <div className="completion-heading">
        <div className="completion-visual">
          <Check size={22} />
        </div>
        <div>
          <p className="eyebrow">{copy.completion.eyebrow}</p>
          <h3>{copy.completion.title(outputLabels[output.format])}</h3>
        </div>
      </div>

      <div
        className={`result-preview${previewUrl ? " has-preview" : ""}${output.hasAlpha ? " has-alpha" : ""}`}
      >
        {previewUrl ? (
          <img src={previewUrl} alt={copy.accessibility.outputPreview(output.name)} />
        ) : (
          <div className="result-preview-fallback" role="status">
            <img src={morfloMarkUrl} alt="" />
            <strong>{outputLabels[output.format]}</strong>
            <span>{copy.completion.previewUnavailable}</span>
          </div>
        )}
        <span className="result-preview-caption">
          <CheckCircle2 size={14} />
          {previewUrl ? copy.completion.outputPreview : copy.completion.outputVerified}
        </span>
      </div>

      <div className="result-flow" aria-label={copy.accessibility.conversionResult}>
        <div className="result-node">
          <span>{copy.completion.original}</span>
          <strong>{extensionLabel(file.extension)}</strong>
          <small>{formatBytes(file.sizeBytes)}</small>
        </div>
        <div className="result-flow-connector" aria-hidden="true">
          <span />
          <Check size={13} />
          <span />
        </div>
        <div className="result-node is-output">
          <span>{copy.completion.output}</span>
          <strong>{outputLabels[output.format]}</strong>
          <small>{formatBytes(output.sizeBytes)}</small>
        </div>
      </div>

      <div className="result-facts">
        <span className={`size-outcome ${sizeOutcome.tone}`}>{sizeOutcome.label}</span>
        {outputMeasure ? <span>{outputMeasure}</span> : null}
      </div>

      <div className="result-file">
        <span>{copy.completion.savedAs}</span>
        <strong title={output.name}>{output.name}</strong>
        <small>
          {destination === "custom"
            ? copy.completion.chosenFolder
            : copy.completion.sourceFolder}
        </small>
      </div>

      <button className="primary-button" type="button" onClick={onReveal}>
        <FolderOpen size={17} />
        {copy.reveal}
      </button>
      <p className="completion-trust">
        <ShieldCheck size={15} />
        {copy.completion.sourceUntouched}
      </p>
    </div>
  );
}

function BatchCompletionPanel({ files }: { files: MediaFile[] }) {
  const succeeded = files.filter((file) => file.status === "succeeded" && file.output);
  const failed = files.filter((file) => file.status === "failed").length;
  const canceled = files.filter((file) => file.status === "canceled").length;
  const inputBytes = succeeded.reduce((total, file) => total + file.sizeBytes, 0);
  const outputBytes = succeeded.reduce(
    (total, file) => total + (file.output?.sizeBytes ?? 0),
    0,
  );
  const sizeOutcome = describeSizeOutcome(inputBytes, outputBytes);
  const formats = Array.from(
    succeeded.reduce((counts, file) => {
      const format = file.output?.format;
      if (format) counts.set(format, (counts.get(format) ?? 0) + 1);
      return counts;
    }, new Map<OutputFormat, number>()),
  )
    .map(([format, count]) => `${count} ${outputLabels[format]}`)
    .join(" · ");

  return (
    <div className="completion-panel batch-completion-panel">
      <div className="completion-heading">
        <div className="completion-visual">
          <Check size={22} />
        </div>
        <div>
          <p className="eyebrow">{copy.completion.batchEyebrow}</p>
          <h3>{copy.completion.batchTitle(succeeded.length, files.length)}</h3>
        </div>
      </div>

      {succeeded.length > 0 ? (
        <>
          <div className="batch-receipt">
            <div>
              <span>{copy.completion.originals}</span>
              <strong>{formatBytes(inputBytes)}</strong>
            </div>
            <div className="batch-flow" aria-hidden="true">
              <span />
              <Check size={15} />
              <span />
            </div>
            <div>
              <span>{copy.completion.outputs}</span>
              <strong>{formatBytes(outputBytes)}</strong>
            </div>
          </div>
          <div className="result-facts batch-facts">
            <span className={`size-outcome ${sizeOutcome.tone}`}>{sizeOutcome.label}</span>
            {formats ? <span>{formats}</span> : null}
          </div>
        </>
      ) : null}

      {failed > 0 || canceled > 0 ? (
        <div className="batch-attention" role="status">
          <AlertCircle size={16} />
          <span>{copy.completion.batchAttention(failed, canceled)}</span>
        </div>
      ) : null}

      <p className="batch-guidance">{copy.completion.batchGuidance}</p>
      <p className="completion-trust">
        <ShieldCheck size={15} />
        {copy.completion.allSourcesUntouched}
      </p>
    </div>
  );
}

function describeSizeOutcome(
  inputBytes: number,
  outputBytes: number,
): { label: string; tone: "smaller" | "similar" | "larger" } {
  if (inputBytes <= 0 || outputBytes <= 0) {
    return { label: copy.completion.measuredLocally, tone: "similar" };
  }
  const ratio = outputBytes / inputBytes;
  if (Math.abs(ratio - 1) < 0.01) {
    return { label: copy.completion.similarSize, tone: "similar" };
  }
  if (ratio < 1) {
    return {
      label: copy.completion.smallerBy(Math.max(1, Math.round((1 - ratio) * 100))),
      tone: "smaller",
    };
  }
  return {
    label: copy.completion.largerBy(Math.max(1, Math.round((ratio - 1) * 100))),
    tone: "larger",
  };
}

function formatOutputMeasure(output: OutputSummary): string | undefined {
  if (output.format === "ico") return copy.completion.multiSizeIcon;
  const facts = [
    output.width && output.height ? formatDimensions(output.width, output.height) : undefined,
    output.durationSeconds !== undefined ? formatDuration(output.durationSeconds) : undefined,
  ].filter((fact): fact is string => Boolean(fact));
  return facts.length > 0 ? facts.join(" · ") : undefined;
}

function ErrorPanel({ file, onRetry }: { file: MediaFile; onRetry: () => void }) {
  const details = file.error?.technicalDetails ?? copy.technical.noDetails;
  return (
    <div className="error-panel" role="alert">
      <div className="error-visual">
        <AlertCircle size={27} />
      </div>
      <p className="eyebrow">{copy.error.eyebrow}</p>
      <h3>{file.error?.title}</h3>
      <p>{file.error?.message}</p>
      <button className="primary-button" type="button" onClick={onRetry}>
        <RefreshCw size={17} />
        {copy.tryAgain}
      </button>
      <details className="technical-details error-details">
        <summary>
          {copy.technicalDetails}
          <ChevronDown size={15} />
        </summary>
        <div className="error-code">
          <code>{details}</code>
          <button
            className="icon-button small"
            type="button"
            aria-label={copy.accessibility.copyTechnicalDetails}
            onClick={() => void navigator.clipboard.writeText(details)}
          >
            <Copy size={15} />
          </button>
        </div>
      </details>
    </div>
  );
}

function ActionBar({
  files,
  engine,
  onConvert,
  onCancelAll,
  onDiagnostics,
}: {
  files: MediaFile[];
  engine: CapabilityRegistry["engine"];
  onConvert: () => void;
  onCancelAll: () => void;
  onDiagnostics: (trigger: HTMLButtonElement) => void;
}) {
  const ready = files.filter((file) => file.status === "ready").length;
  const running = files.filter(
    (file) => file.status === "running" || file.status === "queued",
  ).length;
  const completedFiles = files.filter((file) => file.status === "succeeded");
  const completed = completedFiles.length;
  const measuredFiles = completedFiles.filter((file) => file.output);
  const measuredInputBytes = measuredFiles.reduce((total, file) => total + file.sizeBytes, 0);
  const measuredOutputBytes = measuredFiles.reduce(
    (total, file) => total + (file.output?.sizeBytes ?? 0),
    0,
  );
  const convertLabel =
    ready > 1
      ? copy.action.convertMany(ready)
      : ready === 1
        ? copy.convert
        : completed === files.length
          ? copy.action.allConverted
          : copy.action.nothingReady;
  return (
    <footer className="action-bar">
      <button
        className="engine-indicator"
        type="button"
        aria-label={copy.accessibility.openDiagnostics}
        onClick={(event) => onDiagnostics(event.currentTarget)}
      >
        <span className={engine.available ? "engine-dot available" : "engine-dot"} />
        <span>
          <strong>
            {engine.available
              ? copy.diagnostics.sourceLabels[engine.source]
              : copy.diagnostics.sourceLabels.missing}
          </strong>
          <small>
            {engine.available
              ? `${engine.name}${engine.version ? ` ${engine.version}` : ""}`
              : copy.action.diagnostics}
          </small>
        </span>
      </button>
      <div className="action-summary">
        {completed > 0 ? (
          <span>
            <CheckCircle2 size={15} />
            {copy.action.complete(completed)}
          </span>
        ) : null}
        {measuredFiles.length > 0 ? (
          <span className="output-total">
            {copy.action.outputTotal(
              formatBytes(measuredInputBytes),
              formatBytes(measuredOutputBytes),
            )}
          </span>
        ) : null}
        {running > 0 ? (
          <span>
            <CircleEllipsis className="spin" size={15} />
            {copy.action.active(running)}
          </span>
        ) : null}
      </div>
      <div className="action-buttons">
        {running > 0 ? (
          <button className="secondary-button" type="button" onClick={onCancelAll}>
            <Square size={14} />
            {copy.cancelAll}
          </button>
        ) : null}
        <button
          className="primary-button convert-button"
          type="button"
          disabled={ready === 0 || !engine.available}
          onClick={onConvert}
          title={copy.accessibility.convertShortcut}
        >
          {completed === files.length ? (
            <Check size={17} />
          ) : (
            <Play size={16} fill="currentColor" />
          )}
          {convertLabel}
        </button>
      </div>
    </footer>
  );
}

function EngineDiagnostics({
  capabilities,
  busy,
  error,
  onRefresh,
  onChoose,
  onClose,
}: {
  capabilities: CapabilityRegistry;
  busy: boolean;
  error?: ConversionError | undefined;
  onRefresh: () => void;
  onChoose: () => void;
  onClose: () => void;
}) {
  const panelRef = useRef<HTMLDivElement>(null);
  const closeButtonRef = useRef<HTMLButtonElement>(null);
  const engine = capabilities.engine;
  const detail = error?.technicalDetails ?? engine.diagnostic;

  useEffect(() => {
    closeButtonRef.current?.focus();
    const handleKeyDown = (event: globalThis.KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
        return;
      }
      if (event.key !== "Tab") return;
      const focusable = Array.from(
        panelRef.current?.querySelectorAll<HTMLElement>(
          'button:not(:disabled), summary, [href], input:not(:disabled), select:not(:disabled), [tabindex]:not([tabindex="-1"])',
        ) ?? [],
      );
      if (focusable.length === 0) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last?.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first?.focus();
      }
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  return (
    <div className="diagnostics-backdrop">
      <div
        ref={panelRef}
        className="diagnostics-sheet"
        role="dialog"
        aria-modal="true"
        aria-labelledby="diagnostics-title"
        aria-describedby="diagnostics-summary"
      >
        <header className="diagnostics-heading">
          <div>
            <p className="eyebrow">{copy.diagnostics.eyebrow}</p>
            <h2 id="diagnostics-title">
              {engine.source === "native"
                ? copy.diagnostics.nativeTitleHeading
                : copy.diagnostics.title}
            </h2>
          </div>
          <button
            ref={closeButtonRef}
            className="icon-button"
            type="button"
            onClick={onClose}
            aria-label={copy.accessibility.closeDiagnostics}
          >
            <X size={18} />
          </button>
        </header>

        <section className={`engine-status-card${engine.available ? " is-ready" : ""}`}>
          <span className="engine-status-icon" aria-hidden="true">
            {busy ? (
              <CircleEllipsis className="spin" size={22} />
            ) : engine.available ? (
              <CheckCircle2 size={22} />
            ) : (
              <AlertCircle size={22} />
            )}
          </span>
          <div>
            <strong>
              {busy
                ? copy.diagnostics.checking
                : engine.available
                  ? copy.diagnostics.ready
                  : copy.diagnostics.missing}
            </strong>
            <p id="diagnostics-summary">
              {engine.available
                ? copy.diagnostics.readyBodies[engine.source]
                : copy.diagnostics.missingBody}
            </p>
          </div>
        </section>

        <dl className="engine-facts">
          <div>
            <dt>
              {engine.source === "native" ? copy.diagnostics.builtInName : copy.engineName}
            </dt>
            <dd>{engine.version ?? "—"}</dd>
          </div>
          <div>
            <dt>{copy.diagnostics.source}</dt>
            <dd>{copy.diagnostics.sourceLabels[engine.source]}</dd>
          </div>
        </dl>

        <section className="capability-section" aria-labelledby="capability-title">
          <div className="diagnostics-section-heading">
            <h3 id="capability-title">{copy.diagnostics.outputs}</h3>
            <span>
              {capabilities.outputs.filter((capability) => capability.available).length} /{" "}
              {capabilities.outputs.length}
            </span>
          </div>
          <ul className="capability-grid">
            {capabilities.outputs.map((capability) => (
              <li
                key={capability.format}
                className={capability.available ? "is-available" : ""}
                title={capability.reason}
              >
                <strong>{outputLabels[capability.format]}</strong>
                <span>
                  {capability.available
                    ? copy.diagnostics.available
                    : copy.diagnostics.unavailable}
                </span>
              </li>
            ))}
          </ul>
        </section>

        {error ? (
          <div className="inline-alert danger diagnostics-error" role="alert">
            <AlertCircle size={17} />
            <div>
              <strong>{error.title}</strong>
              <span>{error.message}</span>
            </div>
          </div>
        ) : null}

        {detail ? (
          <details className="technical-details diagnostics-details">
            <summary>
              {copy.technicalDetails}
              <ChevronDown size={15} />
            </summary>
            <div className="error-code">
              <code>{detail}</code>
              <button
                className="icon-button small"
                type="button"
                aria-label={copy.accessibility.copyTechnicalDetails}
                onClick={() => void navigator.clipboard.writeText(detail)}
              >
                <Copy size={15} />
              </button>
            </div>
          </details>
        ) : null}

        <div className="diagnostics-note">
          <ShieldCheck size={17} />
          <div>
            <strong>{copy.privacy}</strong>
            <span>{copy.diagnostics.sessionNote}</span>
          </div>
        </div>

        <footer className="diagnostics-actions">
          <div>
            <strong>{copy.diagnostics.folderHint}</strong>
          </div>
          <span>
            <button
              className="secondary-button"
              type="button"
              onClick={onRefresh}
              disabled={busy}
            >
              <RefreshCw className={busy ? "spin" : undefined} size={16} />
              {copy.diagnostics.checkAgain}
            </button>
            <button className="primary-button" type="button" onClick={onChoose} disabled={busy}>
              <FolderOpen size={16} />
              {copy.diagnostics.chooseFolder}
            </button>
          </span>
        </footer>
      </div>
    </div>
  );
}

function PreferencesPanel({
  collisionPolicy,
  onCollisionPolicy,
  startupReport,
  onClose,
}: {
  collisionPolicy: CollisionPolicy;
  onCollisionPolicy: (value: CollisionPolicy) => void;
  startupReport?: StartupReport | undefined;
  onClose: () => void;
}) {
  const closeButtonRef = useRef<HTMLButtonElement>(null);
  useEffect(() => closeButtonRef.current?.focus(), []);
  return (
    <aside className="preferences-popover" aria-label={copy.accessibility.settingsPanel}>
      <div className="popover-heading">
        <div>
          <p className="eyebrow">{copy.preferences.eyebrow}</p>
          <h2>{copy.preferences.title}</h2>
        </div>
        <button
          ref={closeButtonRef}
          className="icon-button"
          type="button"
          onClick={onClose}
          aria-label={copy.accessibility.closeSettings}
        >
          <X size={18} />
        </button>
      </div>
      <label className="select-field stacked">
        <span>{copy.preferences.collision}</span>
        <select
          value={collisionPolicy}
          onChange={(event) => onCollisionPolicy(event.currentTarget.value as CollisionPolicy)}
        >
          <option value="suffix">{copy.preferences.suffix}</option>
          <option value="skip">{copy.preferences.skip}</option>
        </select>
        <ChevronDown size={16} />
      </label>
      <p className="popover-note">
        <ShieldCheck size={15} />
        {startupReport?.recoveryFailures
          ? copy.preferences.recoveryFailed
          : startupReport?.recoveredTemporaryFiles
            ? copy.preferences.recovered(startupReport.recoveredTemporaryFiles)
            : copy.preferences.noHistory}
      </p>
    </aside>
  );
}

function DirectionStudy() {
  const directions = copy.designStudy.directions;
  return (
    <main className="direction-study">
      <div className="study-heading">
        <p className="eyebrow">{copy.designStudy.eyebrow}</p>
        <h1>{copy.designStudy.title}</h1>
        <p>{copy.designStudy.body}</p>
      </div>
      <div className="direction-grid">
        {directions.map((direction, index) => (
          <article key={direction.id} className={`direction-card direction-${direction.id}`}>
            <header>
              <span>{String.fromCharCode(65 + index)}</span>
              <div>
                <h2>{direction.name}</h2>
                <p>{direction.note}</p>
              </div>
              <strong>{direction.score}</strong>
            </header>
            <div className="mini-app">
              <div className="mini-header">
                <span>{copy.wordmark}</span>
                <i />
              </div>
              <div className="mini-body">
                <div className="mini-queue">
                  <b>{copy.conversionQueue}</b>
                  <MiniRow active />
                  <MiniRow />
                  <MiniRow />
                </div>
                <div className="mini-inspector">
                  <span>{copy.designStudy.outputFormat}</span>
                  <strong>{copy.formats.jpeg}</strong>
                  <div className="mini-choice">
                    <i />
                    <i />
                    <i />
                  </div>
                  <button>{copy.action.convertMany(3)}</button>
                </div>
              </div>
            </div>
            {index === 0 ? (
              <div className="selected-direction">
                <Check size={15} />
                {copy.designStudy.selected}
              </div>
            ) : null}
          </article>
        ))}
      </div>
    </main>
  );
}

function MiniRow({ active = false }: { active?: boolean }) {
  return (
    <div className={`mini-row${active ? " active" : ""}`}>
      <i />
      <span>
        <b />
        <em />
      </span>
      <small />
    </div>
  );
}
