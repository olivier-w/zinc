import { useState, useCallback } from 'react';
import { AnimatePresence, motion } from 'framer-motion';
import type { Download } from '@/lib/types';
import { openFile, openFolder } from '@/lib/tauri';
import { DownloadCard } from './DownloadCard';
import { ChevronUpIcon, ChevronDownIcon, TrashIcon, LoaderIcon } from './Icons';

interface DownloadsTrayProps {
  downloads: Download[];
  onCancel: (id: string) => void;
  onClear: (id: string) => void;
  onClearCompleted: () => void;
  hasCompletedDownloads: boolean;
}

export function DownloadsTray({
  downloads,
  onCancel,
  onClear,
  onClearCompleted,
  hasCompletedDownloads,
}: DownloadsTrayProps) {
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

  const activeCount = downloads.filter(d => d.status === 'downloading' || d.status === 'pending' || d.status === 'transcribing' || d.status.startsWith('transcribing:')).length;

  return (
    <motion.div
      className="downloads-tray"
      initial={{ y: 100 }}
      animate={{ y: 0 }}
      exit={{ y: 100 }}
      transition={{
        type: 'spring',
        stiffness: 300,
        damping: 30,
      }}
    >
      {/* Header bar - always visible */}
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full flex items-center justify-between px-4 py-3 hover:bg-white/5 transition-colors"
      >
        <div className="flex items-center gap-3">
          {isExpanded ? (
            <ChevronDownIcon className="w-4 h-4 text-text-tertiary" />
          ) : (
            <ChevronUpIcon className="w-4 h-4 text-text-tertiary" />
          )}
          <span className="text-sm font-medium text-text-secondary">
            {activeCount > 0 ? (
              <span className="flex items-center gap-2">
                <LoaderIcon className="w-3.5 h-3.5 animate-spin text-accent" />
                <span>{activeCount} downloading</span>
              </span>
            ) : (
              `${downloads.length} download${downloads.length !== 1 ? 's' : ''}`
            )}
          </span>
        </div>

        {hasCompletedDownloads && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              onClearCompleted();
            }}
            className="flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-text-tertiary hover:text-text-secondary hover:bg-bg-tertiary rounded-lg transition-colors"
          >
            <TrashIcon className="w-3.5 h-3.5" />
            <span>Clear completed</span>
          </button>
        )}
      </button>

      {/* Expandable content */}
      <AnimatePresence>
        {isExpanded && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{
              type: 'spring',
              stiffness: 300,
              damping: 30,
            }}
            className="overflow-hidden"
          >
            <div className="downloads-tray-content px-4 pb-4">
              <div className="flex gap-4">
                <AnimatePresence mode="popLayout">
                  {downloads.map(download => (
                    <motion.div
                      key={download.id}
                      layout
                      className="w-64 shrink-0"
                    >
                      <DownloadCard
                        download={download}
                        onCancel={onCancel}
                        onClear={onClear}
                        onOpenFile={handleOpenFile}
                        onOpenFolder={handleOpenFolder}
                      />
                    </motion.div>
                  ))}
                </AnimatePresence>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
}
