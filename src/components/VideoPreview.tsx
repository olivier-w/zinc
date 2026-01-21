import { useState, useMemo } from 'react';
import { motion } from 'framer-motion';
import type { VideoInfo, FormatPreset, SubtitleSettings } from '@/lib/types';
import { formatDuration, formatViewCount, formatBytes, cn } from '@/lib/utils';
import { QUALITY_PRESETS, VIDEO_FORMATS, AUDIO_FORMATS, SUBTITLE_LANGUAGES, TRANSCRIPTION_ENGINES, getSpeedMultiplier, type VideoFormatId, type AudioFormatId } from '@/lib/constants';
import { DownloadIcon, XIcon, ChevronDownIcon } from './Icons';

interface VideoPreviewProps {
  video: VideoInfo;
  onDownload: (format: string, subtitleSettings?: SubtitleSettings) => void;
  onClose: () => void;
  isDownloading?: boolean;
  defaultSubtitlesEnabled?: boolean;
  transcriptionEngine?: string;
  transcriptionModel?: string;
}

export function VideoPreview({
  video,
  onDownload,
  onClose,
  isDownloading = false,
  defaultSubtitlesEnabled = false,
  transcriptionEngine = 'whisper_cpp',
  transcriptionModel = 'base',
}: VideoPreviewProps) {
  const [selectedQuality, setSelectedQuality] = useState<FormatPreset>('best');
  const [selectedContainer, setSelectedContainer] = useState<VideoFormatId>('original');
  const [selectedAudioFormat, setSelectedAudioFormat] = useState<AudioFormatId>('original');
  const [subtitlesEnabled, setSubtitlesEnabled] = useState(defaultSubtitlesEnabled);
  const [subtitleLanguage, setSubtitleLanguage] = useState('auto');
  const [isLanguageDropdownOpen, setIsLanguageDropdownOpen] = useState(false);

  const isAudioOnly = selectedQuality === 'audio';

  const estimatedSize = useMemo(() => {
    const getSize = (f: typeof video.formats[0]) => f.filesize ?? f.filesize_approx ?? 0;

    // For audio-only, just find the best audio stream
    if (selectedQuality === 'audio') {
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
    if (selectedQuality === '1080p') {
      filteredVideo = videoOnlyFormats.filter(f => f.resolution?.includes('1080'));
    } else if (selectedQuality === '720p') {
      filteredVideo = videoOnlyFormats.filter(f => f.resolution?.includes('720'));
    } else if (selectedQuality === '480p') {
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
  }, [video, selectedQuality]);

  // Calculate estimated transcription time based on selected engine
  const transcriptionEta = useMemo(() => {
    if (!subtitlesEnabled || !video.duration || isAudioOnly) return null;

    // Get speed multiplier for the selected engine and model
    // Parakeet requires GPU, so assume GPU for it; others use CPU
    const useGpu = transcriptionEngine === 'parakeet';
    const multiplier = getSpeedMultiplier(transcriptionEngine, transcriptionModel, useGpu);
    const seconds = Math.ceil(video.duration / multiplier) + 10; // +10s overhead

    if (seconds < 60) return `~${seconds}s`;
    const minutes = Math.ceil(seconds / 60);
    return `~${minutes} min`;
  }, [subtitlesEnabled, video.duration, transcriptionEngine, transcriptionModel, isAudioOnly]);

  // Get engine display name
  const engineDisplayName = useMemo(() => {
    const engine = TRANSCRIPTION_ENGINES.find(e => e.id === transcriptionEngine);
    const model = engine?.models.find(m => m.id === transcriptionModel);
    if (engine && model) {
      return `${engine.name} (${model.name})`;
    }
    return `${transcriptionEngine} (${transcriptionModel})`;
  }, [transcriptionEngine, transcriptionModel]);

  const selectedLanguageName = SUBTITLE_LANGUAGES.find(l => l.code === subtitleLanguage)?.name || 'Auto-detect';

  return (
    <motion.div
      initial={{ opacity: 0, y: 20, scale: 0.98 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, y: 20, scale: 0.98 }}
      transition={{ type: 'spring', stiffness: 400, damping: 25 }}
      className="w-full max-w-4xl max-h-[calc(100vh-200px)] min-h-[280px] glass rounded-2xl overflow-y-auto relative"
    >
      {/* Close button */}
      <button
        onClick={onClose}
        className="absolute top-3 right-3 z-10 p-2 rounded-lg bg-black/50 hover:bg-black/70 text-white/80 hover:text-white transition-colors"
        aria-label="Close preview"
      >
        <XIcon className="w-5 h-5" />
      </button>

      {/* Hero layout: side-by-side on larger screens */}
      <div className="flex flex-col md:flex-row">
        {/* Large thumbnail */}
        <div className="relative md:w-1/2 h-32 md:h-auto md:min-h-[200px] bg-bg-tertiary shrink-0">
          {video.thumbnail ? (
            <img
              src={video.thumbnail}
              alt=""
              className="w-full h-full object-cover"
              loading="lazy"
            />
          ) : (
            <div className="w-full h-full bg-bg-tertiary" />
          )}

          {/* Gradient overlay - top and bottom */}
          <div className="absolute inset-0 bg-gradient-to-b from-black/70 via-transparent to-black/70" />

          {/* Channel and views - top left */}
          <div className="absolute top-3 left-3">
            {video.channel && (
              <p className="text-sm font-medium text-white/90 drop-shadow-md">{video.channel}</p>
            )}
            {video.view_count !== null && (
              <p className="text-xs text-white/70 drop-shadow-md">
                {formatViewCount(video.view_count)}
              </p>
            )}
          </div>

          {/* Duration badge - bottom left */}
          {video.duration !== null && (
            <span className="absolute bottom-3 left-3 px-2 py-1 text-sm font-medium bg-black/80 text-white rounded-md tabular-nums">
              {formatDuration(video.duration)}
            </span>
          )}
        </div>

        {/* Content side */}
        <div className="flex-1 p-3 md:p-4 flex flex-col min-h-0 min-w-0 overflow-hidden">
          {/* Title */}
          <h2 className="text-sm md:text-base font-semibold text-text-primary leading-snug mb-2 pr-8 line-clamp-2 whitespace-normal">
            {video.title}
          </h2>

          {/* Quality pills */}
          <div className="mb-2">
            <p className="text-xs text-text-tertiary mb-1.5 uppercase tracking-wide">Quality</p>
            <div className="flex flex-wrap gap-1.5">
              {QUALITY_PRESETS.map((preset) => (
                <button
                  key={preset.id}
                  onClick={() => setSelectedQuality(preset.id)}
                  className={cn(
                    'px-2.5 py-1 text-sm font-medium rounded-lg transition-all',
                    selectedQuality === preset.id
                      ? 'bg-accent text-white'
                      : 'bg-bg-tertiary text-text-secondary hover:text-text-primary hover:bg-bg-tertiary/80'
                  )}
                >
                  {preset.shortLabel}
                </button>
              ))}
            </div>
          </div>

          {/* Format pills */}
          <div className="mb-2">
            <p className="text-xs text-text-tertiary mb-1.5 uppercase tracking-wide">Format</p>
            <div className="flex flex-wrap gap-1.5">
              {(isAudioOnly ? AUDIO_FORMATS : VIDEO_FORMATS).map((format) => (
                <button
                  key={format.id}
                  onClick={() => isAudioOnly
                    ? setSelectedAudioFormat(format.id as AudioFormatId)
                    : setSelectedContainer(format.id as VideoFormatId)
                  }
                  className={cn(
                    'px-2.5 py-1 text-sm font-medium rounded-lg transition-all',
                    (isAudioOnly ? selectedAudioFormat : selectedContainer) === format.id
                      ? 'bg-accent text-white'
                      : 'bg-bg-tertiary text-text-secondary hover:text-text-primary hover:bg-bg-tertiary/80'
                  )}
                >
                  {format.label}
                </button>
              ))}
            </div>
          </div>

          {/* Size estimate */}
          {estimatedSize && (
            <p className="text-xs text-text-tertiary mb-2">
              Estimated size: ~{estimatedSize}
            </p>
          )}

          {/* Subtitles section - only for video formats */}
          {!isAudioOnly && (
            <div className="mb-3">
              <div className="flex items-center justify-between mb-1.5">
                <p className="text-xs text-text-tertiary uppercase tracking-wide">Subtitles</p>
                <button
                  onClick={() => setSubtitlesEnabled(!subtitlesEnabled)}
                  className={cn(
                    'relative inline-flex h-5 w-9 items-center rounded-full transition-colors',
                    subtitlesEnabled ? 'bg-accent' : 'bg-bg-tertiary'
                  )}
                >
                  <span
                    className={cn(
                      'inline-block h-3.5 w-3.5 transform rounded-full bg-white transition-transform',
                      subtitlesEnabled ? 'translate-x-4.5' : 'translate-x-0.5'
                    )}
                  />
                </button>
              </div>

              {subtitlesEnabled && (
                <div className="px-3 py-2 rounded-lg bg-bg-tertiary/50 space-y-2">
                  {/* Language dropdown */}
                  <div className="relative">
                    <button
                      onClick={() => setIsLanguageDropdownOpen(!isLanguageDropdownOpen)}
                      className="w-full flex items-center justify-between px-2.5 py-1.5 text-sm rounded-md bg-bg-secondary hover:bg-bg-secondary/80 transition-colors"
                    >
                      <span className="text-text-secondary">{selectedLanguageName}</span>
                      <ChevronDownIcon className={cn(
                        'w-4 h-4 text-text-tertiary transition-transform',
                        isLanguageDropdownOpen && 'rotate-180'
                      )} />
                    </button>

                    {isLanguageDropdownOpen && (
                      <div className="absolute z-20 top-full left-0 right-0 mt-1 max-h-40 overflow-y-auto rounded-lg bg-bg-secondary border border-border shadow-lg">
                        {SUBTITLE_LANGUAGES.map((lang) => (
                          <button
                            key={lang.code}
                            onClick={() => {
                              setSubtitleLanguage(lang.code);
                              setIsLanguageDropdownOpen(false);
                            }}
                            className={cn(
                              'w-full px-3 py-1.5 text-sm text-left hover:bg-bg-tertiary transition-colors',
                              subtitleLanguage === lang.code ? 'text-accent' : 'text-text-secondary'
                            )}
                          >
                            {lang.name}
                          </button>
                        ))}
                      </div>
                    )}
                  </div>

                  {/* ETA display */}
                  {transcriptionEta && (
                    <p className="text-xs text-text-tertiary">
                      Est. time: {transcriptionEta} ({engineDisplayName})
                    </p>
                  )}
                </div>
              )}
            </div>
          )}

          {/* Download button - full width with glow */}
          <motion.button
            onClick={() => {
              const format = isAudioOnly ? selectedAudioFormat : selectedContainer;
              const settings: SubtitleSettings | undefined = !isAudioOnly ? {
                enabled: subtitlesEnabled,
                engine: transcriptionEngine,
                model: transcriptionModel,
                language: subtitleLanguage,
              } : undefined;
              onDownload(`${selectedQuality}:${format}`, settings);
            }}
            disabled={isDownloading}
            className={cn(
              'w-full mt-auto py-3 px-4 rounded-xl',
              'btn-gradient text-white font-medium',
              'flex items-center justify-center gap-2',
              'disabled:opacity-50 disabled:cursor-not-allowed'
            )}
            whileTap={{ scale: 0.98 }}
          >
            <DownloadIcon className="w-5 h-5" />
            <span>{isDownloading ? 'Starting download...' : 'Download'}</span>
          </motion.button>
        </div>
      </div>
    </motion.div>
  );
}
