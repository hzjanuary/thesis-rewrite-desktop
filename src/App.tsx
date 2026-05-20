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
    const unlistenDownload = listen<ModelDownloadProgress>(
      "model-download-progress",
      (event) => {
        setDownloadProgress(Math.round(event.payload.percent));
        setStatusMessage("Downloading model");
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
      void unlistenTokens.then((unlisten) => unlisten());
      void unlistenRewriteStatus.then((unlisten) => unlisten());
    };
  }, []);

  async function startSetup() {
    setIsDownloading(true);
    setStatusMessage("Starting download");

    try {
      await invoke("download_model");
      setDownloadProgress(100);
      setIsModelReady(true);
      setStatusMessage("Model ready");
    } catch (error) {
      setStatusMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setIsDownloading(false);
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
              Qwen2.5-1.5B-Instruct GGUF will be stored in the app data folder
              and loaded locally for offline rewriting.
            </p>

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

            <button
              className="mt-8 w-full rounded-md bg-emerald-400 px-4 py-3 text-sm font-semibold text-zinc-950 transition hover:bg-emerald-300 disabled:cursor-not-allowed disabled:bg-zinc-700 disabled:text-zinc-400"
              disabled={isDownloading}
              onClick={startSetup}
              type="button"
            >
              {isDownloading ? "Preparing..." : "Start Setup"}
            </button>
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

export default App;
