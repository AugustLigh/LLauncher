import {
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
  useCallback,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
const Context = createContext(null);
export const taskActive = (task) =>
  ["running", "pausing", "cancelling"].includes(task?.status);
const commands = {
  repair: "repair_game",
  install: "start_download",
  update: "start_update",
  integrity: "verify_game_integrity",
  proton: "download_dwproton",
};
const message = (error) =>
  typeof error === "string" ? error : error?.message || String(error);
export function TaskProvider({ children, onComplete }) {
  const [tasks, setTasks] = useState({});
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState(null);
  const current = useRef(tasks);
  current.current = tasks;
  const complete = useRef(onComplete);
  complete.current = onComplete;
  const locks = useRef(new Set());
  const revisions = useRef({});
  const refresh = useCallback(async () => {
    const before = { ...revisions.current };
    const next = await invoke("get_transfers");
    if (next) {
      const merged = { ...current.current };
      for (const [slot, task] of Object.entries(next)) {
        if (before[slot] === revisions.current[slot]) merged[slot] = task;
      }
      current.current = merged;
      setTasks(merged);
    }
    setLoadError(null);
    return next;
  }, []);
  useEffect(() => {
    let disposed = false;
    const seen = new Set();
    const pending = listen("task://changed", ({ payload }) => {
      if (disposed) return;
      seen.add(payload.slot);
      revisions.current[payload.slot] =
        (revisions.current[payload.slot] || 0) + 1;
      current.current = { ...current.current, [payload.slot]: payload };
      setTasks(current.current);
      if (payload.status === "completed") complete.current?.(payload);
    });
    pending
      .then(() => invoke("get_transfers"))
      .then((next) => {
        if (disposed) return;
        setTasks((prev) => {
          const merged = { ...prev };
          for (const [slot, task] of Object.entries(next || {}))
            if (!seen.has(slot)) merged[slot] = task;
          current.current = merged;
          return merged;
        });
      })
      .catch((e) => {
        if (!disposed) setLoadError(message(e));
      })
      .finally(() => {
        if (!disposed) setLoading(false);
      });
    return () => {
      disposed = true;
      pending.then((fn) => fn()).catch(() => {});
    };
  }, []);
  const start = async (kind, args = {}) => {
    const slot = kind === "proton" ? "proton" : "game";
    if (locks.current.has(slot) || taskActive(current.current[slot]))
      return false;
    locks.current.add(slot);
    const optimistic = {
      ...current.current[slot],
      slot,
      kind,
      status: "running",
      progress: { stage: "fetching" },
      error: null,
    };
    current.current = { ...current.current, [slot]: optimistic };
    setTasks(current.current);
    try {
      await invoke(commands[kind], args);
      await refresh();
      return true;
    } catch (e) {
      try {
        await refresh();
      } catch {
        /* The command's error remains actionable. */
      }
      if (
        message(e) !== "Download cancelled" &&
        current.current[slot]?.status !== "error"
      ) {
        current.current = {
          ...current.current,
          [slot]: {
            ...current.current[slot],
            status: "error",
            error: message(e),
          },
        };
        setTasks(current.current);
      }
      return false;
    } finally {
      locks.current.delete(slot);
    }
  };
  const stop = async (slot, discard = false) => {
    try {
      await invoke("stop_transfer", { slot, discard });
      await refresh();
    } catch (e) {
      throw new Error(message(e));
    }
  };
  return (
    <Context.Provider
      value={{
        tasks,
        loading,
        loadError,
        start,
        stop,
        refresh,
        busy: Object.values(tasks).some(taskActive),
      }}
    >
      {children}
    </Context.Provider>
  );
}
export default function useTasks() {
  return useContext(Context);
}
