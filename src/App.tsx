import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

type WritingStyle = "Academic" | "Concise" | "Professional";

type ModelDownloadProgress = {
  percent: number;
  downloaded_bytes: number;
  total_bytes: number | null;
};

type ModelLoadStatusPayload = {
  downloaded: boolean;
  loaded: boolean;
  busy: boolean;
  model_name: string;
  model_path: string | null;
  model_bytes: number | null;
  message: string;
};

type RewriteTokenPayload = {
  token: string;
  done: boolean;
};

type RewriteStatusPayload = {
  message: string;
  busy: boolean;
};

const styles: WritingStyle[] = ["Academic", "Concise", "Professional"];

function App() {
  const [isModelReady, setIsModelReady] = useState(false);
  const [isModelDownloaded, setIsModelDownloaded] = useState(false);
  const [isModelBusy, setIsModelBusy] = useState(false);
  const [modelName, setModelName] = useState("Qwen3-1.7B-GGUF Q8_0");
  const [modelBytes, setModelBytes] = useState<number | null>(null);
  const [isDownloading, setIsDownloading] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [inputText, setInputText] = useState("");
  const [style, setStyle] = useState<WritingStyle>("Academic");
  const [outputText, setOutputText] = useState("");
  const [isRewriting, setIsRewriting] = useState(false);
  const [statusMessage, setStatusMessage] = useState("Model setup required");
  const [rewriteStatus, setRewriteStatus] = useState("Idle");

  const canRewrite = useMemo(
    () => isModelReady && inputText.trim().length > 0 && !isRewriting,
    [inputText, isModelReady, isRewriting],
  );

  useEffect(() => {
    void refreshModelStatus();

    const unlistenDownload = listen<ModelDownloadProgress>(
      "model-download-progress",
      (event) => {
        setDownloadProgress(Math.round(event.payload.percent));
        setStatusMessage("Downloading model");
      },
    );

    const unlistenModelLoad = listen<ModelLoadStatusPayload>(
      "model-load-status",
      (event) => {
        applyModelStatus(event.payload);
      },
    );

    const unlistenTokens = listen<RewriteTokenPayload>(
      "rewrite-token",
      (event) => {
        if (event.payload.done) {
          setIsRewriting(false);
          return;
        }

        setOutputText((current) => `${current}${event.payload.token}`);
      },
    );

    const unlistenRewriteStatus = listen<RewriteStatusPayload>(
      "rewrite-status",
      (event) => {
        setRewriteStatus(event.payload.message);
        setIsRewriting(event.payload.busy);
      },
    );

    return () => {
      void unlistenDownload.then((unlisten) => unlisten());
      void unlistenModelLoad.then((unlisten) => unlisten());
      void unlistenTokens.then((unlisten) => unlisten());
      void unlistenRewriteStatus.then((unlisten) => unlisten());
    };
  }, []);

  function applyModelStatus(payload: ModelLoadStatusPayload) {
    setIsModelDownloaded(payload.downloaded);
    setIsModelReady(payload.loaded);
    setIsModelBusy(payload.busy);
    setModelName(payload.model_name);
    setModelBytes(payload.model_bytes);
    setStatusMessage(payload.message);
    if (payload.downloaded && payload.model_bytes) {
      setDownloadProgress(100);
    }
  }

  async function refreshModelStatus() {
    try {
      const status = await invoke<ModelLoadStatusPayload>("model_status");
      applyModelStatus(status);
    } catch (error) {
      setStatusMessage(error instanceof Error ? error.message : String(error));
    }
  }

  async function startSetup() {
    setIsDownloading(true);
    setIsModelBusy(true);
    setStatusMessage("Starting download");

    try {
      const status = await invoke<ModelLoadStatusPayload>("download_model");
      setDownloadProgress(100);
      applyModelStatus(status);
    } catch (error) {
      setStatusMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setIsDownloading(false);
      setIsModelBusy(false);
    }
  }

  async function loadLocalModel() {
    setIsModelBusy(true);
    setStatusMessage("Loading model into memory");

    try {
      const status = await invoke<ModelLoadStatusPayload>("load_model");
      applyModelStatus(status);
    } catch (error) {
      setStatusMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setIsModelBusy(false);
    }
  }

  async function rewriteText() {
    if (!canRewrite) {
      return;
    }

    setOutputText("");
    setIsRewriting(true);
    setRewriteStatus("Sending request to backend");

    try {
      await invoke("rewrite_text", { text: inputText, style });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setIsRewriting(false);
      setRewriteStatus(message);
      setOutputText(message);
    }
  }

  return (
    <main className="min-h-screen bg-zinc-950 text-zinc-100">
      {!isModelReady ? (
        <section className="mx-auto flex min-h-screen w-full max-w-xl flex-col justify-center px-6">
          <div className="border border-zinc-800 bg-zinc-900/70 p-8 shadow-2xl shadow-black/30">
            <p className="text-sm font-medium uppercase tracking-[0.18em] text-emerald-400">
              First Launch
            </p>
            <h1 className="mt-3 text-3xl font-semibold text-white">
              Download local writing model
            </h1>
            <p className="mt-3 text-sm leading-6 text-zinc-400">
              Qwen3-1.7B-GGUF will be stored in the app data folder
              and loaded locally for offline rewriting.
            </p>

            <div className="mt-6 grid gap-3 text-sm">
              <div className="flex items-center justify-between border border-zinc-800 bg-zinc-950/60 px-4 py-3">
                <span className="text-zinc-500">Model</span>
                <span className="text-right text-zinc-200">{modelName}</span>
              </div>
              <div className="grid grid-cols-3 border border-zinc-800 bg-zinc-950/60 text-center">
                <div className="px-3 py-3">
                  <p className="text-zinc-500">Downloaded</p>
                  <p className={isModelDownloaded ? "text-emerald-400" : "text-zinc-400"}>
                    {isModelDownloaded ? "Yes" : "No"}
                  </p>
                </div>
                <div className="border-x border-zinc-800 px-3 py-3">
                  <p className="text-zinc-500">Loaded</p>
                  <p className={isModelReady ? "text-emerald-400" : "text-zinc-400"}>
                    {isModelReady ? "Yes" : "No"}
                  </p>
                </div>
                <div className="px-3 py-3">
                  <p className="text-zinc-500">Size</p>
                  <p className="text-zinc-300">{formatBytes(modelBytes)}</p>
                </div>
              </div>
            </div>

            <div className="mt-8">
              <div className="mb-2 flex items-center justify-between text-sm">
                <span className="text-zinc-300">{statusMessage}</span>
                <span className="font-medium text-zinc-100">
                  {downloadProgress}%
                </span>
              </div>
              <div className="h-2 overflow-hidden rounded-full bg-zinc-800">
                <div
                  className="h-full bg-emerald-400 transition-all duration-300"
                  style={{ width: `${downloadProgress}%` }}
                />
              </div>
            </div>

            <div className="mt-8 grid grid-cols-1 gap-3 sm:grid-cols-2">
              <button
                className="rounded-md bg-zinc-800 px-4 py-3 text-sm font-semibold text-zinc-100 transition hover:bg-zinc-700 disabled:cursor-not-allowed disabled:bg-zinc-800 disabled:text-zinc-500"
                disabled={isDownloading || isModelBusy}
                onClick={refreshModelStatus}
                type="button"
              >
                Check Model
              </button>
              <button
                className="rounded-md bg-emerald-400 px-4 py-3 text-sm font-semibold text-zinc-950 transition hover:bg-emerald-300 disabled:cursor-not-allowed disabled:bg-zinc-700 disabled:text-zinc-400"
                disabled={isDownloading || isModelBusy}
                onClick={isModelDownloaded ? loadLocalModel : startSetup}
                type="button"
              >
                {isDownloading || isModelBusy
                  ? "Preparing..."
                  : isModelDownloaded
                    ? "Load Model"
                    : "Start Setup"}
              </button>
            </div>
          </div>
        </section>
      ) : (
        <section className="grid min-h-screen grid-cols-1 lg:grid-cols-2">
          <div className="flex min-h-[50vh] flex-col border-b border-zinc-800 bg-zinc-950 p-6 lg:min-h-screen lg:border-b-0 lg:border-r">
            <div className="flex items-center justify-between gap-4">
              <div>
                <p className="text-sm font-medium text-emerald-400">
                  Thesis Rewriter
                </p>
                <h1 className="text-2xl font-semibold text-white">Input</h1>
                <p className="mt-1 text-sm text-zinc-500">
                  LLM loaded: {modelName}
                </p>
              </div>
              <div
                aria-label="Writing style"
                className="style-segments"
                role="radiogroup"
              >
                {styles.map((item) => (
                  <button
                    aria-checked={style === item}
                    className="style-segment"
                    key={item}
                    onClick={() => setStyle(item)}
                    role="radio"
                    type="button"
                  >
                    {item}
                  </button>
                ))}
              </div>
            </div>

            <textarea
              className="mt-6 min-h-0 flex-1 rounded-md border border-zinc-800 bg-zinc-900/60 p-4 text-base leading-7 text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-emerald-400"
              onChange={(event) => setInputText(event.target.value)}
              placeholder="Paste academic text to rewrite..."
              value={inputText}
            />

            <button
              className="mt-5 rounded-md bg-emerald-400 px-4 py-3 text-sm font-semibold text-zinc-950 transition hover:bg-emerald-300 disabled:cursor-not-allowed disabled:bg-zinc-800 disabled:text-zinc-500"
              disabled={!canRewrite}
              onClick={rewriteText}
              type="button"
            >
              {isRewriting ? "Rewriting..." : "Rewrite"}
            </button>
            <p className="mt-3 min-h-5 text-sm text-zinc-500">{rewriteStatus}</p>
          </div>

          <div className="flex min-h-[50vh] flex-col bg-zinc-900 p-6 lg:min-h-screen">
            <div>
              <p className="text-sm font-medium text-zinc-500">
                Streaming Output
              </p>
              <h2 className="text-2xl font-semibold text-white">Result</h2>
            </div>

            <div className="mt-6 min-h-0 flex-1 rounded-md border border-zinc-800 bg-zinc-950/70 p-4">
              <p className="whitespace-pre-wrap text-base leading-7 text-zinc-200">
                {outputText || (
                  <span className="text-zinc-600">
                    Rewritten text will appear here.
                  </span>
                )}
              </p>
            </div>
          </div>
        </section>
      )}
    </main>
  );
}

function formatBytes(bytes: number | null) {
  if (!bytes) {
    return "--";
  }

  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unitIndex = 0;

  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }

  return `${value.toFixed(unitIndex === 0 ? 0 : 1)} ${units[unitIndex]}`;
}

export default App;
