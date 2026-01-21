import { useState, useEffect, useCallback } from 'react';
import type { AppConfig } from '@/lib/types';
import { getConfig, updateConfig } from '@/lib/tauri';

const defaultConfig: AppConfig = {
  output_dir: '',
  default_format: 'mp4',
  default_quality: 'best',
  theme: 'system',
  generate_subtitles: false,
  whisper_model: 'base',
  transcription_engine: 'whisper_cpp',
  transcription_model: 'base',
  network_interface: null,
};

let cachedConfig: AppConfig | null = null;

export function useSettings() {
  const [config, setConfig] = useState<AppConfig>(() => cachedConfig ?? defaultConfig);
  const [isLoading, setIsLoading] = useState(!cachedConfig);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (cachedConfig) return;

    let mounted = true;

    async function loadConfig() {
      try {
        const loaded = await getConfig();
        if (mounted) {
          cachedConfig = loaded;
          setConfig(loaded);
          setIsLoading(false);
        }
      } catch (err) {
        if (mounted) {
          setError(err instanceof Error ? err.message : 'Failed to load settings');
          setIsLoading(false);
        }
      }
    }

    loadConfig();

    return () => {
      mounted = false;
    };
  }, []);

  const saveConfig = useCallback(async (newConfig: Partial<AppConfig>) => {
    const updated = { ...config, ...newConfig };
    setConfig(updated);
    cachedConfig = updated;

    try {
      await updateConfig(updated);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save settings');
      throw err;
    }
  }, [config]);

  const setOutputDir = useCallback((dir: string) => {
    return saveConfig({ output_dir: dir });
  }, [saveConfig]);

  const setDefaultFormat = useCallback((format: string) => {
    return saveConfig({ default_format: format });
  }, [saveConfig]);

  const setDefaultQuality = useCallback((quality: string) => {
    return saveConfig({ default_quality: quality });
  }, [saveConfig]);

  const setTheme = useCallback((theme: 'system' | 'light' | 'dark') => {
    return saveConfig({ theme });
  }, [saveConfig]);

  const setGenerateSubtitles = useCallback((value: boolean) => {
    return saveConfig({ generate_subtitles: value });
  }, [saveConfig]);

  const setWhisperModel = useCallback((value: string) => {
    return saveConfig({ whisper_model: value });
  }, [saveConfig]);

  return {
    config,
    isLoading,
    error,
    saveConfig,
    setOutputDir,
    setDefaultFormat,
    setDefaultQuality,
    setTheme,
    setGenerateSubtitles,
    setWhisperModel,
  };
}
