//! TTG 共享 Git 核心：安全的 Git 调用、状态/日志/Diff 解析、Hunk 暂存与 AI JSON 输出。
mod diff;
mod process;

pub use diff::{DiffDocument, DiffHunk, DiffLine, DiffLineKind};
use serde::Serialize;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

const READ_TIMEOUT: Duration = Duration::from_secs(30);
const STATUS_LIMIT: usize = 8 * 1024 * 1024;
const DIFF_LIMIT: usize = 64 * 1024 * 1024;
const LOG_LIMIT: usize = 64 * 1024 * 1024;

#[derive(Clone, Debug, Default, Serialize)]
pub struct BranchInfo {
    pub head: String,
    pub upstream: Option<String>,
    pub ahead: Option<u32>,
    pub behind: Option<u32>,
    pub detached: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct StatusEntry {
    pub kind: String,
    pub path: String,
    pub raw: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RepositoryStatus {
    pub branch: BranchInfo,
    pub entries: Vec<StatusEntry>,
    pub dirty: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct CommitSummary {
    pub id: String,
    pub short_id: String,
    pub author: String,
    pub date: String,
    pub subject: String,
}

pub struct GitRepository {
    root: PathBuf,
    git: PathBuf,
    write_lock: Mutex<()>,
}

impl GitRepository {
    pub fn open(path: &Path, explicit_git: Option<&Path>) -> Result<Self, String> {
        let root = path.canonicalize().map_err(|e| e.to_string())?;
        if !root.is_dir() {
            return Err("Repository path is not a directory".into());
        }
        let git = process::discover_git(explicit_git)?;
        let output = process::run_git(
            &git,
            &root,
            strings(&["--no-pager", "rev-parse", "--show-toplevel"]),
            None,
            1024 * 1024,
            Some(READ_TIMEOUT),
        )?;
        let top_level = String::from_utf8(output)
            .map_err(|_| "Git returned a non-UTF-8 repository root")?
            .trim()
            .to_string();
        if top_level.is_empty() {
            return Err("Not a Git repository".into());
        }
        let canonical_top = PathBuf::from(top_level)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if canonical_top != root {
            return Err(format!(
                "Open the repository root instead: {}",
                canonical_top.display()
            ));
        }
        Ok(Self {
            root,
            git,
            write_lock: Mutex::new(()),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn git_path(&self) -> &Path {
        &self.git
    }

    pub fn status(&self) -> Result<RepositoryStatus, String> {
        let output = self.run(
            strings(&[
                "--no-pager",
                "--literal-pathspecs",
                "status",
                "--porcelain=v2",
                "-z",
                "--branch",
                "--ignore-submodules=all",
            ]),
            None,
            STATUS_LIMIT,
            Some(READ_TIMEOUT),
        )?;
        parse_status(&output)
    }

    pub fn status_json(&self) -> Result<String, String> {
        serde_json::to_string(&self.status()?).map_err(|e| e.to_string())
    }

    pub fn log(&self, limit: usize) -> Result<Vec<CommitSummary>, String> {
        if limit == 0 || limit > 50_000 {
            return Err("Log limit must be between 1 and 50000".into());
        }
        let limit_arg = format!("--max-count={limit}");
        let output = self.run(
            strings(&[
                "--no-pager",
                "log",
                &limit_arg,
                "--date=iso-strict",
                "--format=%H%x00%h%x00%an%x00%ad%x00%s%x00",
            ]),
            None,
            LOG_LIMIT,
            Some(READ_TIMEOUT),
        )?;
        parse_log(&output)
    }

    pub fn diff(&self) -> Result<DiffDocument, String> {
        let output = self.run(
            strings(&[
                "--no-pager",
                "--literal-pathspecs",
                "diff",
                "--no-color",
                "--no-ext-diff",
                "--no-textconv",
                "--ignore-submodules=all",
                "--unified=3",
                "--",
            ]),
            None,
            DIFF_LIMIT,
            Some(READ_TIMEOUT),
        )?;
        Ok(diff::parse_diff(output))
    }

    pub fn stage_hunk(&self, index: usize, expected_header: &str) -> Result<(), String> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| "Repository write lock is unavailable")?;
        let document = self.diff()?;
        let patch = document.patch_for_hunk(index, expected_header)?;
        self.run(
            strings(&[
                "--no-pager",
                "--literal-pathspecs",
                "apply",
                "--cached",
                "--whitespace=nowarn",
                "-",
            ]),
            Some(&patch),
            1024 * 1024,
            None,
        )?;
        Ok(())
    }

    fn run(
        &self,
        args: Vec<OsString>,
        input: Option<&[u8]>,
        output_limit: usize,
        timeout: Option<Duration>,
    ) -> Result<Vec<u8>, String> {
        process::run_git(&self.git, &self.root, args, input, output_limit, timeout)
    }
}

fn strings(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}

fn parse_status(output: &[u8]) -> Result<RepositoryStatus, String> {
    let records: Vec<&[u8]> = output
        .split(|byte| *byte == 0)
        .filter(|r| !r.is_empty())
        .collect();
    let mut branch = BranchInfo::default();
    let mut entries = Vec::new();
    let mut index = 0usize;
    while index < records.len() {
        let record = records[index];
        if record.starts_with(b"# ") {
            parse_branch_header(record, &mut branch)?;
        } else {
            let kind = record.first().copied().ok_or("empty status record")?;
            entries.push(parse_status_entry(record)?);
            if kind == b'2' {
                index += 1;
            }
        }
        index += 1;
    }
    Ok(RepositoryStatus {
        dirty: !entries.is_empty(),
        branch,
        entries,
    })
}

fn parse_branch_header(record: &[u8], branch: &mut BranchInfo) -> Result<(), String> {
    let text = String::from_utf8(record.to_vec()).map_err(|_| "invalid branch header encoding")?;
    let text = text.trim_end_matches(['\r', '\n']);
    if let Some(value) = text.strip_prefix("# branch.head ") {
        branch.head = value.to_string();
        branch.detached = value == "(detached)";
    } else if let Some(value) = text.strip_prefix("# branch.upstream ") {
        branch.upstream = Some(value.to_string());
    } else if let Some(value) = text.strip_prefix("# branch.ab ") {
        let mut parts = value.split(' ');
        branch.ahead = Some(
            parts
                .next()
                .and_then(|v| v.strip_prefix('+'))
                .ok_or("invalid ahead count")?
                .parse()
                .map_err(|_| "invalid ahead count")?,
        );
        branch.behind = Some(
            parts
                .next()
                .and_then(|v| v.strip_prefix('-'))
                .ok_or("invalid behind count")?
                .parse()
                .map_err(|_| "invalid behind count")?,
        );
    }
    Ok(())
}

fn parse_status_entry(record: &[u8]) -> Result<StatusEntry, String> {
    let raw = String::from_utf8_lossy(record).to_string();
    let kind = record.first().copied().ok_or("empty status record")?;
    let path = match kind {
        b'?' | b'!' => raw
            .get(2..)
            .filter(|_| raw.len() >= 2)
            .ok_or("invalid untracked record")?
            .to_string(),
        b'1' => raw
            .splitn(9, ' ')
            .nth(8)
            .ok_or("invalid ordinary status record")?
            .to_string(),
        b'2' => raw
            .splitn(10, ' ')
            .nth(9)
            .ok_or("invalid rename status record")?
            .to_string(),
        b'u' => raw
            .splitn(11, ' ')
            .nth(10)
            .ok_or("invalid unmerged status record")?
            .to_string(),
        _ => return Err("unsupported porcelain v2 record".into()),
    };
    let kind = match kind {
        b'1' => "ordinary",
        b'2' => "renamed",
        b'u' => "unmerged",
        b'?' => "untracked",
        b'!' => "ignored",
        _ => "other",
    };
    Ok(StatusEntry {
        kind: kind.into(),
        path,
        raw,
    })
}

fn parse_log(output: &[u8]) -> Result<Vec<CommitSummary>, String> {
    let mut commits = Vec::new();
    for record in output
        .split(|byte| *byte == b'\n')
        .filter(|r| !r.is_empty())
    {
        let fields: Vec<&[u8]> = record.split(|byte| *byte == 0).collect();
        if fields.len() < 5 {
            return Err("invalid structured Git log record".into());
        }
        commits.push(CommitSummary {
            id: ascii(fields[0], "commit id")?,
            short_id: ascii(fields[1], "short commit id")?,
            author: String::from_utf8_lossy(fields[2]).to_string(),
            date: ascii(fields[3], "commit date")?,
            subject: String::from_utf8_lossy(fields[4]).to_string(),
        });
    }
    Ok(commits)
}

fn ascii(value: &[u8], label: &str) -> Result<String, String> {
    if value.iter().all(|byte| byte.is_ascii()) {
        Ok(String::from_utf8_lossy(value).to_string())
    } else {
        Err(format!("{label} is not ASCII"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_branch_and_nul_status_records() {
        let status = parse_status(
            b"# branch.head main\0# branch.upstream origin/main\0# branch.ab +2 -1\x001 M. N... 100644 100644 100644 aaaaaaa bbbbbbb file with spaces.txt\0? \xe4\xb8\xad\xe6\x96\x87.txt\0",
        )
        .unwrap();
        assert_eq!(status.branch.head, "main");
        assert_eq!(status.branch.ahead, Some(2));
        assert_eq!(status.branch.behind, Some(1));
        assert_eq!(status.entries.len(), 2);
        assert_eq!(status.entries[0].path, "file with spaces.txt");
        assert!(status.dirty);
    }

    #[test]
    fn rejects_short_log_records() {
        assert!(parse_log(b"abc\n").is_err());
    }

    #[test]
    fn status_json_escapes_repository_content() {
        let status = RepositoryStatus {
            branch: BranchInfo {
                head: "main\"script".into(),
                ..BranchInfo::default()
            },
            entries: vec![StatusEntry {
                kind: "untracked".into(),
                path: "<script>alert(1)</script>".into(),
                raw: String::new(),
            }],
            dirty: true,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("main\\\"script"));
        assert!(json.contains("<script>"));
    }
}
