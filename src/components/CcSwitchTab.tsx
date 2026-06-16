import { useState } from "react";
import { AppConfig } from "../api";

interface Props {
  config: AppConfig;
  onConfigChange: (next: AppConfig) => Promise<void>;
}

export default function CcSwitchTab({ config, onConfigChange }: Props) {
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
      <div>
        <label className="label">cc-switch 数据库路径（留空使用默认 ~/.cc-switch/cc-switch.db）</label>
        <input
          className="input"
          value={draft.cc_switch_db_path}
          onChange={(e) => setDraft({ ...draft, cc_switch_db_path: e.target.value })}
          placeholder="~/.cc-switch/cc-switch.db"
        />
      </div>

      <label className="flex items-start gap-3 bg-panel2 border border-border p-3 rounded-md cursor-pointer">
        <input
          type="checkbox"
          className="mt-1 accent-primary"
          checked={draft.restart_cc_switch_after_switch}
          onChange={(e) =>
            setDraft({
              ...draft,
              restart_cc_switch_after_switch: e.target.checked,
            })
          }
        />
        <div>
          <div className="font-medium">切换后自动重启 cc-switch GUI</div>
          <div className="text-xs text-muted">
            cc-switch 启动时把数据库装进内存，外部修改它感知不到；如果它正在运行，本工具会优雅地结束并重新启动它。
          </div>
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
