import { create } from 'zustand';
import { commands, GameState, LauncherContent } from '../bindings';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

export interface LaunchFailed {
  exit_code: number | null;
  log_tail: string;
  hint: string | null;
}

interface GameStoreState {
  gameState: GameState | null;
  stateLoading: boolean;
  stateError: string | null;

  content: LauncherContent | null;
  contentLoading: boolean;
  contentError: string | null;

  running: boolean;
  launchFailure: LaunchFailed | null;

  refreshGameState: () => Promise<void>;
  refreshContent: (language?: string) => Promise<void>;
  markRunning: () => void;
  dismissFailure: () => void;
  initListeners: () => Promise<() => void>;
}

let gameStateRequestId = 0;
let contentRequestId = 0;

export const useGameStore = create<GameStoreState>((set, get) => ({
  gameState: null,
  stateLoading: true,
  stateError: null,

  content: null,
  contentLoading: true,
  contentError: null,

  running: false,
  launchFailure: null as LaunchFailed | null,

  refreshGameState: async () => {
    const id = ++gameStateRequestId;
    set({ stateLoading: true, stateError: null });
    try {
      const res = await commands.checkGameState();
      if (res.status === 'error') throw new Error(res.error);
      if (id === gameStateRequestId) {
        set({ gameState: res.data, stateLoading: false });
      }
    } catch (e: any) {
      if (id === gameStateRequestId) {
        set({ stateError: e.message || String(e), stateLoading: false });
      }
    }
  },

  refreshContent: async (language) => {
    // If language isn't loaded yet, don't fetch content.
    if (!language && !get().content) return;
    const id = ++contentRequestId;
    set({ contentLoading: true, contentError: null });
    try {
      const res = await commands.getLauncherContent();
      if (res.status === 'error') throw new Error(res.error);
      if (id === contentRequestId) {
        set({ content: res.data, contentLoading: false });
      }
    } catch (e: any) {
      if (id === contentRequestId) {
        set({ contentError: e.message || String(e), contentLoading: false });
      }
    }
  },

  markRunning: () => set({ running: true }),
  dismissFailure: () => set({ launchFailure: null }),

  initListeners: async () => {
    try {
      const res = await commands.isGameRunning();
      if (res.status === 'ok') set({ running: res.data });
    } catch (e) {}

    const unlistens: UnlistenFn[] = [];
    
    unlistens.push(await listen('game://started', () => set({ running: true })));
    unlistens.push(await listen('game://exited', () => {
      set({ running: false });
      const win = getCurrentWindow();
      win.show().catch(() => {});
      win.setFocus().catch(() => {});
    }));
    unlistens.push(await listen<LaunchFailed>('launch://failed', (event) => {
      set({ launchFailure: event.payload });
    }));

    return () => {
      unlistens.forEach((u) => u());
    };
  }
}));
