import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export default function useGameStats() {
  const [stats, setStats] = useState(null);

  const fetchStats = useCallback(async () => {
    try {
      const s = await invoke('get_settings');
      setStats({
        totalPlaytimeSecs: s.total_playtime_secs || 0,
        lastPlayed: s.last_played || 0,
      });
    } catch {
      // stats are cosmetic
    }
  }, []);

  useEffect(() => {
    fetchStats();
    let unlisten;
    listen('game://exited', fetchStats).then((u) => {
      unlisten = u;
    });
    return () => {
      if (unlisten) unlisten();
    };
  }, [fetchStats]);

  return stats;
}
