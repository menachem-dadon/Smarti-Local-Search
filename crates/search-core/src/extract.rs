use crate::types::Chunk;
use std::path::Path;

pub fn kind(ext: &str) -> &'static str {
    match ext {
        "pdf" => "pdf",
        "docx" | "pptx" | "xlsx" | "ods" => "document",
        "jpg" | "jpeg" | "png" | "webp" | "bmp" | "tiff" | "tif" | "gif" => "image",
        "wav" | "mp3" | "m4a" | "aac" | "flac" | "ogg" | "opus" | "wma" => "audio",
        "mp4" | "mkv" | "mov" | "webm" | "avi" | "m4v" => "video",
        "py" | "js" | "ts" | "jsx" | "tsx" | "rs" | "go" | "java" | "kt" | "kts" | "c" | "h"
        | "cpp" | "hpp" | "cs" | "php" | "rb" | "swift" | "dart" | "sh" | "ps1" | "bat" | "cmd"
        | "vue" | "svelte" => "code",
        "txt" | "md" | "markdown" | "rst" | "log" | "ini" | "cfg" | "conf" | "toml" | "yaml"
        | "yml" | "json" | "jsonl" | "xml" | "csv" | "tsv" | "sql" | "html" | "htm" | "css"
        | "scss" | "less" | "eml" => "text",
        _ => "binary",
    }
}
pub fn decode(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xff, 0xfe]) {
        return encoding_rs::UTF_16LE.decode(&bytes[2..]).0.into_owned();
    }
    if bytes.starts_with(&[0xfe, 0xff]) {
        return encoding_rs::UTF_16BE.decode(&bytes[2..]).0.into_owned();
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => s.trim_start_matches('\u{feff}').into(),
        Err(_) => encoding_rs::WINDOWS_1255.decode(bytes).0.into_owned(),
    }
}
pub fn text_chunks(text: &str, modality: &str) -> Vec<Chunk> {
    let lines: Vec<_> = text.lines().collect();
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < lines.len() {
        let mut end = start;
        let mut count = 0;
        let mut chars = 0;
        while end < lines.len() {
            let n = lines[end].split_whitespace().count();
            let c = lines[end].chars().count();
            if end > start && (count + n > 400 || chars + c > 1800) {
                break;
            }
            count += n;
            chars += c;
            end += 1;
        }
        let content = lines[start..end].join("\n");
        if !content.trim().is_empty() {
            // A single unbroken line is split at Unicode scalar boundaries.
            let values: Vec<char> = content.chars().collect();
            for part in values.chunks(1800) {
                let text: String = part.iter().collect();
                chunks.push(Chunk {
                    modality: modality.into(),
                    hash: blake3::hash(text.as_bytes()).to_hex().to_string(),
                    text,
                    heading: lines[start]
                        .trim_start_matches('#')
                        .chars()
                        .take(100)
                        .collect(),
                    line_start: Some(start as u32 + 1),
                    line_end: Some(end as u32),
                    ..Default::default()
                });
            }
        }
        let mut overlap = end;
        let mut words = 0;
        while overlap > start + 1 && words < 60 {
            overlap -= 1;
            words += lines[overlap].split_whitespace().count();
        }
        start = if end == lines.len() {
            end
        } else if overlap > start {
            overlap
        } else {
            end
        };
    }
    chunks
}
pub fn code_chunks(text: &str, ext: &str) -> Vec<Chunk> {
    let language = match ext {
        "rs" => Some(tree_sitter_rust::LANGUAGE.into()),
        "py" => Some(tree_sitter_python::LANGUAGE.into()),
        "js" | "jsx" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "ts" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "tsx" => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
        _ => None,
    };
    let Some(language) = language else {
        return text_chunks(text, "code");
    };
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&language).is_err() {
        return text_chunks(text, "code");
    };
    let Some(tree) = parser.parse(text, None) else {
        return text_chunks(text, "code");
    };
    let mut result = Vec::new();
    let mut cursor = tree.root_node().walk();
    for node in tree.root_node().named_children(&mut cursor) {
        if let Ok(content) = node.utf8_text(text.as_bytes()) {
            let mut chunks = text_chunks(content, "code");
            for chunk in &mut chunks {
                let offset = node.start_position().row as u32;
                chunk.line_start = chunk.line_start.map(|n| n + offset);
                chunk.line_end = chunk.line_end.map(|n| n + offset);
                chunk.heading = content
                    .lines()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(120)
                    .collect();
            }
            result.extend(chunks);
        }
    }
    if result.is_empty() {
        text_chunks(text, "code")
    } else {
        result
    }
}
pub fn sensitive(p: &Path) -> bool {
    let name = p
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    name == ".env"
        || name.starts_with(".env.")
        || ["id_rsa", "id_ed25519", "credentials", "login data"].contains(&name.as_str())
        || p.extension().is_some_and(|x| {
            ["pem", "key", "pfx", "p12"].contains(&x.to_string_lossy().to_lowercase().as_str())
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_chunks() {
        let s = "שלום".repeat(2000);
        let c = text_chunks(&s, "text");
        assert!(c.len() > 3);
        assert!(c.iter().all(|c| c.text.chars().count() <= 1800));
        assert_eq!(c.iter().map(|c| c.text.clone()).collect::<String>(), s);
    }
    #[test]
    fn code_symbols() {
        let c = code_chunks("fn a() { println!(\"x\"); }\nfn b() {}", "rs");
        assert_eq!(c.len(), 2);
        assert_eq!(c[1].line_start, Some(2));
    }
    #[test]
    fn utf16() {
        assert_eq!(decode(&[255, 254, 65, 0, 66, 0]), "AB");
    }
    #[test]
    fn binary_classification() {
        assert_eq!(kind("exe"), "binary");
        assert!(sensitive(Path::new(".env.local")));
    }
}
