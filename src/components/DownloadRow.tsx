import { memo, useCallback, useState, useMemo } from 'react';
import { motion, AnimatePresence } from 'motion/react';
import type { Download } from '@/lib/types';
import { cn, truncate } from '@/lib/utils';
import { getSpeedMultiplier } from '@/lib/constants';
import { ProgressRing } from './ProgressRing';
import { XIcon, FolderIcon, PlayIcon, TrashIcon, ChevronDownIcon } from './Icons';

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
  return error;
}

interface DownloadRowProps {
  download: Download;
  onCancel: (id: string) => void;
  onClear: (id: string) => void;
  onOpenFile: (path: string) => void;
  onOpenFolder: (path: string) => void;
}

export const DownloadRow = memo(function DownloadRow({
  download,
  onCancel,
  onClear,
  onOpenFile,
  onOpenFolder,
}: DownloadRowProps) {
  const isDownloading = download.status === 'downloading' || download.status === 'pending';
  const isTranscribing = download.status === 'transcribing' || download.status.startsWith('transcribing:');
  const isActive = isDownloading || isTranscribing;

  // Start expanded for active downloads
  const [isExpanded, setIsExpanded] = useState(isActive);

  const handleCancel = useCallback(() => onCancel(download.id), [onCancel, download.id]);
  const handleClear = useCallback(() => onClear(download.id), [onClear, download.id]);
  const handleOpenFile = useCallback(() => {
    if (download.output_path) onOpenFile(download.output_path);
  }, [onOpenFile, download.output_path]);
  const handleOpenFolder = useCallback(() => {
    if (download.output_path) onOpenFolder(download.output_path);
  }, [onOpenFolder, download.output_path]);

  const isCompleted = download.status === 'completed';
  const isError = download.status === 'error';
  const isCancelled = download.status === 'cancelled';

  // Determine progress ring status
  const ringStatus = useMemo(() => {
    if (isDownloading) return 'downloading';
    if (isTranscribing) return 'transcribing';
    if (isCompleted) return 'completed';
    if (isError) return 'error';
    if (isCancelled) return 'cancelled';
    return 'pending';
  }, [isDownloading, isTranscribing, isCompleted, isError, isCancelled]);

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

  // Status text for collapsed view
  const statusText = useMemo(() => {
    if (isDownloading) return download.speed || `${download.progress.toFixed(0)}%`;
    if (isTranscribing) {
      const msg = download.transcription_message || transcribeStage;
      return msg.replace('...', '');
    }
    if (isCompleted) return 'Done';
    if (isError) return 'Error';
    if (isCancelled) return 'Cancelled';
    return '';
  }, [isDownloading, isTranscribing, isCompleted, isError, isCancelled, download.speed, download.progress, download.transcription_message, transcribeStage]);

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
        isActive && 'border-accent/20',
        !isCompleted && !isError && !isCancelled && !isActive && 'border-border/50'
      )}
    >
      {/* Collapsed row - always visible */}
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full flex items-center gap-3 px-3 py-2.5 hover:bg-white/5 transition-colors text-left"
      >
        {/* Progress ring */}
        <ProgressRing
          status={ringStatus}
          progress={progressValue}
          size={24}
        />

        {/* Title - truncated */}
        <span className="flex-1 text-sm text-text-primary truncate min-w-0">
          {truncate(download.title, 50)}
        </span>

        {/* Status/Speed */}
        <span className={cn(
          'text-xs tabular-nums shrink-0',
          isActive && 'text-accent',
          isCompleted && 'text-success',
          isError && 'text-error',
          isCancelled && 'text-text-tertiary'
        )}>
          {statusText}
        </span>

        {/* Action button */}
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
          {isDownloading && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                handleCancel();
              }}
              className="p-1.5 rounded-md hover:bg-error/20 text-text-secondary hover:text-error transition-colors"
              aria-label="Cancel download"
            >
              <XIcon className="w-4 h-4" />
            </button>
          )}
          {!isActive && (
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
            <div className="px-3 pb-3 pt-1 flex gap-3 border-t border-border/30">
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

                {/* Progress info */}
                {isDownloading && (
                  <p className="text-xs text-text-secondary">
                    <span className="text-accent font-medium">{download.progress.toFixed(1)}%</span>
                    {download.speed && <span className="ml-2">{download.speed}</span>}
                    {download.eta && <span className="ml-2 text-text-tertiary">ETA {download.eta}</span>}
                  </p>
                )}

                {/* Transcription info */}
                {isTranscribing && (
                  <p className="text-xs text-text-secondary">
                    <span className="text-accent">{download.transcription_message || transcribeStage}</span>
                    {download.transcription_progress != null && download.transcription_progress > 0 && (
                      <span className="ml-2 font-medium">{download.transcription_progress.toFixed(0)}%</span>
                    )}
                    {transcriptionEta && download.transcription_progress != null && download.transcription_progress < 10 && (
                      <span className="ml-2 text-text-tertiary">ETA {transcriptionEta}</span>
                    )}
                  </p>
                )}

                {/* Completed info */}
                {isCompleted && (
                  <p className="text-xs text-text-tertiary">
                    {download.format.toUpperCase()}
                    {download.error && (
                      <span className="ml-2 text-warning">{download.error}</span>
                    )}
                  </p>
                )}

                {/* Error message */}
                {isError && download.error && (
                  <p className="text-xs text-error">{formatErrorMessage(download.error)}</p>
                )}

                {/* Cancelled */}
                {isCancelled && (
                  <p className="text-xs text-text-tertiary">Download cancelled</p>
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
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
});
