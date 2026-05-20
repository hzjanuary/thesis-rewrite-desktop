# Thesis Rewriter

A native desktop app for rewriting and polishing academic text with a local language model. The app uses Tauri v2 to run a React interface inside a lightweight desktop shell, while model downloads and GGUF inference run in Rust through `llama.cpp`.

## Overview

Thesis Rewriter is designed as an offline-first application:

- On first launch, the Rust backend downloads a GGUF model from Hugging Face into the operating system's application data directory.
- The React frontend receives model download progress through Tauri events and displays a setup screen.
- After the model has been downloaded, Rust loads it into local memory with `llama-cpp-2`.
- When the user rewrites text, the frontend calls the `rewrite_text` Tauri command; the backend generates the result token by token and streams it back to the UI with the `rewrite-token` event.
- After the model has been downloaded, the app does not call any external AI API during rewriting.

## Features

- Native desktop app built with Tauri v2, without bundling a separate browser runtime.
- React + TypeScript + Tailwind CSS 4 interface with dark mode by default.
- First-launch setup screen with a model download progress bar.
- Split editor layout: input on the left, streaming output on the right.
- Three rewrite styles: `Academic`, `Concise`, and `Professional`.
- GGUF model stored in the application data folder, not bundled into the installer.
- Token streaming for a typing effect powered by local inference.

## Current Status

The project already includes the scaffold and the core application flow:

- Frontend: [src/App.tsx](src/App.tsx) and [src/App.css](src/App.css).
- Tauri/Rust backend: [src-tauri/src/main.rs](src-tauri/src/main.rs).
- Tauri configuration: [src-tauri/tauri.conf.json](src-tauri/tauri.conf.json).
- Rust dependencies: [src-tauri/Cargo.toml](src-tauri/Cargo.toml).

The model is currently configured in source code:

```text
Repository: Qwen/Qwen3-1.7B-GGUF
File:       Qwen3-1.7B-Q8_0.gguf
Size:       approximately 1.83 GB
License:    Apache-2.0
```

Note: the original project requirements mentioned Qwen2.5-1.5B-Instruct 4-bit, but the current code uses Qwen3-1.7B-GGUF Q8_0. This README describes the current source code.

## Tech Stack

- Desktop: Tauri v2
- Frontend: React 19, Vite 7, TypeScript, Tailwind CSS 4
- Backend: Rust 2021
- Inference: `llama-cpp-2`
- Download streaming: `reqwest`, `tokio`, `futures-util`
- Model format: GGUF

## System Requirements

Required tools:

- Node.js 20+ recommended
- npm
- Rust stable and Cargo
- Tauri system dependencies for your operating system
- Internet connection for the first model download
- At least 3-4 GB of free RAM for the Q8_0 model and a basic inference context

Linux also requires the WebKit/GTK packages for your distribution. See the Tauri v2 prerequisites if you are building on a new machine.

## Installation

```bash
npm install
```

If Rust dependencies have not been fetched yet, Cargo will download them when you run `cargo check`, `npm run tauri dev`, or `npm run tauri build`.

## Development

Run the desktop app:

```bash
npm run tauri dev
```

Run only the Vite frontend to inspect the UI in a browser:

```bash
npm run dev
```

Note: browser mode is only useful for checking the UI. Tauri commands such as `download_model`, `load_model`, and `rewrite_text` require the Tauri runtime.

## Build

Build the frontend:

```bash
npm run build
```

Check the Rust backend:

```bash
cd src-tauri
cargo check
```

Build the desktop package:

```bash
npm run tauri build
```

The installer or binary output is generated under `src-tauri/target/release/bundle/`, depending on the target platform.

## Usage Flow

1. Open the app with `npm run tauri dev` or a built release.
2. If the model is missing, the app shows the setup screen.
3. Click `Start Setup` to download the GGUF model from Hugging Face.
4. The backend stores the model in the operating system's app data folder.
5. After the model has loaded, the app switches to the editor.
6. Paste academic text into the input panel.
7. Choose a rewrite style.
8. Click `Rewrite`.
9. The result streams token by token into the output panel.

## Model Storage Directory

The backend uses Tauri's `app.path().app_data_dir()`, then creates this path:

```text
<app-data-dir>/models/Qwen--Qwen3-1.7B-GGUF/Qwen3-1.7B-Q8_0.gguf
```

The exact `<app-data-dir>` location depends on the operating system and the Tauri configuration. Common locations include:

- Windows: under `%APPDATA%` or `%LOCALAPPDATA%`
- macOS: under `~/Library/Application Support/`
- Linux: under `~/.local/share/`

The model is not included in the installer and can be deleted manually if it needs to be downloaded again.

## Tauri Commands And Events

The frontend calls Rust with `invoke` and listens for events with `listen`.

### Commands

| Command | Parameters | Description |
| --- | --- | --- |
| `model_status` | none | Checks whether the model has been downloaded and loaded |
| `download_model` | none | Downloads the GGUF file into the app data folder, then loads the model |
| `load_model` | none | Loads an already downloaded model into memory |
| `rewrite_text` | `{ text, style }` | Rewrites text and streams tokens back to the frontend |

### Events

| Event | Payload | Description |
| --- | --- | --- |
| `model-download-progress` | `{ percent, downloaded_bytes, total_bytes }` | Model download progress |
| `model-load-status` | `{ downloaded, loaded, busy, model_name, model_path, model_bytes, message }` | Model status |
| `rewrite-status` | `{ message, busy }` | Inference status |
| `rewrite-token` | `{ token, done }` | Streaming output token |

## Architecture

```text
React UI
  | invoke / listen
  v
Tauri IPC
  |
  v
Rust commands
  |-- model_status
  |-- download_model -> reqwest stream -> app data folder
  |-- load_model     -> llama-cpp-2 -> GGUF
  |-- rewrite_text   -> llama context + sampler -> token events
```

Model state is stored in `Mutex<ModelState>`:

- `LlamaBackend`
- `LlamaModel`
- loaded model path
- display model name

For each rewrite request, the backend creates a dedicated inference context with:

- context window: `4096` tokens
- output limit: `700` tokens
- sampler: `top_k(20)`, `top_p(0.8)`, `temp(0.7)`
- system prompt containing `/no_think` to prefer concise output without visible reasoning

## Project Structure

```text
.
├── src/
│   ├── App.tsx              # Setup screen and editor UI
│   ├── App.css              # Tailwind import and supporting CSS
│   └── main.tsx             # React entrypoint
├── src-tauri/
│   ├── src/main.rs          # Tauri commands, download, model loading, inference
│   ├── src/lib.rs           # Tauri library entrypoint template
│   ├── Cargo.toml           # Rust dependencies
│   ├── tauri.conf.json      # Tauri app config
│   └── capabilities/        # Tauri v2 capability permissions
├── public/                  # Static Vite assets
├── package.json             # npm scripts and frontend dependencies
├── vite.config.ts           # Vite config for Tauri
└── README.md
```

## Scripts

| Script | Description |
| --- | --- |
| `npm run dev` | Runs the Vite dev server |
| `npm run build` | Typechecks the frontend and builds Vite |
| `npm run preview` | Previews the frontend build |
| `npm run tauri` | Runs the Tauri CLI |
| `npm run tauri dev` | Runs the desktop app in development mode |
| `npm run tauri build` | Builds the desktop bundle |

## Model Configuration

The model is configured in [src-tauri/src/main.rs](src-tauri/src/main.rs):

```rust
const MODEL_REPO: &str = "Qwen/Qwen3-1.7B-GGUF";
const MODEL_FILE: &str = "Qwen3-1.7B-Q8_0.gguf";
const MODEL_URL: &str =
    "https://huggingface.co/Qwen/Qwen3-1.7B-GGUF/resolve/main/Qwen3-1.7B-Q8_0.gguf";
const MODEL_DISPLAY_NAME: &str = "Qwen3-1.7B-GGUF Q8_0";
```

If you switch to another model, make sure that:

- The file is a GGUF file compatible with llama.cpp.
- The model has a suitable chat template, or the `chatml` fallback is acceptable.
- The model size fits the target user's available RAM.
- The model license permits the intended use of the application.

## Security And Privacy

- Rewrite content is processed locally after the model has been downloaded.
- The app needs Internet access for the first model download from Hugging Face.
- The model file is stored outside the installer in the app data folder.
- The current Tauri capability configuration only enables `core:default` and `opener:default`.

## Troubleshooting

### `Start Setup` is slow or the model download fails

Check your Internet connection and available disk space. The Q8_0 model is approximately 1.83 GB, and a temporary `.part` file is created while downloading.

### The app reports `Model downloaded but not loaded`

Click `Load Model`. If the problem persists, delete the model file from the app data folder and download it again.

### `Input is too long for the current 4096-token local context`

Shorten the input. The current context is set to `4096` tokens and the maximum output is `700` tokens.

### Tauri build fails on Linux with WebKit/GTK errors

Install the correct Tauri system dependencies for your distribution.

### Inference repeats itself or quality is unstable

Tune the sampler, prompt, `presence_penalty`, or choose another quantization. Qwen recommends different parameters for thinking and non-thinking modes; the app currently uses `/no_think` and sampling values close to the non-thinking recommendations.

## Verification

At the time this README was updated:

```bash
npm run build
cd src-tauri && cargo check
```

Both commands completed successfully.

## Suggested Roadmap

- Add a button to cancel a running inference request.
- Add model and quantization selection in the UI.
- Add checksum verification after model download.
- Add download resume support when a `.part` file already exists.
- Add logging with `tauri-plugin-log`.
- Add output copy/export actions.
- Add tests for the prompt builder and model path handling.
- Move backend inference into a dedicated module instead of keeping everything in `main.rs`.

## References

- Tauri v2 docs: https://v2.tauri.app/
- Tauri command and event IPC: https://v2.tauri.app/develop/calling-rust/
- Tauri create project: https://v2.tauri.app/start/
- llama.cpp: https://github.com/ggml-org/llama.cpp
- `llama-cpp-2` crate: https://docs.rs/llama-cpp-2/latest/llama_cpp_2/
- Qwen3-1.7B-GGUF model card: https://huggingface.co/Qwen/Qwen3-1.7B-GGUF
- Qwen2.5-1.5B-Instruct-GGUF model card: https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF

## License

This project is licensed under the MIT License. See [LICENSE.txt](LICENSE.txt).

The Qwen3-1.7B-GGUF model on Hugging Face is published under the Apache-2.0 license. Review the model card and related terms before distributing the application.
