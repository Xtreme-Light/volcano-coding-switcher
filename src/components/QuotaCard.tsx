import { ArkAccount, QuotaSnapshot } from "../api";
import { fmtCountdown, fmtTime, periodLabel, ratioOf } from "../utils/fmt";
import UsageRing from "./UsageRing";

interface Props {
  snapshot: QuotaSnapshot | null;
  onRefresh: () => void;
  refreshing: boolean;
  error: string | null;
  accounts: ArkAccount[];
  selectedAccountId: string | null;
  onSelectAccount: (id: string | null) => void;
}

export default function QuotaCard({
  snapshot,
  onRefresh,
  refreshing,
  error,
  accounts,
  selectedAccountId,
  onSelectAccount,
}: Props) {
  const periods = snapshot?.periods ?? [];
  const updatedAt =
    snapshot?.update_timestamp && snapshot.update_timestamp > 0
      ? new Date(snapshot.update_timestamp * 1000).toLocaleString()
      : snapshot?.fetched_at
        ? new Date(snapshot.fetched_at).toLocaleString()
        : "-";

  return (
    <section className="card flex flex-col gap-4">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <h2 className="text-base font-semibold">用量</h2>
          <div className="text-xs text-muted mt-0.5">
            {snapshot ? `数据统计截止 ${updatedAt}` : "尚未加载"}
            {snapshot?.plan_type ? (
              <span className="ml-2 pill">{snapshot.plan_type}</span>
            ) : null}
            {snapshot?.source === "mock" ? (
              <span className="ml-2 pill text-warn border-warn/40">MOCK</span>
            ) : null}
          </div>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          {accounts.length > 0 ? (
            <select
              className="input py-1 px-2 text-xs w-auto"
              value={selectedAccountId ?? ""}
              onChange={(e) => onSelectAccount(e.target.value || null)}
              title="切换查看不同方舟账号的用量"
            >
              {accounts.map((a) => (
                <option key={a.id} value={a.id}>
                  {a.name}
                </option>
              ))}
            </select>
          ) : null}
          <button className="btn" onClick={onRefresh} disabled={refreshing}>
            {refreshing ? "刷新中…" : "刷新"}
          </button>
        </div>
      </div>

      {error ? (
        <div className="text-danger text-xs bg-danger/10 border border-danger/30 rounded-md p-2">
          {error}
        </div>
      ) : null}

      {periods.length === 0 ? (
        <div className="text-muted text-sm">暂无周期数据，请先在设置中配置方舟账号并绑定。</div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {periods.map((p) => {
            const r = ratioOf(p);
            const usage =
              p.quota > 0
                ? `${p.used.toLocaleString()} / ${p.quota.toLocaleString()}`
                : `已使用 ${(p.percent || 0).toFixed(2)}%`;
            return (
              <div
                key={p.level}
                className="flex flex-col items-center gap-2 bg-panel2 rounded-lg p-3 border border-border"
              >
                <UsageRing ratio={r} size={104} thickness={10} label={periodLabel(p.level)} />
                <div className="text-xs text-muted">{usage}</div>
                <div className="text-[11px] text-muted">
                  {fmtCountdown(p.reset_time)}后刷新
                </div>
                <div className="text-[10px] text-muted">重置 {fmtTime(p.reset_time)}</div>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}
