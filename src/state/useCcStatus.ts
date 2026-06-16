import { useCallback, useEffect, useState } from "react";
import { api, CcProvider, DetectResult } from "../api";

export function useCcStatus() {
  const [detect, setDetect] = useState<DetectResult | null>(null);
  const [providers, setProviders] = useState<CcProvider[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const d = await api.detectCcSwitch();
      setDetect(d);
      if (d.installed) {
        const list = await api.listCcProviders();
        setProviders(list);
      } else {
        setProviders([]);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { detect, providers, refresh, loading, error };
}
