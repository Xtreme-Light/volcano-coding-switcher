import { useCallback, useEffect, useState } from "react";
import { api, listen, QuotaSnapshot } from "../api";

export function useQuota() {
  const [snapshot, setSnapshot] = useState<QuotaSnapshot | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async (accountId?: string | null) => {
    try {
      setRefreshing(true);
      setError(null);
      const s = accountId
        ? await api.fetchQuotaByAccount(accountId)
        : await api.fetchQuota();
      setSnapshot(s);
    } catch (e) {
      setError(String(e));
    } finally {
      setRefreshing(false);
    }
  }, []);

  useEffect(() => {
    let dispose: (() => void) | null = null;
    listen<QuotaSnapshot>("quota-updated", (s) => setSnapshot(s)).then((d) => {
      dispose = d;
    });
    return () => {
      if (dispose) dispose();
    };
  }, []);

  return { snapshot, refresh, refreshing, error };
}
