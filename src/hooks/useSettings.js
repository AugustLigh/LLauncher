import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

export default function useSettings() {
  const [settings, setSettings] = useState(null);
  const [loading, setLoading] = useState(true);

  // Re-read the backend's copy. Needed after commands that change settings on
  // their own (game import/adoption, a Proton download, a finished install) so
  // the frontend snapshot does not go stale.
  const reload = useCallback(async () => {
    try {
      const fresh = await invoke('get_settings');
      setSettings(fresh);
      return fresh;
    } catch (e) {
      console.error('Failed to load settings:', e);
      return null;
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  const saveSettings = useCallback(async (newSettings) => {
    try {
      await invoke('save_settings', { settings: newSettings });
      // The backend keeps its own values for the fields it owns (installed
      // version, play stats), so read back what was actually stored.
      await reload();
    } catch (e) {
      console.error('Failed to save settings:', e);
      throw e;
    }
  }, [reload]);

  return { settings, loading, reload, saveSettings };
}
