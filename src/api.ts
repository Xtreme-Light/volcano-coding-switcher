// 后端命令薄封装。Tauri v2: window.__TAURI__.core.invoke
// 类型定义与 src-tauri/src 下的 Rust 结构体保持一致。

declare global {
  interface Window {
    __TAURI__?: {
      core?: { invoke: (cmd: string, args?: unknown) => Promise<unknown> };
      event?: {
        listen: (
          event: string,
          handler: (e: { payload: unknown }) => void
        ) => Promise<() => void>;
      };
      shell?: {
        open: (url: string, openWith?: string) => Promise<void>;
      };
    };
  }
}

const tauri = window.__TAURI__;
export const isTauri = !!tauri?.core?.invoke;

/// 用系统默认浏览器打开外链（依赖 tauri-plugin-shell）。
export async function openUrl(url: string): Promise<void> {
  if (tauri?.shell?.open) {
    await tauri.shell.open(url);
    return;
  }
  // 兜底：非 Tauri 环境（如纯浏览器调试）用 window.open
  window.open(url, "_blank", "noopener,noreferrer");
}

export async function invoke<T = unknown>(
  cmd: string,
  args?: Record<string, unknown>
): Promise<T> {
  if (!tauri?.core?.invoke) {
    throw new Error("Tauri runtime 不可用，请通过 cargo tauri dev 启动");
  }
  return tauri.core.invoke(cmd, args) as Promise<T>;
}

export function listen<T = unknown>(
  event: string,
  handler: (payload: T) => void
): Promise<() => void> {
  if (!tauri?.event?.listen) return Promise.resolve(() => {});
  return tauri.event.listen(event, (e) => handler(e.payload as T));
}

// ---- 类型 ----

export interface ArkAccount {
  id: string;
  name: string;
  api_key?: string;
  access_key_id: string;
  access_key_secret: string;
  region: string;
  /// 此账号是否使用 Code Plan 接口；true → GetCodingPlanUsage，false → GetAFPUsage。
  use_coding_plan: boolean;
  /// OpenAPI 版本，默认 2024-01-01。
  api_version: string;
}

export interface AppConfig {
  accounts: ArkAccount[];
  bindings: Record<string, string>;
  threshold: number;
  poll_interval_secs: number;
  auto_switch: boolean;
  current_plan: string;
  cc_switch_db_path: string;
  restart_cc_switch_after_switch: boolean;
}

export interface PeriodUsage {
  level: string;
  quota: number;
  used: number;
  percent: number;
  subscribe_time: number;
  reset_time: number;
}

export interface QuotaSnapshot {
  plan_type: string;
  status: string;
  update_timestamp: number;
  fetched_at: number;
  source: "real" | "mock" | string;
  raw_response: string;
  periods: PeriodUsage[];
}

export interface CcProvider {
  id: string;
  name: string;
  is_current: boolean;
  base_url: string;
  auth_token: string;
  settings_config: string;
}

export interface DetectResult {
  installed: boolean;
  path: string;
  claude_provider_count: number;
  active_provider: string | null;
}

export interface BindingView {
  provider_id: string;
  provider_name: string;
  is_current: boolean;
  account_id: string | null;
  account_name: string | null;
  region: string | null;
}

export interface BindResult {
  conflict: boolean;
  previous_account_id: string | null;
  previous_account_name: string | null;
  bound: boolean;
}

/// 单个 cc-switch 套餐的用量查询结果（来自 fetch_all_quotas）。
export interface ProviderQuota {
  provider_id: string;
  provider_name: string;
  is_current: boolean;
  account_id: string | null;
  account_name: string | null;
  /// 0~1 范围。"近5小时"周期（session/FiveHour）的使用率，
  /// 用于套餐列表展示和"选最低用量套餐"切换策略。
  short_term_ratio: number;
  /// 出错时填错误信息（snapshot 仍可能为 null）。
  error: string | null;
  snapshot: QuotaSnapshot | null;
}

// ---- 命令 ----

export const api = {
  getConfig: () => invoke<AppConfig>("get_config"),
  saveConfig: (config: AppConfig) => invoke<void>("save_config", { config }),

  fetchQuota: () => invoke<QuotaSnapshot>("fetch_quota"),
  /// 按指定账号 ID 查询用量（用于用量区域切换账号显示）。
  fetchQuotaByAccount: (accountId: string) =>
    invoke<QuotaSnapshot>("fetch_quota_by_account", { accountId }),
  /// 查询所有"已绑定方舟账号"的 cc-switch 套餐用量。
  fetchAllQuotas: () => invoke<ProviderQuota[]>("fetch_all_quotas"),

  detectCcSwitch: () => invoke<DetectResult>("detect_cc_switch"),
  listCcProviders: () => invoke<CcProvider[]>("list_cc_providers"),
  getActiveCcProvider: () => invoke<CcProvider | null>("get_active_cc_provider"),
  switchPlan: (plan: string) => invoke<string>("switch_plan", { plan }),

  listAccounts: () => invoke<ArkAccount[]>("list_accounts"),
  upsertAccount: (account: {
    id?: string;
    name: string;
    access_key_id: string;
    access_key_secret: string;
    region: string;
    api_key?: string;
    use_coding_plan?: boolean;
    api_version?: string;
  }) => invoke<ArkAccount>("upsert_account", { account }),
  deleteAccount: (id: string) => invoke<void>("delete_account", { id }),

  listBindings: () => invoke<BindingView[]>("list_bindings"),
  bindProvider: (request: {
    provider_id: string;
    account_id: string;
    overwrite?: boolean;
  }) => invoke<BindResult>("bind_provider", { request }),
  unbindProvider: (provider_id: string) =>
    invoke<void>("unbind_provider", { providerId: provider_id }),
};
