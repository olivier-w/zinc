import { useState, useCallback, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import type { AppConfig, YtDlpStatus, YtDlpInstallProgress } from '@/lib/types';
import { selectDirectory, getYtdlpStatus, updateYtdlp, checkYtdlpUpdate, onYtdlpInstallProgress } from '@/lib/tauri';
import { cn, truncate } from '@/lib/utils';
import { FolderIcon, XIcon, ChevronDownIcon, RefreshIcon, CheckIcon, LoaderIcon } from './Icons';

interface SettingsProps {
  isOpen: boolean;
  onClose: () => void;
  config: AppConfig;
  onSave: (config: Partial<AppConfig>) => Promise<void>;
}

const qualityOptions = [
  { value: 'best', label: 'Best Available' },
  { value: '1080p', label: '1080p' },
  { value: '720p', label: '720p' },
  { value: '480p', label: '480p' },
  { value: 'audio', label: 'Audio Only' },
];

const formatOptions = [
  { value: 'original', label: 'Original' },
  { value: 'mp4', label: 'MP4' },
  { value: 'webm', label: 'WebM' },
  { value: 'mkv', label: 'MKV' },
  { value: 'mp3', label: 'MP3 (Audio)' },
];


export function Settings({ isOpen, onClose, config, onSave }: SettingsProps) {
  const [isSelectingDir, setIsSelectingDir] = useState(false);
  const [ytdlpStatus, setYtdlpStatus] = useState<YtDlpStatus | null>(null);
  const [isCheckingUpdate, setIsCheckingUpdate] = useState(false);
  const [isUpdating, setIsUpdating] = useState(false);
  const [updateProgress, setUpdateProgress] = useState<YtDlpInstallProgress | null>(null);
  const [availableUpdate, setAvailableUpdate] = useState<string | null>(null);

  // Fetch yt-dlp status when settings open
  useEffect(() => {
    if (isOpen) {
      getYtdlpStatus().then(setYtdlpStatus).catch(() => {});
      setAvailableUpdate(null);
    }
  }, [isOpen]);

  // Listen for update progress
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    onYtdlpInstallProgress((progress) => {
      setUpdateProgress(progress);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, []);

  const handleCheckUpdate = useCallback(async () => {
    setIsCheckingUpdate(true);
    setAvailableUpdate(null);
    try {
      const latest = await checkYtdlpUpdate();
      setAvailableUpdate(latest);
    } catch (err) {
      console.error('Failed to check for updates:', err);
    } finally {
      setIsCheckingUpdate(false);
    }
  }, []);

  const handleUpdate = useCallback(async () => {
    setIsUpdating(true);
    setUpdateProgress(null);
    try {
      await updateYtdlp();
      const status = await getYtdlpStatus();
      setYtdlpStatus(status);
      setAvailableUpdate(null);
    } catch (err) {
      console.error('Failed to update yt-dlp:', err);
    } finally {
      setIsUpdating(false);
      setUpdateProgress(null);
    }
  }, []);

  const handleSelectDirectory = useCallback(async () => {
    setIsSelectingDir(true);
    try {
      const dir = await selectDirectory();
      if (dir) {
        await onSave({ output_dir: dir });
      }
    } catch (err) {
      console.error('Failed to select directory:', err);
    } finally {
      setIsSelectingDir(false);
    }
  }, [onSave]);

  const handleQualityChange = useCallback(async (quality: string) => {
    await onSave({ default_quality: quality });
  }, [onSave]);

  const handleFormatChange = useCallback(async (format: string) => {
    await onSave({ default_format: format });
  }, [onSave]);


  return (
    <AnimatePresence>
      {isOpen && (
        <>
          {/* Backdrop */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 bg-black/50 backdrop-blur-sm z-40"
            onClick={onClose}
          />

          {/* Panel */}
          <motion.div
            initial={{ opacity: 0, x: 300 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: 300 }}
            transition={{ type: 'spring', stiffness: 400, damping: 30 }}
            className="fixed top-0 right-0 bottom-0 w-full max-w-md bg-bg-secondary border-l border-border z-50 overflow-y-auto"
          >
            {/* Header */}
            <div className="flex items-center justify-between p-4 border-b border-border sticky top-0 bg-bg-secondary">
              <h2 className="text-lg font-semibold text-text-primary">Settings</h2>
              <button
                onClick={onClose}
                className="p-2 rounded-lg text-text-secondary hover:text-text-primary hover:bg-bg-tertiary transition-colors"
                aria-label="Close settings"
              >
                <XIcon className="w-5 h-5" />
              </button>
            </div>

            <div className="p-4 space-y-6">
              {/* Output Directory */}
              <section>
                <label className="block text-sm font-medium text-text-primary mb-2">
                  Download Location
                </label>
                <button
                  onClick={handleSelectDirectory}
                  disabled={isSelectingDir}
                  className={cn(
                    'w-full flex items-center gap-3 px-4 py-3',
                    'bg-bg-tertiary rounded-lg border border-border',
                    'hover:border-border-hover transition-colors',
                    'text-left disabled:opacity-50'
                  )}
                >
                  <FolderIcon className="w-5 h-5 text-text-secondary shrink-0" />
                  <span className="text-sm text-text-primary truncate flex-1">
                    {truncate(config.output_dir || 'Select folder...', 40)}
                  </span>
                  <ChevronDownIcon className="w-4 h-4 text-text-tertiary shrink-0 rotate-[-90deg]" />
                </button>
              </section>

              {/* Default Quality */}
              <section>
                <label className="block text-sm font-medium text-text-primary mb-2">
                  Default Quality
                </label>
                <div className="grid grid-cols-2 gap-2">
                  {qualityOptions.map((option) => (
                    <button
                      key={option.value}
                      onClick={() => handleQualityChange(option.value)}
                      className={cn(
                        'px-4 py-2.5 rounded-lg text-sm font-medium transition-colors',
                        config.default_quality === option.value
                          ? 'bg-accent text-white'
                          : 'bg-bg-tertiary text-text-secondary hover:text-text-primary'
                      )}
                    >
                      {option.label}
                    </button>
                  ))}
                </div>
              </section>

              {/* Default Format */}
              <section>
                <label className="block text-sm font-medium text-text-primary mb-2">
                  Default Format
                </label>
                <div className="grid grid-cols-2 gap-2">
                  {formatOptions.map((option) => (
                    <button
                      key={option.value}
                      onClick={() => handleFormatChange(option.value)}
                      className={cn(
                        'px-4 py-2.5 rounded-lg text-sm font-medium transition-colors',
                        config.default_format === option.value
                          ? 'bg-accent text-white'
                          : 'bg-bg-tertiary text-text-secondary hover:text-text-primary'
                      )}
                    >
                      {option.label}
                    </button>
                  ))}
                </div>
              </section>

              {/* yt-dlp */}
              <section className="pt-4 border-t border-border">
                <label className="block text-sm font-medium text-text-primary mb-3">
                  yt-dlp
                </label>

                {ytdlpStatus?.status === 'installed' || ytdlpStatus?.status === 'update_available' ? (
                  <div className="space-y-3">
                    <div className="flex items-center justify-between px-4 py-3 bg-bg-tertiary rounded-lg">
                      <div>
                        <p className="text-sm text-text-primary">
                          Version {ytdlpStatus.status === 'installed' ? ytdlpStatus.version : ytdlpStatus.current}
                        </p>
                        <p className="text-xs text-text-tertiary mt-0.5">Managed by Zinc</p>
                      </div>
                      <div className="flex items-center gap-2">
                        {availableUpdate ? (
                          <button
                            onClick={handleUpdate}
                            disabled={isUpdating}
                            className={cn(
                              'flex items-center gap-2 px-3 py-1.5 text-sm font-medium rounded-lg transition-colors',
                              'bg-accent text-white hover:bg-accent/90 disabled:opacity-50'
                            )}
                          >
                            {isUpdating ? (
                              <>
                                <LoaderIcon className="w-4 h-4 animate-spin" />
                                Updating...
                              </>
                            ) : (
                              <>Update to {availableUpdate}</>
                            )}
                          </button>
                        ) : availableUpdate === null && !isCheckingUpdate ? (
                          <button
                            onClick={handleCheckUpdate}
                            disabled={isCheckingUpdate}
                            className={cn(
                              'flex items-center gap-2 px-3 py-1.5 text-sm font-medium rounded-lg transition-colors',
                              'bg-bg-secondary text-text-secondary hover:text-text-primary hover:bg-bg-tertiary'
                            )}
                          >
                            <RefreshIcon className="w-4 h-4" />
                            Check for Updates
                          </button>
                        ) : isCheckingUpdate ? (
                          <span className="flex items-center gap-2 text-sm text-text-tertiary">
                            <LoaderIcon className="w-4 h-4 animate-spin" />
                            Checking...
                          </span>
                        ) : (
                          <span className="flex items-center gap-2 text-sm text-success">
                            <CheckIcon className="w-4 h-4" />
                            Up to date
                          </span>
                        )}
                      </div>
                    </div>

                    {isUpdating && updateProgress && (
                      <div className="px-4">
                        <div className="h-1.5 bg-bg-tertiary rounded-full overflow-hidden">
                          <motion.div
                            className="h-full bg-accent"
                            initial={{ width: 0 }}
                            animate={{ width: `${updateProgress.percentage}%` }}
                            transition={{ duration: 0.2 }}
                          />
                        </div>
                      </div>
                    )}

                    {ytdlpStatus?.status === 'update_available' && !availableUpdate && (
                      <p className="text-xs text-accent px-1">
                        Update available: {ytdlpStatus.latest}
                      </p>
                    )}
                  </div>
                ) : ytdlpStatus?.status === 'not_installed' ? (
                  <p className="text-sm text-text-tertiary">
                    yt-dlp is not installed. Return to the main screen to install it.
                  </p>
                ) : ytdlpStatus?.status === 'error' ? (
                  <p className="text-sm text-error">
                    {ytdlpStatus.message}
                  </p>
                ) : (
                  <p className="text-sm text-text-tertiary">
                    Loading...
                  </p>
                )}
              </section>
            </div>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}
