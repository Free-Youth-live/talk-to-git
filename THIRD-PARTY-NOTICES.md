# Third-Party Notices

本文件登记阶段 0 直接引入的第三方开源软件。依赖在选型和升级时使用最新稳定版，验证后由 `Cargo.lock` 固定准确版本。完整传递依赖以锁文件为准，对外或开源发布前还需生成完整许可清单并附相应许可证文本。

本项目自身（不羁的青春 / FreeYouth，Copyright (c) 2026 不羁的青春（FreeYouth））采用**木兰宽松许可证，第 2 版（Mulan PSL v2，SPDX-License-Identifier: MulanPSL-2.0）**授权，许可证全文见根目录 `LICENSE`。上表全部直接依赖均为 MIT / Apache-2.0 宽松许可证，与木兰宽松许可证 v2 兼容；Git for Windows（GPL-2.0）仅作为系统工具调用，不再分发。

## Rust 直接依赖

| 软件 | 版本 | 许可证 | 来源 |
|---|---:|---|---|
| eframe / egui | 0.37.0 | MIT OR Apache-2.0 | https://github.com/emilk/egui |
| serde | 1.0.229 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| serde_json | 1.0.154 | MIT OR Apache-2.0 | https://github.com/serde-rs/json |
| wait-timeout | 0.2.1 | MIT OR Apache-2.0 | https://github.com/alexcrichton/wait-timeout |

## 便携构建工具

| 软件 | 版本 | 许可证 | 来源 |
|---|---:|---|---|
| Rust | 1.99.0 | MIT OR Apache-2.0 | https://github.com/rust-lang/rust |
| Git for Windows（系统工具，不再分发） | 2.56.0.windows.1 | GPL-2.0 | https://github.com/git-for-windows/git |

## CI 直接引用

| 软件 | 版本 | 许可证 | 来源 |
|---|---:|---|---|
| actions/checkout | 7.0.1 | MIT | https://github.com/actions/checkout |

`ttg-fixtures` 自身仅使用 Rust 标准库，没有额外第三方运行时依赖。
`ttg-i18n` 自身仅使用 Rust 标准库，没有额外第三方依赖。
