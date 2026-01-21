import { memo, useCallback, useState, useMemo } from 'react';
import { motion } from 'framer-motion';
import type { Download } from '@/lib/types';
import { cn, truncate } from '@/lib/utils';
import { getSpeedMultiplier } from '@/lib/constants';
import { CheckIcon, XIcon, AlertCircleIcon, FolderIcon, PlayIcon, TrashIcon, SubtitlesIcon, LoaderIcon } from './Icons';

const TRANSCRIBE_STAGE_MESSAGES: Record<string, string> = {
  extracting: 'Extracting audio...',
  transcribing: 'Transcribing...',
  embedding: 'Embedding subtitles...',
  finalizing: 'Finalizing...',
};

interface DownloadCardProps {
  download: Download;
  onCancel: (id: string) => void;
  onClear: (id: string) => void;
  onOpenFile: (path: string) => void;
  onOpenFolder: (path: string) => void;
}

export const DownloadCard = memo(function DownloadCard({
  download,
  onCancel,
  onClear,
  onOpenFile,
  onOpenFolder,
}: DownloadCardProps) {
  const [isHovered, setIsHovered] = useState(false);

  const handleCancel = useCallback(() => onCancel(download.id), [onCancel, download.id]);
  const handleClear = useCallback(() => onClear(download.id), [onClear, download.id]);
  const handleOpenFile = useCallback(() => {
    if (download.output_path) onOpenFile(download.output_path);
  }, [onOpenFile, download.output_path]);
  const handleOpenFolder = useCallback(() => {
    if (download.output_path) onOpenFolder(download.output_path);
  }, [onOpenFolder, download.output_path]);

  const isDownloading = download.status === 'downloading' || download.status === 'pending';
  const isTranscribing = download.status === 'transcribing' || download.status.startsWith('transcribing:');
  const isActive = isDownloading || isTranscribing;
  const isCompleted = download.status === 'completed';
  const isError = download.status === 'error';
  const isCancelled = download.status === 'cancelled';

  // Parse transcription stage from status like "transcribing:extracting"
  const getTranscribeStage = (): string => {
    if (!isTranscribing) return '';
    const stage = download.status.split(':')[1];
    return TRANSCRIBE_STAGE_MESSAGES[stage] || 'Generating subtitles...';
  };

  // Calculate estimated transcription time
  const transcriptionEta = useMemo(() => {
    if (!isTranscribing || !download.duration || !download.whisper_model) return null;
    // Use engine-specific speed multiplier (assume GPU for parakeet, CPU for others)
    const engine = download.transcription_engine || 'whisper_cpp';
    const useGpu = engine === 'parakeet';
    const multiplier = getSpeedMultiplier(engine, download.whisper_model, useGpu);
    const seconds = Math.ceil(download.duration / multiplier) + 10; // +10s overhead
    if (seconds < 60) return `~${seconds}s`;
    const minutes = Math.ceil(seconds / 60);
    return `~${minutes} min`;
  }, [isTranscribing, download.duration, download.whisper_model, download.transcription_engine]);

  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: 10, scale: 0.98 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, scale: 0.95 }}
      transition={{
        type: 'spring',
        stiffness: 400,
        damping: 30,
        delay: 0.05,
      }}
      className={cn(
        'group relative rounded-xl overflow-hidden card-lift',
        'bg-bg-secondary border transition-all',
        isCompleted && 'border-success/30',
        isError && 'border-error/30',
        isCancelled && 'border-border opacity-60',
        isActive && 'border-accent/30',
        !isCompleted && !isError && !isCancelled && !isActive && 'border-border'
      )}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      {/* Thumbnail Section */}
      <div className="relative aspect-video bg-bg-tertiary overflow-hidden">
        {download.thumbnail ? (
          <img
            src={download.thumbnail}
            alt=""
            className={cn(
              'w-full h-full object-cover transition-transform duration-300',
              isHovered && 'scale-105'
            )}
            loading="lazy"
          />
        ) : (
          <div className="w-full h-full bg-bg-tertiary" />
        )}

        {/* Gradient overlay */}
        <div className="absolute inset-0 thumbnail-overlay" />

        {/* Progress bar overlaid on thumbnail bottom */}
        {isDownloading && (
          <div className="absolute bottom-0 left-0 right-0 h-1 bg-black/50">
            <motion.div
              className="h-full progress-shimmer"
              initial={{ width: 0 }}
              animate={{ width: `${download.progress}%` }}
              transition={{ duration: 0.3, ease: 'easeOut' }}
            />
          </div>
        )}
        {isTranscribing && (
          <div className="absolute bottom-0 left-0 right-0 h-1 bg-black/50 overflow-hidden">
            <div className="h-full w-full bg-accent/80 animate-pulse" />
          </div>
        )}

        {/* Status badge - top right */}
        {isCompleted && (
          <div className="absolute top-2 right-2 badge-success px-2 py-1 rounded-md flex items-center gap-1">
            <CheckIcon className="w-3 h-3" />
            <span className="text-xs font-medium">Done</span>
          </div>
        )}
        {isTranscribing && (
          <div className="absolute top-2 right-2 bg-accent/90 text-white px-2 py-1 rounded-md flex items-center gap-1">
            <SubtitlesIcon className="w-3 h-3" />
            <span className="text-xs font-medium">Subtitles</span>
          </div>
        )}
        {isError && (
          <div className="absolute top-2 right-2 badge-error px-2 py-1 rounded-md flex items-center gap-1">
            <AlertCircleIcon className="w-3 h-3" />
            <span className="text-xs font-medium">Error</span>
          </div>
        )}

        {/* Action overlay - visible on hover (desktop) or always (touch devices) */}
        {!isActive && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: isHovered ? 1 : 0 }}
            className="absolute inset-0 bg-black/60 flex items-center justify-center gap-3 touch-action-visible"
          >
            {isCompleted && download.output_path && (
              <>
                <button
                  onClick={handleOpenFile}
                  className="p-3 rounded-full bg-white/20 hover:bg-white/30 text-white transition-colors"
                  aria-label="Play file"
                >
                  <PlayIcon className="w-6 h-6" />
                </button>
                <button
                  onClick={handleOpenFolder}
                  className="p-3 rounded-full bg-white/20 hover:bg-white/30 text-white transition-colors"
                  aria-label="Open folder"
                >
                  <FolderIcon className="w-6 h-6" />
                </button>
              </>
            )}
            <button
              onClick={handleClear}
              className="p-3 rounded-full bg-error/80 hover:bg-error text-white transition-colors"
              aria-label="Remove from list"
            >
              <TrashIcon className="w-6 h-6" />
            </button>
          </motion.div>
        )}
      </div>

      {/* Content Section */}
      <div className="p-3">
        <h3 className="text-sm font-medium text-text-primary line-clamp-2 leading-snug mb-1">
          {truncate(download.title, 80)}
        </h3>

        {/* Active download info */}
        {isDownloading && (
          <div className="flex items-center justify-between text-xs text-text-secondary mt-2">
            <span className="tabular-nums font-medium text-accent">{download.progress.toFixed(1)}%</span>
            <span className="flex items-center gap-2">
              {download.speed && <span className="tabular-nums">{download.speed}</span>}
              {download.eta && <span className="tabular-nums text-text-tertiary">ETA {download.eta}</span>}
            </span>
          </div>
        )}

        {/* Transcribing info */}
        {isTranscribing && (
          <div className="flex items-center justify-between text-xs text-text-secondary mt-2">
            <div className="flex items-center gap-2">
              <LoaderIcon className="w-3 h-3 animate-spin text-accent" />
              <span className="text-accent">{getTranscribeStage()}</span>
            </div>
            {transcriptionEta && (
              <span className="text-text-tertiary tabular-nums">ETA {transcriptionEta}</span>
            )}
          </div>
        )}

        {/* Error message */}
        {isError && download.error && (
          <p className="mt-1 text-xs text-error line-clamp-2" title={download.error}>{download.error}</p>
        )}

        {/* Completed state */}
        {isCompleted && (
          <>
            <p className="text-xs text-text-tertiary">
              {download.format.toUpperCase()}
            </p>
            {/* Warning for completed downloads with errors (e.g., subtitle generation failed) */}
            {download.error && (
              <p className="mt-1 text-xs text-warning line-clamp-2" title={download.error}>{download.error}</p>
            )}
          </>
        )}

        {/* Cancelled state */}
        {isCancelled && (
          <p className="text-xs text-text-tertiary">Cancelled</p>
        )}

        {/* Actions - only show for downloading state (not transcribing) */}
        {isDownloading && (
          <div className="flex items-center gap-1 mt-2 pt-2 border-t border-border">
            <button
              onClick={handleCancel}
              className="flex items-center gap-1 px-2 py-1 text-xs text-text-secondary hover:text-error hover:bg-error/10 rounded transition-colors"
              aria-label="Cancel download"
            >
              <XIcon className="w-3.5 h-3.5" />
              <span>Cancel</span>
            </button>
          </div>
        )}
      </div>
    </motion.div>
  );
});
