import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// Drives the "check file integrity" action: invokes the backend resource-sync
// command and tracks its verify/download progress via the integrity:// events.
// Cancellation reuses the shared download flag (cancel_download).
export default function useIntegrityCheck() {
  const [checking, setChecking] = useState(false);
  const [progress, setProgress] = useState(null);
  const [result, setResult] = useState(null);
  const [error, setError] = useState(null);

  useEffect(() => {
    const pending = [];

    pending.push(listen('integrity://progress', (event) => {
      setProgress(event.payload);
    }));

    pending.push(listen('integrity://complete', (event) => {
      setChecking(false);
      setProgress(null);
      setResult(event.payload);
    }));

    pending.push(listen('integrity://error', (event) => {
      setChecking(false);
      setProgress(null);
      // Cancellation is user-initiated, not a failure.
      if (!/cancelled/i.test(event.payload.message)) {
        setError(event.payload.message);
      }
    }));

    return () => {
      // Wait for every registration to settle before unregistering, so a
      // listener whose promise had not resolved at teardown is not leaked.
      Promise.all(pending).then((us) => us.forEach((u) => u()));
    };
  }, []);

  const start = useCallback(async () => {
    setChecking(true);
    setError(null);
    setResult(null);
    setProgress(null);
    try {
      await invoke('verify_game_integrity');
    } catch (e) {
      setChecking(false);
      const message = typeof e === 'string' ? e : e.message || 'Integrity check failed';
      // Cancellation is user-initiated, not a failure.
      if (!/cancelled/i.test(message)) {
        setError(message);
      } else {
        setProgress(null);
      }
    }
  }, []);

  const cancel = useCallback(async () => {
    try {
      await invoke('cancel_download');
    } catch (e) {
      console.error('Failed to cancel integrity check:', e);
    }
  }, []);

  return { checking, progress, result, error, start, cancel };
}
