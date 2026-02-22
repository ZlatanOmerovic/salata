//! Directive extraction and resolution.
//!
//! Directives are pre-execution instructions that appear outside runtime blocks.
//! Processing happens in two phases:
//!
//! 1. **Extraction** (during parsing) — the parser identifies directive lines
//!    (`#include`, `#status`, `#content-type`, `#header`, `#cookie`, `#redirect`)
//!    and records them in the [`ParsedDocument`].
//!
//! 2. **Resolution** (after include expansion) — [`resolve_includes`] performs
//!    recursive text substitution of `#include` directives, then
//!    [`resolve_directives`] validates the remaining directives (no duplicates
//!    for `#status`/`#content-type`, no directives inside runtime blocks) and
//!    produces a [`ResolvedDirectives`] struct.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::error::{SalataError, SalataResult};
use crate::parser::{self, Directive, DirectiveKind, ParsedDocument, Segment};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum #include recursion depth.
const MAX_INCLUDE_DEPTH: usize = 16;

// ---------------------------------------------------------------------------
// Resolved directives
// ---------------------------------------------------------------------------

/// The result of resolving all directives in a document.
/// Produced before runtime execution begins.
#[derive(Debug, Clone)]
pub struct ResolvedDirectives {
    /// HTTP status code (default 200, overridden by `#status`).
    pub status: u16,
    /// Content-Type header (default from config, overridden by `#content-type`).
    pub content_type: Option<String>,
    /// Custom response headers from `#header` directives.
    pub headers: Vec<(String, String)>,
    /// Response cookies from `#cookie` directives.
    pub cookies: Vec<String>,
    /// Redirect location from `#redirect` directive.
    pub redirect: Option<String>,
}

impl Default for ResolvedDirectives {
    fn default() -> Self {
        Self {
            status: 200,
            content_type: None,
            headers: Vec::new(),
            cookies: Vec::new(),
            redirect: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Include resolution
// ---------------------------------------------------------------------------

/// Resolve `#include` directives by recursively substituting file contents.
/// Returns a new `ParsedDocument` with includes inlined and all directives
/// collected (but not yet validated for duplicates).
///
/// The `base_dir` is the directory relative to which include paths are resolved.
pub fn resolve_includes(doc: ParsedDocument, base_dir: &Path) -> SalataResult<ParsedDocument> {
    resolve_includes_inner(doc, base_dir, 0, &mut HashSet::new())
}

fn resolve_includes_inner(
    doc: ParsedDocument,
    base_dir: &Path,
    depth: usize,
    visited: &mut HashSet<PathBuf>,
) -> SalataResult<ParsedDocument> {
    if depth > MAX_INCLUDE_DEPTH {
        return Err(SalataError::IncludeDepthExceeded {
            file: doc.file.clone(),
            max_depth: MAX_INCLUDE_DEPTH,
        });
    }

    // Separate include directives from other directives.
    let mut includes: Vec<Directive> = Vec::new();
    let mut other_directives: Vec<Directive> = Vec::new();
    for d in doc.directives {
        match &d.kind {
            DirectiveKind::Include { .. } => includes.push(d),
            _ => other_directives.push(d),
        }
    }

    if includes.is_empty() {
        return Ok(ParsedDocument {
            file: doc.file,
            segments: doc.segments,
            directives: other_directives,
        });
    }

    // Read the original file, substitute includes at text level, then re-parse.
    // This handles nested directives in included files correctly.
    let source = std::fs::read_to_string(&doc.file).map_err(|e| SalataError::ConfigRead {
        path: doc.file.clone(),
        source: e,
    })?;

    let substituted = substitute_includes_text(&source, &doc.file, base_dir, depth, visited)?;
    let reparsed = parser::parse(&substituted, &doc.file)?;

    // The reparsed document may itself have includes (from included files).
    resolve_includes_inner(reparsed, base_dir, depth, visited)
}

/// Perform text-level include substitution: replace `#include "file"` lines
/// with the contents of the referenced files.
fn substitute_includes_text(
    source: &str,
    source_file: &Path,
    base_dir: &Path,
    depth: usize,
    visited: &mut HashSet<PathBuf>,
) -> SalataResult<String> {
    if depth > MAX_INCLUDE_DEPTH {
        return Err(SalataError::IncludeDepthExceeded {
            file: source_file.to_path_buf(),
            max_depth: MAX_INCLUDE_DEPTH,
        });
    }

    let mut result = String::with_capacity(source.len());
    let mut inside_runtime = false;

    for (line_idx, line) in source.lines().enumerate() {
        let line_num = line_idx + 1;
        let trimmed = line.trim();

        // Track runtime block boundaries (simplistic — matches parser logic).
        if !inside_runtime {
            if is_runtime_open(trimmed) {
                inside_runtime = true;
            }
        } else if is_runtime_close(trimmed) {
            inside_runtime = false;
        }

        if !inside_runtime {
            if let Some(path_str) = parse_include_line(trimmed) {
                let include_path = base_dir.join(&path_str);
                let canonical = include_path
                    .canonicalize()
                    .unwrap_or_else(|_| include_path.clone());

                if !include_path.exists() {
                    return Err(SalataError::IncludeNotFound {
                        path: include_path,
                        source_file: source_file.to_path_buf(),
                        line: line_num,
                    });
                }

                if !visited.insert(canonical.clone()) {
                    return Err(SalataError::IncludeDepthExceeded {
                        file: include_path,
                        max_depth: MAX_INCLUDE_DEPTH,
                    });
                }

                let included_source = std::fs::read_to_string(&include_path).map_err(|e| {
                    SalataError::ConfigRead {
                        path: include_path.clone(),
                        source: e,
                    }
                })?;

                let include_base = include_path.parent().unwrap_or(base_dir);

                let substituted = substitute_includes_text(
                    &included_source,
                    &include_path,
                    include_base,
                    depth + 1,
                    visited,
                )?;

                result.push_str(&substituted);
                if !substituted.ends_with('\n') {
                    result.push('\n');
                }

                visited.remove(&canonical);
                continue;
            }
        }

        result.push_str(line);
        result.push('\n');
    }

    Ok(result)
}

/// Quick check if a trimmed line is an `#include` directive. Returns the path if so.
fn parse_include_line(trimmed: &str) -> Option<String> {
    let rest = trimmed.strip_prefix("#include ")?;
    let rest = rest.trim();
    if rest.len() >= 2 && rest.starts_with('"') && rest.ends_with('"') {
        Some(rest[1..rest.len() - 1].to_string())
    } else {
        None
    }
}

/// Simplistic runtime open tag detection (matches RUNTIME_TAGS).
fn is_runtime_open(trimmed: &str) -> bool {
    const TAGS: &[&str] = &["python", "ruby", "javascript", "typescript", "php", "shell"];
    for tag in TAGS {
        if trimmed.starts_with(&format!("<{tag}>")) || trimmed.starts_with(&format!("<{tag} ")) {
            return true;
        }
    }
    false
}

/// Simplistic runtime close tag detection.
fn is_runtime_close(trimmed: &str) -> bool {
    const TAGS: &[&str] = &["python", "ruby", "javascript", "typescript", "php", "shell"];
    for tag in TAGS {
        if trimmed.contains(&format!("</{tag}>")) {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Directive validation & resolution
// ---------------------------------------------------------------------------

/// Validate and resolve directives from a parsed document.
/// Checks: no duplicate #status/#content-type, no directives inside runtime blocks.
///
/// Call this AFTER `resolve_includes` but BEFORE runtime execution.
pub fn resolve_directives(doc: &ParsedDocument) -> SalataResult<ResolvedDirectives> {
    validate_no_directives_in_runtime_blocks(doc)?;

    let mut resolved = ResolvedDirectives::default();
    let mut seen_status = false;
    let mut seen_content_type = false;

    for d in &doc.directives {
        match &d.kind {
            DirectiveKind::Include { .. } => {
                // Includes should have been resolved already. If one remains,
                // it means resolve_includes wasn't called — treat as error.
                return Err(SalataError::DirectiveInvalid {
                    file: doc.file.clone(),
                    line: d.line,
                    message: "unresolved #include (resolve_includes must run first)".into(),
                });
            }
            DirectiveKind::Status { code } => {
                if seen_status {
                    return Err(SalataError::DuplicateDirective {
                        directive: "status".into(),
                        file: doc.file.clone(),
                    });
                }
                resolved.status = *code;
                seen_status = true;
            }
            DirectiveKind::ContentType { mime } => {
                if seen_content_type {
                    return Err(SalataError::DuplicateDirective {
                        directive: "content-type".into(),
                        file: doc.file.clone(),
                    });
                }
                resolved.content_type = Some(mime.clone());
                seen_content_type = true;
            }
            DirectiveKind::Header { name, value } => {
                resolved.headers.push((name.clone(), value.clone()));
            }
            DirectiveKind::Cookie { raw } => {
                resolved.cookies.push(raw.clone());
            }
            DirectiveKind::Redirect { location } => {
                resolved.redirect = Some(location.clone());
            }
        }
    }

    Ok(resolved)
}

/// Scan runtime blocks for directive-like patterns and return an error if found.
/// Directives must NOT appear inside runtime blocks — they are HTML-level only.
fn validate_no_directives_in_runtime_blocks(doc: &ParsedDocument) -> SalataResult<()> {
    let directive_prefixes = &[
        "#include ",
        "#status ",
        "#content-type ",
        "#header ",
        "#cookie ",
        "#redirect ",
    ];

    for seg in &doc.segments {
        if let Segment::RuntimeBlock(block) = seg {
            for (i, line) in block.code.lines().enumerate() {
                let trimmed = line.trim();
                for prefix in directive_prefixes {
                    if trimmed.starts_with(prefix) {
                        return Err(SalataError::DirectiveInvalid {
                            file: doc.file.clone(),
                            line: block.start_line + i + 1,
                            message: format!(
                                "directive {} found inside <{}> block (directives are only allowed outside runtime blocks)",
                                prefix.trim(),
                                block.language
                            ),
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Convenience: full pipeline
// ---------------------------------------------------------------------------

/// Full directive pipeline: parse file, resolve includes, validate & resolve directives.
/// Returns the document (with includes inlined) and the resolved directives.
pub fn parse_and_resolve(
    source: &str,
    file: &Path,
    base_dir: &Path,
) -> SalataResult<(ParsedDocument, ResolvedDirectives)> {
    let doc = parser::parse(source, file)?;
    let doc = resolve_includes(doc, base_dir)?;
    let resolved = resolve_directives(&doc)?;
    Ok((doc, resolved))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use std::fs;
    use std::path::Path;

    // Helper: parse source and resolve directives (no includes).
    fn resolve(source: &str) -> SalataResult<ResolvedDirectives> {
        let doc = parser::parse(source, Path::new("test.slt"))?;
        resolve_directives(&doc)
    }

    fn resolve_err(source: &str) -> SalataError {
        resolve(source).expect_err("should fail")
    }

    // -- Status directive --

    #[test]
    fn default_status_200() {
        let r = resolve("<p>hello</p>").unwrap();
        assert_eq!(r.status, 200);
    }

    #[test]
    fn status_set_once() {
        let r = resolve("#status 404\n<p>not found</p>").unwrap();
        assert_eq!(r.status, 404);
    }

    #[test]
    fn status_301() {
        let r = resolve("#status 301\n#redirect \"/new\"\n").unwrap();
        assert_eq!(r.status, 301);
        assert_eq!(r.redirect.as_deref(), Some("/new"));
    }

    #[test]
    fn duplicate_status_rejected() {
        let err = resolve_err("#status 200\n#status 404\n<p>hi</p>");
        assert!(
            matches!(err, SalataError::DuplicateDirective { ref directive, .. } if directive == "status"),
            "expected DuplicateDirective for status, got: {err}"
        );
    }

    // -- Content-Type directive --

    #[test]
    fn default_content_type_none() {
        let r = resolve("<p>hello</p>").unwrap();
        assert!(r.content_type.is_none());
    }

    #[test]
    fn content_type_set() {
        let r = resolve("#content-type application/json\n{}").unwrap();
        assert_eq!(r.content_type.as_deref(), Some("application/json"));
    }

    #[test]
    fn duplicate_content_type_rejected() {
        let err = resolve_err("#content-type text/html\n#content-type application/json\n");
        assert!(
            matches!(err, SalataError::DuplicateDirective { ref directive, .. } if directive == "content-type"),
            "expected DuplicateDirective for content-type, got: {err}"
        );
    }

    // -- Header directive (multiple allowed) --

    #[test]
    fn single_header() {
        let r = resolve("#header \"X-Custom\" \"value\"\n<p>hi</p>").unwrap();
        assert_eq!(r.headers.len(), 1);
        assert_eq!(r.headers[0], ("X-Custom".into(), "value".into()));
    }

    #[test]
    fn multiple_headers_allowed() {
        let source = "#header \"X-One\" \"1\"\n#header \"X-Two\" \"2\"\n<p>hi</p>";
        let r = resolve(source).unwrap();
        assert_eq!(r.headers.len(), 2);
        assert_eq!(r.headers[0].0, "X-One");
        assert_eq!(r.headers[1].0, "X-Two");
    }

    // -- Cookie directive (multiple allowed) --

    #[test]
    fn single_cookie() {
        let r = resolve("#cookie \"session\" \"abc\" httponly\n<p>hi</p>").unwrap();
        assert_eq!(r.cookies.len(), 1);
        assert!(r.cookies[0].contains("session"));
    }

    #[test]
    fn multiple_cookies_allowed() {
        let source = "#cookie \"session\" \"abc\" httponly\n#cookie \"theme\" \"dark\"\n<p>hi</p>";
        let r = resolve(source).unwrap();
        assert_eq!(r.cookies.len(), 2);
    }

    // -- Redirect directive --

    #[test]
    fn redirect_set() {
        let r = resolve("#redirect \"/other-page\"\n").unwrap();
        assert_eq!(r.redirect.as_deref(), Some("/other-page"));
    }

    #[test]
    fn no_redirect_default() {
        let r = resolve("<p>hello</p>").unwrap();
        assert!(r.redirect.is_none());
    }

    // -- All directives together --

    #[test]
    fn all_directives_together() {
        let source = "\
#status 200
#content-type text/html
#header \"X-Frame-Options\" \"DENY\"
#cookie \"sid\" \"xyz\" httponly secure
#redirect \"/landing\"
<p>hello</p>
";
        let r = resolve(source).unwrap();
        assert_eq!(r.status, 200);
        assert_eq!(r.content_type.as_deref(), Some("text/html"));
        assert_eq!(r.headers.len(), 1);
        assert_eq!(r.cookies.len(), 1);
        assert_eq!(r.redirect.as_deref(), Some("/landing"));
    }

    // -- Directives inside runtime blocks rejected --

    #[test]
    fn status_inside_runtime_rejected() {
        let source = "<python>\n#status 404\nprint('hi')\n</python>\n";
        let err = resolve_err(source);
        assert!(
            matches!(err, SalataError::DirectiveInvalid { ref message, .. } if message.contains("#status")),
            "expected DirectiveInvalid for #status in block, got: {err}"
        );
    }

    #[test]
    fn include_inside_runtime_rejected() {
        let source = "<ruby>\n#include \"header.slt\"\nputs 'hi'\n</ruby>\n";
        let err = resolve_err(source);
        assert!(
            matches!(err, SalataError::DirectiveInvalid { ref message, .. } if message.contains("#include")),
            "expected DirectiveInvalid for #include in block, got: {err}"
        );
    }

    #[test]
    fn header_inside_runtime_rejected() {
        let source = "<javascript>\n#header \"X\" \"Y\"\nconsole.log('hi');\n</javascript>\n";
        let err = resolve_err(source);
        assert!(
            matches!(err, SalataError::DirectiveInvalid { ref message, .. } if message.contains("#header")),
            "expected DirectiveInvalid for #header in block, got: {err}"
        );
    }

    #[test]
    fn content_type_inside_runtime_rejected() {
        let source = "<php>\n#content-type text/plain\necho 'hi';\n</php>\n";
        let err = resolve_err(source);
        assert!(
            matches!(err, SalataError::DirectiveInvalid { ref message, .. } if message.contains("#content-type")),
            "expected DirectiveInvalid, got: {err}"
        );
    }

    #[test]
    fn cookie_inside_runtime_rejected() {
        let source = "<shell>\n#cookie \"x\" \"y\"\necho hi\n</shell>\n";
        let err = resolve_err(source);
        assert!(
            matches!(err, SalataError::DirectiveInvalid { ref message, .. } if message.contains("#cookie")),
            "expected DirectiveInvalid, got: {err}"
        );
    }

    #[test]
    fn redirect_inside_runtime_rejected() {
        let source = "<typescript>\n#redirect \"/bad\"\nconsole.log('hi');\n</typescript>\n";
        let err = resolve_err(source);
        assert!(
            matches!(err, SalataError::DirectiveInvalid { ref message, .. } if message.contains("#redirect")),
            "expected DirectiveInvalid, got: {err}"
        );
    }

    // -- Macros inside runtime blocks NOT rejected (they belong there) --

    #[test]
    fn set_get_macros_inside_runtime_ok() {
        let source = "<python>\n#set(\"key\", 42)\nval = #get(\"key\")\n</python>\n";
        let r = resolve(source);
        assert!(r.is_ok(), "macros inside runtime blocks should be allowed");
    }

    // -- #include resolution (filesystem tests) --

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "salata_directive_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn include_basic() {
        let dir = temp_dir();
        let header = "<header>Navigation</header>\n";
        fs::write(dir.join("header.slt"), header).unwrap();

        let main_source = "#include \"header.slt\"\n<p>body</p>\n";
        let main_file = dir.join("index.slt");
        fs::write(&main_file, main_source).unwrap();

        let doc = parser::parse(main_source, &main_file).unwrap();
        let doc = resolve_includes(doc, &dir).unwrap();

        // The included content should be present in segments.
        let html: String = doc
            .segments
            .iter()
            .filter_map(|s| match s {
                Segment::Html(h) => Some(h.as_str()),
                _ => None,
            })
            .collect();
        assert!(html.contains("<header>Navigation</header>"));
        assert!(html.contains("<p>body</p>"));

        cleanup(&dir);
    }

    #[test]
    fn include_with_runtime_blocks() {
        let dir = temp_dir();
        let partial = "<python>\nprint('from include')\n</python>\n";
        fs::write(dir.join("partial.slt"), partial).unwrap();

        let main_source = "<p>before</p>\n#include \"partial.slt\"\n<p>after</p>\n";
        let main_file = dir.join("index.slt");
        fs::write(&main_file, main_source).unwrap();

        let doc = parser::parse(main_source, &main_file).unwrap();
        let doc = resolve_includes(doc, &dir).unwrap();

        let has_python = doc.segments.iter().any(|s| {
            matches!(s, Segment::RuntimeBlock(b) if b.language == "python" && b.code.contains("from include"))
        });
        assert!(has_python, "included runtime block should be present");

        cleanup(&dir);
    }

    #[test]
    fn include_nested() {
        let dir = temp_dir();
        fs::write(dir.join("inner.slt"), "<span>inner</span>\n").unwrap();
        fs::write(
            dir.join("middle.slt"),
            "#include \"inner.slt\"\n<span>middle</span>\n",
        )
        .unwrap();

        let main_source = "#include \"middle.slt\"\n<p>main</p>\n";
        let main_file = dir.join("index.slt");
        fs::write(&main_file, main_source).unwrap();

        let doc = parser::parse(main_source, &main_file).unwrap();
        let doc = resolve_includes(doc, &dir).unwrap();

        let html: String = doc
            .segments
            .iter()
            .filter_map(|s| match s {
                Segment::Html(h) => Some(h.as_str()),
                _ => None,
            })
            .collect();
        assert!(html.contains("inner"));
        assert!(html.contains("middle"));
        assert!(html.contains("main"));

        cleanup(&dir);
    }

    #[test]
    fn include_with_directives_in_included_file() {
        let dir = temp_dir();
        let header = "#header \"X-Included\" \"yes\"\n<nav>nav</nav>\n";
        fs::write(dir.join("header.slt"), header).unwrap();

        let main_source = "#status 200\n#include \"header.slt\"\n<p>body</p>\n";
        let main_file = dir.join("index.slt");
        fs::write(&main_file, main_source).unwrap();

        let doc = parser::parse(main_source, &main_file).unwrap();
        let doc = resolve_includes(doc, &dir).unwrap();
        let resolved = resolve_directives(&doc).unwrap();

        assert_eq!(resolved.status, 200);
        assert_eq!(resolved.headers.len(), 1);
        assert_eq!(resolved.headers[0].0, "X-Included");

        cleanup(&dir);
    }

    #[test]
    fn include_depth_limit() {
        let dir = temp_dir();
        // Create a chain of includes that exceeds the max depth.
        for i in 0..=MAX_INCLUDE_DEPTH + 1 {
            let content = if i <= MAX_INCLUDE_DEPTH {
                format!("#include \"file{}.slt\"\n", i + 1)
            } else {
                "<p>end</p>\n".to_string()
            };
            fs::write(dir.join(format!("file{i}.slt")), content).unwrap();
        }

        let main_source = "#include \"file0.slt\"\n";
        let main_file = dir.join("index.slt");
        fs::write(&main_file, main_source).unwrap();

        let doc = parser::parse(main_source, &main_file).unwrap();
        let result = resolve_includes(doc, &dir);

        assert!(
            matches!(result, Err(SalataError::IncludeDepthExceeded { .. })),
            "expected IncludeDepthExceeded, got: {result:?}"
        );

        cleanup(&dir);
    }

    #[test]
    fn include_circular_detected() {
        let dir = temp_dir();
        fs::write(dir.join("a.slt"), "#include \"b.slt\"\n").unwrap();
        fs::write(dir.join("b.slt"), "#include \"a.slt\"\n").unwrap();

        let main_source = "#include \"a.slt\"\n";
        let main_file = dir.join("index.slt");
        fs::write(&main_file, main_source).unwrap();

        let doc = parser::parse(main_source, &main_file).unwrap();
        let result = resolve_includes(doc, &dir);

        assert!(result.is_err(), "circular includes should be detected");

        cleanup(&dir);
    }

    #[test]
    fn include_file_not_found() {
        let dir = temp_dir();
        let main_source = "#include \"nonexistent.slt\"\n<p>body</p>\n";
        let main_file = dir.join("index.slt");
        fs::write(&main_file, main_source).unwrap();

        let doc = parser::parse(main_source, &main_file).unwrap();
        let result = resolve_includes(doc, &dir);

        assert!(
            matches!(result, Err(SalataError::IncludeNotFound { .. })),
            "expected IncludeNotFound, got: {result:?}"
        );

        cleanup(&dir);
    }

    // -- parse_and_resolve convenience --

    #[test]
    fn parse_and_resolve_no_includes() {
        let dir = temp_dir();
        let source = "#status 201\n#header \"X-Test\" \"1\"\n<p>hello</p>\n";
        let file = dir.join("test.slt");
        fs::write(&file, source).unwrap();

        let (doc, resolved) = parse_and_resolve(source, &file, &dir).unwrap();
        assert_eq!(resolved.status, 201);
        assert_eq!(resolved.headers.len(), 1);
        assert!(doc
            .segments
            .iter()
            .any(|s| matches!(s, Segment::Html(h) if h.contains("hello"))));

        cleanup(&dir);
    }

    // -- Unresolved include error --

    #[test]
    fn unresolved_include_in_resolve_directives() {
        // Manually construct a doc with an #include still present (simulating
        // resolve_includes not being called).
        let doc = ParsedDocument {
            file: PathBuf::from("test.slt"),
            segments: vec![Segment::Html("<p>hi</p>\n".into())],
            directives: vec![Directive {
                kind: DirectiveKind::Include {
                    path: "header.slt".into(),
                },
                line: 1,
            }],
        };
        let result = resolve_directives(&doc);
        assert!(
            matches!(result, Err(SalataError::DirectiveInvalid { ref message, .. }) if message.contains("unresolved")),
            "expected unresolved #include error, got: {result:?}"
        );
    }
}
