// 通用格式化 / 颜色映射

export function fmtNumber(n: number | null | undefined): string {
  if (n === null || n === undefined) return "-";
  return Number(n).toLocaleString();
}

export function fmtTime(unixSec: number | undefined | null): string {
  if (!unixSec || unixSec <= 0) return "-";
  return new Date(unixSec * 1000).toLocaleString();
}

export function fmtCountdown(unixSec: number | undefined | null): string {
  if (!unixSec || unixSec <= 0) return "-";
  let diff = Math.max(0, unixSec * 1000 - Date.now());
  const days = Math.floor(diff / 86_400_000);
  diff -= days * 86_400_000;
  const hours = Math.floor(diff / 3_600_000);
  diff -= hours * 3_600_000;
  const minutes = Math.floor(diff / 60_000);
  const hh = String(hours).padStart(2, "0");
  const mm = String(minutes).padStart(2, "0");
  return days > 0 ? `${days}天${hh}时${mm}分钟` : `${hh}时${mm}分钟`;
}

export const PERIOD_LABELS: Record<string, string> = {
  session: "近5小时",
  weekly: "近1周",
  monthly: "近1月",
  daily: "近1日",
  FiveHour: "近5小时",
  Daily: "近1日",
  Weekly: "近1周",
  Monthly: "近1月",
};

export function periodLabel(level: string): string {
  return PERIOD_LABELS[level] ?? level;
}

/** 0~1 范围的占用比 */
export function ratioOf(period: { quota: number; used: number; percent: number }): number {
  if (period.quota && period.quota > 0) return period.used / period.quota;
  return Math.max(0, Math.min(1, (period.percent || 0) / 100));
}

export type Severity = "ok" | "warn" | "danger";

export function severityOf(ratio: number): Severity {
  if (ratio >= 1.0) return "danger";
  if (ratio > 0.8) return "warn";
  return "ok";
}

export function colorFor(sev: Severity): string {
  return sev === "danger" ? "#ff4d4f" : sev === "warn" ? "#ffa940" : "#52c41a";
}
