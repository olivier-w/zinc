import { useState, useEffect, useCallback, useMemo } from 'react';
import type { Download, VideoInfo, SubtitleSettings } from '@/lib/types';
import {
  startDownload as apiStartDownload,
  cancelDownload as apiCancelDownload,
  clearDownload as apiClearDownload,
  clearCompletedDownloads as apiClearCompleted,
  addLocalTranscription as apiAddLocalTranscription,
  startLocalTranscription as apiStartLocalTranscription,
  updateTranscriptionSettings as apiUpdateTranscriptionSettings,
  getDownloads,
  onDownloadProgress,
} from '@/lib/tauri';

export function useDownload() {
  const [downloads, setDownloads] = useState<Map<string, Download>>(new Map());
  const [isLoading, setIsLoading] = useState(true);

  // Load existing downloads on mount
  useEffect(() => {
    let mounted = true;

    async function loadDownloads() {
      try {
        const existing = await getDownloads();
        if (mounted) {
          setDownloads(new Map(existing.map(d => [d.id, d])));
        }
      } catch (err) {
        console.error('Failed to load downloads:', err);
      } finally {
        if (mounted) setIsLoading(false);
      }
    }

    loadDownloads();

    return () => {
      mounted = false;
    };
  }, []);

  // Listen for download progress updates
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let mounted = true;

    async function setupListener() {
      try {
        const fn = await onDownloadProgress((download) => {
          console.log('[useDownload] Received download-progress event:', download.id, download.status);
          if (mounted) {
            setDownloads(prev => {
              const next = new Map(prev);
              next.set(download.id, download);
              return next;
            });
          }
        });
        if (mounted) {
          unlisten = fn;
        }
      } catch (err) {
        console.error('Failed to setup download progress listener:', err);
      }
    }

    setupListener();

    return () => {
      mounted = false;
      unlisten?.();
    };
  }, []);

  const startDownload = useCallback(async (
    videoInfo: VideoInfo,
    format: string,
    subtitleSettings?: SubtitleSettings
  ): Promise<string> => {
    const downloadId = await apiStartDownload(
      videoInfo.url,
      format,
      videoInfo.title,
      videoInfo.thumbnail,
      subtitleSettings,
      videoInfo.duration
    );

    // Optimistic update
    setDownloads(prev => {
      const next = new Map(prev);
      next.set(downloadId, {
        id: downloadId,
        url: videoInfo.url,
        title: videoInfo.title,
        thumbnail: videoInfo.thumbnail,
        status: 'pending',
        progress: 0,
        speed: null,
        eta: null,
        output_path: null,
        format,
        error: null,
        duration: videoInfo.duration,
        whisper_model: subtitleSettings?.enabled ? subtitleSettings.model : null,
        transcription_engine: subtitleSettings?.enabled ? subtitleSettings.engine : null,
        transcription_progress: null,
        transcription_message: null,
        task_type: 'download',
        source_path: null,
      });
      return next;
    });

    return downloadId;
  }, []);

  const cancelDownload = useCallback(async (downloadId: string) => {
    await apiCancelDownload(downloadId);
    setDownloads(prev => {
      const next = new Map(prev);
      const download = next.get(downloadId);
      if (download) {
        next.set(downloadId, { ...download, status: 'cancelled' });
      }
      return next;
    });
  }, []);

  const clearDownload = useCallback(async (downloadId: string) => {
    await apiClearDownload(downloadId);
    setDownloads(prev => {
      const next = new Map(prev);
      next.delete(downloadId);
      return next;
    });
  }, []);

  const clearCompleted = useCallback(async () => {
    await apiClearCompleted();
    setDownloads(prev => {
      const next = new Map(prev);
      for (const [id, download] of next) {
        if (['completed', 'error', 'cancelled'].includes(download.status)) {
          next.delete(id);
        }
      }
      return next;
    });
  }, []);

  const addLocalTranscription = useCallback(async (
    filePath: string,
    engine: string,
    model: string,
    style: string
  ): Promise<string> => {
    // Extract filename for title
    const parts = filePath.replace(/\\/g, '/').split('/');
    const title = parts[parts.length - 1];

    const taskId = await apiAddLocalTranscription(filePath, title, engine, model, style);

    // Optimistic update
    setDownloads(prev => {
      const next = new Map(prev);
      next.set(taskId, {
        id: taskId,
        url: '',
        title,
        thumbnail: null,
        status: 'pending',
        progress: 0,
        speed: null,
        eta: null,
        output_path: filePath,
        format: '',
        error: null,
        duration: null,
        whisper_model: model,
        transcription_engine: engine,
        transcription_progress: null,
        transcription_message: null,
        task_type: 'local_transcribe',
        source_path: filePath,
      });
      return next;
    });

    return taskId;
  }, []);

  const startLocalTranscription = useCallback(async (taskId: string) => {
    await apiStartLocalTranscription(taskId);
    setDownloads(prev => {
      const next = new Map(prev);
      const task = next.get(taskId);
      if (task) {
        next.set(taskId, { ...task, status: 'transcribing:extracting' });
      }
      return next;
    });
  }, []);

  const updateTranscriptionSettings = useCallback(async (
    taskId: string,
    settings: { engine?: string; model?: string; style?: string }
  ) => {
    await apiUpdateTranscriptionSettings(taskId, settings.engine, settings.model, settings.style);
    setDownloads(prev => {
      const next = new Map(prev);
      const task = next.get(taskId);
      if (task) {
        next.set(taskId, {
          ...task,
          transcription_engine: settings.engine ?? task.transcription_engine,
          whisper_model: settings.model ?? task.whisper_model,
        });
      }
      return next;
    });
  }, []);

  // Derived state - reverse so latest downloads appear first
  const downloadList = useMemo(
    () => Array.from(downloads.values()).reverse(),
    [downloads]
  );

  const activeDownloads = useMemo(
    () => downloadList.filter(d => d.status === 'downloading' || d.status === 'pending'),
    [downloadList]
  );

  const completedDownloads = useMemo(
    () => downloadList.filter(d => d.status === 'completed'),
    [downloadList]
  );

  const hasActiveDownloads = activeDownloads.length > 0;
  const hasCompletedDownloads = completedDownloads.length > 0;

  return {
    downloads: downloadList,
    activeDownloads,
    completedDownloads,
    hasActiveDownloads,
    hasCompletedDownloads,
    isLoading,
    startDownload,
    cancelDownload,
    clearDownload,
    clearCompleted,
    addLocalTranscription,
    startLocalTranscription,
    updateTranscriptionSettings,
  };
}
