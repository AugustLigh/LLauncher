import { create } from 'zustand';
import { commands, AppSettings } from '../bindings';
import { useSystemStore } from './systemStore';
import { useGameStore } from './gameStore';

interface SettingsState {
  settings: AppSettings | null;
  loading: boolean;
  error: string | null;
  reload: () => Promise<AppSettings | null>;
  saveSettings: (newSettings: AppSettings) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: null,
  loading: true,
  error: null,

  reload: async () => {
    try {
      const res = await commands.getSettings();
      if (res.status === 'error') throw new Error(res.error);
      set({ settings: res.data, error: null, loading: false });
      return res.data;
    } catch (e: any) {
      console.error('Failed to load settings:', e);
      set({ error: e.message || String(e), loading: false });
      return null;
    }
  },

  saveSettings: async (newSettings: AppSettings) => {
    try {
      const res = await commands.saveSettings(newSettings);
      if (res.status === 'error') throw new Error(res.error);
      // The backend keeps its own values for the fields it owns (installed
      // version, play stats), so read back what was actually stored.
      await get().reload();
      await Promise.all([
        useSystemStore.getState().refresh(),
        useGameStore.getState().refreshGameState(),
      ]);
    } catch (e) {
      console.error('Failed to save settings:', e);
      throw e;
    }
  }
}));
