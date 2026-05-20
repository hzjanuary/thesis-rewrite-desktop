// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    num::NonZeroU32,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use futures_util::StreamExt;
use llama_cpp_2::{
    context::params::LlamaContextParams,
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::{params::LlamaModelParams, AddBos, LlamaChatMessage, LlamaChatTemplate, LlamaModel},
    sampling::LlamaSampler,
};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::AsyncWriteExt;

const MODEL_REPO: &str = "Qwen/Qwen3-1.7B-GGUF";
const MODEL_FILE: &str = "Qwen3-1.7B-Q8_0.gguf";
const MODEL_URL: &str =
    "https://huggingface.co/Qwen/Qwen3-1.7B-GGUF/resolve/main/Qwen3-1.7B-Q8_0.gguf";
const MODEL_DISPLAY_NAME: &str = "Qwen3-1.7B-GGUF Q8_0";
const DEFAULT_CONTEXT_TOKENS: u32 = 4096;
const MAX_GENERATED_TOKENS: usize = 700;

struct ModelState {
    backend: Option<LlamaBackend>,
    model: Option<LlamaModel>,
    loaded_path: Option<PathBuf>,
    loaded_name: Option<String>,
}

impl ModelState {
    fn new() -> Self {
        Self {
            backend: None,
            model: None,
            loaded_path: None,
            loaded_name: None,
        }
    }
}

#[derive(Clone, serde::Serialize)]
struct ModelDownloadProgress {
    percent: f32,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
}

#[derive(Clone, serde::Serialize)]
struct ModelLoadStatusPayload {
    downloaded: bool,
    loaded: bool,
    busy: bool,
    model_name: String,
    model_path: Option<String>,
    model_bytes: Option<u64>,
    message: String,
}

#[derive(Clone, serde::Serialize)]
struct RewriteTokenPayload {
    token: String,
    done: bool,
}

#[derive(Clone, serde::Serialize)]
struct RewriteStatusPayload {
    message: String,
    busy: bool,
}

#[tauri::command]
async fn model_status(
    app: AppHandle,
    state: State<'_, Mutex<ModelState>>,
) -> Result<ModelLoadStatusPayload, String> {
    let status = current_model_status(&app, &state)?;
    app.emit("model-load-status", status.clone())
        .map_err(|error| error.to_string())?;
    Ok(status)
}

#[tauri::command]
async fn load_model(
    app: AppHandle,
    state: State<'_, Mutex<ModelState>>,
) -> Result<ModelLoadStatusPayload, String> {
    let path = model_path(&app)?;
    if !path.exists() {
        let status = ModelLoadStatusPayload {
            downloaded: false,
            loaded: false,
            busy: false,
            model_name: MODEL_DISPLAY_NAME.to_string(),
            model_path: Some(path.to_string_lossy().to_string()),
            model_bytes: None,
            message: "Model file not downloaded yet".to_string(),
        };
        app.emit("model-load-status", status.clone())
            .map_err(|error| error.to_string())?;
        return Ok(status);
    }

    load_model_from_disk(&app, &state, path)
}

#[tauri::command]
async fn download_model(
    app: AppHandle,
    state: State<'_, Mutex<ModelState>>,
) -> Result<ModelLoadStatusPayload, String> {
    let path = model_path(&app)?;
    ensure_parent_dir(&path).await?;

    if path.exists() {
        emit_download_progress(
            &app,
            100.0,
            path.metadata().ok().map_or(0, |m| m.len()),
            None,
        )?;
        return load_model_from_disk(&app, &state, path);
    }

    emit_load_status(
        &app,
        false,
        false,
        true,
        Some(&path),
        None,
        "Downloading Qwen3 multilingual writing model",
    )?;

    let tmp_path = path.with_extension("gguf.part");
    let response = reqwest::get(MODEL_URL)
        .await
        .map_err(|error| format!("Failed to start model download: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Model download failed: {error}"))?;

    let total_bytes = response.content_length();
    let mut downloaded_bytes = 0_u64;
    let mut file = tokio::fs::File::create(&tmp_path)
        .await
        .map_err(|error| format!("Failed to create model file: {error}"))?;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| format!("Model download interrupted: {error}"))?;
        file.write_all(&chunk)
            .await
            .map_err(|error| format!("Failed writing model file: {error}"))?;
        downloaded_bytes += u64::try_from(chunk.len()).unwrap_or(0);

        let percent = total_bytes.map_or(0.0, |total| {
            if total == 0 {
                0.0
            } else {
                (downloaded_bytes as f32 / total as f32) * 100.0
            }
        });
        emit_download_progress(&app, percent, downloaded_bytes, total_bytes)?;
    }

    file.flush()
        .await
        .map_err(|error| format!("Failed flushing model file: {error}"))?;
    drop(file);

    tokio::fs::rename(&tmp_path, &path)
        .await
        .map_err(|error| format!("Failed finalizing model file: {error}"))?;
    emit_download_progress(&app, 100.0, downloaded_bytes, total_bytes)?;

    load_model_from_disk(&app, &state, path)
}

#[tauri::command]
async fn rewrite_text(
    app: AppHandle,
    state: State<'_, Mutex<ModelState>>,
    text: String,
    style: String,
) -> Result<(), String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Input text is empty".to_string());
    }

    app.emit(
        "rewrite-status",
        RewriteStatusPayload {
            message: "Running local LLM inference".to_string(),
            busy: true,
        },
    )
    .map_err(|error| error.to_string())?;

    let result = generate_rewrite(&app, &state, trimmed, &style);

    let final_message = match &result {
        Ok(()) => "Rewrite complete".to_string(),
        Err(error) => error.clone(),
    };

    app.emit(
        "rewrite-token",
        RewriteTokenPayload {
            token: String::new(),
            done: true,
        },
    )
    .map_err(|error| error.to_string())?;
    app.emit(
        "rewrite-status",
        RewriteStatusPayload {
            message: final_message,
            busy: false,
        },
    )
    .map_err(|error| error.to_string())?;

    result
}

fn generate_rewrite(
    app: &AppHandle,
    state: &State<'_, Mutex<ModelState>>,
    text: &str,
    style: &str,
) -> Result<(), String> {
    let guard = state
        .lock()
        .map_err(|_| "Model state lock was poisoned".to_string())?;
    let backend = guard.backend.as_ref().ok_or_else(|| {
        "LLM backend is not initialized. Run Setup or Load Model first.".to_string()
    })?;
    let model = guard
        .model
        .as_ref()
        .ok_or_else(|| "LLM model is not loaded. Run Setup or Load Model first.".to_string())?;

    let prompt = build_rewrite_prompt(model, text, style)?;
    let prompt_tokens = model
        .str_to_token(&prompt, AddBos::Never)
        .map_err(|error| format!("Failed to tokenize prompt: {error}"))?;

    if prompt_tokens.len() + MAX_GENERATED_TOKENS >= DEFAULT_CONTEXT_TOKENS as usize {
        return Err("Input is too long for the current 4096-token local context".to_string());
    }

    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(DEFAULT_CONTEXT_TOKENS))
        .with_n_batch(512)
        .with_n_ubatch(512)
        .with_n_threads(worker_threads())
        .with_n_threads_batch(worker_threads());
    let mut ctx = model
        .new_context(backend, ctx_params)
        .map_err(|error| format!("Failed to create inference context: {error}"))?;

    let mut batch = LlamaBatch::new(prompt_tokens.len().max(512), 1);
    batch
        .add_sequence(&prompt_tokens, 0, false)
        .map_err(|error| format!("Failed to prepare prompt batch: {error}"))?;
    ctx.decode(&mut batch)
        .map_err(|error| format!("Failed to evaluate prompt: {error}"))?;

    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs() as u32);
    let mut sampler = LlamaSampler::chain_simple([
        LlamaSampler::top_k(20),
        LlamaSampler::top_p(0.8, 1),
        LlamaSampler::min_p(0.0, 1),
        LlamaSampler::temp(0.7),
        LlamaSampler::dist(seed),
    ]);
    let mut decoder = encoding_rs::UTF_8.new_decoder();
    let mut current_position = i32::try_from(prompt_tokens.len())
        .map_err(|_| "Prompt token count exceeds supported context".to_string())?;

    for _ in 0..MAX_GENERATED_TOKENS {
        let token = sampler.sample(&ctx, -1);
        if model.is_eog_token(token) {
            break;
        }

        sampler.accept(token);
        let piece = model
            .token_to_piece(token, &mut decoder, false, None)
            .map_err(|error| format!("Failed to decode generated token: {error}"))?;
        if !piece.is_empty() {
            app.emit(
                "rewrite-token",
                RewriteTokenPayload {
                    token: piece,
                    done: false,
                },
            )
            .map_err(|error| error.to_string())?;
        }

        batch.clear();
        batch
            .add(token, current_position, &[0], true)
            .map_err(|error| format!("Failed to prepare generated token: {error}"))?;
        ctx.decode(&mut batch)
            .map_err(|error| format!("Failed during token generation: {error}"))?;
        current_position += 1;
    }

    Ok(())
}

fn build_rewrite_prompt(model: &LlamaModel, text: &str, style: &str) -> Result<String, String> {
    let style_instruction = match style {
        "Concise" => "Make the writing concise, remove redundancy, and preserve all key meaning.",
        "Professional" => {
            "Use a professional tone with clear structure, precise wording, and polished flow."
        }
        _ => "Use a formal academic tone with improved clarity, cohesion, and scholarly wording.",
    };
    let system = "You are a multilingual academic writing assistant. Rewrite the user text in the same language as the input. Do not translate unless the input asks for translation. Preserve citations, terms, numbers, and meaning. Return only the rewritten text. /no_think";
    let user = format!("{style_instruction}\n\nText to rewrite:\n{text}");
    let messages = [
        LlamaChatMessage::new("system".to_string(), system.to_string())
            .map_err(|error| format!("Failed to build system prompt: {error}"))?,
        LlamaChatMessage::new("user".to_string(), user)
            .map_err(|error| format!("Failed to build user prompt: {error}"))?,
    ];
    let template = match model.chat_template(None) {
        Ok(template) => template,
        Err(_) => LlamaChatTemplate::new("chatml")
            .map_err(|error| format!("Failed to build fallback chat template: {error}"))?,
    };

    model
        .apply_chat_template(&template, &messages, true)
        .map_err(|error| format!("Failed to apply chat template: {error}"))
}

fn load_model_from_disk(
    app: &AppHandle,
    state: &State<'_, Mutex<ModelState>>,
    path: PathBuf,
) -> Result<ModelLoadStatusPayload, String> {
    emit_load_status(
        app,
        true,
        false,
        true,
        Some(&path),
        path.metadata().ok().map(|m| m.len()),
        "Loading model into local memory",
    )?;

    let mut guard = state
        .lock()
        .map_err(|_| "Model state lock was poisoned".to_string())?;

    if guard.loaded_path.as_ref() == Some(&path) && guard.model.is_some() {
        drop(guard);
        let status = current_model_status(app, state)?;
        app.emit("model-load-status", status.clone())
            .map_err(|error| error.to_string())?;
        return Ok(status);
    }

    if guard.backend.is_none() {
        guard.backend = Some(
            LlamaBackend::init()
                .map_err(|error| format!("Failed to initialize llama.cpp: {error}"))?,
        );
    }

    let backend = guard
        .backend
        .as_ref()
        .ok_or_else(|| "llama.cpp backend is unavailable".to_string())?;
    let model_params = LlamaModelParams::default();
    let model = LlamaModel::load_from_file(backend, &path, &model_params)
        .map_err(|error| format!("Failed to load GGUF model: {error}"))?;

    guard.model = Some(model);
    guard.loaded_path = Some(path.clone());
    guard.loaded_name = Some(MODEL_DISPLAY_NAME.to_string());
    drop(guard);

    let status = current_model_status(app, state)?;
    app.emit("model-load-status", status.clone())
        .map_err(|error| error.to_string())?;
    Ok(status)
}

fn current_model_status(
    app: &AppHandle,
    state: &State<'_, Mutex<ModelState>>,
) -> Result<ModelLoadStatusPayload, String> {
    let path = model_path(app)?;
    let downloaded = path.exists();
    let model_bytes = path.metadata().ok().map(|metadata| metadata.len());
    let guard = state
        .lock()
        .map_err(|_| "Model state lock was poisoned".to_string())?;
    let loaded = guard.model.is_some();
    let message = match (downloaded, loaded) {
        (_, true) => "Model loaded and ready".to_string(),
        (true, false) => "Model downloaded but not loaded".to_string(),
        (false, false) => "Model setup required".to_string(),
    };

    Ok(ModelLoadStatusPayload {
        downloaded,
        loaded,
        busy: false,
        model_name: guard
            .loaded_name
            .clone()
            .unwrap_or_else(|| MODEL_DISPLAY_NAME.to_string()),
        model_path: Some(path.to_string_lossy().to_string()),
        model_bytes,
        message,
    })
}

fn emit_load_status(
    app: &AppHandle,
    downloaded: bool,
    loaded: bool,
    busy: bool,
    model_path: Option<&Path>,
    model_bytes: Option<u64>,
    message: &str,
) -> Result<(), String> {
    app.emit(
        "model-load-status",
        ModelLoadStatusPayload {
            downloaded,
            loaded,
            busy,
            model_name: MODEL_DISPLAY_NAME.to_string(),
            model_path: model_path.map(|path| path.to_string_lossy().to_string()),
            model_bytes,
            message: message.to_string(),
        },
    )
    .map_err(|error| error.to_string())
}

fn emit_download_progress(
    app: &AppHandle,
    percent: f32,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
) -> Result<(), String> {
    app.emit(
        "model-download-progress",
        ModelDownloadProgress {
            percent,
            downloaded_bytes,
            total_bytes,
        },
    )
    .map_err(|error| error.to_string())
}

fn model_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to resolve app data folder: {error}"))?
        .join("models")
        .join(MODEL_REPO.replace('/', "--"))
        .join(MODEL_FILE))
}

async fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Model path has no parent directory".to_string())?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|error| format!("Failed to create model directory: {error}"))
}

fn worker_threads() -> i32 {
    std::thread::available_parallelism().map_or(4, |threads| {
        let count = threads.get().saturating_sub(1).clamp(2, 8);
        i32::try_from(count).unwrap_or(4)
    })
}

fn main() {
    tauri::Builder::default()
        .manage(Mutex::new(ModelState::new()))
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            model_status,
            load_model,
            download_model,
            rewrite_text
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Tauri application");
}
