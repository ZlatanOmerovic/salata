//! File system watcher for automatic page refresh during development.
//!
//! When `hot_reload = true` in `config.toml`, this module watches the served
//! directory tree for file modifications, creations, and deletions using the
//! `notify` crate. On any relevant filesystem event, the shared parse cache
//! is cleared so that the next request re-reads and re-parses the affected
//! `.slt` files.

use std::path::Path;
use std::sync::Arc;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use salata_core::cache::ParseCache;

/// Start a recursive file watcher on `root_dir` that clears the parse cache
/// whenever files are created, modified, or deleted.
///
/// Returns the watcher handle, which must be kept alive for watching to
/// continue. If the watcher cannot be created (e.g., the directory does not
/// exist), the error is propagated to the caller.
pub fn start_watcher(
    root_dir: &Path,
    cache: Arc<ParseCache>,
) -> Result<RecommendedWatcher, notify::Error> {
    let mut watcher =
        notify::recommended_watcher(move |res: Result<Event, notify::Error>| match res {
            Ok(event) => {
                if matches!(
                    event.kind,
                    EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
                ) {
                    cache.clear();
                }
            }
            Err(e) => {
                eprintln!("salata-server: file watcher error: {e}");
            }
        })?;

    watcher.watch(root_dir, RecursiveMode::Recursive)?;
    Ok(watcher)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn watcher_starts_on_valid_directory() {
        let dir = std::env::temp_dir().join("salata_hot_reload_test");
        let _ = fs::create_dir_all(&dir);

        let cache = Arc::new(ParseCache::new());
        let result = start_watcher(&dir, cache);
        assert!(result.is_ok());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn watcher_fails_on_nonexistent_directory() {
        let cache = Arc::new(ParseCache::new());
        let result = start_watcher(Path::new("/nonexistent/dir"), cache);
        assert!(result.is_err());
    }
}
