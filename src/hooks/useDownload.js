import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export default function useDownload(onComplete) {
  const [downloading, setDownloading] = useState(false);
  // Latest callback without making it an effect dependency: re-registering
  // every listener because `t` changed identity dropped events mid-download.
  const onCompleteRef = useRef(onComplete);
  onCompleteRef.current = onComplete;
  const [progress, setProgress] = useState(null);
  const [error, setError] = useState(null);
  // Whether the current stop should discard partial files (Cancel) or keep
  // them for a later resume (Pause).
  const discardRef = useRef(false);

  useEffect(() => {
    const pending = [];

    pending.push(listen('download://progress', (event) => {
      setProgress({ stage: 'downloading', ...event.payload });
    }));

    pending.push(listen('download://file-complete', (event) => {
      setProgress((prev) =>
        prev ? { ...prev, ...event.payload } : { stage: 'downloading', ...event.payload }
      );
    }));

    pending.push(listen('download://verify-progress', (event) => {
      setProgress({ stage: 'verifying', ...event.payload });
    }));

    pending.push(listen('download://extract-progress', (event) => {
      setProgress({ stage: 'extracting', ...event.payload });
    }));

    pending.push(listen('download://complete', (event) => {
      setDownloading(false);
      setProgress(null);
      if (onCompleteRef.current) onCompleteRef.current(event.payload.version);
    }));

    pending.push(listen('download://error', (event) => {
      setDownloading(false);
      // Cancellation is user-initiated (pause): partial files are kept and
      // the next start resumes via HTTP Range, so it is not an error.
      if (/cancelled/i.test(event.payload.message)) {
        setProgress(null);
      } else {
        setError(event.payload.message);
      }
    }));

    // The smart-update delta path reports on its own channel; normalise it into
    // the same progress shape so the existing progress bar works.
    pending.push(listen('update://progress', (event) => {
      const p = event.payload;
      setProgress({
        stage: p.stage === 'downloading' ? 'downloading' : 'verifying',
        file_index: p.files_done,
        total_files: p.total_files,
        file_name: '',
        bytes_downloaded: p.bytes_done,
        bytes_total: p.bytes_total,
        speed_bps: p.speed_bps,
      });
    }));

    pending.push(listen('update://complete', (event) => {
      setDownloading(false);
      setProgress(null);
      if (onCompleteRef.current) onCompleteRef.current(event.payload.version);
    }));

    pending.push(listen('update://error', (event) => {
      setDownloading(false);
      if (/cancelled/i.test(event.payload.message)) {
        setProgress(null);
      } else {
        setError(event.payload.message);
      }
    }));

    return () => {
      // Wait for every registration to settle before unregistering, so a
      // listener whose promise had not resolved at teardown is not leaked.
      Promise.all(pending).then((us) => us.forEach((u) => u()));
    };
  }, []);

  const startDownload = useCallback(async () => {
    setDownloading(true);
    setError(null);
    setProgress(null);
    discardRef.current = false;
    try {
      await invoke('start_download');
    } catch (e) {
      setDownloading(false);
      const message = typeof e === 'string' ? e : e.message || 'Download failed';
      // A paused/cancelled download rejects with "Download cancelled" — that is
      // expected, not an error to surface. Discard partial files only on Cancel.
      if (/cancelled/i.test(message)) {
        setProgress(null);
        if (discardRef.current) {
          await invoke('clear_download_cache').catch(() => {});
          discardRef.current = false;
        }
      } else {
        setError(message);
      }
    }
  }, []);

  // Update an existing install to the latest version. The backend chooses the
  // cheaper safe path (per-file VFS delta vs full packs); progress arrives on
  // either the update:// (delta) or download:// (packs) channel.
  const startUpdate = useCallback(async () => {
    setDownloading(true);
    setError(null);
    setProgress(null);
    discardRef.current = false;
    try {
      await invoke('start_update');
    } catch (e) {
      setDownloading(false);
      const message = typeof e === 'string' ? e : e.message || 'Update failed';
      if (/cancelled/i.test(message)) {
        setProgress(null);
        if (discardRef.current) {
          await invoke('clear_download_cache').catch(() => {});
          discardRef.current = false;
        }
      } else {
        setError(message);
      }
    }
  }, []);

  // Pause: stop but keep partial files; clicking the action again resumes.
  const pauseDownload = useCallback(async () => {
    discardRef.current = false;
    try {
      await invoke('cancel_download');
    } catch (e) {
      console.error('Failed to pause download:', e);
    }
  }, []);

  // Cancel: stop and discard the partial download cache.
  const cancelDownload = useCallback(async () => {
    discardRef.current = true;
    try {
      await invoke('cancel_download');
    } catch (e) {
      console.error('Failed to cancel download:', e);
    }
  }, []);

  return { downloading, progress, error, startDownload, startUpdate, pauseDownload, cancelDownload };
}
