import { useEffect, useRef, useState, useCallback } from 'react';
import { checkOllamaHealth } from '../api/tauri';
import type { HealthStatus } from '../api/types';

export function useOllamaHealth() {
  const [health, setHealth] = useState<HealthStatus | null>(null);
  const [isChecking, setIsChecking] = useState<boolean>(true);
  const isMountedRef = useRef<boolean>(true);

  const fetchHealth = useCallback(async () => {
    setIsChecking(true);
    try {
      const status = await checkOllamaHealth();
      if (isMountedRef.current) {
        setHealth(status);
      }
    } catch (err) {
      if (isMountedRef.current) {
        setHealth({
          is_healthy: false,
          version: null,
          base_url: 'http://127.0.0.1:11434',
          error: String(err),
        });
      }
    } finally {
      if (isMountedRef.current) {
        setIsChecking(false);
      }
    }
  }, []);

  useEffect(() => {
    isMountedRef.current = true;
    fetchHealth();

    // 30-second interval polling
    const intervalId = setInterval(() => {
      fetchHealth();
    }, 30000);

    return () => {
      isMountedRef.current = false;
      clearInterval(intervalId);
    };
  }, [fetchHealth]);

  return {
    health,
    isChecking,
    refreshHealth: fetchHealth,
  };
}
