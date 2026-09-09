import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
export default function useLauncherContent(language) {
  const [content, setContent] = useState(null),
    [loading, setLoading] = useState(true),
    [error, setError] = useState(null);
  const request = useRef(0);
  const refresh = useCallback(async () => {
    if (!language) return;
    const id = ++request.current;
    setLoading(true);
    setError(null);
    try {
      const next = await invoke("get_launcher_content");
      if (id === request.current) setContent(next);
    } catch (e) {
      if (id === request.current) setError(e);
    } finally {
      if (id === request.current) setLoading(false);
    }
  }, [language]);
  useEffect(() => {
    refresh();
    return () => {
      request.current++;
    };
  }, [refresh]);
  return { content, loading, error, refresh };
}
