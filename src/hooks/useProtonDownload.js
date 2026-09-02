import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export default function useProtonDownload(onComplete) {
  const onCompleteRef = useRef(onComplete);
  onCompleteRef.current = onComplete;
  const [downloading, setDownloading] = useState(false);
  const [progress, setProgress] = useState(null);
  const [error, setError] = useState(null);

  useEffect(() => {
    const pending = [];

    pending.push(listen('proton://progress', (event) => {
      setProgress(event.payload);
    }));

    pending.push(listen('proton://complete', (event) => {
      setDownloading(false);
      setProgress(null);
      if (onCompleteRef.current) onCompleteRef.current(event.payload);
    }));

    pending.push(listen('proton://error', (event) => {
      setDownloading(false);
      setError(event.payload.message);
    }));

    return () => {
      // Wait for every registration to settle before unregistering, so a
      // listener whose promise had not resolved at teardown is not leaked.
      Promise.all(pending).then((us) => us.forEach((u) => u()));
    };
  }, []);

  // `release` is optional: without one the backend installs the recommended
  // build. Only forward a real release object — this is easy to wire straight
  // into an onClick, and a click event must not end up serialised as the
  // release (the IPC layer chokes on it with a JSON error).
  const startDownload = useCallback(async (release) => {
    const wanted = release && typeof release === 'object' && typeof release.tag_name === 'string'
      ? release
      : null;
    setDownloading(true);
    setError(null);
    setProgress(null);
    try {
      await invoke('download_dwproton', { release: wanted });
    } catch (e) {
      setDownloading(false);
      setError(typeof e === 'string' ? e : e.message || 'Proton download failed');
    }
  }, []);

  const cancelDownload = useCallback(async () => {
    try {
      await invoke('cancel_proton_download');
    } catch (e) {
      console.error('Failed to cancel proton download:', e);
    }
  }, []);

  return { downloading, progress, error, startDownload, cancelDownload };
}
