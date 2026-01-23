import { useState, useMemo, useEffect } from 'react';
import { motion, AnimatePresence } from 'motion/react';
import type { VideoInfo, FormatPreset, SubtitleSettings, TranscriptionEngine } from '@/lib/types';
import { formatDuration, formatViewCount, formatBytes, cn } from '@/lib/utils';
import { QUALITY_PRESETS, VIDEO_FORMATS, AUDIO_FORMATS, type VideoFormatId, type AudioFormatId } from '@/lib/constants';
import { getTranscriptionEngines } from '@/lib/tauri';
import { DownloadIcon, XIcon, SubtitlesIcon } from './Icons';

interface VideoCardProps {
  video: VideoInfo;
  onDownload: (format: string, subtitleSettings?: SubtitleSettings) => void;
  onClose: () => void;
  isDownloading?: boolean;
  defaultSubtitlesEnabled?: boolean;
  transcriptionEngine?: string;
  transcriptionModel?: string;
}

export function VideoCard({
  video,
  onDownload,
  onClose,
  isDownloading = false,
  defaultSubtitlesEnabled = false,
  transcriptionEngine = 'whisper_rs',
  transcriptionModel = 'base',
}: VideoCardProps) {
  const [selectedQuality, setSelectedQuality] = useState<FormatPreset>('best');
  const [selectedContainer, setSelectedContainer] = useState<VideoFormatId>('original');
  const [selectedAudioFormat, setSelectedAudioFormat] = useState<AudioFormatId>('original');
  const [subtitlesEnabled, setSubtitlesEnabled] = useState(defaultSubtitlesEnabled);

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
    const videoOnlyFormats = video.formats.filter(f => f.vcodec !== 'none' && f.acodec === 'none');
    const audioOnlyFormats = video.formats.filter(f => f.vcodec === 'none' && f.acodec !== 'none');

    const bestAudio = audioOnlyFormats.sort((a, b) => getSize(b) - getSize(a))[0];
    const audioSize = bestAudio ? getSize(bestAudio) : 0;

    let filteredVideo = videoOnlyFormats;
    if (selectedQuality === '1080p') {
      filteredVideo = videoOnlyFormats.filter(f => f.resolution?.includes('1080'));
    } else if (selectedQuality === '720p') {
      filteredVideo = videoOnlyFormats.filter(f => f.resolution?.includes('720'));
    } else if (selectedQuality === '480p') {
      filteredVideo = videoOnlyFormats.filter(f => f.resolution?.includes('480'));
    }

    const bestVideo = filteredVideo.sort((a, b) => getSize(b) - getSize(a))[0];

    if (bestVideo) {
      const totalSize = getSize(bestVideo) + audioSize;
      return totalSize > 0 ? formatBytes(totalSize) : null;
    }

    const combinedFormats = video.formats.filter(f => f.vcodec !== 'none' && f.acodec !== 'none');
    const bestCombined = combinedFormats.sort((a, b) => getSize(b) - getSize(a))[0];
    return bestCombined ? formatBytes(getSize(bestCombined) || null) : null;
  }, [video, selectedQuality]);

  // Calculate estimated transcription time based on selected engine
  const transcriptionEta = useMemo(() => {
    if (!subtitlesEnabled || !video.duration || isAudioOnly) return null;

    const model = currentEngine?.models.find(m => m.id === selectedModel);
    const useGpu = selectedEngine === 'whisper_rs';
    const multiplier = model ? (useGpu ? model.speed_gpu : model.speed_cpu) : 5;
    const seconds = Math.ceil(video.duration / multiplier) + 10;

    if (seconds < 60) return `~${seconds}s`;
    const minutes = Math.ceil(seconds / 60);
    return `~${minutes} min`;
  }, [subtitlesEnabled, video.duration, selectedEngine, selectedModel, currentEngine, isAudioOnly]);

  const handleDownload = () => {
    const format = isAudioOnly ? selectedAudioFormat : selectedContainer;
    const settings: SubtitleSettings | undefined = !isAudioOnly ? {
      enabled: subtitlesEnabled,
      engine: selectedEngine,
      model: selectedModel,
      style: selectedStyle,
    } : undefined;
    onDownload(`${selectedQuality}:${format}`, settings);
  };

  return (
    <div className="w-full max-w-xl rounded-2xl overflow-hidden relative">
      {/* Full-width thumbnail with overlay */}
      <div className="relative aspect-video bg-bg-tertiary">
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

        {/* Strong gradient overlay for text readability */}
        <div className="absolute inset-0 video-card-overlay" />

        {/* Close button - top left */}
        <button
          onClick={onClose}
          className="absolute top-3 left-3 z-10 p-2 rounded-lg bg-black/50 hover:bg-black/70 text-white/80 hover:text-white transition-colors"
          aria-label="Close preview"
        >
          <XIcon className="w-5 h-5" />
        </button>

        {/* Duration badge - top right */}
        {video.duration !== null && (
          <span className="absolute top-3 right-3 px-2 py-1 text-sm font-medium bg-black/80 text-white rounded-md tabular-nums">
            {formatDuration(video.duration)}
          </span>
        )}

        {/* Overlay content - positioned at bottom */}
        <div className="absolute inset-x-0 bottom-0 p-4 flex flex-col gap-3">
          {/* Channel and view count */}
          <div className="flex items-center gap-2 text-xs">
            {video.channel && (
              <span className="text-white/80 font-medium">{video.channel}</span>
            )}
            {video.channel && video.view_count !== null && (
              <span className="text-white/50">&bull;</span>
            )}
            {video.view_count !== null && (
              <span className="text-white/60">{formatViewCount(video.view_count)}</span>
            )}
          </div>

          {/* Title - large and bold */}
          <h2 className="text-lg font-semibold text-white leading-snug line-clamp-2">
            {video.title}
          </h2>

          {/* Quality pills - always visible */}
          <div className="flex flex-wrap gap-1.5">
            {QUALITY_PRESETS.map((preset) => (
              <button
                key={preset.id}
                onClick={() => setSelectedQuality(preset.id)}
                className={cn(
                  'pill-glass px-2.5 py-1 text-xs font-medium rounded-md transition-all',
                  selectedQuality === preset.id && 'pill-glass-selected'
                )}
              >
                {preset.shortLabel}
              </button>
            ))}
          </div>

          {/* Format pills - always visible */}
          <div className="flex flex-wrap gap-1.5">
            {(isAudioOnly ? AUDIO_FORMATS : VIDEO_FORMATS).map((format) => (
              <button
                key={format.id}
                onClick={() => isAudioOnly
                  ? setSelectedAudioFormat(format.id as AudioFormatId)
                  : setSelectedContainer(format.id as VideoFormatId)
                }
                className={cn(
                  'pill-glass px-2.5 py-1 text-xs font-medium rounded-md transition-all',
                  (isAudioOnly ? selectedAudioFormat : selectedContainer) === format.id && 'pill-glass-selected'
                )}
              >
                {format.label}
              </button>
            ))}
          </div>

          {/* Subtitles toggle row - only for video */}
          {!isAudioOnly && (
            <div className="flex items-center gap-3">
              <button
                onClick={() => setSubtitlesEnabled(!subtitlesEnabled)}
                className={cn(
                  'flex items-center gap-2 pill-glass px-3 py-1.5 rounded-md transition-all',
                  subtitlesEnabled && 'pill-glass-selected'
                )}
              >
                <SubtitlesIcon className="w-4 h-4" />
                <span className="text-xs font-medium">Subtitles</span>
              </button>

              {/* Subtitle options - shown when enabled */}
              <AnimatePresence>
                {subtitlesEnabled && availableEngines.length > 0 && (
                  <motion.div
                    initial={{ opacity: 0, x: -10 }}
                    animate={{ opacity: 1, x: 0 }}
                    exit={{ opacity: 0, x: -10 }}
                    className="flex items-center gap-2"
                  >
                    {/* Engine selector */}
                    {availableEngines.length > 1 && (
                      <select
                        value={selectedEngine}
                        onChange={(e) => {
                          setSelectedEngine(e.target.value);
                          const eng = engines.find(en => en.id === e.target.value);
                          const firstModel = eng?.models.find(m => m.installed);
                          if (firstModel) setSelectedModel(firstModel.id);
                        }}
                        className="pill-glass px-2 py-1 text-xs rounded-md bg-transparent border-none cursor-pointer"
                      >
                        {availableEngines.map((engine) => (
                          <option key={engine.id} value={engine.id} className="bg-bg-secondary">
                            {engine.name}
                          </option>
                        ))}
                      </select>
                    )}

                    {/* Model selector */}
                    {installedModels.length > 1 && (
                      <select
                        value={selectedModel}
                        onChange={(e) => setSelectedModel(e.target.value)}
                        className="pill-glass px-2 py-1 text-xs rounded-md bg-transparent border-none cursor-pointer"
                      >
                        {installedModels.map((model) => (
                          <option key={model.id} value={model.id} className="bg-bg-secondary">
                            {model.name}
                          </option>
                        ))}
                      </select>
                    )}

                    {/* Style selector */}
                    <select
                      value={selectedStyle}
                      onChange={(e) => setSelectedStyle(e.target.value as 'word' | 'sentence')}
                      className="pill-glass px-2 py-1 text-xs rounded-md bg-transparent border-none cursor-pointer"
                    >
                      <option value="sentence" className="bg-bg-secondary">Sentence</option>
                      <option value="word" className="bg-bg-secondary">Word</option>
                    </select>

                    {/* ETA */}
                    {transcriptionEta && (
                      <span className="text-xs text-white/50">{transcriptionEta}</span>
                    )}
                  </motion.div>
                )}
              </AnimatePresence>

              {/* Warning if no models installed */}
              {subtitlesEnabled && availableEngines.length === 0 && engines.length > 0 && (
                <span className="text-xs text-warning">No models installed</span>
              )}
            </div>
          )}

          {/* Download button - full width, prominent */}
          <motion.button
            onClick={handleDownload}
            disabled={isDownloading}
            className={cn(
              'w-full mt-1 py-3 px-4 rounded-xl',
              'btn-gradient text-white font-medium text-sm',
              'flex items-center justify-center gap-2',
              'disabled:opacity-50 disabled:cursor-not-allowed'
            )}
            whileTap={{ scale: 0.98 }}
          >
            <DownloadIcon className="w-4 h-4" />
            <span>{isDownloading ? 'Starting...' : 'Download'}</span>
            {estimatedSize && (
              <span className="text-white/70 ml-1">~{estimatedSize}</span>
            )}
          </motion.button>
        </div>
      </div>
    </div>
  );
}
