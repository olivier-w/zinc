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

// Transcription engine definitions - these are fallback defaults
// The actual engines and models are fetched from the backend
export const TRANSCRIPTION_ENGINES = [
  {
    id: 'whisper_rs',
    name: 'Whisper (GPU)',
    description: 'Fast native GPU engine (16-32x realtime)',
    gpu_required: false,
    models: [
      { id: 'tiny', name: 'Tiny', size: '75 MB', speed_gpu: 32, speed_cpu: 8 },
      { id: 'base', name: 'Base', size: '142 MB', speed_gpu: 16, speed_cpu: 4 },
      { id: 'small', name: 'Small', size: '466 MB', speed_gpu: 6, speed_cpu: 2 },
      { id: 'medium', name: 'Medium', size: '1.5 GB', speed_gpu: 2, speed_cpu: 0.5 },
      { id: 'large-v3', name: 'Large v3', size: '3.1 GB', speed_gpu: 1, speed_cpu: 0.2 },
    ],
  },
  {
    id: 'moonshine',
    name: 'Moonshine',
    description: 'Fast CPU fallback (5-15x realtime)',
    gpu_required: false,
    models: [
      { id: 'tiny', name: 'Tiny', size: '190 MB', speed_gpu: 50, speed_cpu: 15 },
      { id: 'base', name: 'Base', size: '400 MB', speed_gpu: 30, speed_cpu: 10 },
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
