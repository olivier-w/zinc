import { useState, useCallback, useEffect } from 'react';
import { motion, AnimatePresence } from 'motion/react';
import type { AppConfig, YtDlpStatus, YtDlpInstallProgress, WhisperStatus, TranscriptionEngine, TranscriptionInstallProgress, NetworkInterface } from '@/lib/types';
import { selectDirectory, getYtdlpStatus, updateYtdlp, checkYtdlpUpdate, onYtdlpInstallProgress, getWhisperStatus, checkFfmpeg, getTranscriptionEngines, downloadTranscriptionModel, onTranscriptionInstallProgress, listNetworkInterfaces } from '@/lib/tauri';
import { cn, truncate } from '@/lib/utils';
import { QUALITY_PRESETS, FORMAT_OPTIONS } from '@/lib/constants';
import { FolderIcon, XIcon, ChevronDownIcon, RefreshIcon, CheckIcon, LoaderIcon, DownloadIcon } from './Icons';
import { ProgressBar } from './ProgressBar';

interface SettingsProps {
  isOpen: boolean;
  onClose: () => void;
  config: AppConfig;
  onSave: (config: Partial<AppConfig>) => Promise<void>;
}


export function Settings({ isOpen, onClose, config, onSave }: SettingsProps) {
  const [isSelectingDir, setIsSelectingDir] = useState(false);
  const [ytdlpStatus, setYtdlpStatus] = useState<YtDlpStatus | null>(null);
  const [isCheckingUpdate, setIsCheckingUpdate] = useState(false);
  const [isUpdating, setIsUpdating] = useState(false);
  const [updateProgress, setUpdateProgress] = useState<YtDlpInstallProgress | null>(null);
  const [availableUpdate, setAvailableUpdate] = useState<string | null>(null);

  // Whisper state (legacy - kept for existing installations display)
  const [whisperStatus, setWhisperStatus] = useState<WhisperStatus | null>(null);
  const [hasFfmpeg, setHasFfmpeg] = useState<boolean | null>(null);

  // Transcription engine state
  const [engines, setEngines] = useState<TranscriptionEngine[]>([]);
  const [isDownloadingEngineModel, setIsDownloadingEngineModel] = useState<{engine: string, model: string} | null>(null);
  const [engineProgress, setEngineProgress] = useState<TranscriptionInstallProgress | null>(null);
  const [downloadError, setDownloadError] = useState<string | null>(null);

  // Network interface state
  const [networkInterfaces, setNetworkInterfaces] = useState<NetworkInterface[]>([]);
  const [isNetworkDropdownOpen, setIsNetworkDropdownOpen] = useState(false);

  // Fetch yt-dlp status when settings open
  useEffect(() => {
    if (isOpen) {
      getYtdlpStatus().then((status) => {
        setYtdlpStatus(status);
        if (status.status === 'update_available') {
          setAvailableUpdate(status.latest);
        }
      }).catch(() => {});
      getWhisperStatus().then(setWhisperStatus).catch(() => {});
      checkFfmpeg().then(setHasFfmpeg).catch(() => setHasFfmpeg(false));
      getTranscriptionEngines().then(setEngines).catch(() => {});
      listNetworkInterfaces().then(setNetworkInterfaces).catch(() => setNetworkInterfaces([]));
      setAvailableUpdate(null);
      setDownloadError(null);
      setIsNetworkDropdownOpen(false);
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

  // Listen for transcription engine install/download progress
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    onTranscriptionInstallProgress((progress) => {
      setEngineProgress(progress);
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

  const handleSubtitleToggle = useCallback(async () => {
    await onSave({ generate_subtitles: !config.generate_subtitles });
  }, [onSave, config.generate_subtitles]);

  // Transcription engine handlers
  const handleEngineChange = useCallback(async (engineId: string) => {
    // Find the engine and its first installed model
    const engine = engines.find(e => e.id === engineId);
    const installedModel = engine?.models.find(m => m.installed);

    // Save both engine and model in a single call to avoid race conditions
    const updates: Partial<AppConfig> = { transcription_engine: engineId };
    if (installedModel) {
      updates.transcription_model = installedModel.id;
    }

    await onSave(updates);
  }, [onSave, engines]);

  const handleEngineModelChange = useCallback(async (modelId: string) => {
    await onSave({ transcription_model: modelId });
  }, [onSave]);

  const handleDownloadEngineModel = useCallback(async (engineId: string, modelId: string) => {
    setIsDownloadingEngineModel({ engine: engineId, model: modelId });
    setEngineProgress(null);
    setDownloadError(null);
    try {
      await downloadTranscriptionModel(engineId, modelId);
      const updatedEngines = await getTranscriptionEngines();
      setEngines(updatedEngines);
    } catch (err) {
      console.error('Failed to download engine model:', err);
      const errorMessage = err instanceof Error ? err.message : String(err);
      setDownloadError(errorMessage);
    } finally {
      setIsDownloadingEngineModel(null);
      setEngineProgress(null);
    }
  }, []);

  const handleNetworkInterfaceChange = useCallback(async (ipv4: string | null) => {
    await onSave({ network_interface: ipv4 });
    setIsNetworkDropdownOpen(false);
  }, [onSave]);

  // Helper to check if an engine is available
  const isEngineAvailable = (engine: TranscriptionEngine) => {
    return engine.status === 'Available';
  };

  // Helper to check if an engine is not installed
  const isEngineNotInstalled = (engine: TranscriptionEngine) => {
    return engine.status === 'NotInstalled';
  };

  // Helper to check if an engine is unavailable due to missing GPU
  const isEngineUnavailable = (engine: TranscriptionEngine) => {
    return typeof engine.status === 'object' && 'Unavailable' in engine.status;
  };

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
                  {QUALITY_PRESETS.map((option) => (
                    <button
                      key={option.id}
                      onClick={() => handleQualityChange(option.id)}
                      className={cn(
                        'px-4 py-2.5 rounded-lg text-sm font-medium transition-colors',
                        config.default_quality === option.id
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
                  {FORMAT_OPTIONS.map((option) => (
                    <button
                      key={option.id}
                      onClick={() => handleFormatChange(option.id)}
                      className={cn(
                        'px-4 py-2.5 rounded-lg text-sm font-medium transition-colors',
                        config.default_format === option.id
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

                    {/* Update Channel */}
                    <div>
                      <label className="block text-xs font-medium text-text-secondary px-1 mb-2">
                        Update Channel
                      </label>
                      <div className="grid grid-cols-3 gap-2">
                        {(['stable', 'nightly', 'master'] as const).map((ch) => (
                          <button
                            key={ch}
                            onClick={async () => {
                              await onSave({ ytdlp_channel: ch });
                              setAvailableUpdate(null);
                              setIsCheckingUpdate(true);
                              try {
                                const latest = await checkYtdlpUpdate();
                                setAvailableUpdate(latest);
                              } catch {
                                // silently fail — user can still manually check
                              } finally {
                                setIsCheckingUpdate(false);
                              }
                            }}
                            className={cn(
                              'px-4 py-2.5 rounded-lg text-sm font-medium transition-colors capitalize',
                              config.ytdlp_channel === ch
                                ? 'bg-accent text-white'
                                : 'bg-bg-tertiary text-text-secondary hover:text-text-primary'
                            )}
                          >
                            {ch}
                          </button>
                        ))}
                      </div>
                      <p className="text-xs text-text-tertiary mt-2 px-1">
                        Nightly and master builds include the latest fixes but may be less stable
                      </p>
                    </div>

                    {isUpdating && updateProgress && (
                      <div className="px-4">
                        <ProgressBar percentage={updateProgress.percentage} />
                      </div>
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

              {/* Network Interface */}
              <section className="pt-4 border-t border-border">
                <label className="block text-sm font-medium text-text-primary mb-2">
                  Network Interface
                </label>
                <p className="text-xs text-text-tertiary mb-3">
                  Select which network adapter to use for downloads
                </p>

                <div className="relative">
                  <button
                    onClick={() => setIsNetworkDropdownOpen(!isNetworkDropdownOpen)}
                    className={cn(
                      'w-full flex items-center justify-between px-4 py-3',
                      'bg-bg-tertiary rounded-lg border border-border',
                      'hover:border-border-hover transition-colors text-left'
                    )}
                  >
                    <span className="text-sm text-text-primary">
                      {config.network_interface
                        ? (() => {
                            const iface = networkInterfaces.find(i => i.ipv4 === config.network_interface);
                            return iface
                              ? `${iface.name} (${iface.ipv4})`
                              : `${config.network_interface} (unavailable)`;
                          })()
                        : 'Any interface (default)'}
                    </span>
                    <ChevronDownIcon className={cn(
                      'w-4 h-4 text-text-tertiary transition-transform',
                      isNetworkDropdownOpen && 'rotate-180'
                    )} />
                  </button>

                  {/* Warning if selected interface is unavailable */}
                  {config.network_interface && !networkInterfaces.find(i => i.ipv4 === config.network_interface) && (
                    <p className="text-xs text-warning mt-2 px-1">
                      Previously selected interface is no longer available
                    </p>
                  )}

                  {/* Dropdown */}
                  {isNetworkDropdownOpen && (
                    <div className="absolute top-full left-0 right-0 mt-1 bg-bg-tertiary border border-border rounded-lg shadow-lg z-10 max-h-64 overflow-y-auto">
                      {/* Default option */}
                      <button
                        onClick={() => handleNetworkInterfaceChange(null)}
                        className={cn(
                          'w-full px-4 py-3 text-left text-sm transition-colors',
                          'hover:bg-bg-secondary',
                          config.network_interface === null && 'bg-accent/10 text-accent'
                        )}
                      >
                        Any interface (default)
                      </button>

                      {/* Available interfaces */}
                      {networkInterfaces.map((iface) => {
                        const isSelected = config.network_interface === iface.ipv4;
                        const isDisabled = !iface.is_up || !iface.ipv4;

                        return (
                          <button
                            key={iface.id}
                            onClick={() => !isDisabled && iface.ipv4 && handleNetworkInterfaceChange(iface.ipv4)}
                            disabled={isDisabled}
                            className={cn(
                              'w-full px-4 py-3 text-left text-sm transition-colors',
                              'hover:bg-bg-secondary',
                              isSelected && 'bg-accent/10 text-accent',
                              isDisabled && 'opacity-50 cursor-not-allowed hover:bg-transparent'
                            )}
                          >
                            <span className="flex items-center justify-between">
                              <span>
                                {iface.name}
                                {iface.ipv4 && (
                                  <span className="text-text-tertiary ml-2">({iface.ipv4})</span>
                                )}
                              </span>
                              {!iface.is_up && (
                                <span className="text-xs text-text-tertiary">Disconnected</span>
                              )}
                            </span>
                          </button>
                        );
                      })}

                      {networkInterfaces.length === 0 && (
                        <p className="px-4 py-3 text-sm text-text-tertiary">
                          No network interfaces found
                        </p>
                      )}
                    </div>
                  )}
                </div>
              </section>

              {/* Subtitles */}
              <section className="pt-4 border-t border-border">
                <label className="block text-sm font-medium text-text-primary mb-3">
                  Subtitles
                </label>

                {/* Toggle */}
                <div className="flex items-center justify-between px-4 py-3 bg-bg-tertiary rounded-lg mb-3">
                  <div>
                    <p className="text-sm text-text-primary">Generate subtitles</p>
                    <p className="text-xs text-text-tertiary mt-0.5">Auto-transcribe audio</p>
                  </div>
                  <button
                    onClick={handleSubtitleToggle}
                    className={cn(
                      'relative w-11 h-6 rounded-full transition-colors',
                      config.generate_subtitles ? 'bg-accent' : 'bg-bg-secondary'
                    )}
                  >
                    <span
                      className={cn(
                        'absolute top-1 w-4 h-4 rounded-full bg-white transition-transform',
                        config.generate_subtitles ? 'left-6' : 'left-1'
                      )}
                    />
                  </button>
                </div>

                {/* ffmpeg warning */}
                {hasFfmpeg === false && (
                  <div className="px-4 py-3 bg-warning/10 border border-warning/20 rounded-lg mb-3">
                    <p className="text-sm text-warning">
                      ffmpeg is required for subtitle generation but was not found. Please install ffmpeg.
                    </p>
                  </div>
                )}

                {/* Transcription Engine Selection */}
                {config.generate_subtitles && (
                  <div className="space-y-3">
                    <label className="block text-xs font-medium text-text-secondary px-1">
                      Transcription Engine
                    </label>

                    {engines.map((engine) => {
                      const isSelected = config.transcription_engine === engine.id;
                      const available = isEngineAvailable(engine);
                      const unavailable = isEngineUnavailable(engine);
                      const notInstalled = isEngineNotInstalled(engine);
                      const selectedEngineModels = engine.models;
                      const hasInstalledModel = selectedEngineModels.some(m => m.installed);

                      return (
                        <div
                          key={engine.id}
                          className={cn(
                            'rounded-lg border transition-colors',
                            isSelected ? 'border-accent bg-accent/5' : 'border-border bg-bg-tertiary',
                            unavailable && 'opacity-60'
                          )}
                        >
                          {/* Engine Header */}
                          <button
                            onClick={() => handleEngineChange(engine.id)}
                            disabled={unavailable || !available || !hasInstalledModel}
                            className={cn(
                              'w-full flex items-start gap-3 px-4 py-3 text-left',
                              'disabled:cursor-not-allowed',
                              (!unavailable && available && hasInstalledModel) && 'hover:bg-bg-secondary/50 cursor-pointer'
                            )}
                          >
                            {/* Radio button */}
                            <div className={cn(
                              'mt-0.5 w-4 h-4 rounded-full border-2 shrink-0 flex items-center justify-center',
                              isSelected ? 'border-accent' : 'border-text-tertiary'
                            )}>
                              {isSelected && (
                                <div className="w-2 h-2 rounded-full bg-accent" />
                              )}
                            </div>

                            {/* Engine info */}
                            <div className="flex-1 min-w-0">
                              <div className="flex items-center gap-2">
                                <span className="text-sm font-medium text-text-primary">{engine.name}</span>
                                {engine.gpu_required && (
                                  <span className="px-1.5 py-0.5 text-[10px] font-medium rounded bg-warning/20 text-warning">
                                    GPU
                                  </span>
                                )}
                                {available && hasInstalledModel && (
                                  <span className="px-1.5 py-0.5 text-[10px] font-medium rounded bg-success/20 text-success">
                                    Ready
                                  </span>
                                )}
                                {notInstalled && (
                                  <span className="px-1.5 py-0.5 text-[10px] font-medium rounded bg-text-tertiary/20 text-text-tertiary">
                                    Not Installed
                                  </span>
                                )}
                                {unavailable && (
                                  <span className="px-1.5 py-0.5 text-[10px] font-medium rounded bg-error/20 text-error">
                                    Unavailable
                                  </span>
                                )}
                              </div>
                              <p className="text-xs text-text-tertiary mt-0.5">{engine.description}</p>
                            </div>
                          </button>

                          {/* Models section (expanded when selected or when no models installed) */}
                          {(isSelected || !hasInstalledModel) && !unavailable && (
                            <div className="px-4 pb-3 pt-1 border-t border-border/50 space-y-2">
                              <div className="flex items-center justify-between">
                                <span className="text-xs text-text-tertiary">Models</span>
                              </div>

                              {selectedEngineModels.map((model) => (
                                <div
                                  key={model.id}
                                  className={cn(
                                    'flex items-center justify-between px-3 py-2 rounded-md',
                                    model.installed ? 'bg-bg-secondary' : 'bg-bg-secondary/50'
                                  )}
                                >
                                  <div className="flex items-center gap-2">
                                    {model.installed ? (
                                      <button
                                        onClick={() => isSelected && handleEngineModelChange(model.id)}
                                        className="flex items-center gap-2"
                                      >
                                        <div className={cn(
                                          'w-3 h-3 rounded-full border',
                                          config.transcription_model === model.id && isSelected
                                            ? 'border-accent bg-accent'
                                            : 'border-text-tertiary'
                                        )} />
                                        <span className="text-sm text-text-primary">{model.name}</span>
                                      </button>
                                    ) : (
                                      <>
                                        <span className="w-3 h-3 rounded-full border border-border" />
                                        <span className="text-sm text-text-tertiary">{model.name}</span>
                                      </>
                                    )}
                                    <span className="text-xs text-text-tertiary">({model.size})</span>
                                    <span className="text-[10px] text-text-tertiary">
                                      ~{model.speed_cpu}x{engine.gpu_required ? ` CPU / ~${model.speed_gpu}x GPU` : ''}
                                    </span>
                                  </div>

                                  {!model.installed && (
                                    <button
                                      onClick={() => handleDownloadEngineModel(engine.id, model.id)}
                                      disabled={isDownloadingEngineModel !== null}
                                      className={cn(
                                        'flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium rounded transition-colors',
                                        'bg-accent/10 text-accent hover:bg-accent/20 disabled:opacity-50'
                                      )}
                                    >
                                      {isDownloadingEngineModel?.engine === engine.id && isDownloadingEngineModel?.model === model.id ? (
                                        <>
                                          <LoaderIcon className="w-3 h-3 animate-spin" />
                                          Downloading...
                                        </>
                                      ) : (
                                        <>
                                          <DownloadIcon className="w-3 h-3" />
                                          Download
                                        </>
                                      )}
                                    </button>
                                  )}
                                </div>
                              ))}

                              {/* Progress bar for downloads */}
                              {(isDownloadingEngineModel?.engine === engine.id) && engineProgress && (
                                <div className="pt-2">
                                  <p className="text-xs text-text-tertiary mb-1">{engineProgress.stage}</p>
                                  <ProgressBar percentage={engineProgress.percentage} />
                                </div>
                              )}

                              {/* Error message */}
                              {downloadError && !isDownloadingEngineModel && (
                                <div className="pt-2">
                                  <p className="text-xs text-error">{downloadError}</p>
                                </div>
                              )}
                            </div>
                          )}
                        </div>
                      );
                    })}

                    {engines.length === 0 && (
                      <p className="text-sm text-text-tertiary px-1">Loading engines...</p>
                    )}
                  </div>
                )}

                {/* Legacy Whisper section (for existing installations) */}
                {!config.generate_subtitles && (whisperStatus?.status === 'installed' || whisperStatus?.status === 'model_missing') && (
                  <div className="space-y-3 mt-3">
                    <label className="block text-xs font-medium text-text-secondary px-1">
                      Legacy Whisper.cpp
                    </label>
                    <div className="flex items-center justify-between px-4 py-3 bg-bg-tertiary rounded-lg">
                      <div className="flex-1 min-w-0 mr-3">
                        <p className="text-sm text-text-primary truncate">
                          {whisperStatus.version}
                        </p>
                        <p className="text-xs text-text-tertiary mt-0.5">Managed by Zinc</p>
                      </div>
                      <span className="flex items-center gap-2 text-sm text-success shrink-0">
                        <CheckIcon className="w-4 h-4" />
                        Installed
                      </span>
                    </div>
                  </div>
                )}
              </section>
            </div>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}
