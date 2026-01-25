import { useState, useMemo, useEffect } from 'react';
import { motion, AnimatePresence } from 'motion/react';
import type { TranscriptionEngine, TranscribeProgress } from '@/lib/types';
import { cn } from '@/lib/utils';
import { getTranscriptionEngines, transcribeLocalFile } from '@/lib/tauri';
import { XIcon, SubtitlesIcon, LoaderIcon, CheckIcon, AlertCircleIcon } from './Icons';
import { ProgressBar } from './ProgressBar';

interface LocalTranscribeCardProps {
  filePath: string;
  onClose: () => void;
  transcriptionEngine?: string;
  transcriptionModel?: string;
  progress: TranscribeProgress | null;
  onStartTranscription: () => void;
}

export function LocalTranscribeCard({
  filePath,
  onClose,
  transcriptionEngine = 'whisper_rs',
  transcriptionModel = 'base',
  progress,
  onStartTranscription,
}: LocalTranscribeCardProps) {
  // Engines fetched from backend
  const [engines, setEngines] = useState<TranscriptionEngine[]>([]);

  // Local engine/model state - initialized from props
  const [selectedEngine, setSelectedEngine] = useState(transcriptionEngine);
  const [selectedModel, setSelectedModel] = useState(transcriptionModel);
  const [selectedStyle, setSelectedStyle] = useState<'word' | 'sentence'>('sentence');

  // Transcription state
  const [isTranscribing, setIsTranscribing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isComplete, setIsComplete] = useState(false);

  // Extract just the filename from the full path
  const fileName = useMemo(() => {
    const parts = filePath.replace(/\\/g, '/').split('/');
    return parts[parts.length - 1];
  }, [filePath]);

  // Fetch engines from backend and set initial selection
  useEffect(() => {
    const initEngine = selectedEngine;
    const initModel = selectedModel;

    getTranscriptionEngines().then((fetchedEngines) => {
      setEngines(fetchedEngines);

      // Find engines with installed models
      const enginesWithModels = fetchedEngines.filter(e => e.models.some(m => m.installed));
      if (enginesWithModels.length === 0) return;

      // Check if current engine has installed models
      const currentEng = fetchedEngines.find(e => e.id === initEngine);
      const currentHasModels = currentEng?.models.some(m => m.installed);

      if (!currentHasModels) {
        // Switch to first available engine
        const firstAvailable = enginesWithModels[0];
        setSelectedEngine(firstAvailable.id);
        const firstModel = firstAvailable.models.find(m => m.installed);
        if (firstModel) setSelectedModel(firstModel.id);
      } else {
        // Check if current model is installed
        const modelInstalled = currentEng?.models.find(m => m.id === initModel && m.installed);
        if (!modelInstalled) {
          const firstInstalled = currentEng?.models.find(m => m.installed);
          if (firstInstalled) setSelectedModel(firstInstalled.id);
        }
      }
    }).catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []); // Only run once on mount

  // Track completion from progress - derive state instead of using effect
  const isCompleteFromProgress = progress?.stage === 'complete';

  // Update local complete state when progress indicates completion
  useEffect(() => {
    if (isCompleteFromProgress && !isComplete) {
      // Use setTimeout to avoid synchronous setState in effect
      const timer = setTimeout(() => {
        setIsComplete(true);
        setIsTranscribing(false);
      }, 0);
      return () => clearTimeout(timer);
    }
  }, [isCompleteFromProgress, isComplete]);

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

  const handleStartTranscription = async () => {
    setIsTranscribing(true);
    setError(null);
    setIsComplete(false);
    onStartTranscription();

    try {
      await transcribeLocalFile(filePath, selectedEngine, selectedModel, selectedStyle);
      // Progress will update via events, completion handled in useEffect
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      setIsTranscribing(false);
    }
  };

  // Progress display
  const progressPercentage = progress?.progress ?? 0;
  const progressMessage = progress?.message ?? '';
  const progressStage = progress?.stage ?? '';

  return (
    <div className="w-full max-w-xl rounded-2xl overflow-hidden glass">
      {/* Header */}
      <div className="flex items-start gap-3 p-4 border-b border-white/10">
        <div className="flex-shrink-0 w-10 h-10 rounded-lg bg-accent/20 flex items-center justify-center">
          <SubtitlesIcon className="w-5 h-5 text-accent" />
        </div>
        <div className="flex-1 min-w-0">
          <h3 className="text-sm font-medium text-text-primary truncate" title={fileName}>
            {fileName}
          </h3>
          <p className="text-xs text-text-tertiary truncate mt-0.5" title={filePath}>
            {filePath}
          </p>
        </div>
        <button
          onClick={onClose}
          disabled={isTranscribing}
          className="p-1.5 rounded-lg hover:bg-white/10 text-text-secondary hover:text-text-primary transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          aria-label="Close"
        >
          <XIcon className="w-4 h-4" />
        </button>
      </div>

      {/* Content */}
      <div className="p-4 space-y-4">
        {/* Engine/Model/Style selectors */}
        {!isTranscribing && !isComplete && availableEngines.length > 0 && (
          <div className="flex flex-wrap items-center gap-2">
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
                className="pill-glass px-3 py-1.5 text-sm rounded-lg bg-transparent border-none cursor-pointer"
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
                className="pill-glass px-3 py-1.5 text-sm rounded-lg bg-transparent border-none cursor-pointer"
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
              className="pill-glass px-3 py-1.5 text-sm rounded-lg bg-transparent border-none cursor-pointer"
            >
              <option value="sentence" className="bg-bg-secondary">Sentence</option>
              <option value="word" className="bg-bg-secondary">Word</option>
            </select>
          </div>
        )}

        {/* No models warning */}
        {!isTranscribing && !isComplete && availableEngines.length === 0 && engines.length > 0 && (
          <div className="flex items-center gap-2 text-warning text-sm">
            <AlertCircleIcon className="w-4 h-4" />
            <span>No transcription models installed. Install models in Settings.</span>
          </div>
        )}

        {/* Progress display */}
        <AnimatePresence mode="wait">
          {isTranscribing && (
            <motion.div
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              className="space-y-3"
            >
              <div className="flex items-center gap-2">
                <LoaderIcon className="w-4 h-4 animate-spin text-accent" />
                <span className="text-sm text-text-secondary">
                  {progressMessage || `Transcribing (${progressStage})...`}
                </span>
              </div>
              <ProgressBar percentage={progressPercentage} className="h-2" />
            </motion.div>
          )}

          {isComplete && (
            <motion.div
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              className="flex items-center gap-2 text-success"
            >
              <CheckIcon className="w-5 h-5" />
              <span className="text-sm font-medium">Subtitles added successfully</span>
            </motion.div>
          )}

          {error && (
            <motion.div
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              className="flex items-start gap-2 text-error"
            >
              <AlertCircleIcon className="w-4 h-4 mt-0.5 flex-shrink-0" />
              <span className="text-sm">{error}</span>
            </motion.div>
          )}
        </AnimatePresence>

        {/* Action button */}
        {!isTranscribing && !isComplete && (
          <motion.button
            onClick={handleStartTranscription}
            disabled={availableEngines.length === 0}
            className={cn(
              'w-full py-3 px-4 rounded-xl',
              'btn-gradient text-white font-medium text-sm',
              'flex items-center justify-center gap-2',
              'disabled:opacity-50 disabled:cursor-not-allowed'
            )}
            whileTap={{ scale: 0.98 }}
          >
            <SubtitlesIcon className="w-4 h-4" />
            <span>Generate Subtitles</span>
          </motion.button>
        )}

        {/* Done button */}
        {isComplete && (
          <motion.button
            onClick={onClose}
            className={cn(
              'w-full py-3 px-4 rounded-xl',
              'bg-success/20 text-success font-medium text-sm',
              'flex items-center justify-center gap-2'
            )}
            whileTap={{ scale: 0.98 }}
          >
            <CheckIcon className="w-4 h-4" />
            <span>Done</span>
          </motion.button>
        )}

        {/* Retry button on error */}
        {error && !isTranscribing && (
          <motion.button
            onClick={handleStartTranscription}
            disabled={availableEngines.length === 0}
            className={cn(
              'w-full py-3 px-4 rounded-xl',
              'bg-error/20 text-error font-medium text-sm',
              'flex items-center justify-center gap-2',
              'disabled:opacity-50 disabled:cursor-not-allowed'
            )}
            whileTap={{ scale: 0.98 }}
          >
            <span>Retry</span>
          </motion.button>
        )}
      </div>
    </div>
  );
}
