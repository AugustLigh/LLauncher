import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

export default function useGameRunning() {
  const [running, setRunning] = useState(false);

  useEffect(() => {
    invoke('is_game_running').then(setRunning).catch(() => {});

    let unlisten;
    listen('game://exited', () => {
      setRunning(false);
      // Bring the launcher back when the game ends (it may have been hidden
      // on launch).
      const win = getCurrentWindow();
      win.show().catch(() => {});
      win.setFocus().catch(() => {});
    }).then((u) => {
      unlisten = u;
    });
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  const markRunning = useCallback(() => setRunning(true), []);

  return { running, markRunning };
}
