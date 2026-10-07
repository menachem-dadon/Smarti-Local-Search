use notify::{Event, EventKind, event::ModifyKind};
use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Default)]
pub(crate) struct Changes {
    paths: HashMap<PathBuf, (Instant, bool)>,
    pub rescan: bool,
}

impl Changes {
    pub fn retry(&mut self, path: PathBuf, recursive: bool) {
        if self.paths.len() >= 8192 && !self.paths.contains_key(&path) {
            self.rescan = true;
            return;
        }
        let entry = self.paths.entry(path).or_insert((Instant::now(), false));
        entry.0 = Instant::now();
        entry.1 |= recursive;
    }
    pub fn record(&mut self, event: Event, mut included: impl FnMut(&std::path::Path) -> bool) {
        if event.need_rescan() {
            self.rescan = true;
        }
        if event.kind.is_access() {
            return;
        }
        let recursive = matches!(
            event.kind,
            EventKind::Create(_)
                | EventKind::Modify(ModifyKind::Name(_))
                | EventKind::Any
                | EventKind::Other
        );
        for path in event.paths.into_iter().filter(|path| included(path)) {
            // A Windows watcher overflow is an exceptional reconciliation,
            // coalesced once. Ordinary folder metadata never queues a root.
            self.retry(path, recursive);
        }
    }
    pub fn ready(&mut self) -> Vec<(PathBuf, bool)> {
        let mut ready: Vec<_> = self
            .paths
            .iter()
            .filter(|(_, (time, _))| time.elapsed() > Duration::from_millis(500))
            .map(|(path, (_, recursive))| (path.clone(), *recursive))
            .collect();
        // Move destinations precede deletions, preserving NTFS identities.
        // Parent subtrees cover child events in the same batch.
        ready.sort_by_cached_key(|(path, _)| {
            (!path.exists(), path.components().count(), path.clone())
        });
        ready.truncate(256);
        for (path, _) in &ready {
            self.paths.remove(path);
        }
        ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_events_debounce_preserve_rename_scope_and_bound_overflow() {
        let mut changes = Changes::default();
        let path = PathBuf::from("new-folder");
        changes.record(
            Event::new(EventKind::Create(notify::event::CreateKind::Folder)).add_path(path.clone()),
            |_| true,
        );
        for _ in 0..100 {
            changes.record(
                Event::new(EventKind::Modify(ModifyKind::Any)).add_path(path.clone()),
                |_| true,
            );
        }
        assert!(changes.ready().is_empty());
        changes.paths.get_mut(&path).unwrap().0 = Instant::now() - Duration::from_secs(1);
        assert_eq!(changes.ready(), vec![(path, true)]);
        for id in 0..9000 {
            changes.record(
                Event::new(EventKind::Modify(ModifyKind::Any))
                    .add_path(PathBuf::from(format!("file-{id}"))),
                |_| true,
            );
        }
        assert_eq!(changes.paths.len(), 8192);
        assert!(changes.rescan);
        let mut excluded = Changes::default();
        excluded.record(
            Event::new(EventKind::Modify(ModifyKind::Any)).add_path(".cache/a.bin".into()),
            |_| false,
        );
        assert!(excluded.paths.is_empty());
        assert!(!excluded.rescan);
    }
}
