import { useState, useCallback, useEffect, lazy, Suspense, useMemo } from 'react';
import { AnimatePresence, LayoutGroup, motion } from 'framer-motion';
import { URLInput } from './components/URLInput';
import { VideoPreview } from './components/VideoPreview';
import { DownloadsTray } from './components/DownloadsTray';
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

type LayoutState = 'empty' | 'preview' | 'active';

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

  // Derive layout state
  const layoutState: LayoutState = useMemo(() => {
    if (videoInfo || isLoadingInfo) return 'preview';
    if (downloads.length > 0) return 'active';
    return 'empty';
  }, [videoInfo, isLoadingInfo, downloads.length]);

  // Calculate bottom padding based on downloads tray
  const bottomPadding = downloads.length > 0 ? 'pb-56' : 'pb-8';

  return (
    <div className="h-screen bg-bg-primary flex flex-col overflow-hidden">
      {/* Main content - full screen, centered */}
      <main className={`flex-1 flex flex-col items-center px-6 py-8 overflow-y-auto ${bottomPadding}`}>
        <LayoutGroup>
          {/* yt-dlp setup UI */}
          {ytdlpStatus?.status === 'not_installed' && !isInstalling && (
            <motion.div
              initial={{ opacity: 0, y: -10 }}
              animate={{ opacity: 1, y: 0 }}
              className="w-full max-w-2xl flex flex-col items-center gap-4 px-6 py-8 glass rounded-2xl mb-8"
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
              className="w-full max-w-2xl flex flex-col items-center gap-4 px-6 py-8 glass rounded-2xl mb-8"
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
              className="w-full max-w-2xl flex flex-col items-center gap-4 px-6 py-8 bg-error/10 border border-error/20 rounded-2xl mb-8"
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
            <div className="w-full max-w-2xl flex items-center gap-3 px-4 py-3 bg-warning/10 border border-warning/20 rounded-lg text-warning mb-8">
              <AlertCircleIcon className="w-5 h-5 shrink-0" />
              <p className="text-sm">{ytdlpStatus.message}</p>
            </div>
          )}

          {/* URL Input - hero when empty, compact otherwise */}
          <div
            className={`w-full flex flex-col items-center ${
              layoutState === 'empty' ? 'flex-1 justify-center' : 'pt-4 mb-6'
            }`}
          >
            <URLInput
              onSubmit={handleUrlSubmit}
              isLoading={isLoadingInfo}
              disabled={!ytdlpReady || isInstalling}
              variant={layoutState === 'empty' ? 'hero' : 'compact'}
            />

            {/* Helper text only in empty state */}
            <AnimatePresence>
              {layoutState === 'empty' && ytdlpReady && (
                <motion.p
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  transition={{ duration: 0.15 }}
                  className="mt-4 text-text-tertiary text-sm helper-text"
                >
                  Press <kbd className="px-1.5 py-0.5 rounded bg-bg-tertiary border border-border text-text-secondary text-xs">Ctrl+V</kbd> to paste from clipboard
                </motion.p>
              )}
            </AnimatePresence>
          </div>

          {/* Video Preview */}
          <AnimatePresence>
            {videoInfo && (
              <motion.div
                className="w-full flex justify-center"
                initial={{ opacity: 0, y: 30 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -20, scale: 0.97 }}
              >
                <VideoPreview
                  video={videoInfo}
                  onDownload={handleDownload}
                  onClose={handleClosePreview}
                  defaultSubtitlesEnabled={config?.generate_subtitles ?? false}
                  transcriptionEngine={config?.transcription_engine ?? 'whisper_cpp'}
                  transcriptionModel={config?.transcription_model ?? 'base'}
                />
              </motion.div>
            )}
          </AnimatePresence>

          {/* Empty state message when active (downloads exist but no preview) */}
          {layoutState === 'active' && !videoInfo && !isLoadingInfo && (
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ delay: 0.1 }}
              className="text-center py-12"
            >
              <p className="text-text-secondary text-sm">
                Ready to download more
              </p>
              <p className="text-text-tertiary text-xs mt-1">
                Paste another URL above
              </p>
            </motion.div>
          )}
        </LayoutGroup>
      </main>

      {/* Floating settings button */}
      <motion.button
        onClick={() => setIsSettingsOpen(true)}
        className={`settings-float ${downloads.length > 0 ? 'tray-collapsed' : ''}`}
        whileHover={{ rotate: 45, scale: 1.05 }}
        whileTap={{ scale: 0.95 }}
        aria-label="Open settings"
      >
        <SettingsIcon className="w-5 h-5" />
      </motion.button>

      {/* Downloads tray at bottom */}
      <AnimatePresence>
        {downloads.length > 0 && (
          <DownloadsTray
            downloads={downloads}
            onCancel={cancelDownload}
            onClear={clearDownload}
            onClearCompleted={clearCompleted}
            hasCompletedDownloads={hasCompletedDownloads}
          />
        )}
      </AnimatePresence>

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
