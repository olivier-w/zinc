# Zinc

A desktop video downloader with AI-powered subtitle generation.

## Features

- Download videos from YouTube and other platforms via yt-dlp
- Quality selection (Best, 4K, 2K, 1080p, 720p, 480p, Audio)
- Format conversion (Original, MP4, WebM, MKV)
- AI subtitle generation with multiple engines and models
- GPU-accelerated transcription via CUDA

## Transcription Engines

### Whisper (GPU) - Primary Engine

Native Rust implementation using whisper-rs with CUDA support.

| Model | Size | GPU Speed | CPU Speed |
|-------|------|-----------|-----------|
| Tiny | 75 MB | 32x | 8x |
| Base | 142 MB | 16x | 4x |
| Small | 466 MB | 6x | 2x |
| Medium | 1.5 GB | 2x | 0.5x |
| Large v3 | 3.1 GB | 1x | 0.2x |

- Supports 99+ languages with auto-detection
- Models downloaded from Hugging Face (GGML format)
- Transcription styles: sentence (natural phrases) or word (karaoke)

### Moonshine - CPU Fallback

Fast ONNX-based engine via sherpa-onnx.

| Model | Size | GPU Speed | CPU Speed |
|-------|------|-----------|-----------|
| Tiny | 35 MB | 50x | 15x |
| Base | 70 MB | 30x | 10x |

- English only
- Optimized for CPU inference

## Requirements

- **ffmpeg** - Required for audio extraction and subtitle embedding (must be in PATH)
- **CUDA Toolkit 12.x** - Optional, for GPU-accelerated transcription

## Development

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

## Tech Stack

- **Frontend:** React 19, TypeScript, Tailwind CSS, Framer Motion
- **Backend:** Tauri 2, Rust
- **Transcription:** whisper-rs (CUDA), sherpa-onnx
- **Video:** yt-dlp, ffmpeg
