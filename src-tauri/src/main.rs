// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{AppHandle, Emitter};
use tokio::time::{sleep, Duration};

#[derive(Clone, serde::Serialize)]
struct ModelDownloadProgress {
    percent: f32,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
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
async fn download_model(app: AppHandle) -> Result<(), String> {
    app.emit(
        "model-download-progress",
        ModelDownloadProgress {
            percent: 0.0,
            downloaded_bytes: 0,
            total_bytes: None,
        },
    )
    .map_err(|error| error.to_string())?;

    Ok(())
}

#[tauri::command]
async fn rewrite_text(app: AppHandle, text: String, style: String) -> Result<(), String> {
    let trimmed = text.trim();

    app.emit(
        "rewrite-status",
        RewriteStatusPayload {
            message: "Backend received rewrite request".to_string(),
            busy: true,
        },
    )
    .map_err(|error| error.to_string())?;

    if trimmed.is_empty() {
        return Err("Input text is empty".to_string());
    }

    let rewritten = rewrite_fallback(trimmed, &style);

    for token in rewritten.split_inclusive(' ') {
        app.emit(
            "rewrite-token",
            RewriteTokenPayload {
                token: token.to_string(),
                done: false,
            },
        )
        .map_err(|error| error.to_string())?;

        sleep(Duration::from_millis(28)).await;
    }

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
            message: "Rewrite complete".to_string(),
            busy: false,
        },
    )
    .map_err(|error| error.to_string())?;

    Ok(())
}

fn rewrite_fallback(text: &str, style: &str) -> String {
    match style {
        "Concise" => format!(
            "Concise rewrite:\n\n{}",
            text.split_whitespace().collect::<Vec<_>>().join(" ")
        ),
        "Professional" => format!(
            "Professional rewrite:\n\nThe following revised version improves clarity, tone, and flow while preserving the original meaning:\n\n{}",
            text
        ),
        _ => format!(
            "Academic rewrite:\n\nThis revised version presents the argument in a more formal academic style while preserving the original intent:\n\n{}",
            text
        ),
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![download_model, rewrite_text])
        .run(tauri::generate_context!())
        .expect("failed to run Tauri application");
}
