use crate::{extract, types::Settings};
use std::collections::HashSet;
use std::path::Path;

// Avoid broad names such as "backup", "bin", "vendor", or "artifacts": they
// can contain the user's only copy of documents or intentionally indexed code.
pub const ADDITIONAL_DEFAULTS: &[&str] = &[
    ".git",
    ".svn",
    ".hg",
    ".cache",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".tox",
    ".nox",
    ".gradle",
    ".npm",
    ".old-node-modules",
    "node_modules.old",
    "node_modules.bak",
    ".pnpm-store",
    ".yarn/cache",
    ".yarn/unplugged",
    ".nuget/packages",
    ".vs",
    ".turbo",
    ".parcel-cache",
    ".vite",
    ".webpack",
    ".rollup.cache",
    ".svelte-kit",
    ".nuxt",
    ".angular/cache",
    "CMakeFiles",
    ".cxx",
    ".dart_tool",
    ".ipynb_checkpoints",
    "__pypackages__",
    "*.egg-info",
    "site-packages",
    "_internal",
    "obj",
    "bower_components",
    "coverage",
    "htmlcov",
    "Windows/WinSxS",
    "Windows/SoftwareDistribution",
    "Windows/Temp",
    "Windows/Installer",
    "ProgramData/Package Cache",
    "Recovery",
    "FileHistory",
    "WindowsImageBackup",
    "*-backup-20[0-9][0-9][01][0-9][0-3][0-9]-[0-9]*",
    "backup-20[0-9][0-9][01][0-9][0-3][0-9]-[0-9]*",
];

fn normalized(value: &str) -> String {
    let value = value.trim().replace('\\', "/").to_lowercase();
    let value = if let Some(unc) = value.strip_prefix("//?/unc/") {
        format!("//{unc}")
    } else {
        value.strip_prefix("//?/").unwrap_or(&value).to_owned()
    };
    value.trim_end_matches('/').to_owned()
}

pub struct Exclusions {
    sensitive: bool,
    names: Vec<String>,
    components: HashSet<String>,
    absolute: Vec<String>,
    patterns: Vec<glob::Pattern>,
}
impl Exclusions {
    pub fn new(settings: &Settings, extra: &[String]) -> Self {
        let mut result = Self {
            sensitive: !settings.sensitive_files,
            names: Vec::new(),
            components: HashSet::new(),
            absolute: Vec::new(),
            patterns: Vec::new(),
        };
        for rule in settings.exclusions.iter().chain(extra) {
            let original = rule.trim();
            let rule = normalized(original);
            if rule.is_empty() {
                continue;
            }
            // Windows drive, UNC, and native absolute paths use a component
            // boundary, so excluding C:/One does not exclude C:/OneMore.
            if rule.as_bytes().get(1) == Some(&b':') || rule.starts_with('/') {
                // Rule matching never probes a remote server. Local paths may
                // use Windows' short (8.3) user/profile aliases.
                let canonical = if rule.starts_with("//") {
                    rule
                } else {
                    std::fs::canonicalize(original)
                        .map(|p| normalized(&crate::engine::display_path(&p)))
                        .unwrap_or(rule)
                };
                result.absolute.push(canonical);
            } else if rule.contains(['*', '?', '[']) {
                if let Ok(pattern) = glob::Pattern::new(&rule) {
                    result.patterns.push(pattern);
                }
            } else if rule.contains('/') {
                result.names.push(rule);
            } else {
                result.components.insert(rule);
            }
        }
        result
    }
    pub fn matches(&self, path: &Path, root: &Path) -> bool {
        if self.sensitive && extract::sensitive(path) {
            return true;
        }
        let full = normalized(&path.to_string_lossy());
        if self
            .absolute
            .iter()
            .any(|x| full == *x || full.starts_with(&format!("{x}/")))
        {
            return true;
        }
        let relative = normalized(&path.strip_prefix(root).unwrap_or(path).to_string_lossy());
        if relative.split('/').any(|c| self.components.contains(c)) {
            return true;
        }
        // Checking ancestors also enforces rules on watcher events and queued
        // files which bypass WalkDir's directory pruning.
        let mut part = relative.as_str();
        loop {
            let name = part.rsplit('/').next().unwrap_or(part);
            if self.names.iter().any(|x| {
                part == x
                    || part.starts_with(&format!("{x}/"))
                    || part.contains(&format!("/{x}/"))
                    || part.ends_with(&format!("/{x}"))
            }) || self
                .patterns
                .iter()
                .any(|p| p.matches(part) || p.matches(name))
            {
                return true;
            }
            match part.rsplit_once('/') {
                Some((parent, _)) => part = parent,
                None => break,
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absolute_paths_case_boundaries_and_unc() {
        let settings = Settings {
            exclusions: vec![
                "C:\\Users\\One\\Cache\\".into(),
                "\\\\server\\share\\private".into(),
            ],
            ..Default::default()
        };
        let rules = Exclusions::new(&settings, &[]);
        let root = Path::new("C:/Users/One");
        assert!(rules.matches(Path::new("c:/users/one/cache/file.txt"), root));
        assert!(!rules.matches(Path::new("C:/Users/One/CacheMore/file.txt"), root));
        assert!(rules.matches(Path::new("\\\\server\\share\\private\\file.txt"), root));
        assert!(!rules.matches(Path::new("\\\\server\\share\\private-more\\file.txt"), root));
    }
    #[test]
    fn defaults_prune_generated_files_and_timestamped_backups_only() {
        let rules = Exclusions::new(&Settings::default(), &[]);
        let root = Path::new("C:/Work");
        for name in [
            ".cache/x",
            "obj/debug/file",
            ".git/config",
            "Tool-backup-20261007-123000/project/a.txt",
            "backup-20261007-123000/a.txt",
        ] {
            assert!(rules.matches(&root.join(name), root), "{name}");
        }
        for name in [
            "backup/family.docx",
            "bin/script.py",
            "artifacts/report.pdf",
            "vendor/custom.rs",
            "Project/src/main.rs",
        ] {
            assert!(!rules.matches(&root.join(name), root), "{name}");
        }
    }
}
