import { useEffect, useState } from "react";
import { ArkAccount, api } from "../api";

interface Props {
  onLog: (msg: string) => void;
}

interface FormState {
  id: string;
  name: string;
  access_key_id: string;
  access_key_secret: string;
  region: string;
  use_coding_plan: boolean;
  api_version: string;
}

const empty: FormState = {
  id: "",
  name: "",
  access_key_id: "",
  access_key_secret: "",
  region: "cn-beijing",
  use_coding_plan: true,
  api_version: "2024-01-01",
};

export default function AccountsTab({ onLog }: Props) {
  const [accounts, setAccounts] = useState<ArkAccount[]>([]);
  const [editing, setEditing] = useState<FormState | null>(null);
  const [busy, setBusy] = useState(false);

  const reload = async () => {
    const list = await api.listAccounts();
    setAccounts(list);
  };

  useEffect(() => {
    reload();
  }, []);

  const startNew = () => setEditing({ ...empty });
  const startEdit = (a: ArkAccount) =>
    setEditing({
      id: a.id,
      name: a.name,
      access_key_id: a.access_key_id,
      access_key_secret: a.access_key_secret,
      region: a.region || "cn-beijing",
      use_coding_plan: a.use_coding_plan ?? true,
      api_version: a.api_version || "2024-01-01",
    });

  const submit = async () => {
    if (!editing) return;
    if (!editing.name.trim()) {
      onLog("账号名不能为空");
      return;
    }
    if (!editing.access_key_id.trim() || !editing.access_key_secret.trim()) {
      onLog("AK / SK 不能为空");
      return;
    }
    setBusy(true);
    try {
      await api.upsertAccount({
        id: editing.id || undefined,
        name: editing.name.trim(),
        access_key_id: editing.access_key_id.trim(),
        access_key_secret: editing.access_key_secret.trim(),
        region: editing.region.trim() || "cn-beijing",
        use_coding_plan: editing.use_coding_plan,
        api_version: editing.api_version.trim() || "2024-01-01",
      });
      onLog(editing.id ? `已更新账号 ${editing.name}` : `已新增账号 ${editing.name}`);
      setEditing(null);
      await reload();
    } catch (e) {
      onLog(`保存账号失败: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  const remove = async (a: ArkAccount) => {
    if (!confirm(`删除账号「${a.name}」？相关绑定也会被解除。`)) return;
    try {
      await api.deleteAccount(a.id);
      onLog(`已删除账号 ${a.name}`);
      await reload();
    } catch (e) {
      onLog(`删除账号失败: ${e}`);
    }
  };

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between gap-2">
        <p className="text-xs text-muted">
          每个账号对应一组方舟 AccessKey，可在「绑定」页绑给一个或多个 cc-switch 套餐。用量接口（Code Plan / AFP）跟随账号配置。
        </p>
        <button className="btn btn-primary shrink-0 px-2.5 py-1.5 text-xs" onClick={startNew}>
          + 新增
        </button>
      </div>

      <div className="space-y-1.5">
        {accounts.length === 0 ? (
          <div className="text-muted text-sm">暂无账号，点击右上"新增"创建。</div>
        ) : (
          accounts.map((a) => (
            <div
              key={a.id}
              className="bg-panel2 border border-border rounded-md px-3 py-2 flex items-center justify-between gap-2"
            >
              <div className="min-w-0 flex-1">
                <div className="font-medium text-sm truncate">{a.name}</div>
                <div className="text-[11px] text-muted truncate">
                  AK: {a.access_key_id || "(空)"} · {a.region} ·{" "}
                  {a.use_coding_plan ? "Code Plan" : "AFP"} · v{a.api_version}
                </div>
              </div>
              <div className="flex gap-1.5 shrink-0">
                <button
                  className="btn px-2 py-1 text-xs"
                  onClick={() => startEdit(a)}
                >
                  编辑
                </button>
                <button
                  className="btn px-2 py-1 text-xs text-danger border-danger/40 hover:border-danger"
                  onClick={() => remove(a)}
                >
                  删除
                </button>
              </div>
            </div>
          ))
        )}
      </div>

      {editing ? (
        <div className="bg-panel2 border border-border rounded-md p-3 space-y-3">
          <h3 className="text-sm font-semibold">{editing.id ? "编辑账号" : "新增账号"}</h3>
          <div>
            <label className="label">账号名（仅用于在本工具中区分）</label>
            <input
              className="input"
              value={editing.name}
              onChange={(e) => setEditing({ ...editing, name: e.target.value })}
              placeholder="主账号 / 团队账号 / xxx"
            />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="label">Access Key ID</label>
              <input
                className="input"
                value={editing.access_key_id}
                onChange={(e) =>
                  setEditing({ ...editing, access_key_id: e.target.value })
                }
              />
            </div>
            <div>
              <label className="label">区域</label>
              <input
                className="input"
                value={editing.region}
                onChange={(e) => setEditing({ ...editing, region: e.target.value })}
                placeholder="cn-beijing"
              />
            </div>
          </div>
          <div>
            <label className="label">Access Key Secret</label>
            <input
              type="password"
              className="input"
              value={editing.access_key_secret}
              onChange={(e) =>
                setEditing({ ...editing, access_key_secret: e.target.value })
              }
            />
          </div>
          <div className="bg-panel border border-border rounded-md p-2.5 space-y-2.5">
            <label className="flex items-start gap-2.5 cursor-pointer">
              <input
                type="checkbox"
                className="mt-0.5 accent-primary"
                checked={editing.use_coding_plan}
                onChange={(e) =>
                  setEditing({ ...editing, use_coding_plan: e.target.checked })
                }
              />
              <div>
                <div className="font-medium text-sm">Code Plan 套餐</div>
                <div className="text-[11px] text-muted">
                  勾选 → <code>GetCodingPlanUsage</code>；取消 → <code>GetAFPUsage</code>。不同账号可配不同类型。
                </div>
              </div>
            </label>
            <div>
              <label className="label">OpenAPI Version</label>
              <input
                className="input"
                value={editing.api_version}
                onChange={(e) =>
                  setEditing({ ...editing, api_version: e.target.value })
                }
                placeholder="2024-01-01"
              />
            </div>
          </div>
          <div className="flex gap-2 justify-end">
            <button className="btn px-2.5 py-1.5 text-xs" onClick={() => setEditing(null)} disabled={busy}>
              取消
            </button>
            <button className="btn btn-primary px-2.5 py-1.5 text-xs" onClick={submit} disabled={busy}>
              {busy ? "保存中…" : "保存"}
            </button>
          </div>
        </div>
      ) : null}
    </div>
  );
}
