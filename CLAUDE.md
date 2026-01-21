# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Important: Windows Environment

This project runs on Windows. **Never redirect output to `/dev/null` or `nul`** - these create problematic files on Windows. If you need to suppress output, use alternative approaches or simply don't redirect.

## Build & Development Commands

```bash
# Install dependencies
bun install

# Development - frontend only (hot reload on port 5173)
bun run dev

# Development - full Tauri app with hot reload
bun run tauri:dev

# Build for production
bun run tauri:build

# Lint
bun run lint

# Type check
bunx tsc -b
```

## Architecture

Zinc is a desktop video downloader built with **Tauri 2** (Rust backend) and **React 19** (TypeScript frontend).

### Frontend (`/src`)
- **Entry:** `main.tsx` → `App.tsx`
- **Components:** `/components` - UI components (URLInput, VideoPreview, DownloadList, Settings, etc.)
- **Hooks:** `/hooks` - State management (useDownload, useSettings, useToast)
- **Tauri Bridge:** `/lib/tauri.ts` - Wraps `invoke()` calls to backend commands
- **Types:** `/lib/types.ts` - TypeScript interfaces matching Rust structs

### Backend (`/src-tauri/src`)
- **Entry:** `main.rs` → `lib.rs` (Tauri builder setup)
- **Commands:** `commands.rs` - Tauri command handlers exposed to frontend
- **Config:** `config.rs` - User settings persistence (JSON in config directory)
- **yt-dlp:** `ytdlp.rs` - Video info fetching and download execution
- **yt-dlp Manager:** `ytdlp_manager.rs` - Auto-install/update of yt-dlp binary
- **Transcription:** `transcription/` - Multi-engine transcription system
- **Transcription Manager:** `transcription_manager.rs` - Orchestrates transcription pipeline

### Transcription System (`/src-tauri/src/transcription/`)

The transcription system uses a trait-based plugin architecture:

- **`engine.rs`** - Defines `TranscriptionEngine` trait and common types
- **`mod.rs`** - `TranscriptionDispatcher` manages available engines
- **Engine implementations:**
  - `moonshine.rs` - Moonshine via sherpa-onnx (fast, English-only)
  - `parakeet.rs` - Parakeet TDT via Python/sherpa-onnx (GPU-optimized, 25 European languages)
  - `whisper_cpp.rs` - whisper.cpp (multi-language, most accurate)

**Adding a new engine:** Implement `TranscriptionEngine` trait and register in `TranscriptionDispatcher::new()`.

**Parakeet GPU:** Uses Python with sherpa-onnx for CUDA support. The Python script is bundled in `src-tauri/resources/` and deployed at runtime. GPU setup installs pip packages and copies CUDA DLLs to `sherpa_onnx/lib/`.

### IPC Pattern
1. Frontend calls `invoke('command_name', { args })` via `/lib/tauri.ts`
2. Backend handles in `commands.rs` with `#[tauri::command]` functions
3. Long operations use Tauri events for progress updates (`download-progress`, `ytdlp-install-progress`, `transcribe-progress`, `parakeet-gpu-setup-progress`)
4. State shared via `Arc<AppState>` with `Mutex` for async access

### Key Data Flows
- **Download:** Frontend → `start_download` command → spawns yt-dlp process → progress via mpsc channel → Tauri event → frontend state update
- **Download with subtitles:** After video download completes: extract audio (ffmpeg, 16kHz mono WAV) → transcribe (selected engine) → generate SRT → embed subtitles (ffmpeg) → replace original video
- **Config:** Stored as JSON in OS config directory, loaded on startup, saved on change
- **Binaries:** Managed in `{app_data}/com.zinc.app/bin/`, falls back to system PATH
- **Models:** Stored in `{app_data}/com.zinc.app/models/{engine}/`

### Bundled Resources (`/src-tauri/resources/`)
Files here are bundled with the app via `tauri.conf.json` resources config and deployed at runtime:
- `transcribe_parakeet.py` - Python script for CUDA-accelerated Parakeet transcription

## Styling

- Tailwind CSS with custom CSS variables (dark theme only)
- Color tokens: `bg-primary`, `bg-secondary`, `text-primary`, `text-secondary`, `accent`, `error`, `success`, `warning`
- Animations via Framer Motion

## External Dependencies

- **yt-dlp:** Downloaded from GitHub releases, used for video downloading
- **sherpa-onnx:** Downloaded from GitHub releases, used for Moonshine transcription; Python package for Parakeet CUDA
- **whisper.cpp:** Downloaded from GitHub releases, used for Whisper transcription
- **ffmpeg:** Required for subtitle generation (audio extraction and embedding) - must be installed by user and available in PATH
- **Python 3.10+:** Required for Parakeet GPU acceleration (optional - falls back to CPU)
