import { useState } from "react";
import { AppConfig } from "../api";

interface Props {
  config: AppConfig;
  onConfigChange: (next: AppConfig) => Promise<void>;
}

export default function PolicyTab({ config, onConfigChange }: Props) {
  const [draft, setDraft] = useState(config);
  const [saving, setSaving] = useState(false);
  const [hint, setHint] = useState("");
  const dirty = JSON.stringify(draft) !== JSON.stringify(config);

  const save = async () => {
    setSaving(true);
    try {
      await onConfigChange(draft);
      setHint("已保存");
      setTimeout(() => setHint(""), 2000);
    } catch (e) {
      setHint(`保存失败: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-4">
      <p className="text-sm text-muted">用量越过阈值时本工具会发出系统通知，并按需切换到 cc-switch 中下一个套餐。</p>

      <div>
        <label className="label">临近阈值（0~1，任一周期使用率超过即触发）</label>
        <input
          type="number"
          step="0.01"
          min={0}
          max={1}
          className="input"
          value={draft.threshold}
          onChange={(e) =>
            setDraft({ ...draft, threshold: parseFloat(e.target.value) || 0 })
          }
        />
      </div>

      <div>
        <label className="label">轮询间隔（秒，最低 30）</label>
        <input
          type="number"
          min={30}
          className="input"
          value={draft.poll_interval_secs}
          onChange={(e) =>
            setDraft({
              ...draft,
              poll_interval_secs: parseInt(e.target.value, 10) || 30,
            })
          }
        />
      </div>

      <label className="flex items-start gap-3 bg-panel2 border border-border p-3 rounded-md cursor-pointer">
        <input
          type="checkbox"
          className="mt-1 accent-primary"
          checked={draft.auto_switch}
          onChange={(e) => setDraft({ ...draft, auto_switch: e.target.checked })}
        />
        <div>
          <div className="font-medium">达到阈值时自动切换到下一个套餐</div>
          <div className="text-xs text-muted">从 cc-switch 套餐列表中选择一个非当前激活项。</div>
        </div>
      </label>

      <div className="flex items-center justify-between">
        <span className="text-xs text-muted">{hint}</span>
        <button className="btn btn-primary" disabled={!dirty || saving} onClick={save}>
          {saving ? "保存中…" : "保存"}
        </button>
      </div>
    </div>
  );
}
