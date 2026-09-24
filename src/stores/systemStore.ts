import { create } from 'zustand';
import { commands, SystemCheck } from '../bindings';

interface SystemState {
  systemCheck: SystemCheck | null;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

let systemRequestId = 0;

export const useSystemStore = create<SystemState>((set) => ({
  systemCheck: null,
  loading: true,
  error: null,

  refresh: async () => {
    const id = ++systemRequestId;
    set({ loading: true, error: null });
    try {
      const res = await commands.checkSystemRequirements();
      if (res.status === 'error') throw new Error(res.error);
      if (id === systemRequestId) {
        set({ systemCheck: res.data, loading: false });
      }
    } catch (e: any) {
      if (id === systemRequestId) {
        set({ error: e.message || String(e), loading: false });
      }
    }
  }
}));
