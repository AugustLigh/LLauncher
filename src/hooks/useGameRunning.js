import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

export default function useGameRunning() {
  const [running, setRunning] = useState(false);

  useEffect(() => {
    invoke('is_game_running').then(setRunning).catch(() => {});

    const unlisteners = [];
    // Launches can also start from the tray menu.
    listen('game://started', () => setRunning(true)).then((u) => unlisteners.push(u));
    listen('game://exited', () => {
      setRunning(false);
      // Bring the launcher back when the game ends (it may have been hidden
      // on launch).
      const win = getCurrentWindow();
      win.show().catch(() => {});
      win.setFocus().catch(() => {});
    }).then((u) => unlisteners.push(u));
    return () => {
      unlisteners.forEach((u) => u());
    };
  }, []);

  const markRunning = useCallback(() => setRunning(true), []);

  return { running, markRunning };
}
