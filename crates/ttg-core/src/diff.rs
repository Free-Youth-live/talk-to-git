//! 统一 Diff 解析：文件/Hunk 结构化与 Hunk 补丁生成。
use serde::Serialize;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DiffLineKind {
    Header,
    Hunk,
    Added,
    Deleted,
    Context,
    Meta,
}

#[derive(Clone, Debug, Serialize)]
pub struct DiffLine {
    pub text: String,
    pub kind: DiffLineKind,
}

#[derive(Clone, Debug, Serialize)]
pub struct DiffHunk {
    pub index: usize,
    pub header: String,
    pub start_line: usize,
    pub end_line: usize,
    file_header_start: usize,
    file_header_end: usize,
    start_byte: usize,
    end_byte: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct DiffDocument {
    pub lines: Vec<DiffLine>,
    pub hunks: Vec<DiffHunk>,
    pub file_count: usize,
    #[serde(skip)]
    source: Vec<u8>,
}

impl DiffDocument {
    pub fn patch_for_hunk(&self, index: usize, expected_header: &str) -> Result<Vec<u8>, String> {
        if self.file_count != 1 {
            return Err("Hunk staging requires exactly one text file".into());
        }
        let unsupported = [
            "GIT binary patch",
            "Binary files",
            "rename from",
            "rename to",
            "new file mode",
            "deleted file mode",
            "Submodule",
        ];
        let source_text = String::from_utf8_lossy(&self.source);
        if unsupported.iter().any(|value| source_text.contains(value)) {
            return Err("This file type is not supported by the prototype Hunk staging".into());
        }
        let hunk = self
            .hunks
            .get(index)
            .ok_or("Hunk no longer exists; refresh the diff")?;
        if hunk.header != expected_header {
            return Err("Diff changed; refresh before staging the Hunk".into());
        }
        let mut patch = Vec::new();
        patch.extend_from_slice(&self.source[hunk.file_header_start..hunk.file_header_end]);
        patch.extend_from_slice(&self.source[hunk.start_byte..hunk.end_byte]);
        Ok(patch)
    }
}

pub fn parse_diff(source: Vec<u8>) -> DiffDocument {
    let ranges = line_ranges(&source);
    let mut lines = Vec::with_capacity(ranges.len());
    let mut hunks: Vec<DiffHunk> = Vec::new();
    let mut file_count = 0usize;
    let mut file_header_start = 0usize;
    let mut file_header_end = 0usize;
    let mut current_hunk: Option<usize> = None;

    for (line_index, &(start, end)) in ranges.iter().enumerate() {
        let raw = &source[start..end];
        let text = String::from_utf8_lossy(raw)
            .trim_end_matches(['\r', '\n'])
            .to_string();
        if raw.starts_with(b"diff --git ") {
            file_count += 1;
            file_header_start = start;
            file_header_end = start;
            current_hunk = None;
        }
        if raw.starts_with(b"@@") {
            if let Some(previous) = current_hunk.take() {
                hunks[previous].end_line = line_index;
                hunks[previous].end_byte = start;
            }
            if file_header_end == file_header_start {
                file_header_end = start;
            }
            let index = hunks.len();
            hunks.push(DiffHunk {
                index,
                header: text.clone(),
                start_line: line_index,
                end_line: ranges.len(),
                file_header_start,
                file_header_end,
                start_byte: start,
                end_byte: source.len(),
            });
            current_hunk = Some(index);
        }
        let kind = classify_line(raw);
        lines.push(DiffLine { text, kind });
    }
    DiffDocument {
        lines,
        hunks,
        file_count,
        source,
    }
}

fn line_ranges(source: &[u8]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = 0usize;
    for (index, byte) in source.iter().enumerate() {
        if *byte == b'\n' {
            ranges.push((start, index + 1));
            start = index + 1;
        }
    }
    if start < source.len() {
        ranges.push((start, source.len()));
    }
    ranges
}

fn classify_line(raw: &[u8]) -> DiffLineKind {
    if raw.starts_with(b"@@") {
        DiffLineKind::Hunk
    } else if raw.starts_with(b"diff --git ")
        || raw.starts_with(b"index ")
        || raw.starts_with(b"---")
        || raw.starts_with(b"+++")
    {
        DiffLineKind::Header
    } else if raw.starts_with(b"+") {
        DiffLineKind::Added
    } else if raw.starts_with(b"-") {
        DiffLineKind::Deleted
    } else if raw.starts_with(b" ") {
        DiffLineKind::Context
    } else {
        DiffLineKind::Meta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> DiffDocument {
        parse_diff(
            b"diff --git a/file.txt b/file.txt\nindex 1111111..2222222 100644\n--- a/file.txt\n+++ b/file.txt\n@@ -1,2 +1,2 @@\n-one\n+one changed\n two\n@@ -4,2 +4,2 @@\n-three\n+three changed\n four\n"
                .to_vec(),
        )
    }

    #[test]
    fn finds_hunks_without_duplicating_headers() {
        let document = sample();
        assert_eq!(document.file_count, 1);
        assert_eq!(document.hunks.len(), 2);
        let patch = document.patch_for_hunk(1, "@@ -4,2 +4,2 @@").unwrap();
        let patch = String::from_utf8(patch).unwrap();
        assert_eq!(patch.matches("diff --git").count(), 1);
        assert!(!patch.contains("one changed"));
        assert!(patch.contains("three changed"));
    }

    #[test]
    fn rejects_stale_hunk_headers() {
        let document = sample();
        assert!(document.patch_for_hunk(0, "stale").is_err());
    }
}
