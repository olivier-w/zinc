import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import type { AppConfig, Download, VideoInfo, YtDlpStatus, YtDlpInstallProgress, WhisperStatus, WhisperModel, WhisperInstallProgress, TranscribeProgress, SubtitleSettings, TranscriptionEngine, TranscriptionModel, TranscriptionInstallProgress, NetworkInterface } from './types';

export async function checkYtdlp(): Promise<boolean> {
  return invoke<boolean>('check_ytdlp');
}

export async function getVideoInfo(url: string): Promise<VideoInfo> {
  return invoke<VideoInfo>('get_video_info', { url });
}

export async function startDownload(
  url: string,
  format: string,
  title: string,
  thumbnail: string | null,
  subtitleSettings?: SubtitleSettings | null,
  duration?: number | null,
): Promise<string> {
  return invoke<string>('start_download', { url, format, title, thumbnail, subtitleSettings, duration });
}

export async function cancelDownload(downloadId: string): Promise<void> {
  return invoke('cancel_download', { downloadId });
}

export async function getDownloads(): Promise<Download[]> {
  return invoke<Download[]>('get_downloads');
}

export async function clearDownload(downloadId: string): Promise<void> {
  return invoke('clear_download', { downloadId });
}

export async function clearCompletedDownloads(): Promise<void> {
  return invoke('clear_completed_downloads');
}

export async function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>('get_config');
}

export async function updateConfig(config: AppConfig): Promise<void> {
  return invoke('update_config', { config });
}

export async function selectDirectory(): Promise<string | null> {
  const selected = await open({
    directory: true,
    multiple: false,
    title: 'Select Download Directory',
  });
  return selected as string | null;
}

export async function openFile(path: string): Promise<void> {
  return invoke('open_file', { path });
}

export async function openFolder(path: string): Promise<void> {
  return invoke('open_folder', { path });
}

export async function getFormatPresets(): Promise<Record<string, string>> {
  return invoke<Record<string, string>>('get_format_presets');
}

export function onDownloadProgress(
  callback: (download: Download) => void
): Promise<UnlistenFn> {
  return listen<Download>('download-progress', (event) => {
    callback(event.payload);
  });
}

// yt-dlp manager functions

export async function getYtdlpStatus(): Promise<YtDlpStatus> {
  return invoke<YtDlpStatus>('get_ytdlp_status');
}

export async function getYtdlpStatusFast(): Promise<YtDlpStatus> {
  return invoke<YtDlpStatus>('get_ytdlp_status_fast');
}

export async function installYtdlp(): Promise<string> {
  return invoke<string>('install_ytdlp');
}

export async function updateYtdlp(): Promise<string> {
  return invoke<string>('update_ytdlp');
}

export async function checkYtdlpUpdate(): Promise<string | null> {
  return invoke<string | null>('check_ytdlp_update');
}

export function onYtdlpInstallProgress(
  callback: (progress: YtDlpInstallProgress) => void
): Promise<UnlistenFn> {
  return listen<YtDlpInstallProgress>('ytdlp-install-progress', (event) => {
    callback(event.payload);
  });
}

// Whisper manager functions

export async function getWhisperStatus(): Promise<WhisperStatus> {
  return invoke<WhisperStatus>('get_whisper_status');
}

export async function installWhisper(): Promise<string> {
  return invoke<string>('install_whisper');
}

export async function downloadWhisperModel(model: string): Promise<void> {
  return invoke<void>('download_whisper_model', { model });
}

export async function getAvailableWhisperModels(): Promise<WhisperModel[]> {
  return invoke<WhisperModel[]>('get_available_whisper_models');
}

export async function checkFfmpeg(): Promise<boolean> {
  return invoke<boolean>('check_ffmpeg');
}

export function onWhisperInstallProgress(
  callback: (progress: WhisperInstallProgress) => void
): Promise<UnlistenFn> {
  return listen<WhisperInstallProgress>('whisper-install-progress', (event) => {
    callback(event.payload);
  });
}

export function onTranscribeProgress(
  callback: (progress: TranscribeProgress) => void
): Promise<UnlistenFn> {
  return listen<TranscribeProgress>('transcribe-progress', (event) => {
    callback(event.payload);
  });
}

// Transcription engine functions

export async function getTranscriptionEngines(): Promise<TranscriptionEngine[]> {
  return invoke<TranscriptionEngine[]>('get_transcription_engines');
}

export async function getEngineModels(engineId: string): Promise<TranscriptionModel[]> {
  return invoke<TranscriptionModel[]>('get_engine_models', { engineId });
}

export async function installTranscriptionEngine(engineId: string): Promise<void> {
  return invoke<void>('install_transcription_engine', { engineId });
}

export async function downloadTranscriptionModel(engineId: string, modelId: string): Promise<void> {
  return invoke<void>('download_transcription_model', { engineId, modelId });
}

export async function getTranscriptionSpeedMultiplier(
  engineId: string,
  modelId: string,
  useGpu: boolean
): Promise<number> {
  return invoke<number>('get_transcription_speed_multiplier', { engineId, modelId, useGpu });
}

export function onTranscriptionInstallProgress(
  callback: (progress: TranscriptionInstallProgress) => void
): Promise<UnlistenFn> {
  return listen<TranscriptionInstallProgress>('transcription-install-progress', (event) => {
    callback(event.payload);
  });
}

// Local file transcription functions (unified with downloads system)

export async function selectVideoFile(): Promise<string | null> {
  const selected = await open({
    multiple: false,
    title: 'Select Video File',
    filters: [
      {
        name: 'Video Files',
        extensions: ['mp4', 'mkv', 'webm', 'mov', 'avi', 'm4v', 'wmv', 'flv'],
      },
    ],
  });
  return selected as string | null;
}

export async function addLocalTranscription(
  filePath: string,
  title: string,
  engine: string,
  model: string,
  style: string
): Promise<string> {
  return invoke<string>('add_local_transcription', { filePath, title, engine, model, style });
}

export async function startLocalTranscription(taskId: string): Promise<void> {
  return invoke('start_local_transcription', { taskId });
}

export async function updateTranscriptionSettings(
  taskId: string,
  engine?: string,
  model?: string,
  style?: string
): Promise<void> {
  return invoke('update_transcription_settings', { taskId, engine, model, style });
}

// Network interface functions

export async function listNetworkInterfaces(): Promise<NetworkInterface[]> {
  return invoke<NetworkInterface[]>('list_network_interfaces');
}
