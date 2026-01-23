import { useState, useCallback } from 'react';
import { AnimatePresence, motion } from 'motion/react';
import type { Download } from '@/lib/types';
import { openFile, openFolder } from '@/lib/tauri';
import { DownloadRow } from './DownloadRow';
import { ChevronDownIcon, TrashIcon, LoaderIcon } from './Icons';

interface DownloadsSectionProps {
  downloads: Download[];
  onCancel: (id: string) => void;
  onClear: (id: string) => void;
  onClearCompleted: () => void;
  hasCompletedDownloads: boolean;
}

export function DownloadsSection({
  downloads,
  onCancel,
  onClear,
  onClearCompleted,
  hasCompletedDownloads,
}: DownloadsSectionProps) {
  const [isExpanded, setIsExpanded] = useState(true);

  const handleOpenFile = useCallback(async (path: string) => {
    try {
      await openFile(path);
    } catch (err) {
      console.error('Failed to open file:', err);
    }
  }, []);

  const handleOpenFolder = useCallback(async (path: string) => {
    try {
      await openFolder(path);
    } catch (err) {
      console.error('Failed to open folder:', err);
    }
  }, []);

  if (downloads.length === 0) {
    return null;
  }

  const activeCount = downloads.filter(d =>
    d.status === 'downloading' ||
    d.status === 'pending' ||
    d.status === 'transcribing' ||
    d.status.startsWith('transcribing:')
  ).length;

  return (
    <motion.div
      initial={{ opacity: 0, y: 15 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: 10 }}
      transition={{
        duration: 0.4,
        ease: [0.16, 1, 0.3, 1],
      }}
      className="w-full max-w-xl mt-6"
    >
      {/* Section header */}
      <div className="flex items-center justify-between mb-2">
        <button
          onClick={() => setIsExpanded(!isExpanded)}
          className="flex items-center gap-2 text-sm text-text-secondary hover:text-text-primary transition-colors"
        >
          <ChevronDownIcon className={`w-4 h-4 transition-transform ${isExpanded ? '' : '-rotate-90'}`} />
          <span className="font-medium">Downloads</span>
          <span className="text-text-tertiary">({downloads.length})</span>

          {/* Active indicator */}
          {activeCount > 0 && (
            <span className="flex items-center gap-1.5 ml-2 text-accent">
              <LoaderIcon className="w-3.5 h-3.5 animate-spin" />
              <span className="text-xs">{activeCount} active</span>
            </span>
          )}
        </button>

        {/* Clear completed button */}
        {hasCompletedDownloads && (
          <button
            onClick={onClearCompleted}
            className="flex items-center gap-1.5 px-2.5 py-1 text-xs text-text-tertiary hover:text-text-secondary hover:bg-bg-tertiary rounded-md transition-colors"
          >
            <TrashIcon className="w-3.5 h-3.5" />
            <span>Clear completed</span>
          </button>
        )}
      </div>

      {/* Download list */}
      {isExpanded && (
        <div className="flex flex-col gap-2">
          <AnimatePresence mode="popLayout" initial={false}>
            {downloads.map(download => (
              <DownloadRow
                key={download.id}
                download={download}
                onCancel={onCancel}
                onClear={onClear}
                onOpenFile={handleOpenFile}
                onOpenFolder={handleOpenFolder}
              />
            ))}
          </AnimatePresence>
        </div>
      )}
    </motion.div>
  );
}
