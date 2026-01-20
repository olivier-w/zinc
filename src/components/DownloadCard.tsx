import { memo, useCallback, useState } from 'react';
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
  const [isHovered, setIsHovered] = useState(false);

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
      exit={{ opacity: 0, scale: 0.95 }}
      transition={{ type: 'spring', stiffness: 400, damping: 25 }}
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
        {isActive && (
          <div className="absolute bottom-0 left-0 right-0 h-1 bg-black/50">
            <motion.div
              className="h-full progress-shimmer"
              initial={{ width: 0 }}
              animate={{ width: `${download.progress}%` }}
              transition={{ duration: 0.3, ease: 'easeOut' }}
            />
          </div>
        )}

        {/* Status badge - top right */}
        {isCompleted && (
          <div className="absolute top-2 right-2 badge-success px-2 py-1 rounded-md flex items-center gap-1">
            <CheckIcon className="w-3 h-3" />
            <span className="text-xs font-medium">Done</span>
          </div>
        )}
        {isError && (
          <div className="absolute top-2 right-2 badge-error px-2 py-1 rounded-md flex items-center gap-1">
            <AlertCircleIcon className="w-3 h-3" />
            <span className="text-xs font-medium">Error</span>
          </div>
        )}

        {/* Hover action overlay - for completed downloads */}
        {isCompleted && download.output_path && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: isHovered ? 1 : 0 }}
            className="absolute inset-0 bg-black/60 flex items-center justify-center gap-3"
          >
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
          </motion.div>
        )}

        {/* Hover action overlay with remove - for non-active downloads */}
        {!isActive && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: isHovered ? 1 : 0 }}
            className="absolute inset-0 pointer-events-none"
          >
            <button
              onClick={handleClear}
              className="absolute top-2 left-2 p-2 rounded-lg bg-error/80 hover:bg-error text-white transition-colors pointer-events-auto"
              aria-label="Remove from list"
            >
              <TrashIcon className="w-4 h-4" />
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
        {isActive && (
          <div className="flex items-center justify-between text-xs text-text-secondary mt-2">
            <span className="tabular-nums font-medium text-accent">{download.progress.toFixed(1)}%</span>
            <span className="flex items-center gap-2">
              {download.speed && <span className="tabular-nums">{download.speed}</span>}
              {download.eta && <span className="tabular-nums text-text-tertiary">ETA {download.eta}</span>}
            </span>
          </div>
        )}

        {/* Error message */}
        {isError && download.error && (
          <p className="mt-1 text-xs text-error line-clamp-2" title={download.error}>{download.error}</p>
        )}

        {/* Completed state */}
        {isCompleted && (
          <p className="text-xs text-text-tertiary">
            {download.format.toUpperCase()}
          </p>
        )}

        {/* Cancelled state */}
        {isCancelled && (
          <p className="text-xs text-text-tertiary">Cancelled</p>
        )}

        {/* Actions - only show for active downloads */}
        {isActive && (
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
