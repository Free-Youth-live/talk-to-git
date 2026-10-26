# Talk To Git（TTG）

> 与 Git 对话 —— 不羁的青春（FreeYouth）出品的 Git 工作台

Talk To Git 是一个以“与 Git 对话”为核心理念的 Git 图形工作台：面向日常提交、审阅与历史浏览，把 Git 的专业能力组织成清晰、可读的界面。当前为**阶段 0 原型**，聚焦 Windows 平台，其余平台随后验证。

## 状态

- 阶段 0 原型：状态页、差异页、历史页与单文件 Hunk 暂存可用；
- 全部 Git 调用经安全隔离层执行，打开不可信仓库按只读方式处理；
- 提供只读的结构化状态输出（`--ai-status`），便于自动化工具消费。

## 构建与运行

```sh
cargo build --release
cargo run --release -p ttg-gui -- --repo <仓库路径>
```

验证入口：`bash scripts/validate-stage0.sh`（格式化、静态检查、测试与端到端路径）。

## 许可

本项目以 **木兰宽松许可证，第 2 版（Mulan PSL v2）** 发布，见 [LICENSE](LICENSE)；第三方依赖与许可见 [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md)。

## 参与

- 问题报告与改进建议欢迎提交 issue（中文或英文均可）；
- **暂缓接收代码 PR**：在贡献流程正式公布前，不接受直接的代码拉取请求；
- 安全相关问题请勿公开提交 issue，披露渠道将在正式发布时公布。

---

# Talk To Git (TTG)

> Talk with Git — a Git workbench by FreeYouth.

Talk To Git is a Git desktop workbench built around a conversational experience with Git: everyday commits, review, and history browsing organized into a clear, readable interface. This is the **Stage 0 prototype**, focused on Windows first; other platforms follow.

## Status

- Stage 0 prototype: status, diff, and history views with single-file Hunk staging;
- All Git invocations run through a hardened isolation layer; untrusted repositories are treated read-only;
- A read-only structured status output (`--ai-status`) is available for automation.

## Build & run

```sh
cargo build --release
cargo run --release -p ttg-gui -- --repo <repository-path>
```

Validation entry point: `bash scripts/validate-stage0.sh`.

## License

Released under the **Mulan Permissive Software License, Version 2 (Mulan PSL v2)**; see [LICENSE](LICENSE). Third-party notices: [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

## Contributing

- Issues and suggestions are welcome in Chinese or English;
- **Code pull requests are paused** until the contribution process is published;
- For security matters, please do not open a public issue; the disclosure channel will be announced at release.
