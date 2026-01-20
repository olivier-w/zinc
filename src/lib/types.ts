export interface VideoFormat {
  format_id: string;
  ext: string;
  resolution: string | null;
  filesize: number | null;
  filesize_approx: number | null;
  format_note: string | null;
  fps: number | null;
  vcodec: string | null;
  acodec: string | null;
  abr: number | null;
  vbr: number | null;
}

export interface VideoInfo {
  id: string;
  title: string;
  thumbnail: string | null;
  duration: number | null;
  channel: string | null;
  view_count: number | null;
  upload_date: string | null;
  description: string | null;
  formats: VideoFormat[];
  url: string;
}

export interface Download {
  id: string;
  url: string;
  title: string;
  thumbnail: string | null;
  status: 'pending' | 'downloading' | 'completed' | 'error' | 'cancelled';
  progress: number;
  speed: string | null;
  eta: string | null;
  output_path: string | null;
  format: string;
  error: string | null;
}

export interface AppConfig {
  output_dir: string;
  default_format: string;
  default_quality: string;
  theme: 'system' | 'light' | 'dark';
}

export type FormatPreset = 'best' | '4k' | '2k' | '1080p' | '720p' | '480p' | 'audio' | 'mp3';

export interface Toast {
  id: string;
  type: 'success' | 'error' | 'info' | 'warning';
  message: string;
  duration?: number;
}

export type YtDlpStatus =
  | { status: 'not_installed' }
  | { status: 'installed'; version: string; path: string }
  | { status: 'update_available'; current: string; latest: string; path: string }
  | { status: 'error'; message: string };

export interface YtDlpInstallProgress {
  downloaded: number;
  total: number | null;
  percentage: number;
}
