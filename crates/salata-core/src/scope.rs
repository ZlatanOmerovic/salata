//! Scope management — shared vs isolated process scope per runtime.
//!
//! In **shared scope** (default), all blocks of the same language within a page
//! run in a single process. Variables and state persist between blocks. Salata
//! concatenates blocks with `__SALATA_BLOCK_BOUNDARY__` markers, sends them as
//! one script, then splits the output back into per-block results.
//!
//! In **isolated scope**, each block spawns a fresh process. This can be set
//! globally per runtime (`shared_scope = false` in config) or per block
//! (`<python scope="isolated">`).

use std::collections::HashMap;

use crate::config::SalataConfig;

/// Build a map of language → shared_scope from config.
/// Only includes enabled runtimes.
/// Used by `runtime::execute_blocks` to determine scope mode per language.
pub fn shared_scope_map(config: &SalataConfig) -> HashMap<String, bool> {
    let mut map = HashMap::new();

    if let Some(r) = &config.runtimes.python {
        if r.enabled {
            map.insert("python".into(), r.shared_scope);
        }
    }
    if let Some(r) = &config.runtimes.ruby {
        if r.enabled {
            map.insert("ruby".into(), r.shared_scope);
        }
    }
    if let Some(r) = &config.runtimes.javascript {
        if r.enabled {
            map.insert("javascript".into(), r.shared_scope);
        }
    }
    if let Some(r) = &config.runtimes.typescript {
        if r.enabled {
            map.insert("typescript".into(), r.shared_scope);
        }
    }
    if let Some(r) = &config.runtimes.php {
        if r.enabled {
            map.insert("php".into(), r.shared_scope);
        }
    }
    if let Some(r) = &config.runtimes.shell {
        if r.enabled {
            map.insert("shell".into(), r.shared_scope);
        }
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_scope_map_defaults() {
        let config = SalataConfig::parse(
            r#"
[runtimes.python]
path = "/usr/bin/python3"

[runtimes.ruby]
path = "/usr/bin/ruby"
shared_scope = false
"#,
        )
        .unwrap();

        let map = shared_scope_map(&config);
        assert_eq!(map.get("python"), Some(&true));
        assert_eq!(map.get("ruby"), Some(&false));
        assert!(!map.contains_key("javascript"));
    }

    #[test]
    fn shared_scope_map_empty_config() {
        let config = SalataConfig::parse("").unwrap();
        let map = shared_scope_map(&config);
        assert!(map.is_empty());
    }
}
