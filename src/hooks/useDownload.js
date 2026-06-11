import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export default function useDownload(onComplete) {
  const [downloading, setDownloading] = useState(false);
  const [progress, setProgress] = useState(null);
  const [error, setError] = useState(null);

  useEffect(() => {
    const unlisteners = [];

    listen('download://progress', (event) => {
      setProgress({ stage: 'downloading', ...event.payload });
    }).then((u) => unlisteners.push(u));

    listen('download://file-complete', (event) => {
      setProgress((prev) =>
        prev ? { ...prev, ...event.payload } : { stage: 'downloading', ...event.payload }
      );
    }).then((u) => unlisteners.push(u));

    listen('download://verify-progress', (event) => {
      setProgress({ stage: 'verifying', ...event.payload });
    }).then((u) => unlisteners.push(u));

    listen('download://extract-progress', (event) => {
      setProgress({ stage: 'extracting', ...event.payload });
    }).then((u) => unlisteners.push(u));

    listen('download://complete', (event) => {
      setDownloading(false);
      setProgress(null);
      if (onComplete) onComplete(event.payload.version);
    }).then((u) => unlisteners.push(u));

    listen('download://error', (event) => {
      setDownloading(false);
      // Cancellation is user-initiated (pause): partial files are kept and
      // the next start resumes via HTTP Range, so it is not an error.
      if (/cancelled/i.test(event.payload.message)) {
        setProgress(null);
      } else {
        setError(event.payload.message);
      }
    }).then((u) => unlisteners.push(u));

    return () => {
      unlisteners.forEach((u) => u());
    };
  }, [onComplete]);

  const startDownload = useCallback(async () => {
    setDownloading(true);
    setError(null);
    setProgress(null);
    try {
      await invoke('start_download');
    } catch (e) {
      setDownloading(false);
      setError(typeof e === 'string' ? e : e.message || 'Download failed');
    }
  }, []);

  const cancelDownload = useCallback(async () => {
    try {
      await invoke('cancel_download');
    } catch (e) {
      console.error('Failed to cancel download:', e);
    }
  }, []);

  return { downloading, progress, error, startDownload, cancelDownload };
}
