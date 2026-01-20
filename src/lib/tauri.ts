import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import type { AppConfig, Download, VideoInfo, YtDlpStatus, YtDlpInstallProgress } from './types';

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
  thumbnail: string | null
): Promise<string> {
  return invoke<string>('start_download', { url, format, title, thumbnail });
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
