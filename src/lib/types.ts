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

export interface SubtitleSettings {
  enabled: boolean;
  engine: string;  // "whisper_cpp", "moonshine", "parakeet"
  model: string;
}

export interface Download {
  id: string;
  url: string;
  title: string;
  thumbnail: string | null;
  status: 'pending' | 'downloading' | 'completed' | 'error' | 'cancelled' | 'transcribing' | string;
  progress: number;
  speed: string | null;
  eta: string | null;
  output_path: string | null;
  format: string;
  error: string | null;
  duration: number | null;
  whisper_model: string | null;
  transcription_engine: string | null;
}

export interface AppConfig {
  output_dir: string;
  default_format: string;
  default_quality: string;
  theme: 'system' | 'light' | 'dark';
  generate_subtitles: boolean;
  whisper_model: string;
  transcription_engine: string;
  transcription_model: string;
  network_interface: string | null; // IPv4 address or null for any interface
}

export interface NetworkInterface {
  id: string;           // Adapter GUID or name
  name: string;         // Friendly name (e.g., "ProtonVPN", "Ethernet")
  ipv4: string | null;  // IPv4 address
  is_up: boolean;       // Connection status
}

// Transcription engine types
// Rust serde serializes unit variants as strings and struct variants as objects
export type EngineStatus =
  | 'Available'
  | 'NotInstalled'
  | { Unavailable: { reason: string } };

export interface TranscriptionModel {
  id: string;
  name: string;
  size: string;
  installed: boolean;
  speed_gpu: number;
  speed_cpu: number;
}

export interface TranscriptionEngine {
  id: string;
  name: string;
  description: string;
  status: EngineStatus;
  gpu_required: boolean;
  gpu_available: boolean;
  languages: string[];
  models: TranscriptionModel[];
}

export interface TranscriptionInstallProgress {
  downloaded: number;
  total: number | null;
  percentage: number;
  stage: string;
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

export type WhisperStatus =
  | { status: 'not_installed' }
  | { status: 'installed'; version: string; path: string }
  | { status: 'model_missing'; version: string; path: string }
  | { status: 'error'; message: string };

export interface WhisperModel {
  id: string;
  name: string;
  size: string;
  installed: boolean;
}

export interface WhisperInstallProgress {
  downloaded: number;
  total: number | null;
  percentage: number;
  stage: string;
}

export interface TranscribeProgress {
  stage: string;
  progress: number;
  message: string;
}

export interface ParakeetGpuStatus {
  python_available: boolean;
  sherpa_onnx_installed: boolean;
  cuda_dlls_ready: boolean;
  gpu_available: boolean;
}

export interface ParakeetGpuSetupProgress {
  downloaded: number;
  total: number | null;
  percentage: number;
  stage: string;
}
