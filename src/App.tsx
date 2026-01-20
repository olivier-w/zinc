import { useState, useCallback, useEffect, lazy, Suspense } from 'react';
import { AnimatePresence, motion } from 'framer-motion';
import { URLInput } from './components/URLInput';
import { VideoPreview } from './components/VideoPreview';
import { DownloadList } from './components/DownloadList';
import { ToastContainer } from './components/Toast';
import { SettingsIcon, AlertCircleIcon, DownloadIcon, LoaderIcon } from './components/Icons';
import { useDownload } from './hooks/useDownload';
import { useSettings } from './hooks/useSettings';
import { useToast } from './hooks/useToast';
import { useTheme } from './hooks/useTheme';
import { getVideoInfo, getYtdlpStatus, installYtdlp, onYtdlpInstallProgress } from './lib/tauri';
import type { VideoInfo, YtDlpStatus, YtDlpInstallProgress } from './lib/types';

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

  // Apply theme
  useTheme(config.theme);

  // Check yt-dlp status on mount
  useEffect(() => {
    getYtdlpStatus()
      .then(setYtdlpStatus)
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

  const handleDownload = useCallback(async (format: string) => {
    if (!videoInfo) return;

    try {
      await startDownload(videoInfo, format);
      success(`Started downloading "${videoInfo.title}"`);
      setVideoInfo(null);
    } catch (err) {
      error(err instanceof Error ? err.message : 'Failed to start download');
    }
  }, [videoInfo, startDownload, success, error]);

  const handleClosePreview = useCallback(() => {
    setVideoInfo(null);
  }, []);

  return (
    <div className="min-h-screen bg-bg-primary flex flex-col">
      {/* Header */}
      <header className="flex items-center justify-between px-6 py-4 border-b border-border">
        <div className="flex items-center gap-3">
          <h1 className="text-xl font-semibold text-text-primary tracking-tight">Zinc</h1>
          {hasActiveDownloads && (
            <span className="px-2 py-0.5 text-xs font-medium bg-accent/20 text-accent rounded-full tabular-nums">
              {downloads.filter(d => d.status === 'downloading' || d.status === 'pending').length} active
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

      {/* Main content */}
      <main className="flex-1 flex flex-col items-center px-6 py-8 gap-8 overflow-y-auto">
        {/* yt-dlp setup UI */}
        {ytdlpStatus?.status === 'not_installed' && !isInstalling && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            className="w-full max-w-2xl flex flex-col items-center gap-4 px-6 py-6 bg-bg-secondary border border-border rounded-xl"
          >
            <div className="flex items-center gap-3 text-text-secondary">
              <DownloadIcon className="w-6 h-6" />
              <p className="text-sm">yt-dlp is required to download videos</p>
            </div>
            <button
              onClick={handleInstallYtdlp}
              className="px-6 py-2.5 bg-accent text-white font-medium rounded-lg hover:bg-accent/90 transition-colors"
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
            className="w-full max-w-2xl flex flex-col items-center gap-4 px-6 py-6 bg-bg-secondary border border-border rounded-xl"
          >
            <div className="flex items-center gap-3 text-text-secondary">
              <LoaderIcon className="w-5 h-5 animate-spin" />
              <p className="text-sm">Downloading yt-dlp...</p>
            </div>
            {installProgress && (
              <div className="w-full max-w-xs">
                <div className="h-2 bg-bg-tertiary rounded-full overflow-hidden">
                  <motion.div
                    className="h-full bg-accent"
                    initial={{ width: 0 }}
                    animate={{ width: `${installProgress.percentage}%` }}
                    transition={{ duration: 0.2 }}
                  />
                </div>
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
            className="w-full max-w-2xl flex flex-col items-center gap-4 px-6 py-6 bg-error/10 border border-error/20 rounded-xl"
          >
            <div className="flex items-center gap-3 text-error">
              <AlertCircleIcon className="w-5 h-5 shrink-0" />
              <p className="text-sm">{installError}</p>
            </div>
            <button
              onClick={handleInstallYtdlp}
              className="px-6 py-2.5 bg-accent text-white font-medium rounded-lg hover:bg-accent/90 transition-colors"
            >
              Retry
            </button>
          </motion.div>
        )}

        {/* yt-dlp error status */}
        {ytdlpStatus?.status === 'error' && !installError && (
          <div className="w-full max-w-2xl flex items-center gap-3 px-4 py-3 bg-warning/10 border border-warning/20 rounded-lg text-warning">
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

        {/* Video Preview */}
        <AnimatePresence mode="wait">
          {videoInfo && (
            <VideoPreview
              video={videoInfo}
              onDownload={handleDownload}
              onClose={handleClosePreview}
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

        {/* Empty state */}
        {downloads.length === 0 && !videoInfo && !isLoadingInfo && (
          <div className="text-center py-12">
            <p className="text-text-tertiary text-sm">
              Paste a video URL above to get started
            </p>
          </div>
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
