import { useState, useCallback, useEffect, lazy, Suspense } from 'react';
import { AnimatePresence, LayoutGroup, motion } from 'framer-motion';
import { URLInput } from './components/URLInput';
import { VideoPreview } from './components/VideoPreview';
import { DownloadList } from './components/DownloadList';
import { ToastContainer } from './components/Toast';
import { ProgressBar } from './components/ProgressBar';
import { SettingsIcon, AlertCircleIcon, DownloadIcon, LoaderIcon } from './components/Icons';
import { useDownload } from './hooks/useDownload';
import { useSettings } from './hooks/useSettings';
import { useToast } from './hooks/useToast';
import { getVideoInfo, getYtdlpStatus, getYtdlpStatusFast, installYtdlp, onYtdlpInstallProgress } from './lib/tauri';
// Dark theme is now the only theme - no light mode support
import type { VideoInfo, YtDlpStatus, YtDlpInstallProgress, SubtitleSettings } from './lib/types';

const Settings = lazy(() =>
  import('./components/Settings').then(m => ({ default: m.Settings }))
);

function App() {
  const [videoInfo, setVideoInfo] = useState<VideoInfo | null>(null);
  const [isLoadingInfo, setIsLoadingInfo] = useState(false);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [ytdlpStatus, setYtdlpStatus] = useState<YtDlpStatus | null>(null);
  const [isInstalling, setIsInstalling] = useState(false);
  const [installProgress, setInstallProgress] = useState<YtDlpInstallProgress | null>(null);
  const [installError, setInstallError] = useState<string | null>(null);

  const {
    downloads,
    hasActiveDownloads,
    hasCompletedDownloads,
    startDownload,
    cancelDownload,
    clearDownload,
    clearCompleted,
  } = useDownload();

  const { config, saveConfig } = useSettings();
  const { toasts, removeToast, success, error } = useToast();

  // Check yt-dlp status on mount - fast local check first, then background update check
  useEffect(() => {
    getYtdlpStatusFast()
      .then((status) => {
        setYtdlpStatus(status);
        // Check for updates in background if installed
        if (status.status === 'installed') {
          getYtdlpStatus()
            .then(setYtdlpStatus)
            .catch(() => {}); // Silently ignore update check failures
        }
      })
      .catch(() => setYtdlpStatus({ status: 'error', message: 'Failed to check yt-dlp status' }));
  }, []);

  // Listen for install progress
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    onYtdlpInstallProgress((progress) => {
      setInstallProgress(progress);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, []);

  const handleInstallYtdlp = useCallback(async () => {
    setIsInstalling(true);
    setInstallError(null);
    setInstallProgress(null);

    try {
      await installYtdlp();
      const status = await getYtdlpStatus();
      setYtdlpStatus(status);
      success('yt-dlp installed successfully');
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to install yt-dlp';
      setInstallError(message);
      error(message);
    } finally {
      setIsInstalling(false);
      setInstallProgress(null);
    }
  }, [success, error]);

  const ytdlpReady = ytdlpStatus?.status === 'installed' || ytdlpStatus?.status === 'update_available';

  const handleUrlSubmit = useCallback(async (url: string) => {
    setIsLoadingInfo(true);
    setVideoInfo(null);

    try {
      const info = await getVideoInfo(url);
      setVideoInfo(info);
    } catch (err) {
      error(err instanceof Error ? err.message : 'Failed to get video info');
    } finally {
      setIsLoadingInfo(false);
    }
  }, [error]);

  const handleDownload = useCallback(async (format: string, subtitleSettings?: SubtitleSettings) => {
    if (!videoInfo) return;

    try {
      await startDownload(videoInfo, format, subtitleSettings);
      success(`Started downloading "${videoInfo.title}"`);
      setVideoInfo(null);
    } catch (err) {
      error(err instanceof Error ? err.message : 'Failed to start download');
    }
  }, [videoInfo, startDownload, success, error]);

  const handleClosePreview = useCallback(() => {
    setVideoInfo(null);
  }, []);

  const activeCount = downloads.filter(d => d.status === 'downloading' || d.status === 'pending').length;

  return (
    <div className="h-screen bg-bg-primary flex flex-col overflow-hidden">
      {/* Header */}
      <header className="flex items-center justify-between px-6 py-4 border-b border-border">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-accent" />
            <h1 className="text-xl font-semibold text-text-primary tracking-tight">Zinc</h1>
          </div>
          {hasActiveDownloads && (
            <span className="badge-accent px-2.5 py-1 text-xs font-medium rounded-full tabular-nums">
              {activeCount} active
            </span>
          )}
        </div>
        <button
          onClick={() => setIsSettingsOpen(true)}
          className="p-2 rounded-lg text-text-secondary hover:text-text-primary hover:bg-bg-secondary transition-colors"
          aria-label="Open settings"
        >
          <SettingsIcon className="w-5 h-5" />
        </button>
      </header>

      {/* Main content - wider max-width */}
      <main className="flex-1 flex flex-col items-center px-6 py-8 gap-8 overflow-y-auto">
        {/* yt-dlp setup UI */}
        {ytdlpStatus?.status === 'not_installed' && !isInstalling && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            className="w-full max-w-3xl flex flex-col items-center gap-4 px-6 py-8 glass rounded-2xl"
          >
            <div className="flex items-center gap-3 text-text-secondary">
              <DownloadIcon className="w-6 h-6" />
              <p className="text-sm">yt-dlp is required to download videos</p>
            </div>
            <button
              onClick={handleInstallYtdlp}
              className="px-6 py-2.5 btn-gradient text-white font-medium rounded-lg"
            >
              Download yt-dlp
            </button>
          </motion.div>
        )}

        {/* Installing yt-dlp */}
        {isInstalling && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            className="w-full max-w-3xl flex flex-col items-center gap-4 px-6 py-8 glass rounded-2xl"
          >
            <div className="flex items-center gap-3 text-text-secondary">
              <LoaderIcon className="w-5 h-5 animate-spin text-accent" />
              <p className="text-sm">Downloading yt-dlp...</p>
            </div>
            {installProgress && (
              <div className="w-full max-w-xs">
                <ProgressBar percentage={installProgress.percentage} className="h-2" />
                <p className="text-xs text-text-tertiary text-center mt-2">
                  {Math.round(installProgress.percentage)}%
                </p>
              </div>
            )}
          </motion.div>
        )}

        {/* Install error */}
        {installError && !isInstalling && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            className="w-full max-w-3xl flex flex-col items-center gap-4 px-6 py-8 bg-error/10 border border-error/20 rounded-2xl"
          >
            <div className="flex items-center gap-3 text-error">
              <AlertCircleIcon className="w-5 h-5 shrink-0" />
              <p className="text-sm">{installError}</p>
            </div>
            <button
              onClick={handleInstallYtdlp}
              className="px-6 py-2.5 btn-gradient text-white font-medium rounded-lg"
            >
              Retry
            </button>
          </motion.div>
        )}

        {/* yt-dlp error status */}
        {ytdlpStatus?.status === 'error' && !installError && (
          <div className="w-full max-w-3xl flex items-center gap-3 px-4 py-3 bg-warning/10 border border-warning/20 rounded-lg text-warning">
            <AlertCircleIcon className="w-5 h-5 shrink-0" />
            <p className="text-sm">{ytdlpStatus.message}</p>
          </div>
        )}

        {/* URL Input */}
        <URLInput
          onSubmit={handleUrlSubmit}
          isLoading={isLoadingInfo}
          disabled={!ytdlpReady || isInstalling}
        />

        {/* Video Preview and Downloads - wrapped in LayoutGroup for coordinated animations */}
        <LayoutGroup>
          <AnimatePresence>
            {videoInfo && (
              <VideoPreview
                video={videoInfo}
                onDownload={handleDownload}
                onClose={handleClosePreview}
                defaultSubtitlesEnabled={config?.generate_subtitles ?? false}
                transcriptionEngine={config?.transcription_engine ?? 'whisper_cpp'}
                transcriptionModel={config?.transcription_model ?? 'base'}
              />
            )}
          </AnimatePresence>

          {/* Downloads List */}
          <DownloadList
            downloads={downloads}
            onCancel={cancelDownload}
            onClear={clearDownload}
            onClearCompleted={clearCompleted}
            hasCompletedDownloads={hasCompletedDownloads}
          />
        </LayoutGroup>

        {/* Empty state - welcoming design */}
        {downloads.length === 0 && !videoInfo && !isLoadingInfo && ytdlpReady && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ delay: 0.2 }}
            className="text-center py-16"
          >
            <div className="inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-bg-secondary border border-border mb-6">
              <DownloadIcon className="w-7 h-7 text-accent" />
            </div>
            <h3 className="text-lg font-medium text-text-primary mb-2">
              Ready to download
            </h3>
            <p className="text-text-secondary text-sm mb-4">
              Paste a video URL to get started
            </p>
            <p className="text-text-tertiary text-xs">
              Tip: Press <kbd className="px-1.5 py-0.5 rounded bg-bg-tertiary border border-border text-text-secondary">Ctrl+V</kbd> to paste from clipboard
            </p>
          </motion.div>
        )}
      </main>

      {/* Settings panel */}
      <Suspense fallback={null}>
        <Settings
          isOpen={isSettingsOpen}
          onClose={() => setIsSettingsOpen(false)}
          config={config}
          onSave={saveConfig}
        />
      </Suspense>

      {/* Toast notifications */}
      <ToastContainer toasts={toasts} onDismiss={removeToast} />
    </div>
  );
}

export default App;
