import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
export default function useSystemCheck() {
  const [systemCheck, setSystemCheck] = useState(null),
    [loading, setLoading] = useState(true),
    [error, setError] = useState(null);
  const id = useRef(0);
  const refresh = useCallback(async () => {
    const request = ++id.current;
    setLoading(true);
    setError(null);
    try {
      const next = await invoke("check_system_requirements");
      if (request === id.current) setSystemCheck(next);
    } catch (e) {
      if (request === id.current)
        setError(typeof e === "string" ? e : e?.message || String(e));
    } finally {
      if (request === id.current) setLoading(false);
    }
  }, []);
  useEffect(() => {
    refresh();
    return () => {
      id.current++;
    };
  }, [refresh]);
  return { systemCheck, loading, error, refresh };
}
