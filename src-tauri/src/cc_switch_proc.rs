//! 在外部修改 cc-switch.db 后，cc-switch GUI 不会热加载。
//! 这里提供"探测进程 / 优雅重启"的能力，让切换在 cc-switch 主窗口里也能立即看到。

use crate::error::{AppError, AppResult};
use std::process::{Command, Stdio};

#[cfg(target_os = "linux")]
fn pids_of(name_substr: &str) -> Vec<u32> {
    use std::fs;
    let mut out = Vec::new();
    let entries = match fs::read_dir("/proc") {
        Ok(e) => e,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let pid_str = entry.file_name();
        let pid_s = match pid_str.to_str() {
            Some(s) => s,
            None => continue,
        };
        let pid: u32 = match pid_s.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let cmd_path = entry.path().join("cmdline");
        let cmdline = match fs::read(&cmd_path) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let line = String::from_utf8_lossy(&cmdline);
        if line
            .split('\0')
            .any(|seg| seg.to_lowercase().contains(&name_substr.to_lowercase()))
        {
            out.push(pid);
        }
    }
    out
}

#[cfg(target_os = "macos")]
fn pids_of(name_substr: &str) -> Vec<u32> {
    let output = match std::process::Command::new("pgrep")
        .arg("-i")
        .arg("-f")
        .arg(name_substr)
        .output()
    {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|l| l.trim().parse().ok())
        .collect()
}

#[cfg(target_os = "windows")]
fn pids_of(name_substr: &str) -> Vec<u32> {
    // 用 PowerShell Get-Process 模糊匹配（避免引入 windows-rs 依赖）。
    let ps = format!(
        "Get-Process | Where-Object {{ $_.ProcessName -like '*{}*' }} | ForEach-Object {{ $_.Id }}",
        name_substr.replace('\'', "")
    );
    let output = match std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps])
        .output()
    {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|l| l.trim().parse().ok())
        .collect()
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn pids_of(_name_substr: &str) -> Vec<u32> {
    Vec::new()
}

/// 取出当前进程的可执行路径。
#[cfg(target_os = "linux")]
fn exe_of(pid: u32) -> Option<String> {
    std::fs::read_link(format!("/proc/{}/exe", pid))
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
}

#[cfg(target_os = "macos")]
fn exe_of(pid: u32) -> Option<String> {
    let output = std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .ok()?;
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() { None } else { Some(path) }
}

#[cfg(target_os = "windows")]
fn exe_of(pid: u32) -> Option<String> {
    let ps = format!(
        "(Get-Process -Id {}).Path",
        pid
    );
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps])
        .output()
        .ok()?;
    let p = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if p.is_empty() { None } else { Some(p) }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn exe_of(_pid: u32) -> Option<String> {
    None
}

#[derive(Debug, Clone)]
pub struct RestartOutcome {
    #[allow(dead_code)]
    pub running: bool,
    #[allow(dead_code)]
    pub restarted: bool,
    pub message: String,
}

/// 优雅重启 cc-switch GUI：发现进程→SIGTERM→等待退出→重新启动。
/// 没有运行则原样返回；找不到可执行路径则只杀不启。
pub fn restart_cc_switch_if_running() -> AppResult<RestartOutcome> {
    let pids = pids_of("cc-switch");
    if pids.is_empty() {
        return Ok(RestartOutcome {
            running: false,
            restarted: false,
            message: "cc-switch 当前未运行，无需重启".to_string(),
        });
    }

    // 抓住第一个进程的 exe 路径用于重新拉起
    let exe = pids.iter().find_map(|p| exe_of(*p));
    tracing::info!(?pids, ?exe, "kill cc-switch for hot reload");

    // 1) SIGTERM / Stop-Process（优雅）
    #[cfg(unix)]
    {
        for pid in &pids {
            unsafe {
                libc_kill(*pid as i32, 15);
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        for pid in &pids {
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T"])
                .output();
        }
    }

    // 2) 等待退出（最多 ~3s）
    for _ in 0..30 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if pids_of("cc-switch").is_empty() {
            break;
        }
    }
    // 还活着就强杀
    #[cfg(unix)]
    {
        let still = pids_of("cc-switch");
        if !still.is_empty() {
            for pid in &still {
                unsafe {
                    libc_kill(*pid as i32, 9);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
    }
    #[cfg(target_os = "windows")]
    {
        let still = pids_of("cc-switch");
        for pid in &still {
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string(), "/T"])
                .output();
        }
    }

    // 3) 重新拉起
    let exe = match exe {
        Some(p) => p,
        None => {
            return Ok(RestartOutcome {
                running: true,
                restarted: false,
                message: "已结束 cc-switch 进程，但无法解析可执行路径，未自动启动；请手动启动 cc-switch".to_string(),
            });
        }
    };
    let spawn = Command::new(&exe)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    match spawn {
        Ok(_) => Ok(RestartOutcome {
            running: true,
            restarted: true,
            message: format!("已重启 cc-switch（{}）", exe),
        }),
        Err(e) => Err(AppError::CcSwitch(format!("重启 cc-switch 失败: {}", e))),
    }
}

// ---- 极轻量级 libc::kill 绑定，避免引入 libc crate ----

#[cfg(unix)]
extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

#[cfg(unix)]
#[allow(non_snake_case)]
unsafe fn libc_kill(pid: i32, sig: i32) {
    let _ = kill(pid, sig);
}
