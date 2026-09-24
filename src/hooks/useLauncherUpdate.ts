import { useState, useEffect } from 'react';
import { getVersion } from '@tauri-apps/api/app';

const RELEASES_API = 'https://api.github.com/repos/AugustLigh/LLauncher/releases/latest';

export interface LauncherUpdateInfo {
  version: string;
  url: string;
}

function isNewer(latest: string, current: string): boolean {
  const a = latest.split('.').map(Number);
  const b = current.split('.').map(Number);
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    const x = a[i] || 0;
    const y = b[i] || 0;
    if (x !== y) return x > y;
  }
  return false;
}

export default function useLauncherUpdate(): LauncherUpdateInfo | null {
  const [update, setUpdate] = useState<LauncherUpdateInfo | null>(null);

  useEffect(() => {
    (async () => {
      try {
        const current = await getVersion();
        const res = await fetch(RELEASES_API, {
          headers: { Accept: 'application/vnd.github+json' },
        });
        if (!res.ok) return;
        const data = await res.json();
        const tag = (data.tag_name || '').replace(/^v/, '');
        if (tag && isNewer(tag, current)) {
          setUpdate({ version: tag, url: data.html_url });
        }
      } catch {
        // Offline or rate-limited — silently skip the check.
      }
    })();
  }, []);

  return update;
}
