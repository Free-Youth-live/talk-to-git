//! 核心诊断工具：在真实仓库上验证 ttg-core 的状态、日志、Diff 与 Hunk 暂存能力。
use std::path::PathBuf;
use ttg_core::GitRepository;

fn main() {
    if let Err(error) = run() {
        eprintln!("core probe failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut repo_path = None;
    let mut git_path = None;
    let mut action = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--repo" => repo_path = Some(PathBuf::from(args.next().ok_or("--repo needs a path")?)),
            "--git" => git_path = Some(PathBuf::from(args.next().ok_or("--git needs a path")?)),
            "--status-json" => action = Some(("status-json".to_string(), String::new())),
            "--log" => {
                action = Some(("log".to_string(), args.next().ok_or("--log needs a limit")?))
            }
            "--diff" => action = Some(("diff".to_string(), String::new())),
            "--stage-hunk" => {
                let index = args.next().ok_or("--stage-hunk needs an index")?;
                let header = args.next().ok_or("--stage-hunk needs a header")?;
                action = Some(("stage-hunk".to_string(), format!("{index}\n{header}")));
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }
    let repo_path = repo_path.ok_or("--repo is required")?;
    let action = action.ok_or("an action is required")?;
    let repo = GitRepository::open(&repo_path, git_path.as_deref())?;
    match action.0.as_str() {
        "status-json" => println!("{}", repo.status_json()?),
        "log" => {
            let limit = action.1.parse().map_err(|_| "invalid log limit")?;
            let commits = repo.log(limit)?;
            println!(
                "commits={} first={} last={}",
                commits.len(),
                commits.first().map(|c| c.subject.as_str()).unwrap_or(""),
                commits.last().map(|c| c.subject.as_str()).unwrap_or("")
            );
        }
        "diff" => {
            let document = repo.diff()?;
            println!(
                "files={} lines={} hunks={}",
                document.file_count,
                document.lines.len(),
                document.hunks.len()
            );
            for hunk in &document.hunks {
                println!("hunk={} {}", hunk.index, hunk.header);
            }
        }
        "stage-hunk" => {
            let (index_text, header) = action.1.split_once('\n').ok_or("missing Hunk header")?;
            let index = index_text.parse().map_err(|_| "invalid Hunk index")?;
            repo.stage_hunk(index, header)?;
            println!("Hunk staged");
        }
        _ => return Err("unsupported action".into()),
    }
    Ok(())
}
