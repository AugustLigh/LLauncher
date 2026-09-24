import { create } from 'zustand';
import { commands, Transfer } from '../bindings';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

export const taskActive = (task?: Transfer | null) =>
  task ? ["running", "pausing", "cancelling"].includes(task.status) : false;

const isBusy = (tasks: Record<string, Transfer>) =>
  Object.values(tasks).some(taskActive);

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

const locks = new Set<string>();
const revisions: Record<string, number> = {};

export const useTaskStore = create<TaskStoreState>((set, get) => ({
  tasks: {},
  loading: true,
  loadError: null,
  busy: false,

  refresh: async () => {
    const before = { ...revisions };
    try {
      const res = await commands.getTransfers();
      if (res) {
        set((s) => {
          const merged = { ...s.tasks };
          for (const [slot, task] of Object.entries(res)) {
            if (before[slot] === revisions[slot]) {
              merged[slot] = task;
            }
          }
          return {
            tasks: merged,
            busy: isBusy(merged),
            loadError: null,
            loading: false,
          };
        });
      }
    } catch (e: any) {
      set({ loadError: e.message || String(e), loading: false });
    }
  },

  start: async (kind: string, args: any = {}) => {
    const slot = kind === "proton" ? "proton" : "game";
    if (locks.has(slot) || taskActive(get().tasks[slot])) return false;
    locks.add(slot);

    const state = get();
    const optimistic: Transfer = {
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
    };

    const optimisticTasks = { ...state.tasks, [slot]: optimistic };
    set({ tasks: optimisticTasks, busy: isBusy(optimisticTasks) });

    try {
      if (kind === "proton") {
        const release = (args && "tag_name" in args)
          ? args
          : (args?.release ?? null);
        const res = await commands.downloadDwproton(release);
        if (res.status === "error") throw new Error(res.error);
      } else if (kind === "install") {
        const res = await commands.startDownload();
        if (res.status === "error") throw new Error(res.error);
      } else if (kind === "update") {
        const res = await commands.startUpdate();
        if (res.status === "error") throw new Error(res.error);
      } else if (kind === "integrity") {
        const res = await commands.verifyGameIntegrity();
        if (res.status === "error") throw new Error(res.error);
      } else if (kind === "repair") {
        const res = await commands.repairGame();
        if (res.status === "error") throw new Error(res.error);
      }

      await get().refresh();
      return true;
    } catch (e: any) {
      try {
        await get().refresh();
      } catch {
        /* The command's error remains actionable. */
      }
      const msg = e.message || String(e);
      if (msg !== "Download cancelled") {
        set((s) => {
          const current = s.tasks[slot];
          if (current?.status === "error") return {};
          const updated = {
            ...s.tasks,
            [slot]: { ...current, status: "error", error: msg },
          };
          return { tasks: updated, busy: isBusy(updated) };
        });
      }
      return false;
    } finally {
      locks.delete(slot);
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
      revisions[payload.slot] = (revisions[payload.slot] || 0) + 1;
      set((s) => {
        const newTasks = { ...s.tasks, [payload.slot]: payload };
        return { tasks: newTasks, busy: isBusy(newTasks) };
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
