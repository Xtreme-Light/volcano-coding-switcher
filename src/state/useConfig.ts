import { useCallback, useEffect, useState } from "react";
import { api, AppConfig } from "../api";

export function useConfig() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const cfg = await api.getConfig();
      setConfig(cfg);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  const save = useCallback(
    async (next: AppConfig) => {
      await api.saveConfig(next);
      setConfig(next);
    },
    []
  );

  return { config, setConfig, save, reload, loading, error };
}
