import type { FormatPreset } from './types';

export const QUALITY_PRESETS: { id: FormatPreset; label: string; shortLabel: string }[] = [
  { id: 'best', label: 'Best Available', shortLabel: 'Best' },
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
