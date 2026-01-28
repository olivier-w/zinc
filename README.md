# Zinc

Desktop video downloader with AI subtitles. Built with Tauri + React.

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
