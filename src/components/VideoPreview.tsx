import { useState, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import type { VideoInfo, FormatPreset } from '@/lib/types';
import { formatDuration, formatViewCount, formatBytes, cn } from '@/lib/utils';
import { DownloadIcon, ChevronDownIcon, XIcon } from './Icons';

interface VideoPreviewProps {
  video: VideoInfo;
  onDownload: (format: string) => void;
  onClose: () => void;
  isDownloading?: boolean;
}

const formatPresets: { id: FormatPreset; label: string; description: string }[] = [
  { id: 'best', label: 'Best Quality', description: 'Highest available quality' },
  { id: '1080p', label: '1080p', description: 'Full HD video' },
  { id: '720p', label: '720p', description: 'HD video' },
  { id: '480p', label: '480p', description: 'Standard quality' },
  { id: 'audio', label: 'Audio Only', description: 'Best audio, no video' },
];

export function VideoPreview({
  video,
  onDownload,
  onClose,
  isDownloading = false,
}: VideoPreviewProps) {
  const [selectedFormat, setSelectedFormat] = useState<FormatPreset>('best');
  const [showFormats, setShowFormats] = useState(false);

  const selectedPreset = useMemo(
    () => formatPresets.find(p => p.id === selectedFormat),
    [selectedFormat]
  );

  const estimatedSize = useMemo(() => {
    const getSize = (f: typeof video.formats[0]) => f.filesize ?? f.filesize_approx ?? 0;

    // For audio-only, just find the best audio stream
    if (selectedFormat === 'audio') {
      const audioFormats = video.formats.filter(f => f.vcodec === 'none' && f.acodec !== 'none');
      const bestAudio = audioFormats.sort((a, b) => getSize(b) - getSize(a))[0];
      return bestAudio ? formatBytes(getSize(bestAudio) || null) : null;
    }

    // For video formats, we need video + audio combined
    // Find video-only streams (has video, no audio)
    const videoOnlyFormats = video.formats.filter(f => f.vcodec !== 'none' && f.acodec === 'none');
    // Find audio-only streams
    const audioOnlyFormats = video.formats.filter(f => f.vcodec === 'none' && f.acodec !== 'none');

    // Get the best audio stream size (will be merged with video)
    const bestAudio = audioOnlyFormats.sort((a, b) => getSize(b) - getSize(a))[0];
    const audioSize = bestAudio ? getSize(bestAudio) : 0;

    // Filter video streams by resolution
    let filteredVideo = videoOnlyFormats;
    if (selectedFormat === '1080p') {
      filteredVideo = videoOnlyFormats.filter(f => f.resolution?.includes('1080'));
    } else if (selectedFormat === '720p') {
      filteredVideo = videoOnlyFormats.filter(f => f.resolution?.includes('720'));
    } else if (selectedFormat === '480p') {
      filteredVideo = videoOnlyFormats.filter(f => f.resolution?.includes('480'));
    }

    // For 'best', get the largest video stream; for specific resolutions, get best match
    const bestVideo = filteredVideo.sort((a, b) => getSize(b) - getSize(a))[0];

    if (bestVideo) {
      const totalSize = getSize(bestVideo) + audioSize;
      return totalSize > 0 ? formatBytes(totalSize) : null;
    }

    // Fallback: check for combined formats (has both video and audio)
    const combinedFormats = video.formats.filter(f => f.vcodec !== 'none' && f.acodec !== 'none');
    const bestCombined = combinedFormats.sort((a, b) => getSize(b) - getSize(a))[0];
    return bestCombined ? formatBytes(getSize(bestCombined) || null) : null;
  }, [video.formats, selectedFormat]);

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: 20 }}
      transition={{ type: 'spring', stiffness: 400, damping: 25 }}
      className="w-full max-w-2xl bg-bg-secondary rounded-xl border border-border"
    >
      {/* Thumbnail and basic info */}
      <div className="flex gap-4 p-4">
        {video.thumbnail ? (
          <div className="relative w-40 h-24 rounded-lg overflow-hidden shrink-0 bg-bg-tertiary">
            <img
              src={video.thumbnail}
              alt=""
              className="w-full h-full object-cover"
              loading="lazy"
            />
            {video.duration !== null && (
              <span className="absolute bottom-1 right-1 px-1.5 py-0.5 text-xs font-medium bg-black/80 text-white rounded tabular-nums">
                {formatDuration(video.duration)}
              </span>
            )}
          </div>
        ) : (
          <div className="w-40 h-24 rounded-lg bg-bg-tertiary shrink-0" />
        )}

        <div className="flex-1 min-w-0">
          <div className="flex items-start justify-between gap-2">
            <h2 className="text-base font-medium text-text-primary line-clamp-2">
              {video.title}
            </h2>
            <button
              onClick={onClose}
              className="p-1 rounded hover:bg-bg-tertiary text-text-secondary hover:text-text-primary transition-colors shrink-0"
              aria-label="Close preview"
            >
              <XIcon className="w-5 h-5" />
            </button>
          </div>

          {video.channel && (
            <p className="text-sm text-text-secondary mt-1">{video.channel}</p>
          )}

          {video.view_count !== null && (
            <p className="text-xs text-text-tertiary mt-1">
              {formatViewCount(video.view_count)}
            </p>
          )}
        </div>
      </div>

      {/* Format selector */}
      <div className="px-4 pb-4">
        <div className="relative">
          <button
            onClick={() => setShowFormats(!showFormats)}
            className={cn(
              'w-full flex items-center justify-between gap-2 px-4 py-3',
              'bg-bg-tertiary rounded-lg border border-border',
              'hover:border-border-hover transition-colors',
              'text-left'
            )}
            aria-expanded={showFormats}
            aria-haspopup="listbox"
          >
            <div>
              <span className="text-sm font-medium text-text-primary">
                {selectedPreset?.label}
              </span>
              <span className="text-xs text-text-tertiary ml-2">
                {selectedPreset?.description}
                {estimatedSize && ` · ~${estimatedSize}`}
              </span>
            </div>
            <motion.div
              animate={{ rotate: showFormats ? 180 : 0 }}
              transition={{ duration: 0.2 }}
            >
              <ChevronDownIcon className="w-5 h-5 text-text-secondary" />
            </motion.div>
          </button>

          <AnimatePresence>
            {showFormats && (
              <motion.ul
                initial={{ opacity: 0, y: -10 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -10 }}
                transition={{ duration: 0.15 }}
                className="absolute top-full left-0 right-0 mt-2 z-10 bg-bg-secondary border border-border rounded-lg overflow-hidden shadow-lg"
                role="listbox"
                aria-label="Select format"
              >
                {formatPresets.map((preset) => (
                  <li key={preset.id}>
                    <button
                      onClick={() => {
                        setSelectedFormat(preset.id);
                        setShowFormats(false);
                      }}
                      className={cn(
                        'w-full px-4 py-3 text-left',
                        'hover:bg-bg-tertiary transition-colors',
                        selectedFormat === preset.id && 'bg-accent/10'
                      )}
                      role="option"
                      aria-selected={selectedFormat === preset.id}
                    >
                      <span className="text-sm font-medium text-text-primary">
                        {preset.label}
                      </span>
                      <span className="text-xs text-text-tertiary ml-2">
                        {preset.description}
                      </span>
                    </button>
                  </li>
                ))}
              </motion.ul>
            )}
          </AnimatePresence>
        </div>

        {/* Download button */}
        <motion.button
          onClick={() => onDownload(selectedFormat)}
          disabled={isDownloading}
          className={cn(
            'w-full mt-3 py-3 px-4 rounded-lg',
            'bg-accent hover:bg-accent-hover text-white font-medium',
            'flex items-center justify-center gap-2',
            'transition-colors disabled:opacity-50 disabled:cursor-not-allowed'
          )}
          whileTap={{ scale: 0.98 }}
        >
          <DownloadIcon className="w-5 h-5" />
          <span>{isDownloading ? 'Starting download...' : 'Download'}</span>
        </motion.button>
      </div>
    </motion.div>
  );
}
