import type { FormatPreset } from './types';

export const QUALITY_PRESETS: { id: FormatPreset; label: string; shortLabel: string }[] = [
  { id: 'best', label: 'Best Available', shortLabel: 'Best' },
  { id: '4k', label: '4K (2160p)', shortLabel: '4K' },
  { id: '2k', label: '2K (1440p)', shortLabel: '2K' },
  { id: '1080p', label: '1080p', shortLabel: '1080p' },
  { id: '720p', label: '720p', shortLabel: '720p' },
  { id: '480p', label: '480p', shortLabel: '480p' },
  { id: 'audio', label: 'Audio Only', shortLabel: 'Audio' },
] as const;

export const VIDEO_FORMATS = [
  { id: 'original', label: 'Original' },
  { id: 'mp4', label: 'MP4' },
  { id: 'webm', label: 'WebM' },
  { id: 'mkv', label: 'MKV' },
] as const;

export const AUDIO_FORMATS = [
  { id: 'original', label: 'Original' },
  { id: 'mp3', label: 'MP3' },
  { id: 'm4a', label: 'M4A' },
  { id: 'opus', label: 'Opus' },
] as const;

export const FORMAT_OPTIONS = [
  { id: 'original', label: 'Original' },
  { id: 'mp4', label: 'MP4' },
  { id: 'webm', label: 'WebM' },
  { id: 'mkv', label: 'MKV' },
  { id: 'mp3', label: 'MP3 (Audio)' },
] as const;

export type VideoFormatId = typeof VIDEO_FORMATS[number]['id'];
export type AudioFormatId = typeof AUDIO_FORMATS[number]['id'];

export const SUBTITLE_LANGUAGES = [
  { code: 'auto', name: 'Auto-detect' },
  { code: 'en', name: 'English' },
  { code: 'es', name: 'Spanish' },
  { code: 'fr', name: 'French' },
  { code: 'de', name: 'German' },
  { code: 'it', name: 'Italian' },
  { code: 'pt', name: 'Portuguese' },
  { code: 'ru', name: 'Russian' },
  { code: 'ja', name: 'Japanese' },
  { code: 'zh', name: 'Chinese' },
  { code: 'ko', name: 'Korean' },
  { code: 'ar', name: 'Arabic' },
  { code: 'hi', name: 'Hindi' },
  { code: 'nl', name: 'Dutch' },
  { code: 'pl', name: 'Polish' },
  { code: 'tr', name: 'Turkish' },
  { code: 'vi', name: 'Vietnamese' },
  { code: 'th', name: 'Thai' },
  { code: 'id', name: 'Indonesian' },
  { code: 'uk', name: 'Ukrainian' },
] as const;

export type SubtitleLanguageCode = typeof SUBTITLE_LANGUAGES[number]['code'];

// Transcription engine definitions
export const TRANSCRIPTION_ENGINES = [
  {
    id: 'moonshine',
    name: 'Moonshine',
    description: 'Fast, edge-optimized (5-15x realtime)',
    gpu_required: false,
    languages: ['auto', 'en', 'es', 'fr', 'de', 'it', 'pt', 'ja', 'zh'],
    models: [
      { id: 'tiny', name: 'Tiny', size: '190 MB', speed_gpu: 50, speed_cpu: 15 },
      { id: 'base', name: 'Base', size: '400 MB', speed_gpu: 30, speed_cpu: 10 },
    ],
  },
  {
    id: 'parakeet',
    name: 'Parakeet TDT',
    description: 'Fast, NVIDIA GPU (~12x realtime)',
    gpu_required: true,
    languages: ['en'],
    models: [
      { id: '0.6b', name: '0.6B', size: '1.2 GB', speed_gpu: 12, speed_cpu: 2 },
    ],
  },
  {
    id: 'whisper_cpp',
    name: 'Whisper.cpp',
    description: 'Portable, multilingual (2-8x realtime)',
    gpu_required: false,
    languages: ['auto', 'en', 'es', 'fr', 'de', 'it', 'pt', 'ru', 'ja', 'zh', 'ko', 'ar', 'hi', 'nl', 'pl', 'tr', 'vi', 'th', 'id', 'uk'],
    models: [
      { id: 'tiny', name: 'Tiny', size: '75 MB', speed_gpu: 12, speed_cpu: 8 },
      { id: 'base', name: 'Base', size: '142 MB', speed_gpu: 8, speed_cpu: 5 },
      { id: 'small', name: 'Small', size: '466 MB', speed_gpu: 4, speed_cpu: 2.5 },
    ],
  },
] as const;

// Get speed multiplier for an engine/model combination
export function getSpeedMultiplier(
  engineId: string,
  modelId: string,
  useGpu: boolean
): number {
  const engine = TRANSCRIPTION_ENGINES.find((e) => e.id === engineId);
  if (!engine) return 5; // Default fallback

  const model = engine.models.find((m) => m.id === modelId);
  if (!model) return 5;

  return useGpu ? model.speed_gpu : model.speed_cpu;
}

// Get engine-specific languages
export function getEngineLanguages(engineId: string): readonly string[] {
  const engine = TRANSCRIPTION_ENGINES.find((e) => e.id === engineId);
  return engine?.languages || ['auto', 'en'];
}
