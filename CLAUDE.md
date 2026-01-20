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
tsc -b
```

## Architecture

Zinc is a desktop video downloader built with **Tauri 2** (Rust backend) and **React 19** (TypeScript frontend).

### Frontend (`/src`)
- **Entry:** `main.tsx` → `App.tsx`
- **Components:** `/components` - UI components (URLInput, VideoPreview, DownloadList, Settings, etc.)
- **Hooks:** `/hooks` - State management (useDownload, useSettings, useTheme, useToast)
- **Tauri Bridge:** `/lib/tauri.ts` - Wraps `invoke()` calls to backend commands
- **Types:** `/lib/types.ts` - TypeScript interfaces matching Rust structs

### Backend (`/src-tauri/src`)
- **Entry:** `main.rs` → `lib.rs` (Tauri builder setup)
- **Commands:** `commands.rs` - Tauri command handlers exposed to frontend
- **Config:** `config.rs` - User settings persistence (JSON in config directory)
- **yt-dlp:** `ytdlp.rs` - Video info fetching and download execution
- **yt-dlp Manager:** `ytdlp_manager.rs` - Auto-install/update of yt-dlp binary

### IPC Pattern
1. Frontend calls `invoke('command_name', { args })` via `/lib/tauri.ts`
2. Backend handles in `commands.rs` with `#[tauri::command]` functions
3. Long operations use Tauri events for progress updates (`download-progress`, `ytdlp-install-progress`)
4. State shared via `Arc<AppState>` with `Mutex` for async access

### Key Data Flows
- **Download:** Frontend → `start_download` command → spawns yt-dlp process → progress via mpsc channel → Tauri event → frontend state update
- **Config:** Stored as JSON in OS config directory, loaded on startup, saved on change
- **yt-dlp binary:** Managed in `{app_data}/com.zinc.app/bin/`, falls back to system PATH

## Styling

- Tailwind CSS with custom CSS variables for theming
- Color tokens: `bg-primary`, `bg-secondary`, `text-primary`, `text-secondary`, `accent`, `error`, `success`, etc.
- Animations via Framer Motion
