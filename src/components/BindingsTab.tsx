import { useEffect, useState } from "react";
import { ArkAccount, BindingView, api } from "../api";

interface Props {
  onLog: (msg: string) => void;
  onCcStatusRefresh: () => void;
}

export default function BindingsTab({ onLog, onCcStatusRefresh }: Props) {
  const [accounts, setAccounts] = useState<ArkAccount[]>([]);
  const [bindings, setBindings] = useState<BindingView[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const reload = async () => {
    setError(null);
    try {
      const [accs, binds] = await Promise.all([api.listAccounts(), api.listBindings()]);
      setAccounts(accs);
      setBindings(binds);
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => {
    reload();
  }, []);

  const setBinding = async (b: BindingView, accountId: string) => {
    if (!accountId) {
      try {
        await api.unbindProvider(b.provider_id);
        onLog(`已解除 ${b.provider_name} 的账号绑定`);
        await reload();
        onCcStatusRefresh();
      } catch (e) {
        onLog(`解绑失败: ${e}`);
      }
      return;
    }
    if (b.account_id === accountId) return;

    setBusy(true);
    try {
      let result = await api.bindProvider({
        provider_id: b.provider_id,
        account_id: accountId,
      });
      if (result.conflict) {
        const ok = confirm(
          `「${b.provider_name}」当前已绑定到「${result.previous_account_name ?? result.previous_account_id}」。\n确定覆盖吗？`
        );
        if (!ok) {
          setBusy(false);
          return;
        }
        result = await api.bindProvider({
          provider_id: b.provider_id,
          account_id: accountId,
          overwrite: true,
        });
      }
      if (result.bound) {
        const accName = accounts.find((a) => a.id === accountId)?.name ?? accountId;
        onLog(`已绑定 ${b.provider_name} → ${accName}`);
      }
      await reload();
      onCcStatusRefresh();
    } catch (e) {
      onLog(`绑定失败: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="space-y-3">
      <p className="text-sm text-muted">
        把 cc-switch 中的每个 Claude 套餐绑定到一个方舟账号。查询用量时本工具会按"当前激活套餐"去找它绑定的账号 AK/SK。
      </p>
      {error ? (
        <div className="text-danger text-xs bg-danger/10 border border-danger/30 rounded-md p-2">
          {error}
        </div>
      ) : null}

      {bindings.length === 0 ? (
        <div className="text-muted text-sm">cc-switch 中没有 Claude 套餐，请先在 cc-switch 内添加。</div>
      ) : (
        <table className="w-full text-sm">
          <thead className="text-xs text-muted">
            <tr>
              <th className="text-left py-2 px-2">套餐</th>
              <th className="text-left py-2 px-2">绑定账号</th>
              <th className="text-left py-2 px-2">区域</th>
              <th className="text-right py-2 px-2 w-28">操作</th>
            </tr>
          </thead>
          <tbody>
            {bindings.map((b) => (
              <tr key={b.provider_id} className="border-t border-border">
                <td className="py-2 px-2">
                  <div className="flex items-center gap-2">
                    <span className="font-medium">{b.provider_name}</span>
                    {b.is_current ? (
                      <span className="pill text-success border-success/40 text-[10px]">
                        当前
                      </span>
                    ) : null}
                  </div>
                </td>
                <td className="py-2 px-2">
                  <select
                    className="input"
                    value={b.account_id ?? ""}
                    disabled={busy}
                    onChange={(e) => setBinding(b, e.target.value)}
                  >
                    <option value="">— 未绑定 —</option>
                    {accounts.map((a) => (
                      <option key={a.id} value={a.id}>
                        {a.name}
                      </option>
                    ))}
                  </select>
                </td>
                <td className="py-2 px-2 text-muted">{b.region ?? "-"}</td>
                <td className="py-2 px-2 text-right">
                  {b.account_id ? (
                    <button
                      className="btn btn-ghost text-xs"
                      onClick={() => setBinding(b, "")}
                    >
                      解绑
                    </button>
                  ) : null}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {accounts.length === 0 ? (
        <div className="text-warn text-xs bg-warn/10 border border-warn/40 rounded-md p-2">
          还没有账号，请先在「方舟账号」标签页里添加。
        </div>
      ) : null}
    </div>
  );
}
