//! 安全的 Git 子进程调用层：环境隔离、stdin 写入与输出读取。
use std::env;
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use wait_timeout::ChildExt;

pub(crate) fn discover_git(explicit: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(path) = explicit {
        return validate_git(path);
    }
    let candidates: Vec<PathBuf> = if cfg!(windows) {
        vec![
            PathBuf::from(r"C:\Program Files\Git\cmd\git.exe"),
            PathBuf::from(r"C:\Program Files\Git\bin\git.exe"),
        ]
    } else {
        vec![
            PathBuf::from("/usr/bin/git"),
            PathBuf::from("/usr/local/bin/git"),
            PathBuf::from("/opt/homebrew/bin/git"),
        ]
    };
    for candidate in candidates {
        if candidate.is_file() {
            return validate_git(&candidate);
        }
    }
    if let Some(path) = env::var_os("PATH") {
        let executable = if cfg!(windows) { "git.exe" } else { "git" };
        for directory in env::split_paths(&path) {
            let candidate = directory.join(executable);
            if candidate.is_file() {
                return validate_git(&candidate);
            }
        }
    }
    Err("Git executable not found".into())
}

fn validate_git(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() || !path.is_file() {
        return Err(format!("invalid Git executable: {}", path.display()));
    }
    path.canonicalize().map_err(|e| e.to_string())
}

pub(crate) fn run_git(
    git: &Path,
    repo: &Path,
    args: Vec<OsString>,
    input: Option<&[u8]>,
    output_limit: usize,
    timeout: Option<Duration>,
) -> Result<Vec<u8>, String> {
    let mut command = Command::new(git);
    command
        .arg("-C")
        .arg(repo)
        .args(args)
        .env_clear()
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for name in [
        "PATH",
        "HOME",
        "USERPROFILE",
        "SystemRoot",
        "SystemDrive",
        "TEMP",
        "TMP",
        "SSH_AUTH_SOCK",
    ] {
        if let Some(value) = env::var_os(name) {
            command.env(name, value);
        }
    }
    command
        .env("LC_ALL", "C")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_PAGER", "cat")
        .env("GIT_EDITOR", "true")
        // 隔离 Git 配置：不可信仓库不能通过系统级/全局级配置执行外部程序；
        // 仓库本地 core.fsmonitor（git status 可能拉起的进程）由环境级配置覆盖
        // （环境级优先于仓库本地配置）。GIT_CONFIG_GLOBAL 用 /dev/null，
        // 因为 Windows 版 Git 拒绝 NUL 作为该变量的值。
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "core.fsmonitor")
        .env("GIT_CONFIG_VALUE_0", "false");

    let mut child = command.spawn().map_err(|e| e.to_string())?;
    if let Some(input) = input {
        use std::io::Write;
        child
            .stdin
            .take()
            .ok_or("failed to open Git stdin")?
            .write_all(input)
            .map_err(|e| e.to_string())?;
    }

    let mut stdout = child.stdout.take().ok_or("failed to capture Git stdout")?;
    let mut stderr = child.stderr.take().ok_or("failed to capture Git stderr")?;
    let stdout_task = thread::spawn(move || read_capped(&mut stdout, output_limit));
    let stderr_task = thread::spawn(move || read_capped(&mut stderr, 64 * 1024));

    let status = if let Some(timeout) = timeout {
        match child.wait_timeout(timeout).map_err(|e| e.to_string())? {
            Some(status) => status,
            None => {
                child.kill().map_err(|e| e.to_string())?;
                let _ = child.wait();
                let _ = stdout_task.join();
                let _ = stderr_task.join();
                return Err("Git read operation timed out".into());
            }
        }
    } else {
        child.wait().map_err(|e| e.to_string())?
    };

    let (stdout, stdout_truncated) = stdout_task.join().map_err(|_| "stdout reader failed")??;
    let (stderr, stderr_truncated) = stderr_task.join().map_err(|_| "stderr reader failed")??;
    if stdout_truncated || stderr_truncated {
        return Err("Git output exceeded the configured limit".into());
    }
    if !status.success() {
        return Err(format!(
            "Git exited with {status}: {}",
            sanitize_stderr(&String::from_utf8_lossy(&stderr))
        ));
    }
    Ok(stdout)
}

fn read_capped(reader: &mut impl Read, limit: usize) -> Result<(Vec<u8>, bool), String> {
    let mut result = Vec::new();
    let mut buffer = [0u8; 8192];
    let mut truncated = false;
    loop {
        let count = reader.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        let remaining = limit.saturating_sub(result.len());
        if remaining > 0 {
            result.extend_from_slice(&buffer[..count.min(remaining)]);
        }
        if count > remaining {
            truncated = true;
        }
    }
    Ok((result, truncated))
}

fn sanitize_stderr(stderr: &str) -> String {
    stderr
        .split_whitespace()
        .map(|token| {
            if let Some(scheme_end) = token.find("://") {
                let authority_start = scheme_end + 3;
                if let Some(at) = token[authority_start..].find('@') {
                    let at = authority_start + at;
                    if token[authority_start..at].contains(':') {
                        return format!("{}***{}", &token[..authority_start], &token[at..]);
                    }
                }
            }
            token.to_string()
        })
        .collect::<Vec<_>>()
        .join(" ")
}
