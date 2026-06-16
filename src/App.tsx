import { useEffect, useState } from "react";
import { ArkAccount, api, listen } from "./api";
import CcSwitchCard from "./components/CcSwitchCard";
import EventLog from "./components/EventLog";
import Header from "./components/Header";
import QuotaCard from "./components/QuotaCard";
import SettingsModal from "./components/SettingsModal";
import { useCcStatus } from "./state/useCcStatus";
import { useConfig } from "./state/useConfig";
import { useQuota } from "./state/useQuota";

export default function App() {
  const { config, save } = useConfig();
  const { snapshot, refresh, refreshing, error: quotaError } = useQuota();
  const ccStatus = useCcStatus();

  const [logs, setLogs] = useState<string[]>([]);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [accounts, setAccounts] = useState<ArkAccount[]>([]);
  const [selectedAccountId, setSelectedAccountId] = useState<string | null>(null);

  const log = (msg: string) => {
    const ts = new Date().toLocaleTimeString();
    setLogs((prev) => [`[${ts}] ${msg}`, ...prev].slice(0, 200));
  };

  const reloadAccounts = async () => {
    try {
      const list = await api.listAccounts();
      setAccounts(list);
      // 如果当前没选或选的已被删除，回退到第一个
      setSelectedAccountId((prev) => {
        if (prev && list.some((a) => a.id === prev)) return prev;
        return list[0]?.id ?? null;
      });
    } catch (e) {
      log(`加载账号列表失败: ${e}`);
    }
  };

  useEffect(() => {
    reloadAccounts();
  }, []);

  useEffect(() => {
    const dispose: Array<() => void> = [];
    listen<string>("quota-error", (msg) => log(`后台拉取错误: ${msg}`)).then((d) =>
      dispose.push(d)
    );
    listen<{ plan?: string }>("plan-switched", (p) => {
      log(`后台已切换到 ${p?.plan ?? "?"}`);
      ccStatus.refresh();
    }).then((d) => dispose.push(d));
    listen<string>("plan-switch-failed", (msg) =>
      log(`后台自动切换失败: ${msg}`)
    ).then((d) => dispose.push(d));
    return () => dispose.forEach((d) => d());
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 初次刷新一次用量（按选中账号）
  useEffect(() => {
    refresh(selectedAccountId);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 设置弹窗关闭后刷新账号列表（可能新增/编辑/删除了账号）
  useEffect(() => {
    if (!settingsOpen) reloadAccounts();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settingsOpen]);

  const handleSelectAccount = (id: string | null) => {
    setSelectedAccountId(id);
    refresh(id);
  };

  return (
    <div className="min-h-screen flex flex-col">
      <Header onOpenSettings={() => setSettingsOpen(true)} />
      <main className="flex-1 max-w-6xl w-full mx-auto px-6 py-6 space-y-6">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <QuotaCard
            snapshot={snapshot}
            onRefresh={() => {
              refresh(selectedAccountId);
              ccStatus.refresh();
            }}
            refreshing={refreshing}
            error={quotaError}
            accounts={accounts}
            selectedAccountId={selectedAccountId}
            onSelectAccount={handleSelectAccount}
          />
          <CcSwitchCard
            detect={ccStatus.detect}
            providers={ccStatus.providers}
            loading={ccStatus.loading}
            onRefresh={ccStatus.refresh}
            onLog={log}
          />
        </div>
        <EventLog lines={logs} />
      </main>

      <SettingsModal
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        config={config}
        onConfigChange={save}
        onLog={log}
        onCcStatusRefresh={ccStatus.refresh}
      />
    </div>
  );
}
