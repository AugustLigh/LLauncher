import { commands, GameSession } from '../bindings';
import { useState, useEffect, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';

const DAY = 86400;

interface SessionStats {
  weekSecs: number;
  weekSessions: number;
  longestSecs: number;
  avgSessionSecs: number;
  sessionCount: number;
  days: number[];
}

export interface GameStats extends SessionStats {
  totalPlaytimeSecs: number;
  lastPlayed: number;
}

function computeSessionStats(sessions: GameSession[]): SessionStats {
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
    const dur = s.duration_secs ?? 0;
    const start = s.start ?? 0;
    totalSecs += dur;
    if (dur > longestSecs) longestSecs = dur;
    if (start >= weekAgo) {
      weekSecs += dur;
      weekSessions += 1;
    }
    // 6 = today, 5 = yesterday, ...
    const dayIndex = 6 - Math.ceil(Math.max(0, todayStart - start) / DAY);
    if (dayIndex >= 0 && dayIndex <= 6) {
      days[dayIndex] += dur;
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
  const [stats, setStats] = useState<GameStats | null>(null);

  const fetchStats = useCallback(async () => {
    try {
      const s = await commands.getSettings();
      if (s.status !== "ok") return;
      let sessions: GameSession[] = [];
      try {
        const sRes = await commands.getGameSessions();
        if (sRes.status === "ok") sessions = sRes.data;
      } catch {
        // session journal is cosmetic
      }
      setStats({
        totalPlaytimeSecs: s.data.total_playtime_secs || 0,
        lastPlayed: s.data.last_played || 0,
        ...computeSessionStats(sessions),
      });
    } catch {
      // stats are cosmetic
    }
  }, []);

  useEffect(() => {
    fetchStats();
    let disposed = false;
    let unlisten: (() => void) | undefined;
    listen('game://exited', fetchStats).then((u) => {
      if (disposed) u();
      else unlisten = u;
    });
    return () => {
      disposed = true;
      if (unlisten) unlisten();
    };
  }, [fetchStats]);

  return stats;
}
