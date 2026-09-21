import { create } from 'zustand';
import { commands, Transfer } from '../bindings';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

export const taskActive = (task?: Transfer | null) =>
  task ? ["running", "pausing", "cancelling"].includes(task.status) : false;

interface TaskStoreState {
  tasks: Record<string, Transfer>;
  loading: boolean;
  loadError: string | null;
  busy: boolean;

  refresh: () => Promise<void>;
  start: (kind: string, args?: any) => Promise<boolean>;
  stop: (slot: string, discard?: boolean) => Promise<void>;
  initListeners: (onComplete?: (task: Transfer) => void) => Promise<() => void>;
}

export const useTaskStore = create<TaskStoreState>((set, get) => ({
  tasks: {},
  loading: true,
  loadError: null,
  get busy() {
    return Object.values(get().tasks).some(taskActive);
  },

  refresh: async () => {
    try {
      const res = await commands.getTransfers();
      set({ tasks: res, loadError: null, loading: false });
    } catch (e: any) {
      set({ loadError: e.message || String(e), loading: false });
    }
  },

  start: async (kind: string, args: any = {}) => {
    const slot = kind === "proton" ? "proton" : "game";
    const state = get();
    if (taskActive(state.tasks[slot])) return false;

    const optimistic = {
      ...(state.tasks[slot] || {}),
      id: "optimistic",
      slot,
      kind,
      status: "running",
      progress: { stage: "fetching" },
      error: null,
      result: {},
      game_dir: "",
      download_dir: "",
    } as Transfer;

    set({ tasks: { ...state.tasks, [slot]: optimistic } });

    try {
      if (kind === "proton") {
        await commands.downloadDwproton(args);
      } else if (kind === "install") {
        await commands.startDownload();
      } else if (kind === "update") {
        await commands.startUpdate();
      } else if (kind === "integrity") {
        await commands.verifyGameIntegrity();
      } else if (kind === "repair") {
        await commands.repairGame();
      }
      
      await get().refresh();
      return true;
    } catch (e: any) {
      const msg = e.message || String(e);
      if (msg !== "Download cancelled") {
        set((s) => ({
          tasks: {
            ...s.tasks,
            [slot]: { ...s.tasks[slot], status: "error", error: msg },
          },
        }));
      }
      return false;
    }
  },

  stop: async (slot: string, discard = false) => {
    try {
      const res = await commands.stopTransfer(slot, discard);
      if (res.status === 'error') throw new Error(res.error);
      await get().refresh();
    } catch (e: any) {
      throw new Error(e.message || String(e));
    }
  },

  initListeners: async (onComplete) => {
    const unlistens: UnlistenFn[] = [];
    
    unlistens.push(await listen('task://changed', (event: any) => {
      const payload = event.payload as Transfer;
      set((s) => {
        const newTasks = { ...s.tasks, [payload.slot]: payload };
        return { tasks: newTasks };
      });
      if (payload.status === "completed" && onComplete) {
        onComplete(payload);
      }
    }));

    await get().refresh();

    return () => {
      unlistens.forEach(u => u());
    };
  }
}));
