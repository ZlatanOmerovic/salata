//! Parsed file caching by path and modification time.
//!
//! Caches the parsed structure (block positions, directives) of `.slt` files
//! keyed by canonical path + mtime. This is **not** output caching — runtime
//! execution always runs fresh. The cache is automatically invalidated when
//! a file's modification time changes.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use crate::error::SalataResult;
use crate::parser::{self, ParsedDocument};

// ---------------------------------------------------------------------------
// Cache key
// ---------------------------------------------------------------------------

/// A cache key: canonical path + last modification time.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    path: PathBuf,
    mtime: SystemTime,
}

impl CacheKey {
    /// Build a cache key for a file. Returns `None` if mtime can't be read.
    fn from_path(path: &Path) -> Option<Self> {
        let canonical = path.canonicalize().ok()?;
        let mtime = std::fs::metadata(&canonical).ok()?.modified().ok()?;
        Some(Self {
            path: canonical,
            mtime,
        })
    }
}

// ---------------------------------------------------------------------------
// Cache
// ---------------------------------------------------------------------------

/// Thread-safe cache for parsed `.slt` documents.
/// Entries are invalidated automatically when the file's mtime changes.
pub struct ParseCache {
    entries: Mutex<HashMap<CacheKey, ParsedDocument>>,
}

impl ParseCache {
    /// Create a new empty cache.
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Look up a cached parse result for `path`. Returns `None` if the file
    /// isn't cached or the cached version is stale (mtime changed).
    pub fn get(&self, path: &Path) -> Option<ParsedDocument> {
        let key = CacheKey::from_path(path)?;
        let entries = self.entries.lock().ok()?;
        entries.get(&key).cloned()
    }

    /// Store a parsed document in the cache.
    pub fn put(&self, path: &Path, doc: &ParsedDocument) {
        if let Some(key) = CacheKey::from_path(path) {
            if let Ok(mut entries) = self.entries.lock() {
                entries.insert(key, doc.clone());
            }
        }
    }

    /// Parse a file, using the cache if possible. On cache miss, parses the
    /// file and stores the result.
    pub fn parse_cached(&self, path: &Path) -> SalataResult<ParsedDocument> {
        if let Some(doc) = self.get(path) {
            return Ok(doc);
        }
        let source = std::fs::read_to_string(path).map_err(crate::error::SalataError::Io)?;
        let doc = parser::parse(&source, path)?;
        self.put(path, &doc);
        Ok(doc)
    }

    /// Remove all entries from the cache.
    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.clear();
        }
    }

    /// Number of entries currently cached.
    pub fn len(&self) -> usize {
        self.entries.lock().map(|e| e.len()).unwrap_or(0)
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for ParseCache {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir() -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("salata_cache_test_{}_{}", std::process::id(), {
                use std::sync::atomic::{AtomicU64, Ordering};
                static C: AtomicU64 = AtomicU64::new(0);
                C.fetch_add(1, Ordering::Relaxed)
            }));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cache_new_is_empty() {
        let cache = ParseCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn cache_miss_returns_none() {
        let cache = ParseCache::new();
        assert!(cache.get(Path::new("/nonexistent/file.slt")).is_none());
    }

    #[test]
    fn cache_put_and_get() {
        let dir = temp_dir();
        let file = dir.join("test.slt");
        fs::write(&file, "<p>hello</p>").unwrap();

        let cache = ParseCache::new();
        let doc = parser::parse("<p>hello</p>", &file).unwrap();
        cache.put(&file, &doc);

        assert_eq!(cache.len(), 1);
        let cached = cache.get(&file).unwrap();
        assert_eq!(cached.segments.len(), doc.segments.len());

        cleanup(&dir);
    }

    #[test]
    fn cache_invalidated_on_mtime_change() {
        let dir = temp_dir();
        let file = dir.join("test.slt");
        fs::write(&file, "<p>v1</p>").unwrap();

        let cache = ParseCache::new();
        let doc = parser::parse("<p>v1</p>", &file).unwrap();
        cache.put(&file, &doc);
        assert!(cache.get(&file).is_some());

        // Modify the file — mtime changes, old cache key misses.
        std::thread::sleep(std::time::Duration::from_millis(50));
        fs::write(&file, "<p>v2</p>").unwrap();

        // parse_cached returns fresh content regardless of cache state.
        let fresh = cache.parse_cached(&file).unwrap();
        let html: String = fresh
            .segments
            .iter()
            .filter_map(|s| match s {
                crate::parser::Segment::Html(h) => Some(h.as_str()),
                _ => None,
            })
            .collect();
        assert!(html.contains("v2"));

        cleanup(&dir);
    }

    #[test]
    fn parse_cached_miss_then_hit() {
        let dir = temp_dir();
        let file = dir.join("cached.slt");
        fs::write(&file, "<p>cached</p>").unwrap();

        let cache = ParseCache::new();
        assert!(cache.is_empty());

        let doc1 = cache.parse_cached(&file).unwrap();
        assert_eq!(cache.len(), 1);

        let doc2 = cache.parse_cached(&file).unwrap();
        assert_eq!(doc1.segments.len(), doc2.segments.len());

        cleanup(&dir);
    }

    #[test]
    fn cache_clear() {
        let dir = temp_dir();
        let file = dir.join("clear.slt");
        fs::write(&file, "<p>clear</p>").unwrap();

        let cache = ParseCache::new();
        cache.parse_cached(&file).unwrap();
        assert!(!cache.is_empty());

        cache.clear();
        assert!(cache.is_empty());

        cleanup(&dir);
    }

    #[test]
    fn cache_multiple_files() {
        let dir = temp_dir();
        let f1 = dir.join("a.slt");
        let f2 = dir.join("b.slt");
        fs::write(&f1, "<p>a</p>").unwrap();
        fs::write(&f2, "<p>b</p>").unwrap();

        let cache = ParseCache::new();
        cache.parse_cached(&f1).unwrap();
        cache.parse_cached(&f2).unwrap();
        assert_eq!(cache.len(), 2);

        assert!(cache.get(&f1).is_some());
        assert!(cache.get(&f2).is_some());

        cleanup(&dir);
    }

    #[test]
    fn parse_cached_with_runtime_blocks() {
        let dir = temp_dir();
        let file = dir.join("blocks.slt");
        fs::write(
            &file,
            "<p>before</p>\n<python>\nprint('hi')\n</python>\n<p>after</p>",
        )
        .unwrap();

        let cache = ParseCache::new();
        let doc = cache.parse_cached(&file).unwrap();

        let has_python = doc.segments.iter().any(
            |s| matches!(s, crate::parser::Segment::RuntimeBlock(b) if b.language == "python"),
        );
        assert!(has_python);

        cleanup(&dir);
    }
}
