import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';

export default function useGameState() {
  const [gameState, setGameState] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);

  // Settings saves and retries can overlap: only the latest check owns the UI.
  const requestId = useRef(0);

  const refresh = useCallback(async () => {
    const id = ++requestId.current;
    setLoading(true);
    setError(null);
    try {
      const state = await invoke('check_game_state');
      if (id === requestId.current) setGameState(state);
    } catch (e) {
      console.error('Failed to check game state:', e);
      if (id === requestId.current) setError(typeof e === 'string' ? e : e?.message || String(e));
    } finally {
      if (id === requestId.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
    return () => { requestId.current += 1; };
  }, [refresh]);

  return { gameState, loading, error, refresh };
}
