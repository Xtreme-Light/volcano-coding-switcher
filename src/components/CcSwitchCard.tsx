import { useState } from "react";
import { CcProvider, DetectResult, ProviderQuota, api } from "../api";

interface Props {
  detect: DetectResult | null;
  providers: CcProvider[];
  loading: boolean;
  onRefresh: () => void;
  onLog: (msg: string) => void;
}

function ratioColor(ratio: number): string {
  if (ratio >= 1) return "text-danger";
  if (ratio >= 0.8) return "text-warn";
  return "text-success";
}

export default function CcSwitchCard({
  detect,
  providers,
  loading,
  onRefresh,
  onLog,
}: Props) {
  const active = providers.find((p) => p.is_current);
  const [target, setTarget] = useState<string>(active?.id ?? providers[0]?.id ?? "");
  const [switching, setSwitching] = useState(false);
  const [allQuotas, setAllQuotas] = useState<ProviderQuota[] | null>(null);
  const [queryingAll, setQueryingAll] = useState(false);

  if (!detect) {
    return (
      <section className="card">
        <h2 className="text-base font-semibold">cc-switch 状态</h2>
        <div className="text-muted text-sm mt-2">{loading ? "检测中…" : "尚未检测"}</div>
      </section>
    );
  }

  if (!detect.installed) {
    return (
      <section className="card">
        <h2 className="text-base font-semibold">cc-switch 状态</h2>
        <div className="mt-3 bg-warn/10 border border-warn/40 rounded-lg p-3 text-sm">
          <div className="font-semibold text-warn">未检测到 cc-switch 数据库</div>
          <div className="text-xs text-muted mt-1">
            期望路径：<code className="bg-panel2 px-1.5 py-0.5 rounded">{detect.path}</code>
          </div>
          <ol className="list-decimal pl-5 mt-2 space-y-1 text-xs leading-relaxed">
            <li>
              前往{" "}
              <a
                className="text-primary underline"
                href="https://github.com/farion1231/cc-switch/releases"
                target="_blank"
                rel="noreferrer"
              >
                cc-switch Releases
              </a>{" "}
              下载安装：Linux .AppImage / .deb，macOS .dmg，Windows .msi
            </li>
            <li>启动 cc-switch，添加至少一个 Claude Provider。</li>
            <li>回到本工具点击"刷新"即可同步。</li>
            <li>如安装路径不同，可在右上角"设置 → cc-switch 集成"中指定。</li>
          </ol>
        </div>
      </section>
    );
  }

  const handleSwitch = async () => {
    if (!target) return;
    setSwitching(true);
    try {
      const msg = await api.switchPlan(target);
      onLog(msg);
      onRefresh();
    } catch (e) {
      onLog(`切换失败: ${e}`);
    } finally {
      setSwitching(false);
    }
  };

  const handleQueryAll = async () => {
    setQueryingAll(true);
    try {
      const list = await api.fetchAllQuotas();
      setAllQuotas(list);
      const ok = list.filter((q) => !q.error);
      onLog(`已查询 ${list.length} 个套餐用量（成功 ${ok.length}）`);
    } catch (e) {
      onLog(`查询所有套餐用量失败: ${e}`);
    } finally {
      setQueryingAll(false);
    }
  };

  // 把 provider 列表与 allQuotas 合并，便于在列表里显示每个套餐的近5小时用量。
  const quotaById = new Map(
    (allQuotas ?? []).map((q) => [q.provider_id, q] as const)
  );
  // 找出"非当前、已绑定且查询成功"的候选里近5小时用量最低的那个，用于提示用户。
  // 未绑定的套餐直接跳过，不参与切换候选。
  const lowestCandidate = (allQuotas ?? [])
    .filter((q) => !q.is_current && q.account_id && !q.error)
    .sort((a, b) => a.short_term_ratio - b.short_term_ratio)[0];

  return (
    <section className="card flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <h2 className="text-base font-semibold">cc-switch 状态</h2>
        <div className="flex gap-2">
          <button
            className="btn"
            onClick={handleQueryAll}
            disabled={queryingAll || providers.length === 0}
            title="查询所有已绑定账号的套餐用量，用于挑选最低用量套餐"
          >
            {queryingAll ? "查询中…" : "查询所有用量"}
          </button>
          <button className="btn" onClick={onRefresh} disabled={loading}>
            {loading ? "刷新中…" : "刷新"}
          </button>
        </div>
      </div>

      <div className="text-xs text-muted space-y-1">
        <div>
          <span className="text-muted">数据库：</span>
          <code className="bg-panel2 px-1.5 py-0.5 rounded text-text">{detect.path}</code>
        </div>
        <div>
          <span className="text-muted">套餐数：</span>
          {providers.length}
        </div>
        <div>
          <span className="text-muted">当前激活：</span>
          {active ? (
            <span className="pill text-success border-success/40">{active.name}</span>
          ) : (
            <span className="text-muted">未设置</span>
          )}
        </div>
      </div>

      {lowestCandidate ? (
        <div className="bg-primary/10 border border-primary/40 rounded-md p-2 text-xs">
          建议切换到{" "}
          <span className="font-semibold text-primary">{lowestCandidate.provider_name}</span>
          （近5小时 {(lowestCandidate.short_term_ratio * 100).toFixed(1)}%，当前最低）
        </div>
      ) : null}

      <div className="divider" />

      <div>
        <div className="label">套餐列表（来自 cc-switch）</div>
        <div className="space-y-2 max-h-44 overflow-auto">
          {providers.map((p) => {
            const q = quotaById.get(p.id);
            return (
              <label
                key={p.id}
                className={`flex items-start gap-3 px-3 py-2 rounded-md border ${
                  target === p.id ? "border-primary bg-primary/10" : "border-border bg-panel2"
                } cursor-pointer hover:border-primary/60 transition`}
              >
                <input
                  type="radio"
                  name="plan"
                  className="mt-1 accent-primary"
                  checked={target === p.id}
                  onChange={() => setTarget(p.id)}
                />
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 flex-wrap">
                    <span className="font-medium">{p.name}</span>
                    {p.is_current ? (
                      <span className="pill text-success border-success/40 text-[10px]">
                        当前
                      </span>
                    ) : null}
                    {q && q.account_id === null ? (
                      <span className="text-[10px] text-muted" title="此套餐未绑定方舟账号，切换时会自动跳过">
                        未绑定
                      </span>
                    ) : null}
                    {q && q.account_id !== null && q.error ? (
                      <span className="text-[10px] text-danger truncate" title={q.error}>
                        ({q.error})
                      </span>
                    ) : null}
                    {q && q.account_id !== null && !q.error ? (
                      <span className={`text-[11px] font-mono ${ratioColor(q.short_term_ratio)}`}>
                        近5小时 {(q.short_term_ratio * 100).toFixed(1)}%
                      </span>
                    ) : null}
                  </div>
                  <div className="text-[11px] text-muted truncate">
                    {p.base_url || "-"}
                    {q?.account_name ? ` · 账号 ${q.account_name}` : ""}
                  </div>
                </div>
              </label>
            );
          })}
        </div>
      </div>

      <button
        className="btn btn-primary w-full"
        disabled={!target || switching || target === active?.id}
        onClick={handleSwitch}
      >
        {switching
          ? "切换中…"
          : target === active?.id
            ? "已是当前套餐"
            : "立即切换到此套餐"}
      </button>
    </section>
  );
}
