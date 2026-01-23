import { useCallback } from 'react';
import { AnimatePresence, motion } from 'motion/react';
import type { Download } from '@/lib/types';
import { openFile, openFolder } from '@/lib/tauri';
import { DownloadCard } from './DownloadCard';
import { TrashIcon } from './Icons';

interface DownloadListProps {
  downloads: Download[];
  onCancel: (id: string) => void;
  onClear: (id: string) => void;
  onClearCompleted: () => void;
  hasCompletedDownloads: boolean;
}

export function DownloadList({
  downloads,
  onCancel,
  onClear,
  onClearCompleted,
  hasCompletedDownloads,
}: DownloadListProps) {
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

  return (
    <motion.div layout className="w-full max-w-4xl">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-sm font-medium text-text-secondary">
          Downloads ({downloads.length})
        </h2>
        {hasCompletedDownloads && (
          <button
            onClick={onClearCompleted}
            className="flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-text-tertiary hover:text-text-secondary hover:bg-bg-tertiary rounded-lg transition-colors"
          >
            <TrashIcon className="w-3.5 h-3.5" />
            <span>Clear completed</span>
          </button>
        )}
      </div>

      {/* Responsive grid: 1 col on mobile, 2 on sm, 3 on md, 4 on lg */}
      <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
        <AnimatePresence mode="popLayout">
          {downloads.map(download => (
            <DownloadCard
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
    </motion.div>
  );
}
