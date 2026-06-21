import { useEffect, useState } from "react";
import { AppConfig } from "../api";
import AccountsTab from "./AccountsTab";
import BindingsTab from "./BindingsTab";
import CcSwitchTab from "./CcSwitchTab";
import PolicyTab from "./PolicyTab";
import QuickSetupTab from "./QuickSetupTab";

interface Props {
  open: boolean;
  onClose: () => void;
  config: AppConfig | null;
  onConfigChange: (next: AppConfig) => Promise<void>;
  onLog: (msg: string) => void;
  onCcStatusRefresh: () => void;
}

const TABS = [
  { id: "quicksetup", label: "快速配置" },
  { id: "accounts", label: "方舟账号" },
  { id: "bindings", label: "账号 ↔ 套餐 绑定" },
  { id: "policy", label: "切换策略" },
  { id: "ccswitch", label: "cc-switch 集成" },
] as const;

type TabId = (typeof TABS)[number]["id"];

export default function SettingsModal({
  open,
  onClose,
  config,
  onConfigChange,
  onLog,
  onCcStatusRefresh,
}: Props) {
  const [tab, setTab] = useState<TabId>("quicksetup");

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open || !config) return null;

  return (
    <div className="fixed inset-0 z-30 bg-black/60 backdrop-blur-sm flex items-stretch justify-end">
      <div className="w-full max-w-2xl bg-panel border-l border-border shadow-2xl h-full flex flex-col">
        <div className="flex items-center justify-between px-5 py-4 border-b border-border">
          <h2 className="text-base font-semibold">设置</h2>
          <button className="btn btn-ghost" onClick={onClose}>
            关闭
          </button>
        </div>
        <div className="flex border-b border-border bg-panel2 overflow-x-auto">
          {TABS.map((t) => (
            <button
              key={t.id}
              onClick={() => setTab(t.id)}
              className={`px-4 py-2.5 text-sm whitespace-nowrap border-b-2 transition ${
                tab === t.id
                  ? "border-primary text-primary"
                  : "border-transparent text-muted hover:text-text"
              }`}
            >
              {t.label}
            </button>
          ))}
        </div>
        <div className="flex-1 overflow-auto p-5">
          {tab === "quicksetup" && (
            <QuickSetupTab onLog={onLog} onDone={onCcStatusRefresh} />
          )}
          {tab === "accounts" && (
            <AccountsTab onLog={onLog} />
          )}
          {tab === "bindings" && (
            <BindingsTab onLog={onLog} onCcStatusRefresh={onCcStatusRefresh} />
          )}
          {tab === "policy" && (
            <PolicyTab config={config} onConfigChange={onConfigChange} />
          )}
          {tab === "ccswitch" && (
            <CcSwitchTab config={config} onConfigChange={onConfigChange} />
          )}
        </div>
      </div>
    </div>
  );
}
