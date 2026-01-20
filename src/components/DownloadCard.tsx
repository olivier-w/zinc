import { memo, useCallback } from 'react';
import { motion } from 'framer-motion';
import type { Download } from '@/lib/types';
import { cn, truncate } from '@/lib/utils';
import { CheckIcon, XIcon, AlertCircleIcon, FolderIcon, PlayIcon, TrashIcon } from './Icons';

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
  const handleCancel = useCallback(() => onCancel(download.id), [onCancel, download.id]);
  const handleClear = useCallback(() => onClear(download.id), [onClear, download.id]);
  const handleOpenFile = useCallback(() => {
    if (download.output_path) onOpenFile(download.output_path);
  }, [onOpenFile, download.output_path]);
  const handleOpenFolder = useCallback(() => {
    if (download.output_path) onOpenFolder(download.output_path);
  }, [onOpenFolder, download.output_path]);

  const isActive = download.status === 'downloading' || download.status === 'pending';
  const isCompleted = download.status === 'completed';
  const isError = download.status === 'error';
  const isCancelled = download.status === 'cancelled';

  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: 20, scale: 0.95 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, scale: 0.95, x: -20 }}
      transition={{ type: 'spring', stiffness: 400, damping: 25 }}
      className={cn(
        'group relative p-4 rounded-xl border transition-all',
        'bg-bg-secondary hover:bg-bg-tertiary/50',
        isCompleted && 'border-success/30',
        isError && 'border-error/30',
        isCancelled && 'border-border opacity-60',
        isActive && 'border-accent/30'
      )}
    >
      <div className="flex gap-3">
        {/* Thumbnail */}
        {download.thumbnail ? (
          <div className="relative w-20 h-14 rounded-lg overflow-hidden shrink-0 bg-bg-tertiary">
            <img
              src={download.thumbnail}
              alt=""
              className="w-full h-full object-cover"
              loading="lazy"
            />
          </div>
        ) : (
          <div className="w-20 h-14 rounded-lg bg-bg-tertiary shrink-0" />
        )}

        {/* Content */}
        <div className="flex-1 min-w-0">
          <div className="flex items-start justify-between gap-2">
            <h3 className="text-sm font-medium text-text-primary truncate">
              {truncate(download.title, 60)}
            </h3>

            {/* Status indicator */}
            <div className="flex items-center gap-1 shrink-0">
              {isCompleted && (
                <span className="text-success">
                  <CheckIcon className="w-4 h-4" />
                </span>
              )}
              {isError && (
                <span className="text-error">
                  <AlertCircleIcon className="w-4 h-4" />
                </span>
              )}
              {isCancelled && (
                <span className="text-text-tertiary">
                  <XIcon className="w-4 h-4" />
                </span>
              )}
            </div>
          </div>

          {/* Progress bar for active downloads */}
          {isActive && (
            <div className="mt-2">
              <div className="flex items-center justify-between text-xs text-text-secondary mb-1">
                <span className="tabular-nums">{download.progress.toFixed(1)}%</span>
                <span className="flex gap-2">
                  {download.speed && <span className="tabular-nums">{download.speed}</span>}
                  {download.eta && <span className="tabular-nums">ETA {download.eta}</span>}
                </span>
              </div>
              <div className="h-1.5 bg-bg-tertiary rounded-full overflow-hidden">
                <motion.div
                  className="h-full progress-shimmer rounded-full"
                  initial={{ width: 0 }}
                  animate={{ width: `${download.progress}%` }}
                  transition={{ duration: 0.3, ease: 'easeOut' }}
                />
              </div>
            </div>
          )}

          {/* Error message */}
          {isError && download.error && (
            <p className="mt-1 text-xs text-error truncate">{download.error}</p>
          )}

          {/* Completed state */}
          {isCompleted && (
            <p className="mt-1 text-xs text-text-tertiary">
              {download.format.toUpperCase()} · Download complete
            </p>
          )}

          {/* Cancelled state */}
          {isCancelled && (
            <p className="mt-1 text-xs text-text-tertiary">Cancelled</p>
          )}
        </div>
      </div>

      {/* Actions */}
      <div className="flex items-center gap-1 mt-3 pt-3 border-t border-border">
        {isActive && (
          <button
            onClick={handleCancel}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs text-text-secondary hover:text-error hover:bg-error/10 rounded-lg transition-colors"
            aria-label="Cancel download"
          >
            <XIcon className="w-3.5 h-3.5" />
            <span>Cancel</span>
          </button>
        )}

        {isCompleted && download.output_path && (
          <>
            <button
              onClick={handleOpenFile}
              className="flex items-center gap-1.5 px-3 py-1.5 text-xs text-text-secondary hover:text-accent hover:bg-accent/10 rounded-lg transition-colors"
              aria-label="Play file"
            >
              <PlayIcon className="w-3.5 h-3.5" />
              <span>Play</span>
            </button>
            <button
              onClick={handleOpenFolder}
              className="flex items-center gap-1.5 px-3 py-1.5 text-xs text-text-secondary hover:text-accent hover:bg-accent/10 rounded-lg transition-colors"
              aria-label="Open folder"
            >
              <FolderIcon className="w-3.5 h-3.5" />
              <span>Folder</span>
            </button>
          </>
        )}

        {!isActive && (
          <button
            onClick={handleClear}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs text-text-secondary hover:text-error hover:bg-error/10 rounded-lg transition-colors ml-auto"
            aria-label="Remove from list"
          >
            <TrashIcon className="w-3.5 h-3.5" />
            <span>Remove</span>
          </button>
        )}
      </div>
    </motion.div>
  );
});
