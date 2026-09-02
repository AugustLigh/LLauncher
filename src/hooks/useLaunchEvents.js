import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';

export default function useLaunchEvents() {
  const [failure, setFailure] = useState(null);

  useEffect(() => {
    const pending = listen('launch://failed', (event) => {
      setFailure(event.payload);
    });
    return () => {
      pending.then((u) => u());
    };
  }, []);

  const dismiss = () => setFailure(null);

  return { failure, dismiss };
}
