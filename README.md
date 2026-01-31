# Zinc

Desktop video downloader with AI subtitles. Built with Tauri + React.

<img width="902" height="732" alt="Screenshot 2026-01-28 183830" src="https://github.com/user-attachments/assets/8fe19fe3-e21e-4f64-a9c8-25d7a772739a" />
<img width="902" height="732" alt="Screenshot 2026-01-28 183827" src="https://github.com/user-attachments/assets/2a77b246-857f-482d-b097-afd6d0d60b20" />
<img width="902" height="732" alt="Screenshot 2026-01-28 183824" src="https://github.com/user-attachments/assets/9444e7cc-4303-44c2-9ed3-ae790ba1e9f8" />
<img width="902" height="732" alt="Screenshot 2026-01-28 183811" src="https://github.com/user-attachments/assets/be3cf635-1641-4619-ac34-17f57eff666a" />


## Disclaimer

This software is provided as-is. We're not responsible for any hardware damage or other issues that may arise from using Zinc. **Use at your own risk.**

## What it does

- Download videos from YouTube and other platforms (via yt-dlp)
- Pick your quality — 4K, 1080p, 720p, audio-only, whatever
- Convert to MP4, WebM, or MKV
- Auto-generate subtitles using local AI (Whisper with CUDA or Moonshine for CPU)
- Supports 99+ languages

## Transcription

Two engines available:

- **Whisper** — GPU-accelerated via CUDA, models from tiny (75MB) to large-v3 (3.1GB), 99+ languages
- **Moonshine** — fast CPU fallback, English only

## Requirements

- **ffmpeg** in your PATH
- **CUDA Toolkit 12.x** (optional, for GPU transcription)

## Dev

### Prerequisites

**Windows:**
- [Bun](https://bun.sh)
- CUDA Toolkit 12.x (for GPU transcription)
- Visual Studio 2022 with C++ Desktop workload
- LLVM/Clang (set `LIBCLANG_PATH` env var)

**macOS:**
- [Bun](https://bun.sh)
- Xcode Command Line Tools (`xcode-select --install`)
- Rust (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- cmake (install from [cmake.org](https://cmake.org/download/) or `brew install cmake`)

```bash
bun install          # install deps
bun run tauri:dev    # run the app (hot reload)
bun run tauri:build  # production build
bun run lint         # lint
bunx tsc -b          # type check
```

## Tech

React 19 + TypeScript + Tailwind · Tauri 2 + Rust · whisper-rs · sherpa-onnx · yt-dlp · ffmpeg

## License

MIT
