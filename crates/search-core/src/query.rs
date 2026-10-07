use crate::types::{FileRecord, Filters};
use anyhow::{Result, bail};
use chrono::NaiveDate;
use regex::Regex;

pub fn parse(input: &str, mut filters: Filters) -> Result<(String, Filters)> {
    let re = Regex::new(r#"(?:[A-Za-z]+:)?(?:"[^"]*"|[^\s]+)"#)?;
    let mut words = Vec::new();
    for m in re.find_iter(input) {
        let t = m.as_str();
        let Some((key, val)) = t.split_once(':') else {
            words.push(t.to_string());
            continue;
        };
        let value = val.trim_matches('"');
        match key.to_lowercase().as_str() {
            "type" => filters.kind = Some(value.into()),
            "ext" => filters.extension = Some(value.trim_start_matches('.').to_lowercase()),
            "path" => filters.path = Some(value.into()),
            "name" => filters.name = Some(value.into()),
            "after" | "before" => {
                let d = NaiveDate::parse_from_str(value, "%Y-%m-%d")?
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc()
                    .timestamp();
                if key == "after" {
                    filters.after = Some(d)
                } else {
                    filters.before = Some(d + 86400)
                }
            }
            "size" => {
                let (op, number) = if let Some(v) = value.strip_prefix('>') {
                    ('>', v)
                } else if let Some(v) = value.strip_prefix('<') {
                    ('<', v)
                } else {
                    bail!("Size filter needs > or <")
                };
                let size = parse_size(number)?;
                if op == '>' {
                    filters.min_size = Some(size)
                } else {
                    filters.max_size = Some(size)
                }
            }
            _ => words.push(t.to_string()),
        }
    }
    Ok((words.join(" "), filters))
}
fn parse_size(s: &str) -> Result<u64> {
    let upper = s.to_uppercase();
    let end = upper
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(upper.len());
    let num = upper[..end].parse::<f64>()?;
    let multiplier = match &upper[end..] {
        "" | "B" => 1.,
        "KB" | "KIB" => 1024.,
        "MB" | "MIB" => 1048576.,
        "GB" | "GIB" => 1073741824.,
        _ => bail!("Unknown size unit"),
    };
    anyhow::ensure!(num.is_finite() && num >= 0., "Invalid size");
    Ok((num * multiplier) as u64)
}
pub fn fts_query(s: &str) -> Option<String> {
    let terms: Vec<_> = s
        .split_whitespace()
        .filter_map(|t| {
            let word = t.trim_matches('"').replace('"', "\"\"");
            (!word.is_empty()).then(|| format!("\"{word}\"*"))
        })
        .collect();
    (!terms.is_empty()).then(|| terms.join(" AND "))
}
pub fn matches(f: &FileRecord, x: &Filters) -> bool {
    x.kind
        .as_ref()
        .is_none_or(|v| v == &f.kind || (v == "pdf" && f.extension == "pdf"))
        && x.extension
            .as_ref()
            .is_none_or(|v| v.eq_ignore_ascii_case(&f.extension))
        && x.path
            .as_ref()
            .is_none_or(|v| f.path.to_lowercase().starts_with(&v.to_lowercase()))
        && x.name
            .as_ref()
            .is_none_or(|v| f.name.to_lowercase().contains(&v.to_lowercase()))
        && x.after.is_none_or(|v| f.modified >= v)
        && x.before.is_none_or(|v| f.modified < v)
        && x.min_size.is_none_or(|v| f.size > v)
        && x.max_size.is_none_or(|v| f.size < v)
}
pub fn rrf(rank: usize, weight: f64) -> f64 {
    weight / (60.0 + rank as f64)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_filters() {
        let (s, f) = parse(
            "שלום path:\"C:\\My Files\" ext:rs after:2026-01-01 size:>10MB",
            Filters::default(),
        )
        .unwrap();
        assert_eq!(s, "שלום");
        assert_eq!(f.path.as_deref(), Some("C:\\My Files"));
        assert_eq!(f.min_size, Some(10485760));
        assert_eq!(f.extension.as_deref(), Some("rs"));
    }
    #[test]
    fn invalid_filters() {
        assert!(parse("size:>oops", Filters::default()).is_err());
        assert!(parse("after:2026-13-01", Filters::default()).is_err());
    }
    #[test]
    fn injection_escaped() {
        assert!(!fts_query("a OR b").unwrap().contains(" OR "));
    }
}
