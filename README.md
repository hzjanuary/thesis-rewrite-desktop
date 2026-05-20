# Thesis Rewriter

Native desktop app hỗ trợ viết lại và đánh bóng văn bản học thuật bằng mô hình ngôn ngữ chạy cục bộ. Ứng dụng dùng Tauri v2 để đưa giao diện React vào desktop shell gọn nhẹ, còn phần tải model và inference GGUF chạy trong Rust thông qua `llama.cpp`.

## Tổng Quan

Thesis Rewriter được thiết kế theo hướng offline-first:

- Lần đầu mở app, backend Rust tải model GGUF từ Hugging Face vào thư mục dữ liệu ứng dụng của hệ điều hành.
- Frontend React nhận tiến độ tải model qua Tauri event và hiển thị màn hình setup.
- Sau khi model đã tải xong, Rust load model vào bộ nhớ cục bộ bằng `llama-cpp-2`.
- Khi người dùng rewrite văn bản, frontend gọi Tauri command `rewrite_text`; backend sinh kết quả từng token và đẩy về UI bằng event `rewrite-token`.
- App không gọi API AI bên ngoài trong quá trình rewrite sau khi model đã được tải về.

## Tính Năng

- Native desktop app bằng Tauri v2, không cần bundle browser runtime riêng.
- Giao diện React + TypeScript + Tailwind CSS 4, dark mode mặc định.
- Màn hình first-launch setup với progress bar tải model.
- Editor chia đôi: input bên trái, output streaming bên phải.
- Ba style rewrite: `Academic`, `Concise`, `Professional`.
- Model GGUF được lưu trong app data folder, không bundle vào installer.
- Token streaming cho hiệu ứng gõ chữ từ local inference.

## Trạng Thái Hiện Tại

Dự án đã có scaffold và core flow cơ bản:

- Frontend: [src/App.tsx](src/App.tsx) và [src/App.css](src/App.css).
- Backend Tauri/Rust: [src-tauri/src/main.rs](src-tauri/src/main.rs).
- Cấu hình Tauri: [src-tauri/tauri.conf.json](src-tauri/tauri.conf.json).
- Dependency Rust: [src-tauri/Cargo.toml](src-tauri/Cargo.toml).

Model đang được cấu hình trong source:

```text
Repository: Qwen/Qwen3-1.7B-GGUF
File:       Qwen3-1.7B-Q8_0.gguf
Size:       khoảng 1.83 GB
License:    Apache-2.0
```

Ghi chú: tài liệu yêu cầu ban đầu có nhắc Qwen2.5-1.5B-Instruct 4-bit, nhưng code hiện tại đang dùng Qwen3-1.7B-GGUF Q8_0. README này mô tả theo đúng source hiện tại.

## Tech Stack

- Desktop: Tauri v2
- Frontend: React 19, Vite 7, TypeScript, Tailwind CSS 4
- Backend: Rust 2021
- Inference: `llama-cpp-2`
- Download streaming: `reqwest`, `tokio`, `futures-util`
- Model format: GGUF

## Yêu Cầu Hệ Thống

Cần cài sẵn:

- Node.js 20+ khuyến nghị
- npm
- Rust stable và Cargo
- Tauri system dependencies theo hệ điều hành
- Kết nối Internet cho lần tải model đầu tiên
- Tối thiểu 3-4 GB RAM trống cho model Q8_0 và context inference cơ bản

Linux cần thêm các gói WebKit/GTK theo distro. Xem hướng dẫn prerequisites của Tauri v2 nếu build trên máy mới.

## Cài Đặt

```bash
npm install
```

Nếu dependency Rust chưa được fetch, Cargo sẽ tự tải khi chạy `cargo check`, `npm run tauri dev` hoặc `npm run tauri build`.

## Chạy Development

Chạy app desktop:

```bash
npm run tauri dev
```

Chạy riêng frontend Vite để kiểm tra UI trong browser:

```bash
npm run dev
```

Lưu ý: chế độ browser chỉ kiểm tra UI. Các Tauri command như `download_model`, `load_model`, `rewrite_text` cần chạy trong Tauri runtime.

## Build

Build frontend:

```bash
npm run build
```

Kiểm tra Rust backend:

```bash
cd src-tauri
cargo check
```

Build gói desktop:

```bash
npm run tauri build
```

Output installer/binary nằm trong `src-tauri/target/release/bundle/` tùy theo platform.

## Luồng Sử Dụng

1. Mở app bằng `npm run tauri dev` hoặc bản build.
2. Nếu model chưa có, app hiện màn hình setup.
3. Bấm `Start Setup` để tải model GGUF từ Hugging Face.
4. Backend lưu model vào app data folder của hệ điều hành.
5. Khi model load xong, app chuyển sang editor.
6. Dán văn bản học thuật vào input.
7. Chọn style rewrite.
8. Bấm `Rewrite`.
9. Kết quả được stream từng token vào panel output.

## Thư Mục Lưu Model

Backend dùng `app.path().app_data_dir()` của Tauri, sau đó tạo path:

```text
<app-data-dir>/models/Qwen--Qwen3-1.7B-GGUF/Qwen3-1.7B-Q8_0.gguf
```

Vị trí `<app-data-dir>` phụ thuộc hệ điều hành và cấu hình Tauri. Thường gặp:

- Windows: trong vùng `%APPDATA%` hoặc `%LOCALAPPDATA%`
- macOS: trong `~/Library/Application Support/`
- Linux: trong `~/.local/share/`

Model không nằm trong installer và có thể được xóa thủ công nếu cần tải lại.

## Tauri Commands Và Events

Frontend gọi Rust bằng `invoke` và nghe event bằng `listen`.

### Commands

| Command | Tham số | Mô tả |
| --- | --- | --- |
| `model_status` | none | Kiểm tra model đã tải/đã load chưa |
| `download_model` | none | Tải GGUF vào app data folder, sau đó load model |
| `load_model` | none | Load model đã tải sẵn vào bộ nhớ |
| `rewrite_text` | `{ text, style }` | Rewrite văn bản và stream token về frontend |

### Events

| Event | Payload | Mô tả |
| --- | --- | --- |
| `model-download-progress` | `{ percent, downloaded_bytes, total_bytes }` | Tiến độ tải model |
| `model-load-status` | `{ downloaded, loaded, busy, model_name, model_path, model_bytes, message }` | Trạng thái model |
| `rewrite-status` | `{ message, busy }` | Trạng thái inference |
| `rewrite-token` | `{ token, done }` | Token output streaming |

## Kiến Trúc

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

State model được giữ trong `Mutex<ModelState>`:

- `LlamaBackend`
- `LlamaModel`
- path model đã load
- tên model hiển thị

Mỗi lần rewrite, backend tạo context inference riêng với:

- context window: `4096` token
- giới hạn output: `700` token
- sampler: `top_k(20)`, `top_p(0.8)`, `temp(0.7)`
- prompt system có `/no_think` để ưu tiên output ngắn gọn, không hiện reasoning

## Cấu Trúc Dự Án

```text
.
├── src/
│   ├── App.tsx              # UI setup screen và editor
│   ├── App.css              # Tailwind import và CSS phụ trợ
│   └── main.tsx             # React entrypoint
├── src-tauri/
│   ├── src/main.rs          # Tauri commands, download, load model, inference
│   ├── src/lib.rs           # Tauri library entrypoint template
│   ├── Cargo.toml           # Rust dependencies
│   ├── tauri.conf.json      # Tauri app config
│   └── capabilities/        # Tauri v2 capability permissions
├── public/                  # Static Vite assets
├── package.json             # npm scripts và frontend deps
├── vite.config.ts           # Vite config cho Tauri
└── README.md
```

## Scripts

| Script | Mô tả |
| --- | --- |
| `npm run dev` | Chạy Vite dev server |
| `npm run build` | Typecheck frontend và build Vite |
| `npm run preview` | Preview frontend build |
| `npm run tauri` | Gọi Tauri CLI |
| `npm run tauri dev` | Chạy desktop app ở dev mode |
| `npm run tauri build` | Build desktop bundle |

## Cấu Hình Model

Model được cấu hình tại [src-tauri/src/main.rs](src-tauri/src/main.rs):

```rust
const MODEL_REPO: &str = "Qwen/Qwen3-1.7B-GGUF";
const MODEL_FILE: &str = "Qwen3-1.7B-Q8_0.gguf";
const MODEL_URL: &str =
    "https://huggingface.co/Qwen/Qwen3-1.7B-GGUF/resolve/main/Qwen3-1.7B-Q8_0.gguf";
const MODEL_DISPLAY_NAME: &str = "Qwen3-1.7B-GGUF Q8_0";
```

Nếu đổi sang model khác, cần đảm bảo:

- File là GGUF tương thích llama.cpp.
- Model có chat template phù hợp, hoặc fallback `chatml` có thể chấp nhận được.
- Dung lượng model phù hợp với RAM máy người dùng.
- License model cho phép mục đích sử dụng của ứng dụng.

## Bảo Mật Và Riêng Tư

- Nội dung rewrite được xử lý cục bộ sau khi model đã được tải.
- App cần Internet để tải model lần đầu từ Hugging Face.
- Model file được lưu ngoài installer trong app data folder.
- Tauri capability hiện tại chỉ bật `core:default` và `opener:default`.

## Troubleshooting

### `Start Setup` tải model chậm hoặc lỗi

Kiểm tra kết nối Internet và dung lượng đĩa còn trống. Model Q8_0 khoảng 1.83 GB, file tạm `.part` sẽ được tạo trong lúc tải.

### App báo `Model downloaded but not loaded`

Bấm `Load Model`. Nếu vẫn lỗi, xóa file model trong app data folder rồi tải lại.

### Lỗi `Input is too long for the current 4096-token local context`

Rút ngắn input. Context hiện tại đặt `4096` token và output tối đa `700` token.

### Build Tauri trên Linux lỗi WebKit/GTK

Cài đúng system dependencies theo Tauri prerequisites cho distro đang dùng.

### Inference lặp lại hoặc chất lượng chưa ổn định

Cần tinh chỉnh sampler, prompt, `presence_penalty` hoặc chọn quantization khác. Qwen khuyến nghị tham số khác nhau cho thinking và non-thinking mode; app hiện dùng `/no_think` và sampling gần với khuyến nghị non-thinking.

## Kiểm Tra Đã Thực Hiện

Tại thời điểm cập nhật README:

```bash
npm run build
cd src-tauri && cargo check
```

Cả hai lệnh đều hoàn tất thành công.

## Roadmap Gợi Ý

- Thêm nút hủy inference đang chạy.
- Thêm tùy chọn model/quantization trong UI.
- Thêm checksum sau khi tải model.
- Thêm resume download nếu file `.part` đã tồn tại.
- Thêm logging bằng `tauri-plugin-log`.
- Thêm export/copy output.
- Thêm test cho prompt builder và model path.
- Tách backend inference thành module riêng thay vì giữ tất cả trong `main.rs`.

## Tham Khảo

- Tauri v2 docs: https://v2.tauri.app/
- Tauri command và event IPC: https://v2.tauri.app/develop/calling-rust/
- Tauri create project: https://v2.tauri.app/start/
- llama.cpp: https://github.com/ggml-org/llama.cpp
- `llama-cpp-2` crate: https://docs.rs/llama-cpp-2/latest/llama_cpp_2/
- Qwen3-1.7B-GGUF model card: https://huggingface.co/Qwen/Qwen3-1.7B-GGUF
- Qwen2.5-1.5B-Instruct-GGUF model card: https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF

## License

Chưa có file `LICENSE` trong repository. Nên thêm license cho source code trước khi public. Model Qwen3-1.7B-GGUF trên Hugging Face được công bố với license Apache-2.0; vẫn cần đọc model card và điều khoản liên quan trước khi phân phối ứng dụng.
