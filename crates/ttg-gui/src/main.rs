//! Talk To Git 的 egui 图形界面（阶段 0 原型）。
//!
//! 界面文案一律经 ttg-i18n 输出（默认中文，可切换英文）；源码中不出现
//! 用户可见的硬编码文案（哨兵测试强制）。CJK 字形按系统字体候选链加载。
use eframe::egui;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use ttg_core::{CommitSummary, DiffDocument, DiffLineKind, GitRepository, RepositoryStatus};
use ttg_i18n::I18n;

fn main() {
    if let Err(error) = run() {
        eprintln!("egui prototype failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let started = Instant::now();
    let mut repo_path = None;
    let mut git_path = None;
    let mut ai_status = false;
    let mut benchmark_startup = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--repo" => repo_path = Some(PathBuf::from(args.next().ok_or("--repo needs a path")?)),
            "--git" => git_path = Some(PathBuf::from(args.next().ok_or("--git needs a path")?)),
            "--ai-status" => ai_status = true,
            "--benchmark-startup" => benchmark_startup = true,
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }
    let repo_path = repo_path.ok_or("--repo is required")?;
    let repo = Arc::new(GitRepository::open(&repo_path, git_path.as_deref())?);
    if ai_status {
        println!("{}", repo.status_json()?);
        return Ok(());
    }
    let i18n = I18n::new()?;
    let title = i18n.text("window-title");
    let app = PrototypeApp::new(repo, started, benchmark_startup, i18n)?;
    eframe::run_native(
        &title,
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::new(app))),
    )
    .map_err(|e| e.to_string())
}

/// 按系统字体候选链加载 CJK 字形，作为默认字体的后备；
/// 全部未命中时保持英文渲染可用（中文将显示为缺字）。
fn install_cjk_font(ctx: &egui::Context) {
    const CANDIDATES: &[&str] = &[
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\simhei.ttf",
        "C:\\Windows\\Fonts\\simsun.ttc",
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
    ];
    for path in CANDIDATES {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts
                .font_data
                .insert("cjk".to_owned(), egui::FontData::from_owned(bytes).into());
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts
                    .families
                    .entry(family)
                    .or_default()
                    .push("cjk".to_owned());
            }
            ctx.set_fonts(fonts);
            return;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Status,
    Diff,
    History,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Density {
    Comfortable,
    Compact,
}

impl Density {
    fn row_height(self) -> f32 {
        match self {
            Self::Comfortable => 28.0,
            Self::Compact => 19.0,
        }
    }

    fn label_key(self) -> &'static str {
        match self {
            Self::Comfortable => "density-comfortable",
            Self::Compact => "density-compact",
        }
    }
}

struct PrototypeApp {
    repo: Arc<GitRepository>,
    i18n: I18n,
    fonts_ready: bool,
    tab: Tab,
    density: Density,
    status: RepositoryStatus,
    diff: DiffDocument,
    commits: Vec<CommitSummary>,
    draft: String,
    error: Option<String>,
    started: Instant,
    benchmark_startup: bool,
}

impl PrototypeApp {
    fn new(
        repo: Arc<GitRepository>,
        started: Instant,
        benchmark_startup: bool,
        i18n: I18n,
    ) -> Result<Self, String> {
        let status = repo.status()?;
        let diff = repo.diff()?;
        let commits = repo.log(50_000)?;
        Ok(Self {
            repo,
            i18n,
            fonts_ready: false,
            tab: Tab::Status,
            density: Density::Comfortable,
            status,
            diff,
            commits,
            draft: String::new(),
            error: None,
            started,
            benchmark_startup,
        })
    }

    fn refresh(&mut self) {
        match self.reload() {
            Ok(()) => self.error = None,
            Err(error) => self.error = Some(error),
        }
    }

    fn reload(&mut self) -> Result<(), String> {
        self.status = self.repo.status()?;
        self.diff = self.repo.diff()?;
        self.commits = self.repo.log(50_000)?;
        Ok(())
    }
}

impl eframe::App for PrototypeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if !self.fonts_ready {
            install_cjk_font(ui.ctx());
            self.fonts_ready = true;
        }
        ui.style_mut().spacing.item_spacing.y = if self.density == Density::Comfortable {
            8.0
        } else {
            3.0
        };
        ui.horizontal(|ui| {
            ui.heading(self.i18n.text("heading-stage"));
            ui.separator();
            ui.label(
                self.i18n
                    .text_plural("commits-count", self.commits.len() as u64),
            );
            if ui.button(self.i18n.text("refresh")).clicked() {
                self.refresh();
            }
            if ui
                .button(self.i18n.text(self.density.label_key()))
                .clicked()
            {
                self.density = if self.density == Density::Comfortable {
                    Density::Compact
                } else {
                    Density::Comfortable
                };
            }
            if ui.button(self.i18n.text("language-switch")).clicked() {
                self.i18n.toggle_language();
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Title(
                    self.i18n.text("window-title"),
                ));
            }
        });
        let branch = &self.status.branch;
        let ahead = branch
            .ahead
            .map(|v| v.to_string())
            .unwrap_or_else(|| "?".to_owned());
        let behind = branch
            .behind
            .map(|v| v.to_string())
            .unwrap_or_else(|| "?".to_owned());
        let upstream = branch
            .upstream
            .clone()
            .unwrap_or_else(|| self.i18n.text("branch-none"));
        ui.label(self.i18n.text_with(
            "branch-line",
            &[
                ("head", branch.head.as_str()),
                ("upstream", upstream.as_str()),
                ("ahead", ahead.as_str()),
                ("behind", behind.as_str()),
            ],
        ));
        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(self.repo.root().display().to_string());
            ui.separator();
            ui.selectable_value(&mut self.tab, Tab::Status, self.i18n.text("tab-changes"));
            ui.selectable_value(&mut self.tab, Tab::Diff, self.i18n.text("tab-diff"));
            ui.selectable_value(&mut self.tab, Tab::History, self.i18n.text("tab-history"));
            ui.separator();
            ui.label(self.i18n.text("ai-local-json"));
        });
        ui.separator();
        match self.tab {
            Tab::Status => self.show_status(ui),
            Tab::Diff => self.show_diff(ui),
            Tab::History => self.show_history(ui),
        }
        if self.benchmark_startup {
            println!("READY_MS={}", self.started.elapsed().as_millis());
            std::process::exit(0);
        }
    }
}

impl PrototypeApp {
    fn show_status(&mut self, ui: &mut egui::Ui) {
        let count = self.status.entries.len().to_string();
        ui.heading(
            self.i18n
                .text_with("changes-count", &[("count", count.as_str())]),
        );
        for entry in &self.status.entries {
            ui.horizontal(|ui| {
                ui.label(&entry.kind);
                ui.monospace(&entry.path);
            });
        }
        if self.status.entries.is_empty() {
            ui.label(self.i18n.text("working-tree-clean"));
        }
        ui.separator();
        ui.label(self.i18n.text("commit-draft"));
        ui.text_edit_multiline(&mut self.draft);
    }

    fn show_history(&self, ui: &mut egui::Ui) {
        let count = self.commits.len().to_string();
        ui.heading(
            self.i18n
                .text_with("history-count", &[("count", count.as_str())]),
        );
        let row_height = self.density.row_height();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show_rows(ui, row_height, self.commits.len(), |ui, rows| {
                for index in rows {
                    let commit = &self.commits[index];
                    ui.horizontal(|ui| {
                        ui.monospace(&commit.short_id);
                        ui.label(&commit.subject);
                        ui.weak(&commit.author);
                    });
                }
            });
    }

    fn show_diff(&mut self, ui: &mut egui::Ui) {
        let summary = self.i18n.text_with(
            "diff-summary",
            &[
                ("files", &self.diff.file_count.to_string()),
                ("lines", &self.diff.lines.len().to_string()),
                ("hunks", &self.diff.hunks.len().to_string()),
            ],
        );
        ui.heading(summary);
        let row_height = self.density.row_height();
        let mut stage = None;
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show_rows(ui, row_height, self.diff.lines.len(), |ui, rows| {
                for line_index in rows {
                    let line = &self.diff.lines[line_index];
                    let hunk = self
                        .diff
                        .hunks
                        .iter()
                        .find(|hunk| hunk.start_line == line_index);
                    ui.horizontal(|ui| {
                        let color = match line.kind {
                            DiffLineKind::Added => egui::Color32::LIGHT_GREEN,
                            DiffLineKind::Deleted => egui::Color32::LIGHT_RED,
                            DiffLineKind::Hunk => egui::Color32::LIGHT_BLUE,
                            _ => ui.visuals().text_color(),
                        };
                        ui.colored_label(color, egui::RichText::new(&line.text).monospace());
                        if let Some(hunk) = hunk
                            && self.diff.file_count == 1
                            && ui.small_button(self.i18n.text("stage-hunk")).clicked()
                        {
                            stage = Some((hunk.index, hunk.header.clone()));
                        }
                    });
                }
            });
        if let Some((index, header)) = stage {
            match self.repo.stage_hunk(index, &header) {
                Ok(()) => self.refresh(),
                Err(error) => self.error = Some(error),
            }
        }
    }
}
