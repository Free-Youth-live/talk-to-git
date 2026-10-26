//! 可再生的测试仓库生成器：标准/大差异/大历史/恶意/锁文件五类样本。
use std::env;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const MARKER: &str = ".ttg-fixtures";

fn main() {
    if let Err(error) = run() {
        eprintln!("fixture generation failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut force = false;
    let mut commits = 50_000u32;
    let mut git_path = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--force" => force = true,
            "--git" => git_path = Some(PathBuf::from(args.next().ok_or("--git needs a path")?)),
            "--commits" => {
                let value = args.next().ok_or("--commits needs a number")?;
                commits = value.parse().map_err(|_| "invalid --commits value")?;
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    let git = Git::discover(git_path)?;
    let root = env::current_dir().map_err(|e| e.to_string())?;
    let output = root.join("fixtures").join("generated");
    prepare_output(&output, force)?;

    create_standard(&git, &output)?;
    create_large_diff(&git, &output)?;
    create_history(&git, &output, commits)?;
    create_malicious(&git, &output)?;
    create_locked(&git, &output)?;

    let version = git.version()?;
    let manifest = format!(
        "fixture-format=1\ngit={version}\nhistory-commits={commits}\nlarge-source-target-bytes={}\n",
        12 * 1024 * 1024
    );
    atomic_write(&output.join("MANIFEST.txt"), manifest.as_bytes())?;
    println!("fixtures created at {}", output.display());
    Ok(())
}

struct Git {
    path: PathBuf,
}

impl Git {
    fn discover(explicit: Option<PathBuf>) -> Result<Self, String> {
        if let Some(path) = explicit {
            return Self::validate(path);
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
                return Self::validate(candidate);
            }
        }

        if let Some(path) = env::var_os("PATH") {
            let executable = if cfg!(windows) { "git.exe" } else { "git" };
            for directory in env::split_paths(&path) {
                let candidate = directory.join(executable);
                if candidate.is_file() {
                    return Self::validate(candidate);
                }
            }
        }
        Err("Git executable not found; pass --git with an absolute path".into())
    }

    fn validate(path: PathBuf) -> Result<Self, String> {
        if !path.is_absolute() || !path.is_file() {
            return Err(format!("invalid Git executable: {}", path.display()));
        }
        let path = path.canonicalize().map_err(|e| e.to_string())?;
        Ok(Self { path })
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.path);
        command.env_clear();
        for name in ["PATH", "HOME", "USERPROFILE", "SystemRoot", "TEMP", "TMP"] {
            if let Some(value) = env::var_os(name) {
                command.env(name, value);
            }
        }
        command
            .env("LC_ALL", "C")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_PAGER", "cat")
            .env("GIT_EDITOR", "true")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null");
        command
    }

    fn run(&self, repo: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
        let args = args.iter().map(OsString::from).collect();
        self.run_vec(Some(repo), args)
    }

    fn run_vec(&self, repo: Option<&Path>, args: Vec<OsString>) -> Result<Vec<u8>, String> {
        let output = self.output(repo, args, None)?;
        Ok(output)
    }

    fn stdin(&self, repo: &Path, args: &[&str], input: &[u8]) -> Result<(), String> {
        let args = args.iter().map(OsString::from).collect();
        self.output(Some(repo), args, Some(input)).map(|_| ())
    }

    fn output(
        &self,
        repo: Option<&Path>,
        args: Vec<OsString>,
        input: Option<&[u8]>,
    ) -> Result<Vec<u8>, String> {
        let mut command = self.command();
        if let Some(repo) = repo {
            command.arg("-C").arg(repo);
        }
        command.args(args);
        if let Some(input) = input {
            let mut child = command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| e.to_string())?;
            child
                .stdin
                .take()
                .ok_or("failed to open Git stdin")?
                .write_all(input)
                .map_err(|e| e.to_string())?;
            let output = child.wait_with_output().map_err(|e| e.to_string())?;
            return check_output(output);
        }
        let output = command.output().map_err(|e| e.to_string())?;
        check_output(output)
    }

    fn version(&self) -> Result<String, String> {
        let output = self.run_vec(None, vec![OsString::from("--version")])?;
        Ok(String::from_utf8_lossy(&output).trim().to_string())
    }
}

fn check_output(output: std::process::Output) -> Result<Vec<u8>, String> {
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(format!(
            "Git exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn prepare_output(output: &Path, force: bool) -> Result<(), String> {
    if output.exists() {
        if !force {
            return Err(format!(
                "{} already exists; pass --force to regenerate",
                output.display()
            ));
        }
        if !output.join(MARKER).is_file() {
            return Err(format!(
                "refusing to replace unmarked directory: {}",
                output.display()
            ));
        }
        fs::remove_dir_all(output).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(output).map_err(|e| e.to_string())?;
    atomic_write(&output.join(MARKER), b"generated fixtures only\n")
}

fn init_repo(git: &Git, path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| e.to_string())?;
    git.run(path, &["init", "--initial-branch=main"])?;
    for (key, value) in [
        ("user.name", "Fixture User"),
        ("user.email", "fixture@example.invalid"),
        ("core.autocrlf", "false"),
    ] {
        git.run(path, &["config", "--local", key, value])?;
    }
    Ok(())
}

fn commit_all(git: &Git, repo: &Path, message: &str) -> Result<(), String> {
    git.run(repo, &["add", "--", "."])?;
    git.stdin(repo, &["commit", "-F", "-"], message.as_bytes())
}

fn create_standard(git: &Git, output: &Path) -> Result<(), String> {
    let repo = output.join("standard");
    let remote = output.join("remotes").join("standard-remote.git");
    init_repo(git, &repo)?;
    fs::create_dir_all(&remote).map_err(|e| e.to_string())?;
    git.run(&remote, &["init", "--bare"])?;

    atomic_write(
        &repo.join("README.md"),
        b"# Standard fixture\n\nReview this repository.\n",
    )?;
    atomic_write(
        &repo.join("file.txt"),
        b"alpha\nbeta\ngamma\ndelta\nepsilon\n",
    )?;
    commit_all(git, &repo, "initial commit\n")?;

    let mut command = git.command();
    let output = command
        .arg("-C")
        .arg(&repo)
        .arg("remote")
        .arg("add")
        .arg("origin")
        .arg(&remote)
        .output()
        .map_err(|e| e.to_string())?;
    check_output(output)?;
    git.run(&repo, &["push", "-u", "origin", "main"])?;

    git.run(&repo, &["checkout", "-b", "feature/review"])?;
    atomic_write(&repo.join("review.txt"), b"review one\nreview two\n")?;
    commit_all(git, &repo, "add review notes\n")?;
    git.run(&repo, &["push", "-u", "origin", "feature/review"])?;
    git.run(&repo, &["tag", "-a", "v0.1.0", "-m", "fixture tag"])?;
    git.run(&repo, &["push", "origin", "v0.1.0"])?;
    git.run(&repo, &["checkout", "main"])?;

    atomic_write(&repo.join("ahead.txt"), b"local ahead commit\n")?;
    commit_all(git, &repo, "local commit ahead of origin\n")?;
    atomic_write(
        &repo.join("README.md"),
        b"# Standard fixture\n\nStaged change.\n",
    )?;
    git.run(&repo, &["add", "--", "README.md"])?;
    atomic_write(
        &repo.join("file.txt"),
        b"alpha\nbeta changed\ngamma\ndelta\nepsilon\n",
    )?;
    atomic_write(&repo.join("untracked.txt"), b"not tracked\n")?;
    Ok(())
}

fn create_large_diff(git: &Git, output: &Path) -> Result<(), String> {
    let repo = output.join("large-diff");
    init_repo(git, &repo)?;
    let target = 12 * 1024 * 1024;
    let mut base = String::new();
    let mut line_number = 0u64;
    while base.len() < target {
        line_number += 1;
        base.push_str(&format!(
            "{line_number:07} deterministic fixture content for a large text diff payload\n"
        ));
    }

    atomic_write(&repo.join("large.txt"), base.as_bytes())?;
    commit_all(git, &repo, "add large baseline\n")?;

    let mut modified = String::with_capacity(base.len() + base.len() / 10);
    for (index, line) in base.lines().enumerate() {
        if (index / 1000) % 2 == 0 {
            modified.push_str(line);
            modified.push_str(" changed");
        } else {
            modified.push_str(line);
        }
        modified.push('\n');
    }
    atomic_write(&repo.join("large.txt"), modified.as_bytes())?;
    Ok(())
}

fn create_history(git: &Git, output: &Path, commits: u32) -> Result<(), String> {
    let repo = output.join("history");
    init_repo(git, &repo)?;
    let mut child = git
        .command()
        .arg("-C")
        .arg(&repo)
        .arg("fast-import")
        .arg("--quiet")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or("failed to open fast-import stdin")?;
    let content = b"history fixture\n";
    writeln!(stdin, "blob").map_err(|e| e.to_string())?;
    writeln!(stdin, "mark :1").map_err(|e| e.to_string())?;
    writeln!(stdin, "data {}", content.len()).map_err(|e| e.to_string())?;
    stdin.write_all(content).map_err(|e| e.to_string())?;
    writeln!(stdin).map_err(|e| e.to_string())?;

    for index in 1..=commits {
        let message = format!("history commit {index:05}\n");
        writeln!(stdin, "commit refs/heads/main").map_err(|e| e.to_string())?;
        writeln!(stdin, "mark :{}", index + 1).map_err(|e| e.to_string())?;
        writeln!(
            stdin,
            "author Fixture User <fixture@example.invalid> 1700000000 +0000"
        )
        .map_err(|e| e.to_string())?;
        writeln!(
            stdin,
            "committer Fixture User <fixture@example.invalid> 1700000000 +0000"
        )
        .map_err(|e| e.to_string())?;
        writeln!(stdin, "data {}", message.len()).map_err(|e| e.to_string())?;
        stdin
            .write_all(message.as_bytes())
            .map_err(|e| e.to_string())?;
        if index > 1 {
            writeln!(stdin, "from :{index}").map_err(|e| e.to_string())?;
        }
        writeln!(stdin, "M 100644 :1 history.txt").map_err(|e| e.to_string())?;
        writeln!(stdin).map_err(|e| e.to_string())?;
    }
    drop(stdin);
    check_output(child.wait_with_output().map_err(|e| e.to_string())?)?;
    git.run(
        &repo,
        &[
            "restore",
            "--source=HEAD",
            "--staged",
            "--worktree",
            "--",
            "history.txt",
        ],
    )?;
    Ok(())
}

fn create_malicious(git: &Git, output: &Path) -> Result<(), String> {
    let repo = output.join("malicious");
    init_repo(git, &repo)?;
    atomic_write(
        &repo.join("payload.html"),
        b"<script>alert('must not execute')</script>\n",
    )?;
    atomic_write(
        &repo.join("payload.svg"),
        b"<svg onload=\"alert('must not execute')\"></svg>\n",
    )?;
    atomic_write(
        &repo.join("bidi.txt"),
        "normal \u{202e}reversed\u{202c} text\n".as_bytes(),
    )?;
    atomic_write(&repo.join("中文 空格 😀.txt"), "中文路径内容\n".as_bytes())?;
    atomic_write(
        &repo.join("-leading-dash.txt"),
        b"must not be parsed as an option\n",
    )?;
    atomic_write(
        &repo.join("long-line.txt"),
        format!("{}\n", "x".repeat(100_000)).as_bytes(),
    )?;
    commit_all(
        git,
        &repo,
        "malicious <script>alert('message')</script> \u{202e}subject\n",
    )?;
    git.run(&repo, &["checkout", "--detach", "HEAD"])?;
    Ok(())
}

fn create_locked(git: &Git, output: &Path) -> Result<(), String> {
    let repo = output.join("locked");
    init_repo(git, &repo)?;
    atomic_write(&repo.join("file.txt"), b"locked fixture\n")?;
    commit_all(git, &repo, "initial locked fixture\n")?;
    atomic_write(
        &repo.join(".git").join("index.lock"),
        b"fixture lock; do not delete automatically\n",
    )?;
    Ok(())
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("path has no parent")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let name = path
        .file_name()
        .ok_or("path has no filename")?
        .to_string_lossy();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let temporary = parent.join(format!(".{name}.{}.{}.tmp", std::process::id(), nonce));
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        file.write_all(contents).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
    }
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    fs::rename(&temporary, path).map_err(|e| e.to_string())
}

#[allow(dead_code)]
fn read_to_string(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut value = String::new();
    file.read_to_string(&mut value).map_err(|e| e.to_string())?;
    Ok(value)
}
