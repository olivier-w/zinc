import { memo, useCallback, useState, useMemo, useEffect } from 'react';
import { motion, AnimatePresence } from 'motion/react';
import type { Download, TranscriptionEngine } from '@/lib/types';
import { cn } from '@/lib/utils';
import { getSpeedMultiplier } from '@/lib/constants';
import { getTranscriptionEngines } from '@/lib/tauri';
import { XIcon, FolderIcon, PlayIcon, TrashIcon, ChevronDownIcon, SubtitlesIcon, CheckIcon } from './Icons';

const TRANSCRIBE_STAGE_MESSAGES: Record<string, string> = {
  extracting: 'Extracting audio...',
  transcribing: 'Transcribing...',
  embedding: 'Embedding subtitles...',
  finalizing: 'Finalizing...',
};

// Format error messages into user-friendly text
function formatErrorMessage(error: string): string {
  if (
    error.includes('WinError 10049') ||
    error.includes('Failed to establish a new connection') ||
    error.includes('The requested address is not valid')
  ) {
    return 'Lost internet connection';
  }
  if (error === 'Cancelled') {
    return 'Cancelled';
  }
  return error;
}

interface DownloadRowProps {
  download: Download;
  onCancel: (id: string) => void;
  onClear: (id: string) => void;
  onOpenFile: (path: string) => void;
  onOpenFolder: (path: string) => void;
  onStartLocalTranscription?: (taskId: string) => void;
  onUpdateTranscriptionSettings?: (taskId: string, settings: { engine?: string; model?: string }) => void;
}

export const DownloadRow = memo(function DownloadRow({
  download,
  onCancel,
  onClear,
  onOpenFile,
  onOpenFolder,
  onStartLocalTranscription,
  onUpdateTranscriptionSettings,
}: DownloadRowProps) {
  const isPendingLocalTranscribe = download.task_type === 'local_transcribe' && download.status === 'pending';
  const isDownloading = download.task_type === 'download' && (download.status === 'downloading' || download.status === 'pending');
  const isTranscribing = download.status === 'transcribing' || download.status.startsWith('transcribing:');
  const isActive = isDownloading || isTranscribing;

  // Start expanded for active downloads and pending local transcriptions
  const [isExpanded, setIsExpanded] = useState(isActive || isPendingLocalTranscribe);

  // Engines for local transcription settings
  const [engines, setEngines] = useState<TranscriptionEngine[]>([]);
  const [selectedEngine, setSelectedEngine] = useState(download.transcription_engine || 'whisper_rs');
  const [selectedModel, setSelectedModel] = useState(download.whisper_model || 'base');

  // Fetch engines when component mounts (only for pending local transcriptions)
  useEffect(() => {
    if (isPendingLocalTranscribe) {
      getTranscriptionEngines().then((fetchedEngines) => {
        setEngines(fetchedEngines);

        // Find engines with installed models
        const enginesWithModels = fetchedEngines.filter(e => e.models.some(m => m.installed));
        if (enginesWithModels.length === 0) return;

        // Check if current engine has installed models
        const currentEng = fetchedEngines.find(e => e.id === selectedEngine);
        const currentHasModels = currentEng?.models.some(m => m.installed);

        if (!currentHasModels) {
          // Switch to first available engine
          const firstAvailable = enginesWithModels[0];
          setSelectedEngine(firstAvailable.id);
          const firstModel = firstAvailable.models.find(m => m.installed);
          if (firstModel) setSelectedModel(firstModel.id);
        } else {
          // Check if current model is installed
          const modelInstalled = currentEng?.models.find(m => m.id === selectedModel && m.installed);
          if (!modelInstalled) {
            const firstInstalled = currentEng?.models.find(m => m.installed);
            if (firstInstalled) setSelectedModel(firstInstalled.id);
          }
        }
      }).catch(() => {});
    }
  }, [isPendingLocalTranscribe, selectedEngine, selectedModel]);

  // Get engines that have at least one installed model
  const availableEngines = useMemo(() => {
    return engines.filter(e => e.models.some(m => m.installed));
  }, [engines]);

  // Get current engine and its installed models
  const currentEngine = useMemo(() => {
    return engines.find(e => e.id === selectedEngine);
  }, [engines, selectedEngine]);

  const installedModels = useMemo(() => {
    return currentEngine?.models.filter(m => m.installed) || [];
  }, [currentEngine]);

  const handleCancel = useCallback(() => onCancel(download.id), [onCancel, download.id]);
  const handleClear = useCallback(() => onClear(download.id), [onClear, download.id]);
  const handleOpenFile = useCallback(() => {
    if (download.output_path) onOpenFile(download.output_path);
  }, [onOpenFile, download.output_path]);
  const handleOpenFolder = useCallback(() => {
    if (download.output_path) onOpenFolder(download.output_path);
  }, [onOpenFolder, download.output_path]);

  const handleStartTranscription = useCallback(() => {
    // Update settings before starting if they changed
    if (onUpdateTranscriptionSettings && (selectedEngine !== download.transcription_engine || selectedModel !== download.whisper_model)) {
      onUpdateTranscriptionSettings(download.id, { engine: selectedEngine, model: selectedModel });
    }
    onStartLocalTranscription?.(download.id);
  }, [download.id, selectedEngine, selectedModel, download.transcription_engine, download.whisper_model, onUpdateTranscriptionSettings, onStartLocalTranscription]);

  const handleEngineChange = useCallback((newEngine: string) => {
    setSelectedEngine(newEngine);
    const eng = engines.find(e => e.id === newEngine);
    const firstModel = eng?.models.find(m => m.installed);
    if (firstModel) setSelectedModel(firstModel.id);
  }, [engines]);

  const isCompleted = download.status === 'completed';
  const isError = download.status === 'error';
  const isCancelled = download.status === 'cancelled';

  // Get progress value
  const progressValue = useMemo(() => {
    if (isDownloading) return download.progress;
    if (isTranscribing) return download.transcription_progress ?? 0;
    if (isCompleted) return 100;
    return 0;
  }, [isDownloading, isTranscribing, isCompleted, download.progress, download.transcription_progress]);

  // Parse transcription stage from status
  const transcribeStage = useMemo(() => {
    if (!isTranscribing) return '';
    const stage = download.status.split(':')[1];
    return TRANSCRIBE_STAGE_MESSAGES[stage] || 'Generating subtitles...';
  }, [isTranscribing, download.status]);

  // Calculate estimated transcription time
  const transcriptionEta = useMemo(() => {
    if (!isTranscribing || !download.duration || !download.whisper_model) return null;
    const engine = download.transcription_engine || 'whisper_cpp';
    const useGpu = engine === 'parakeet';
    const multiplier = getSpeedMultiplier(engine, download.whisper_model, useGpu);
    const seconds = Math.ceil(download.duration / multiplier) + 10;
    if (seconds < 60) return `~${seconds}s`;
    const minutes = Math.ceil(seconds / 60);
    return `~${minutes}m`;
  }, [isTranscribing, download.duration, download.whisper_model, download.transcription_engine]);

  // Status text for collapsed view - combined progress + speed
  const statusText = useMemo(() => {
    if (isPendingLocalTranscribe) return 'Ready';
    if (isDownloading) {
      const pct = `${download.progress.toFixed(1)}%`;
      return download.speed ? `${pct} ${download.speed}` : pct;
    }
    if (isTranscribing) {
      const msg = (download.transcription_message || transcribeStage).replace('...', '');
      if (download.transcription_progress != null && download.transcription_progress > 0) {
        return `${msg} ${download.transcription_progress.toFixed(0)}%`;
      }
      return msg;
    }
    if (isCompleted) return 'Done';
    if (isError) return formatErrorMessage(download.error || 'Error');
    if (isCancelled) return 'Cancelled';
    return '';
  }, [isPendingLocalTranscribe, isDownloading, isTranscribing, isCompleted, isError, isCancelled, download.speed, download.progress, download.transcription_message, download.transcription_progress, transcribeStage, download.error]);

  return (
    <motion.div
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, scale: 0.98 }}
      transition={{
        duration: 0.35,
        ease: [0.16, 1, 0.3, 1],
      }}
      className={cn(
        'rounded-lg overflow-hidden',
        'bg-bg-secondary/50 border transition-all',
        isCompleted && 'border-success/20',
        isError && 'border-error/20',
        isCancelled && 'border-border opacity-60',
        (isActive || isPendingLocalTranscribe) && 'border-accent/20',
        !isCompleted && !isError && !isCancelled && !isActive && !isPendingLocalTranscribe && 'border-border/50'
      )}
    >
      {/* Thin progress bar at top */}
      {isActive && (
        <div className="progress-bar-inline">
          <motion.div
            className={cn(
              'progress-bar-inline-fill',
              isDownloading && 'downloading',
              isTranscribing && 'transcribing'
            )}
            initial={{ width: 0 }}
            animate={{ width: `${progressValue}%` }}
            transition={{ duration: 0.3, ease: 'easeOut' }}
          />
        </div>
      )}

      {/* Collapsed row - always visible */}
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full flex items-center gap-3 px-3 py-2.5 hover:bg-white/5 transition-colors text-left"
      >
        {/* Status icon for non-active states */}
        {!isActive && (
          <div className="w-5 h-5 flex items-center justify-center shrink-0">
            {isCompleted && <CheckIcon className="w-4 h-4 text-success" />}
            {isError && <XIcon className="w-4 h-4 text-error" />}
            {isCancelled && <XIcon className="w-4 h-4 text-text-tertiary" />}
            {isPendingLocalTranscribe && <SubtitlesIcon className="w-4 h-4 text-accent" />}
          </div>
        )}

        {/* Status text */}
        <span className={cn(
          'text-xs tabular-nums flex-1 min-w-0 truncate',
          isDownloading && 'text-accent',
          isTranscribing && 'text-purple-400',
          isPendingLocalTranscribe && 'text-accent',
          isCompleted && 'text-success',
          isError && 'text-error',
          isCancelled && 'text-text-tertiary'
        )}>
          {statusText}
        </span>

        {/* Action buttons */}
        <div className="shrink-0 flex items-center gap-1">
          {isCompleted && download.output_path && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                handleOpenFile();
              }}
              className="p-1.5 rounded-md hover:bg-white/10 text-text-secondary hover:text-text-primary transition-colors"
              aria-label="Play file"
            >
              <PlayIcon className="w-4 h-4" />
            </button>
          )}
          {(isDownloading || isTranscribing) && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                handleCancel();
              }}
              className="p-1.5 rounded-md hover:bg-error/20 text-text-secondary hover:text-error transition-colors"
              aria-label="Cancel"
            >
              <XIcon className="w-4 h-4" />
            </button>
          )}
          {!isActive && !isPendingLocalTranscribe && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                handleClear();
              }}
              className="p-1.5 rounded-md hover:bg-error/20 text-text-tertiary hover:text-error transition-colors"
              aria-label="Remove"
            >
              <TrashIcon className="w-4 h-4" />
            </button>
          )}
          <ChevronDownIcon className={cn(
            'w-4 h-4 text-text-tertiary transition-transform',
            isExpanded && 'rotate-180'
          )} />
        </div>
      </button>

      {/* Expanded details */}
      <AnimatePresence>
        {isExpanded && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ type: 'spring', stiffness: 400, damping: 35 }}
            className="overflow-hidden"
          >
            <div className="px-3 pb-3 pt-1 flex flex-col gap-3 border-t border-border/30">
              {/* Row with thumbnail and details */}
              <div className="flex gap-3">
                {/* Thumbnail */}
                {download.thumbnail && (
                  <div className="w-16 h-9 shrink-0 rounded overflow-hidden bg-bg-tertiary">
                    <img
                      src={download.thumbnail}
                      alt=""
                      className="w-full h-full object-cover"
                    />
                  </div>
                )}

                {/* Details */}
                <div className="flex-1 min-w-0">
                  {/* Full title */}
                  <p className="text-sm text-text-primary line-clamp-2 leading-snug mb-1">
                    {download.title}
                  </p>

                  {/* Source path for local transcription */}
                  {download.task_type === 'local_transcribe' && download.source_path && (
                    <p className="text-xs text-text-tertiary truncate" title={download.source_path}>
                      {download.source_path}
                    </p>
                  )}

                  {/* ETA for downloading */}
                  {isDownloading && download.eta && (
                    <p className="text-xs text-text-tertiary">ETA {download.eta}</p>
                  )}

                  {/* ETA for transcription */}
                  {isTranscribing && transcriptionEta && download.transcription_progress != null && download.transcription_progress < 10 && (
                    <p className="text-xs text-text-tertiary">ETA {transcriptionEta}</p>
                  )}

                  {/* Completed info */}
                  {isCompleted && (
                    <p className="text-xs text-text-tertiary">
                      {download.task_type === 'local_transcribe' ? 'Subtitles added' : download.format.toUpperCase()}
                      {download.error && (
                        <span className="ml-2 text-warning">{download.error}</span>
                      )}
                    </p>
                  )}
                </div>

                {/* Expanded actions */}
                <div className="shrink-0 flex flex-col gap-1">
                  {isCompleted && download.output_path && (
                    <button
                      onClick={handleOpenFolder}
                      className="p-1.5 rounded-md hover:bg-white/10 text-text-secondary hover:text-text-primary transition-colors"
                      aria-label="Open folder"
                    >
                      <FolderIcon className="w-4 h-4" />
                    </button>
                  )}
                </div>
              </div>

              {/* Engine/Model selectors and Start button for pending local transcription */}
              {isPendingLocalTranscribe && availableEngines.length > 0 && (
                <div className="flex flex-wrap items-center gap-2">
                  {/* Engine selector */}
                  {availableEngines.length > 1 && (
                    <select
                      value={selectedEngine}
                      onChange={(e) => handleEngineChange(e.target.value)}
                      className="pill-glass px-3 py-1.5 text-sm rounded-lg bg-transparent border-none cursor-pointer"
                    >
                      {availableEngines.map((engine) => (
                        <option key={engine.id} value={engine.id} className="bg-bg-secondary">
                          {engine.name}
                        </option>
                      ))}
                    </select>
                  )}

                  {/* Model selector */}
                  {installedModels.length > 1 && (
                    <select
                      value={selectedModel}
                      onChange={(e) => setSelectedModel(e.target.value)}
                      className="pill-glass px-3 py-1.5 text-sm rounded-lg bg-transparent border-none cursor-pointer"
                    >
                      {installedModels.map((model) => (
                        <option key={model.id} value={model.id} className="bg-bg-secondary">
                          {model.name}
                        </option>
                      ))}
                    </select>
                  )}

                  <div className="flex-1" />

                  {/* Remove button */}
                  <button
                    onClick={handleClear}
                    className="px-3 py-1.5 text-sm rounded-lg text-text-secondary hover:text-error hover:bg-error/10 transition-colors"
                  >
                    Remove
                  </button>

                  {/* Start button */}
                  <button
                    onClick={handleStartTranscription}
                    className="px-4 py-1.5 text-sm rounded-lg btn-gradient text-white font-medium"
                  >
                    Start
                  </button>
                </div>
              )}

              {/* Warning if no models installed */}
              {isPendingLocalTranscribe && availableEngines.length === 0 && engines.length > 0 && (
                <div className="flex items-center justify-between gap-2">
                  <p className="text-xs text-warning">No transcription models installed. Install models in Settings.</p>
                  <button
                    onClick={handleClear}
                    className="px-3 py-1.5 text-sm rounded-lg text-text-secondary hover:text-error hover:bg-error/10 transition-colors"
                  >
                    Remove
                  </button>
                </div>
              )}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
});
