//! i18n 哨兵测试：目录一致性与硬编码文案检测。
//!
//! ① 键 ⊆ 目录：语言目录键集合必须一致；② 占位符一致；③ 复数变体成对；
//! ④ 源码引用的键都存在于目录、且目录中的键都被源码使用；
//! ⑤ 源码（ttg-gui、ttg-core）不得出现硬编码的 CJK 文案字面量。

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn catalog_paths() -> [(&'static str, PathBuf); 2] {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    [
        ("zh-CN", base.join("zh-CN.ftl")),
        ("en", base.join("en.ftl")),
    ]
}

/// 独立实现的目录解析（刻意不复用 crate 内部实现，避免自证）。
fn parse(path: &Path) -> Vec<(String, String)> {
    let source = fs::read_to_string(path).unwrap_or_else(|e| panic!("读取目录失败 {path:?}: {e}"));
    let mut entries = Vec::new();
    for (index, raw) in source.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, text) = line
            .split_once('=')
            .unwrap_or_else(|| panic!("目录第 {} 行缺少 '='：{line}", index + 1));
        entries.push((key.trim().to_owned(), text.trim().to_owned()));
    }
    entries
}

fn placeholders(text: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else { break };
        names.insert(after[..end].to_owned());
        rest = &after[end + 1..];
    }
    names
}

fn keys(entries: &[(String, String)]) -> BTreeSet<String> {
    entries.iter().map(|(key, _)| key.clone()).collect()
}

/// 收集目录下所有 .rs 源文件（递归）。
fn rust_sources(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    files
}

/// 提取源码中所有双引号字符串字面量（不处理转义；本项目键名与文案不含转义引号）。
fn string_literals(source: &str) -> Vec<String> {
    let mut literals = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find('"') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('"') else { break };
        literals.push(after[..end].to_owned());
        rest = &after[end + 1..];
    }
    literals
}

/// 取 needle 之后紧跟的引号字面量（用于 .text("key") 等调用点）。
fn key_after(source: &str, needle: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = source;
    while let Some(pos) = rest.find(needle) {
        let after = &rest[pos + needle.len()..];
        if let Some(end) = after.find('"') {
            found.push(after[..end].to_owned());
        }
        rest = after;
    }
    found
}

fn is_key_shaped(literal: &str) -> bool {
    let bytes = literal.as_bytes();
    !bytes.is_empty()
        && bytes[0].is_ascii_lowercase()
        && literal.contains('-')
        && literal
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF)
}

#[test]
fn catalogs_share_identical_key_sets() {
    let [(zh_code, zh_path), (en_code, en_path)] = catalog_paths();
    let zh = keys(&parse(&zh_path));
    let en = keys(&parse(&en_path));
    let only_zh: Vec<_> = zh.difference(&en).cloned().collect();
    let only_en: Vec<_> = en.difference(&zh).cloned().collect();
    assert!(
        only_zh.is_empty() && only_en.is_empty(),
        "{zh_code} 独有键：{only_zh:?}；{en_code} 独有键：{only_en:?}"
    );
}

#[test]
fn placeholders_match_across_languages() {
    let [(zh_code, zh_path), (en_code, en_path)] = catalog_paths();
    let zh: std::collections::BTreeMap<_, _> = parse(&zh_path).into_iter().collect();
    let en: std::collections::BTreeMap<_, _> = parse(&en_path).into_iter().collect();
    for (key, zh_text) in &zh {
        let en_text = en.get(key).unwrap_or_else(|| panic!("en 缺少键 {key}"));
        assert_eq!(
            placeholders(zh_text),
            placeholders(en_text),
            "{zh_code}/{en_code} 键 {key} 的占位符不一致"
        );
    }
}

#[test]
fn plural_variants_are_paired() {
    let [(zh_code, zh_path), (en_code, en_path)] = catalog_paths();
    for (code, path) in [(zh_code, zh_path), (en_code, en_path)] {
        let set = keys(&parse(&path));
        for key in &set {
            let (base, variant) = match key.rsplit_once('.') {
                Some((base, variant @ ("one" | "other"))) => (base.to_owned(), variant.to_owned()),
                _ => continue,
            };
            let sibling = if variant == "one" { "other" } else { "one" };
            assert!(
                set.contains(&format!("{base}.{sibling}")),
                "{code} 复数变体缺失：{base}.{sibling}"
            );
        }
    }
}

#[test]
fn gui_uses_catalogs_both_ways() {
    let mut catalog: BTreeSet<String> = BTreeSet::new();
    for (_, path) in catalog_paths() {
        catalog.extend(keys(&parse(&path)));
    }
    let in_catalog = |key: &str| {
        catalog.contains(key)
            || catalog.contains(&format!("{key}.one"))
            || catalog.contains(&format!("{key}.other"))
    };

    let gui_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ttg-gui/src")
        .canonicalize()
        .expect("定位 ttg-gui/src 失败");
    let mut combined = String::new();
    let mut used: BTreeSet<String> = BTreeSet::new();
    for file in rust_sources(&gui_dir) {
        let source = fs::read_to_string(&file).expect("读取 gui 源文件失败");
        for needle in [
            ".text(\"",
            ".text_with(\"",
            ".text_plural(\"",
            ".label_key(\"",
        ] {
            used.extend(key_after(&source, needle));
        }
        for literal in string_literals(&source) {
            if is_key_shaped(&literal) {
                used.insert(literal);
            }
        }
        combined.push_str(&source);
    }
    // 键 ⊆ 目录：走 i18n 调用点的键与连字符形键都必须存在于目录（含复数基名）。
    for key in &used {
        assert!(in_catalog(key), "gui 使用的键不存在于目录：{key}");
    }
    // 目录 ⊆ 使用：目录中每个键（复数变体归一为基名）必须作为引号字面量被 gui 引用。
    for key in &catalog {
        let base = key
            .rsplit_once('.')
            .filter(|(_, v)| *v == "one" || *v == "other")
            .map(|(b, _)| b)
            .unwrap_or(key);
        let quoted = format!("\"{base}\"");
        assert!(
            combined.contains(&quoted),
            "目录中的键未被 gui 使用：{key}（期望源码中出现 {quoted}）"
        );
    }
}

#[test]
fn sources_contain_no_hardcoded_cjk_copy() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    for relative in ["../ttg-gui/src", "../ttg-core/src"] {
        let dir = manifest
            .join(relative)
            .canonicalize()
            .expect("定位源码目录失败");
        for file in rust_sources(&dir) {
            let source = fs::read_to_string(&file).expect("读取源文件失败");
            for (index, raw) in source.lines().enumerate() {
                let trimmed = raw.trim_start();
                if trimmed.starts_with("//") {
                    continue;
                }
                // 粗略去掉行内注释：本项目源码字符串字面量中不含 "//"。
                let code = match raw.find("//") {
                    Some(pos) => &raw[..pos],
                    None => raw,
                };
                if let Some(c) = code.chars().find(|c| is_cjk(*c)) {
                    panic!(
                        "{}:{} 出现硬编码 CJK 文案字符 {c:?}，用户可见文案须经 ttg-i18n",
                        file.display(),
                        index + 1
                    );
                }
            }
        }
    }
}
