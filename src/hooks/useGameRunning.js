import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

export default function useGameRunning() {
  const [running, setRunning] = useState(false);

  useEffect(() => {
    invoke('is_game_running').then(setRunning).catch(() => {});

    const pending = [];
    // Launches can also start from the tray menu.
    pending.push(listen('game://started', () => setRunning(true)));
    pending.push(listen('game://exited', () => {
      setRunning(false);
      // Bring the launcher back when the game ends (it may have been hidden
      // on launch).
      const win = getCurrentWindow();
      win.show().catch(() => {});
      win.setFocus().catch(() => {});
    }));
    return () => {
      // Wait for every registration to settle before unregistering, so a
      // listener whose promise had not resolved at teardown is not leaked.
      Promise.all(pending).then((us) => us.forEach((u) => u()));
    };
  }, []);

  const markRunning = useCallback(() => setRunning(true), []);

  return { running, markRunning };
}
