//! i18n 机制：语义键、嵌入式目录加载、回退链（当前语言 → en → 键名）、
//! 命名参数替换、复数变体选择与缺键检测。
//!
//! 目录格式为 .ftl 风格子集，每行一条：`#` 开头为注释，`key = 文本` 定义词条，
//! 文本内 `{name}` 为命名参数；`key.one` / `key.other` 为复数变体。

use std::collections::BTreeMap;

/// 默认语言：产品定位为中文优先。
pub const DEFAULT_LANGUAGE: &str = "zh-CN";

/// 内置目录：语言代码与目录文本（编译期内嵌，无文件系统依赖）。
const CATALOGS: &[(&str, &str)] = &[
    ("zh-CN", include_str!("../locales/zh-CN.ftl")),
    ("en", include_str!("../locales/en.ftl")),
];

/// 解析目录文本为 键 → 文本 映射；格式错误返回 Err。
pub fn parse_catalog(source: &str) -> Result<BTreeMap<String, String>, String> {
    let mut entries = BTreeMap::new();
    for (index, raw) in source.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, text) = line
            .split_once('=')
            .ok_or_else(|| format!("目录第 {} 行缺少 '='：{line}", index + 1))?;
        let key = key.trim();
        let text = text.trim();
        if key.is_empty() || text.is_empty() {
            return Err(format!("目录第 {} 行键或文本为空：{line}", index + 1));
        }
        if entries.insert(key.to_owned(), text.to_owned()).is_some() {
            return Err(format!("目录第 {} 行键重复：{key}", index + 1));
        }
    }
    Ok(entries)
}

/// 多语言环境：持有全部内置目录与当前语言下标。
pub struct I18n {
    catalogs: Vec<(&'static str, BTreeMap<String, String>)>,
    current: usize,
}

impl I18n {
    /// 加载全部内置目录并选中默认语言。
    pub fn new() -> Result<Self, String> {
        let mut catalogs = Vec::with_capacity(CATALOGS.len());
        for (code, source) in CATALOGS {
            catalogs.push((*code, parse_catalog(source)?));
        }
        let current = catalogs
            .iter()
            .position(|(code, _)| *code == DEFAULT_LANGUAGE)
            .ok_or_else(|| format!("缺少默认语言目录：{DEFAULT_LANGUAGE}"))?;
        Ok(Self { catalogs, current })
    }

    /// 当前语言代码。
    pub fn language(&self) -> &'static str {
        self.catalogs[self.current].0
    }

    /// 全部可用语言代码（按目录声明顺序）。
    pub fn languages(&self) -> Vec<&'static str> {
        self.catalogs.iter().map(|(code, _)| *code).collect()
    }

    /// 切换语言；语言不存在时返回 false 并保持原状。
    pub fn set_language(&mut self, code: &str) -> bool {
        match self.catalogs.iter().position(|(name, _)| *name == code) {
            Some(index) => {
                self.current = index;
                true
            }
            None => false,
        }
    }

    /// 在可用语言间循环切换。
    pub fn toggle_language(&mut self) {
        self.current = (self.current + 1) % self.catalogs.len();
    }

    /// 取词条：当前语言 → en → 可见占位键名。
    pub fn text(&self, key: &str) -> String {
        self.lookup(key)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("⟦{key}⟧"))
    }

    /// 取词条并替换命名参数（未提供的占位符原样保留，便于发现遗漏）。
    pub fn text_with(&self, key: &str, args: &[(&str, &str)]) -> String {
        let mut text = self.text(key);
        for (name, value) in args {
            text = text.replace(&format!("{{{name}}}"), value);
        }
        text
    }

    /// 复数词条：count 为 1 取 .one，其余取 .other，并替换 {count}。
    pub fn text_plural(&self, key: &str, count: u64) -> String {
        let variant = if count == 1 {
            format!("{key}.one")
        } else {
            format!("{key}.other")
        };
        let text = self
            .lookup(&variant)
            .or_else(|| self.lookup(key))
            .map(str::to_owned)
            .unwrap_or_else(|| format!("⟦{variant}⟧"));
        text.replace("{count}", &count.to_string())
    }

    fn lookup(&self, key: &str) -> Option<&str> {
        let current = &self.catalogs[self.current].1;
        if let Some(text) = current.get(key) {
            return Some(text.as_str());
        }
        self.catalogs
            .iter()
            .find(|(code, _)| *code == "en")
            .and_then(|(_, catalog)| catalog.get(key))
            .map(String::as_str)
    }
}
