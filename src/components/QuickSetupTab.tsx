import { useState } from "react";
import { api, openUrl } from "../api";

interface Props {
  onLog: (msg: string) => void;
  onDone: () => void;
}

const ARK_MODELS = [
  "glm-5.2",
  "glm-latest",
  "doubao-seed-2.0-code",
  "doubao-seed-2.0-pro",
  "doubao-seed-2.0-lite",
  "doubao-seed-code",
  "minimax-m2.7",
  "minimax-m3",
  "deepseek-v4-flash",
  "deepseek-v4-pro",
  "kimi-k2.6",
  "kimi-k2.7-code",
];

const ARK_BASE_URL = "https://ark.cn-beijing.volces.com/api/coding";
const API_KEY_URL =
  "https://console.volcengine.com/ark/region:ark+cn-beijing/openManagement?LLM=%7B%7D&OpenModelVisible=false&advancedActiveKey=enterprise";

export default function QuickSetupTab({ onLog, onDone }: Props) {
  const [providerName, setProviderName] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [model, setModel] = useState("glm-5.2");
  const [accountName, setAccountName] = useState("");
  const [accessKeyId, setAccessKeyId] = useState("");
  const [accessKeySecret, setAccessKeySecret] = useState("");
  const [region, setRegion] = useState("cn-beijing");
  const [busy, setBusy] = useState(false);

  const submit = async () => {
    if (!apiKey.trim()) {
      onLog("Coding Plan API Key 不能为空");
      return;
    }
    if (!accessKeyId.trim() || !accessKeySecret.trim()) {
      onLog("AK / SK 不能为空");
      return;
    }
    setBusy(true);
    try {
      const result = await api.setupArkProvider({
        provider_name:
          providerName.trim() || `${model}-${region}`,
        api_key: apiKey.trim(),
        model,
        account_name: accountName.trim() || "默认账号",
        access_key_id: accessKeyId.trim(),
        access_key_secret: accessKeySecret.trim(),
        region: region.trim() || "cn-beijing",
      });
      onLog(
        `已创建 provider「${result.provider.name}」并绑定方舟账号，可前往主页刷新查看用量。`
      );
      onDone();
      // 重置表单
      setProviderName("");
      setApiKey("");
      setAccessKeyId("");
      setAccessKeySecret("");
    } catch (e) {
      onLog(`一键配置失败: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="space-y-4">
      <div className="bg-info/10 border border-info/30 rounded-md p-3 text-sm space-y-1.5">
        <div className="font-semibold text-info">一键配置说明</div>
        <div className="text-xs text-muted leading-relaxed">
          填入火山方舟 Coding Plan 的 <strong>专属 API Key</strong>（用于 Claude Code 鉴权）
          和 <strong>AccessKey</strong>（用于用量查询），本工具会自动：
          <ol className="list-decimal pl-5 mt-1 space-y-0.5">
            <li>在 cc-switch 中创建一个 Claude Provider，配置好方舟 Coding Plan 端点和模型</li>
            <li>创建方舟账号（AK/SK），用于后续用量监控</li>
            <li>自动绑定 Provider ↔ 账号，开启用量查询和自动切换</li>
          </ol>
        </div>
        <div className="text-xs">
          Base URL: <code className="bg-panel2 px-1.5 py-0.5 rounded">{ARK_BASE_URL}</code>
        </div>
      </div>

      <div className="space-y-3">
        <div>
          <label className="label">Provider 名称（在 cc-switch 中显示，留空则自动生成）</label>
          <input
            className="input"
            value={providerName}
            onChange={(e) => setProviderName(e.target.value)}
            placeholder={`例如：${model}-主账号`}
          />
        </div>

        <div>
          <label className="label">
            Coding Plan 专属 API Key{" "}
            <button
              type="button"
              className="text-primary hover:underline text-xs ml-1"
              onClick={() => openUrl(API_KEY_URL)}
            >
              前往获取 →
            </button>
          </label>
          <input
            type="password"
            className="input"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder="与火山方舟平台 API Key 不同，请勿混用"
          />
        </div>

        <div>
          <label className="label">模型</label>
          <select
            className="input"
            value={model}
            onChange={(e) => setModel(e.target.value)}
          >
            {ARK_MODELS.map((m) => (
              <option key={m} value={m}>
                {m}
              </option>
            ))}
          </select>
        </div>

        <div className="border-t border-border pt-3 space-y-3">
          <div className="text-xs text-muted">
            以下 AccessKey 用于用量查询（AK/SK 签名调用方舟 OpenAPI），与上面的 API Key 不同。
          </div>
          <div>
            <label className="label">账号名（仅用于本工具区分）</label>
            <input
              className="input"
              value={accountName}
              onChange={(e) => setAccountName(e.target.value)}
              placeholder="主账号 / 团队账号 / xxx"
            />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="label">Access Key ID</label>
              <input
                className="input"
                value={accessKeyId}
                onChange={(e) => setAccessKeyId(e.target.value)}
              />
            </div>
            <div>
              <label className="label">区域</label>
              <input
                className="input"
                value={region}
                onChange={(e) => setRegion(e.target.value)}
                placeholder="cn-beijing"
              />
            </div>
          </div>
          <div>
            <label className="label">Access Key Secret</label>
            <input
              type="password"
              className="input"
              value={accessKeySecret}
              onChange={(e) => setAccessKeySecret(e.target.value)}
            />
          </div>
        </div>

        <div className="flex justify-end">
          <button
            className="btn btn-primary px-4 py-2 text-sm"
            onClick={submit}
            disabled={busy}
          >
            {busy ? "配置中…" : "一键配置"}
          </button>
        </div>
      </div>
    </div>
  );
}
