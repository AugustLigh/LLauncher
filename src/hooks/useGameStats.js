import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const DAY = 86400;

function computeSessionStats(sessions) {
  const now = Math.floor(Date.now() / 1000);
  const weekAgo = now - 7 * DAY;

  let weekSecs = 0;
  let weekSessions = 0;
  let longestSecs = 0;
  let totalSecs = 0;

  // Seconds played per calendar day, oldest first (7 entries incl. today).
  const startOfToday = new Date();
  startOfToday.setHours(0, 0, 0, 0);
  const todayStart = Math.floor(startOfToday.getTime() / 1000);
  const days = new Array(7).fill(0);

  for (const s of sessions) {
    totalSecs += s.duration_secs;
    if (s.duration_secs > longestSecs) longestSecs = s.duration_secs;
    if (s.start >= weekAgo) {
      weekSecs += s.duration_secs;
      weekSessions += 1;
    }
    // 6 = today, 5 = yesterday, ...
    const dayIndex = 6 - Math.ceil(Math.max(0, todayStart - s.start) / DAY);
    if (dayIndex >= 0 && dayIndex <= 6) {
      days[dayIndex] += s.duration_secs;
    }
  }

  return {
    weekSecs,
    weekSessions,
    longestSecs,
    avgSessionSecs: sessions.length > 0 ? Math.round(totalSecs / sessions.length) : 0,
    sessionCount: sessions.length,
    days,
  };
}

export default function useGameStats() {
  const [stats, setStats] = useState(null);

  const fetchStats = useCallback(async () => {
    try {
      const s = await invoke('get_settings');
      let sessions = [];
      try {
        sessions = await invoke('get_game_sessions');
      } catch {
        // session journal is cosmetic
      }
      setStats({
        totalPlaytimeSecs: s.total_playtime_secs || 0,
        lastPlayed: s.last_played || 0,
        ...computeSessionStats(sessions),
      });
    } catch {
      // stats are cosmetic
    }
  }, []);

  useEffect(() => {
    fetchStats();
    const pending = listen('game://exited', fetchStats);
    return () => {
      pending.then((u) => u());
    };
  }, [fetchStats]);

  return stats;
}
