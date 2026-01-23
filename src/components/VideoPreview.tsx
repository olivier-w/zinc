import { useState, useMemo, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import type { VideoInfo, FormatPreset, SubtitleSettings, TranscriptionEngine } from '@/lib/types';
import { formatDuration, formatViewCount, formatBytes, cn } from '@/lib/utils';
import { QUALITY_PRESETS, VIDEO_FORMATS, AUDIO_FORMATS, type VideoFormatId, type AudioFormatId } from '@/lib/constants';
import { getTranscriptionEngines } from '@/lib/tauri';
import { DownloadIcon, XIcon, ChevronRightIcon } from './Icons';

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
  transcriptionEngine = 'whisper_rs',
  transcriptionModel = 'base',
}: VideoPreviewProps) {
  const [selectedQuality, setSelectedQuality] = useState<FormatPreset>('best');
  const [selectedContainer, setSelectedContainer] = useState<VideoFormatId>('original');
  const [selectedAudioFormat, setSelectedAudioFormat] = useState<AudioFormatId>('original');
  const [subtitlesEnabled, setSubtitlesEnabled] = useState(defaultSubtitlesEnabled);
  const [advancedOpen, setAdvancedOpen] = useState(false);

  // Engines fetched from backend
  const [engines, setEngines] = useState<TranscriptionEngine[]>([]);

  // Local engine/model state - initialized from props
  const [selectedEngine, setSelectedEngine] = useState(transcriptionEngine);
  const [selectedModel, setSelectedModel] = useState(transcriptionModel);
  const [selectedStyle, setSelectedStyle] = useState<'word' | 'sentence'>('sentence');

  // Fetch engines from backend and set initial selection
  useEffect(() => {
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
  }, []); // Only run once on mount

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

    // Get speed multiplier from the actual model data
    const model = currentEngine?.models.find(m => m.id === selectedModel);
    // Use GPU speed for whisper_rs, CPU speed for others
    const useGpu = selectedEngine === 'whisper_rs';
    const multiplier = model ? (useGpu ? model.speed_gpu : model.speed_cpu) : 5;
    const seconds = Math.ceil(video.duration / multiplier) + 10; // +10s overhead

    if (seconds < 60) return `~${seconds}s`;
    const minutes = Math.ceil(seconds / 60);
    return `~${minutes} min`;
  }, [subtitlesEnabled, video.duration, selectedEngine, selectedModel, currentEngine, isAudioOnly]);

  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: 20, scale: 0.98 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, scale: 0.96 }}
      transition={{
        type: 'spring',
        stiffness: 400,
        damping: 30,
        opacity: { duration: 0.15 },
      }}
      className="w-full max-w-4xl glass rounded-2xl relative"
    >
      {/* Close button - top left for less cramped layout */}
      <button
        onClick={onClose}
        className="absolute top-3 left-3 z-10 p-2 rounded-lg bg-black/50 hover:bg-black/70 text-white/80 hover:text-white transition-colors"
        aria-label="Close preview"
      >
        <XIcon className="w-5 h-5" />
      </button>

      {/* Hero layout: side-by-side on larger screens */}
      <div className="flex flex-col md:flex-row">
        {/* Large thumbnail */}
        <div className="relative md:w-1/2 h-32 md:h-auto md:min-h-[180px] bg-bg-tertiary shrink-0">
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
        <div className="flex-1 p-4 md:p-5 flex flex-col min-w-0">
          {/* Title */}
          <h2 className="text-base font-semibold text-text-primary leading-snug mb-1 pr-8 line-clamp-2">
            {video.title}
          </h2>

          {/* Metadata line: channel · duration */}
          <p className="text-[12px] text-text-secondary mb-3">
            {video.channel}
            {video.channel && video.duration !== null && ' · '}
            {video.duration !== null && formatDuration(video.duration)}
          </p>

          {/* Advanced section */}
          <div className="mb-4">
            {/* Advanced toggle button */}
            <button
              onClick={() => setAdvancedOpen(!advancedOpen)}
              className="flex items-center gap-1.5 text-[12px] text-text-secondary hover:text-text-primary transition-colors mb-2"
              aria-expanded={advancedOpen}
              aria-controls="advanced-options-panel"
            >
              <ChevronRightIcon
                className={cn(
                  'w-4 h-4 transition-transform duration-200',
                  advancedOpen && 'rotate-90'
                )}
              />
              <span>Advanced</span>
            </button>

            {/* Collapsible advanced options */}
            <AnimatePresence>
              {advancedOpen && (
                <motion.div
                  id="advanced-options-panel"
                  role="region"
                  aria-label="Advanced options"
                  initial={{ height: 0, opacity: 0 }}
                  animate={{ height: 'auto', opacity: 1 }}
                  exit={{ height: 0, opacity: 0 }}
                  transition={{ type: 'spring', stiffness: 400, damping: 30 }}
                  className="overflow-hidden"
                >
                  <div className="space-y-2.5 pl-5">
                    {/* Quality row */}
                    <motion.div
                      initial={{ opacity: 0, y: -4 }}
                      animate={{ opacity: 1, y: 0 }}
                      transition={{ delay: 0 }}
                      className="flex items-center gap-2.5 h-6"
                    >
                      <span className="text-[11px] text-text-tertiary uppercase tracking-wide w-14 shrink-0">Quality</span>
                      <div className="flex gap-0.5 flex-wrap">
                        {QUALITY_PRESETS.map((preset) => (
                          <button
                            key={preset.id}
                            onClick={() => setSelectedQuality(preset.id)}
                            className={cn(
                              'px-2 py-0.5 text-[11px] font-medium rounded transition-all',
                              selectedQuality === preset.id
                                ? 'bg-accent text-white'
                                : 'bg-bg-tertiary text-text-secondary hover:text-text-primary'
                            )}
                          >
                            {preset.shortLabel}
                          </button>
                        ))}
                      </div>
                    </motion.div>

                    {/* Format row */}
                    <motion.div
                      initial={{ opacity: 0, y: -4 }}
                      animate={{ opacity: 1, y: 0 }}
                      transition={{ delay: 0.05 }}
                      className="flex items-center gap-2.5 h-6"
                    >
                      <span className="text-[11px] text-text-tertiary uppercase tracking-wide w-14 shrink-0">Format</span>
                      <div className="flex gap-0.5">
                        {(isAudioOnly ? AUDIO_FORMATS : VIDEO_FORMATS).map((format) => (
                          <button
                            key={format.id}
                            onClick={() => isAudioOnly
                              ? setSelectedAudioFormat(format.id as AudioFormatId)
                              : setSelectedContainer(format.id as VideoFormatId)
                            }
                            className={cn(
                              'px-2 py-0.5 text-[11px] font-medium rounded transition-all',
                              (isAudioOnly ? selectedAudioFormat : selectedContainer) === format.id
                                ? 'bg-accent text-white'
                                : 'bg-bg-tertiary text-text-secondary hover:text-text-primary'
                            )}
                          >
                            {format.label}
                          </button>
                        ))}
                      </div>
                    </motion.div>

                    {/* Subtitles section - only for video */}
                    {!isAudioOnly && (
                      <div className="space-y-2">
                        {/* Subtitles header row */}
                        <motion.div
                          initial={{ opacity: 0, y: -4 }}
                          animate={{ opacity: 1, y: 0 }}
                          transition={{ delay: 0.1 }}
                          className="flex items-center gap-2.5 h-6"
                        >
                          <span className="text-[11px] text-text-tertiary uppercase tracking-wide w-14 shrink-0">Subtitles</span>
                          <div className="flex items-center gap-2 flex-1">
                            <button
                              onClick={() => setSubtitlesEnabled(!subtitlesEnabled)}
                              aria-expanded={subtitlesEnabled}
                              aria-controls="subtitle-options-panel"
                              className={cn(
                                'relative inline-flex h-5 w-9 items-center rounded-full transition-colors shrink-0',
                                subtitlesEnabled ? 'bg-accent' : 'bg-bg-tertiary'
                              )}
                            >
                              <span
                                className={cn(
                                  'inline-block h-3.5 w-3.5 transform rounded-full bg-white transition-transform',
                                  subtitlesEnabled ? 'translate-x-4.5' : 'translate-x-1'
                                )}
                              />
                            </button>
                            {subtitlesEnabled && transcriptionEta && (
                              <span className="text-[10px] text-text-tertiary">{transcriptionEta}</span>
                            )}
                          </div>
                        </motion.div>

                        {/* Collapsible subtitle options panel */}
                        <AnimatePresence>
                          {subtitlesEnabled && (
                            <motion.div
                              id="subtitle-options-panel"
                              role="region"
                              aria-label="Subtitle options"
                              initial={{ height: 0, opacity: 0 }}
                              animate={{ height: 'auto', opacity: 1 }}
                              exit={{ height: 0, opacity: 0 }}
                              transition={{ type: 'spring', stiffness: 400, damping: 30 }}
                              className="overflow-hidden"
                            >
                              <div className="space-y-2.5 pt-1">
                                {/* No models installed warning */}
                                {availableEngines.length === 0 && engines.length > 0 && (
                                  <motion.div
                                    initial={{ opacity: 0, y: -4 }}
                                    animate={{ opacity: 1, y: 0 }}
                                    transition={{ delay: 0.05 }}
                                    className="flex items-center gap-2.5 h-6"
                                  >
                                    <span className="text-[11px] text-text-tertiary uppercase tracking-wide w-14 shrink-0"></span>
                                    <span className="text-[11px] text-warning">No models installed — check Settings</span>
                                  </motion.div>
                                )}

                                {availableEngines.length > 0 && (
                                  <>
                                    {/* Engine selector - segmented control */}
                                    {availableEngines.length > 1 && (
                                      <motion.div
                                        initial={{ opacity: 0, y: -4 }}
                                        animate={{ opacity: 1, y: 0 }}
                                        transition={{ delay: 0.05 }}
                                        className="flex items-center gap-2.5 h-6"
                                      >
                                        <span className="text-[11px] text-text-tertiary uppercase tracking-wide w-14 shrink-0">Engine</span>
                                        <div className="flex rounded-md overflow-hidden border border-border">
                                          {availableEngines.map((engine) => (
                                            <button
                                              key={engine.id}
                                              onClick={() => {
                                                setSelectedEngine(engine.id);
                                                // Auto-select first installed model for the new engine
                                                const firstModel = engine.models.find(m => m.installed);
                                                if (firstModel) setSelectedModel(firstModel.id);
                                              }}
                                              className={cn(
                                                'px-2.5 py-0.5 text-[11px] font-medium transition-colors',
                                                'border-r border-border last:border-r-0',
                                                selectedEngine === engine.id
                                                  ? 'bg-accent text-white'
                                                  : 'bg-bg-tertiary text-text-secondary hover:text-text-primary hover:bg-bg-tertiary/80'
                                              )}
                                              title={engine.description}
                                            >
                                              {engine.name}
                                            </button>
                                          ))}
                                        </div>
                                      </motion.div>
                                    )}

                                    {/* Model selector - pill buttons */}
                                    {installedModels.length > 1 && (
                                      <motion.div
                                        initial={{ opacity: 0, y: -4 }}
                                        animate={{ opacity: 1, y: 0 }}
                                        transition={{ delay: 0.1 }}
                                        className="flex items-center gap-2.5 h-6"
                                      >
                                        <span className="text-[11px] text-text-tertiary uppercase tracking-wide w-14 shrink-0">Model</span>
                                        <div className="flex gap-0.5 flex-wrap">
                                          {installedModels.map((model, i) => (
                                            <motion.button
                                              key={model.id}
                                              initial={{ opacity: 0, scale: 0.95 }}
                                              animate={{ opacity: 1, scale: 1 }}
                                              transition={{ delay: 0.1 + i * 0.03 }}
                                              onClick={() => setSelectedModel(model.id)}
                                              className={cn(
                                                'px-2 py-0.5 text-[11px] font-medium rounded transition-all',
                                                selectedModel === model.id
                                                  ? 'bg-accent text-white'
                                                  : 'bg-bg-tertiary text-text-secondary hover:text-text-primary'
                                              )}
                                            >
                                              {model.name}
                                            </motion.button>
                                          ))}
                                        </div>
                                      </motion.div>
                                    )}

                                    {/* Style selector */}
                                    <motion.div
                                      initial={{ opacity: 0, y: -4 }}
                                      animate={{ opacity: 1, y: 0 }}
                                      transition={{ delay: 0.15 }}
                                      className="flex items-center gap-2.5 h-6"
                                    >
                                      <span className="text-[11px] text-text-tertiary uppercase tracking-wide w-14 shrink-0">Style</span>
                                      <div className="flex gap-0.5">
                                        <button
                                          onClick={() => setSelectedStyle('sentence')}
                                          className={cn(
                                            'px-2 py-0.5 text-[11px] font-medium rounded transition-all',
                                            selectedStyle === 'sentence'
                                              ? 'bg-accent text-white'
                                              : 'bg-bg-tertiary text-text-secondary hover:text-text-primary'
                                          )}
                                          title="Natural phrase groupings like movie subtitles"
                                        >
                                          Sentence
                                        </button>
                                        <button
                                          onClick={() => setSelectedStyle('word')}
                                          className={cn(
                                            'px-2 py-0.5 text-[11px] font-medium rounded transition-all',
                                            selectedStyle === 'word'
                                              ? 'bg-accent text-white'
                                              : 'bg-bg-tertiary text-text-secondary hover:text-text-primary'
                                          )}
                                          title="One word per subtitle for karaoke-style timing"
                                        >
                                          Word
                                        </button>
                                      </div>
                                    </motion.div>
                                  </>
                                )}
                              </div>
                            </motion.div>
                          )}
                        </AnimatePresence>
                      </div>
                    )}

                    {/* Size estimate */}
                    {estimatedSize && (
                      <motion.p
                        initial={{ opacity: 0, y: -4 }}
                        animate={{ opacity: 1, y: 0 }}
                        transition={{ delay: 0.15 }}
                        className="text-[11px] text-text-tertiary pl-[66px]"
                      >
                        Estimated size: ~{estimatedSize}
                      </motion.p>
                    )}
                  </div>
                </motion.div>
              )}
            </AnimatePresence>
          </div>

          {/* Download button - pushed to bottom */}
          <motion.button
            onClick={() => {
              const format = isAudioOnly ? selectedAudioFormat : selectedContainer;
              const settings: SubtitleSettings | undefined = !isAudioOnly ? {
                enabled: subtitlesEnabled,
                engine: selectedEngine,
                model: selectedModel,
                style: selectedStyle,
              } : undefined;
              onDownload(`${selectedQuality}:${format}`, settings);
            }}
            disabled={isDownloading}
            className={cn(
              'w-full mt-auto py-2.5 px-4 rounded-xl',
              'btn-gradient text-white font-medium text-sm',
              'flex items-center justify-center gap-2',
              'disabled:opacity-50 disabled:cursor-not-allowed'
            )}
            whileTap={{ scale: 0.98 }}
          >
            <DownloadIcon className="w-4 h-4" />
            <span>{isDownloading ? 'Starting...' : 'Download'}</span>
          </motion.button>
        </div>
      </div>
    </motion.div>
  );
}
