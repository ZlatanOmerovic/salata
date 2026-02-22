//! Parser for `.slt` (Salata template) files.
//!
//! Extracts runtime blocks (`<python>`, `<ruby>`, etc.), directives (`#include`,
//! `#status`, etc.), and raw HTML/text segments from a source string. Code inside
//! runtime tags is automatically dedented (common leading whitespace removed) so
//! users can indent blocks within HTML without causing indentation errors.
//!
//! # Example
//!
//! ```text
//! <h1>Title</h1>
//! <python>
//!   name = "World"
//!   print(f"Hello, {name}!")
//! </python>
//! <p>Footer</p>
//! ```
//!
//! Parses into: `[Html("<h1>Title</h1>\n"), RuntimeBlock("python", "name = ..."), Html("<p>Footer</p>\n")]`

use std::path::{Path, PathBuf};

use crate::error::{SalataError, SalataResult};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// The six runtime tag names recognized by Salata.
const RUNTIME_TAGS: &[&str] = &["python", "ruby", "javascript", "typescript", "php", "shell"];

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A parsed `.slt` document — the result of parsing before execution.
#[derive(Debug, Clone)]
pub struct ParsedDocument {
    /// The source file path (for error messages).
    pub file: PathBuf,
    /// Ordered list of segments that make up the document.
    pub segments: Vec<Segment>,
    /// Directives extracted from the document (not yet resolved).
    pub directives: Vec<Directive>,
}

/// A segment of the document — either raw HTML or a runtime code block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    /// Raw HTML/text content to pass through unchanged.
    Html(String),
    /// A runtime code block extracted from a `<language>...</language>` tag.
    RuntimeBlock(RuntimeBlock),
}

/// A single runtime code block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBlock {
    /// The runtime language (e.g. "python", "ruby").
    pub language: String,
    /// The source code inside the tag.
    pub code: String,
    /// Line number where the opening tag starts (1-based).
    pub start_line: usize,
    /// Optional scope attribute from the tag.
    pub scope: Option<BlockScope>,
}

/// Per-block scope override via `scope="isolated"` attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockScope {
    Isolated,
}

/// A directive extracted from the document (not yet resolved).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directive {
    /// The directive type.
    pub kind: DirectiveKind,
    /// Line number where the directive appears (1-based).
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectiveKind {
    /// `#include "file.slt"`
    Include { path: String },
    /// `#status 404`
    Status { code: u16 },
    /// `#content-type application/json`
    ContentType { mime: String },
    /// `#header "Name" "Value"`
    Header { name: String, value: String },
    /// `#cookie "name" "value" [flags...]`
    Cookie { raw: String },
    /// `#redirect "/path"`
    Redirect { location: String },
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// Parse an `.slt` file's contents into a `ParsedDocument`.
///
/// This extracts runtime blocks and directives but does NOT resolve them
/// (no #include substitution, no macro expansion).
pub fn parse(source: &str, file: &Path) -> SalataResult<ParsedDocument> {
    let mut segments: Vec<Segment> = Vec::new();
    let mut directives: Vec<Directive> = Vec::new();

    let mut current_html = String::new();
    let mut inside_runtime: Option<(&str, usize, Option<BlockScope>)> = None; // (tag, start_line, scope)
    let mut runtime_code = String::new();

    for (line_idx, line) in source.lines().enumerate() {
        let line_num = line_idx + 1;

        match inside_runtime {
            None => {
                // Check for runtime opening tag.
                if let Some((tag, scope)) = match_runtime_open(line) {
                    // Flush accumulated HTML.
                    flush_html(&mut current_html, &mut segments);
                    inside_runtime = Some((tag, line_num, scope));
                    runtime_code.clear();
                    // If there's content after the opening tag on the same line,
                    // include it as code.
                    if let Some(after) = content_after_open_tag(line, tag) {
                        // But first check if the closing tag is also on this line.
                        if let Some((code, remainder)) = split_inline_close(after, tag) {
                            segments.push(Segment::RuntimeBlock(RuntimeBlock {
                                language: tag.to_string(),
                                code: code.to_string(),
                                start_line: line_num,
                                scope,
                            }));
                            inside_runtime = None;
                            // Any content after closing tag goes to HTML.
                            if !remainder.is_empty() {
                                current_html.push_str(remainder);
                                current_html.push('\n');
                            }
                        } else {
                            runtime_code.push_str(after);
                            runtime_code.push('\n');
                        }
                    }
                } else if let Some(tag) = detect_runtime_close_outside(line) {
                    return Err(SalataError::Parse {
                        file: file.to_path_buf(),
                        line: line_num,
                        message: format!(
                            "unexpected closing tag </{tag}> without matching opening tag"
                        ),
                    });
                } else {
                    // Check for directives in HTML context.
                    let trimmed = line.trim();
                    if let Some(directive) = try_parse_directive(trimmed, line_num, file)? {
                        // Directive lines are consumed — not included in HTML output.
                        directives.push(directive);
                    } else {
                        current_html.push_str(line);
                        current_html.push('\n');
                    }
                }
            }
            Some((tag, start_line, scope)) => {
                // Check for nested runtime opening tags.
                if let Some((nested_tag, _)) = match_runtime_open(line) {
                    return Err(SalataError::NestedRuntimeTag {
                        tag: nested_tag.to_string(),
                        outer: tag.to_string(),
                        file: file.to_path_buf(),
                        line: line_num,
                    });
                }

                // Check for closing tag.
                let close_tag = format!("</{tag}>");
                if let Some(pos) = line.find(&close_tag) {
                    // Content before the closing tag is code.
                    let before = &line[..pos];
                    if !before.is_empty() {
                        runtime_code.push_str(before);
                    }
                    // Strip common leading whitespace so indented blocks work
                    // correctly, then remove any trailing newline.
                    let code = strip_trailing_newline(&dedent(&strip_trailing_newline(&runtime_code)));
                    segments.push(Segment::RuntimeBlock(RuntimeBlock {
                        language: tag.to_string(),
                        code,
                        start_line,
                        scope,
                    }));
                    inside_runtime = None;
                    runtime_code.clear();

                    // Content after closing tag goes to HTML.
                    let after = &line[pos + close_tag.len()..];
                    if !after.is_empty() {
                        current_html.push_str(after);
                        current_html.push('\n');
                    }
                } else {
                    runtime_code.push_str(line);
                    runtime_code.push('\n');
                }
            }
        }
    }

    // Error if we're still inside a runtime block at EOF.
    if let Some((tag, start_line, _)) = inside_runtime {
        return Err(SalataError::Parse {
            file: file.to_path_buf(),
            line: start_line,
            message: format!("unclosed <{tag}> tag"),
        });
    }

    // Flush any remaining HTML.
    flush_html(&mut current_html, &mut segments);

    Ok(ParsedDocument {
        file: file.to_path_buf(),
        segments,
        directives,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn flush_html(html: &mut String, segments: &mut Vec<Segment>) {
    if !html.is_empty() {
        segments.push(Segment::Html(std::mem::take(html)));
    }
}

fn strip_trailing_newline(s: &str) -> String {
    s.strip_suffix('\n').unwrap_or(s).to_string()
}

/// Remove common leading whitespace from all non-empty lines (like Python's
/// `textwrap.dedent`). This lets users indent code inside runtime tags without
/// causing indentation errors in whitespace-sensitive languages like Python.
fn dedent(s: &str) -> String {
    // Find the minimum indentation across all non-empty lines.
    let min_indent = s
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    if min_indent == 0 {
        return s.to_string();
    }

    s.lines()
        .map(|line| {
            if line.trim().is_empty() {
                ""
            } else {
                &line[min_indent..]
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Check if a line opens a runtime tag. Returns `(tag_name, scope)` if so.
/// Handles `<python>`, `<python scope="isolated">`, etc.
fn match_runtime_open(line: &str) -> Option<(&'static str, Option<BlockScope>)> {
    let trimmed = line.trim();
    for &tag in RUNTIME_TAGS {
        // Exact match: `<tag>` or starts with `<tag>` or `<tag ` (with attributes).
        let open_exact = format!("<{tag}>");
        let open_attr = format!("<{tag} ");

        // Check if the line contains the opening tag.
        if trimmed.starts_with(&open_exact)
            || trimmed.starts_with(&open_attr)
            || line.contains(&open_exact)
            || line.contains(&open_attr)
        {
            // Confirm it's actually an opening tag and not something like `</tag>`.
            if let Some(idx) = line.find(&format!("<{tag}")) {
                // Make sure it's not a closing tag.
                if idx > 0 && line.as_bytes()[idx - 1] == b'/' {
                    continue;
                }
                // Check the char after `<tag` — must be `>` or whitespace.
                let after_tag = &line[idx + 1 + tag.len()..];
                if after_tag.starts_with('>') || after_tag.starts_with(' ') {
                    let scope = parse_scope_attr(after_tag);
                    return Some((tag, scope));
                }
            }
        }
    }
    None
}

/// Parse the `scope="isolated"` attribute from the text after the tag name.
fn parse_scope_attr(after_tag: &str) -> Option<BlockScope> {
    // Simple attribute parsing — look for scope="isolated".
    if let Some(idx) = after_tag.find("scope=") {
        let rest = &after_tag[idx + 6..];
        let rest = rest.trim_start_matches('"').trim_start_matches('\'');
        if rest.starts_with("isolated") {
            return Some(BlockScope::Isolated);
        }
    }
    None
}

/// Extract content after the opening tag on the same line.
/// E.g., for `<python>print("hi")</python>`, returns `Some(r#"print("hi")</python>"#)`.
fn content_after_open_tag<'a>(line: &'a str, tag: &str) -> Option<&'a str> {
    // Find the `>` that closes the opening tag.
    let open_start = line.find(&format!("<{tag}"))?;
    let rest = &line[open_start..];
    let gt_pos = rest.find('>')?;
    let after = &rest[gt_pos + 1..];
    if after.is_empty() {
        None
    } else {
        Some(after)
    }
}

/// Check if the content contains the closing tag for `tag`.
/// Returns `(code_before, content_after)` if found.
fn split_inline_close<'a>(content: &'a str, tag: &str) -> Option<(&'a str, &'a str)> {
    let close = format!("</{tag}>");
    let pos = content.find(&close)?;
    Some((&content[..pos], &content[pos + close.len()..]))
}

/// Detect a closing runtime tag on a line that's outside any runtime block.
fn detect_runtime_close_outside(line: &str) -> Option<&'static str> {
    for &tag in RUNTIME_TAGS {
        let close = format!("</{tag}>");
        if line.contains(&close) {
            return Some(tag);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Directive parsing
// ---------------------------------------------------------------------------

/// Try to parse a directive from a trimmed line. Returns `None` if the line
/// is not a directive. Returns an error for malformed directives.
fn try_parse_directive(
    trimmed: &str,
    line_num: usize,
    file: &Path,
) -> SalataResult<Option<Directive>> {
    if !trimmed.starts_with('#') {
        return Ok(None);
    }

    // Skip macro-like patterns (#set, #get) — those are only valid inside runtime blocks
    // and will be handled by the macro expander.
    if trimmed.starts_with("#set(") || trimmed.starts_with("#get(") {
        return Ok(None);
    }

    if let Some(rest) = trimmed.strip_prefix("#include ") {
        let path =
            parse_quoted_string(rest.trim()).ok_or_else(|| SalataError::DirectiveInvalid {
                file: file.to_path_buf(),
                line: line_num,
                message: "#include requires a quoted path".into(),
            })?;
        return Ok(Some(Directive {
            kind: DirectiveKind::Include { path },
            line: line_num,
        }));
    }

    if let Some(rest) = trimmed.strip_prefix("#status ") {
        let code: u16 = rest
            .trim()
            .parse()
            .map_err(|_| SalataError::DirectiveInvalid {
                file: file.to_path_buf(),
                line: line_num,
                message: format!("#status requires a numeric code, got: {rest}"),
            })?;
        return Ok(Some(Directive {
            kind: DirectiveKind::Status { code },
            line: line_num,
        }));
    }

    if let Some(rest) = trimmed.strip_prefix("#content-type ") {
        let mime = rest.trim().to_string();
        if mime.is_empty() {
            return Err(SalataError::DirectiveInvalid {
                file: file.to_path_buf(),
                line: line_num,
                message: "#content-type requires a MIME type".into(),
            });
        }
        return Ok(Some(Directive {
            kind: DirectiveKind::ContentType { mime },
            line: line_num,
        }));
    }

    if let Some(rest) = trimmed.strip_prefix("#header ") {
        let (name, value) =
            parse_two_quoted_strings(rest.trim()).ok_or_else(|| SalataError::DirectiveInvalid {
                file: file.to_path_buf(),
                line: line_num,
                message: "#header requires two quoted strings".into(),
            })?;
        return Ok(Some(Directive {
            kind: DirectiveKind::Header { name, value },
            line: line_num,
        }));
    }

    if let Some(rest) = trimmed.strip_prefix("#cookie ") {
        let raw = rest.trim().to_string();
        if raw.is_empty() {
            return Err(SalataError::DirectiveInvalid {
                file: file.to_path_buf(),
                line: line_num,
                message: "#cookie requires arguments".into(),
            });
        }
        return Ok(Some(Directive {
            kind: DirectiveKind::Cookie { raw },
            line: line_num,
        }));
    }

    if let Some(rest) = trimmed.strip_prefix("#redirect ") {
        let location = parse_quoted_string(rest.trim()).unwrap_or_else(|| rest.trim().to_string());
        if location.is_empty() {
            return Err(SalataError::DirectiveInvalid {
                file: file.to_path_buf(),
                line: line_num,
                message: "#redirect requires a location".into(),
            });
        }
        return Ok(Some(Directive {
            kind: DirectiveKind::Redirect { location },
            line: line_num,
        }));
    }

    // Not a recognized directive — treat as regular HTML (could be a CSS color like #fff).
    Ok(None)
}

/// Parse a double-quoted string, returning the inner content.
fn parse_quoted_string(s: &str) -> Option<String> {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        Some(s[1..s.len() - 1].to_string())
    } else {
        None
    }
}

/// Parse two consecutive double-quoted strings: `"foo" "bar"`.
fn parse_two_quoted_strings(s: &str) -> Option<(String, String)> {
    let s = s.trim();
    if !s.starts_with('"') {
        return None;
    }
    // Find the end of the first quoted string.
    let end_first = s[1..].find('"')? + 1;
    let first = s[1..end_first].to_string();
    let rest = s[end_first + 1..].trim();
    if !rest.starts_with('"') || !rest.ends_with('"') || rest.len() < 2 {
        return None;
    }
    let second = rest[1..rest.len() - 1].to_string();
    Some((first, second))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn p(source: &str) -> ParsedDocument {
        parse(source, Path::new("test.slt")).expect("parse should succeed")
    }

    fn perr(source: &str) -> SalataError {
        parse(source, Path::new("test.slt")).expect_err("parse should fail")
    }

    // -- Basic parsing --

    #[test]
    fn pure_html() {
        let doc = p("<h1>Hello</h1>\n<p>World</p>\n");
        assert_eq!(doc.segments.len(), 1);
        assert!(matches!(&doc.segments[0], Segment::Html(h) if h.contains("Hello")));
        assert!(doc.directives.is_empty());
    }

    #[test]
    fn single_python_block() {
        let doc = p("<html>\n<python>\nprint('hi')\n</python>\n</html>\n");
        assert_eq!(doc.segments.len(), 3); // html, runtime, html
        match &doc.segments[1] {
            Segment::RuntimeBlock(b) => {
                assert_eq!(b.language, "python");
                assert_eq!(b.code, "print('hi')");
                assert_eq!(b.start_line, 2);
                assert!(b.scope.is_none());
            }
            _ => panic!("expected RuntimeBlock"),
        }
    }

    #[test]
    fn single_ruby_block() {
        let doc = p("<ruby>\nputs 'hello'\n</ruby>\n");
        assert_eq!(doc.segments.len(), 1);
        match &doc.segments[0] {
            Segment::RuntimeBlock(b) => {
                assert_eq!(b.language, "ruby");
                assert_eq!(b.code, "puts 'hello'");
            }
            _ => panic!("expected RuntimeBlock"),
        }
    }

    // -- Multiple runtimes --

    #[test]
    fn multiple_runtime_blocks() {
        let source = "\
<python>
x = 1
</python>
<p>middle</p>
<javascript>
console.log(2);
</javascript>
";
        let doc = p(source);
        let runtimes: Vec<&RuntimeBlock> = doc
            .segments
            .iter()
            .filter_map(|s| match s {
                Segment::RuntimeBlock(b) => Some(b),
                _ => None,
            })
            .collect();
        assert_eq!(runtimes.len(), 2);
        assert_eq!(runtimes[0].language, "python");
        assert_eq!(runtimes[0].code, "x = 1");
        assert_eq!(runtimes[1].language, "javascript");
        assert_eq!(runtimes[1].code, "console.log(2);");
    }

    #[test]
    fn all_six_runtimes() {
        let source = "\
<python>1</python>
<ruby>2</ruby>
<javascript>3</javascript>
<typescript>4</typescript>
<php>5</php>
<shell>6</shell>
";
        let doc = p(source);
        let langs: Vec<&str> = doc
            .segments
            .iter()
            .filter_map(|s| match s {
                Segment::RuntimeBlock(b) => Some(b.language.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            langs,
            vec!["python", "ruby", "javascript", "typescript", "php", "shell"]
        );
    }

    // -- Inline runtime tags --

    #[test]
    fn inline_runtime_tag() {
        let doc = p("<p><python>print('hi')</python></p>\n");
        let runtimes: Vec<&RuntimeBlock> = doc
            .segments
            .iter()
            .filter_map(|s| match s {
                Segment::RuntimeBlock(b) => Some(b),
                _ => None,
            })
            .collect();
        assert_eq!(runtimes.len(), 1);
        assert_eq!(runtimes[0].code, "print('hi')");
    }

    #[test]
    fn inline_tag_with_surrounding_html() {
        let doc = p("<div><ruby>puts 'x'</ruby></div>\n");
        // Should have: html "<div>", runtime block, html "</div>\n"
        assert!(doc.segments.len() >= 2);
        let runtimes: Vec<&RuntimeBlock> = doc
            .segments
            .iter()
            .filter_map(|s| match s {
                Segment::RuntimeBlock(b) => Some(b),
                _ => None,
            })
            .collect();
        assert_eq!(runtimes.len(), 1);
        assert_eq!(runtimes[0].language, "ruby");
    }

    // -- Nested tag rejection --

    #[test]
    fn nested_runtime_tag_rejected() {
        let source = "<python>\n<ruby>\nputs 'bad'\n</ruby>\n</python>\n";
        let err = perr(source);
        assert!(
            matches!(err, SalataError::NestedRuntimeTag { ref tag, ref outer, .. }
                if tag == "ruby" && outer == "python"),
            "expected NestedRuntimeTag, got: {err}"
        );
    }

    #[test]
    fn nested_same_language_rejected() {
        let source = "<python>\n<python>\nprint('bad')\n</python>\n</python>\n";
        let err = perr(source);
        assert!(matches!(err, SalataError::NestedRuntimeTag { .. }));
    }

    #[test]
    fn unclosed_tag_errors() {
        let source = "<python>\nprint('hi')\n";
        let err = perr(source);
        assert!(
            matches!(err, SalataError::Parse { ref message, .. } if message.contains("unclosed")),
            "expected unclosed tag error, got: {err}"
        );
    }

    #[test]
    fn stray_closing_tag_errors() {
        let source = "</python>\n";
        let err = perr(source);
        assert!(
            matches!(err, SalataError::Parse { ref message, .. } if message.contains("unexpected closing")),
            "expected unexpected closing tag error, got: {err}"
        );
    }

    // -- Style and script passthrough --

    #[test]
    fn style_tag_passthrough() {
        let source = "<style>\nbody { color: red; }\n</style>\n";
        let doc = p(source);
        assert_eq!(doc.segments.len(), 1);
        match &doc.segments[0] {
            Segment::Html(h) => assert!(h.contains("<style>")),
            _ => panic!("expected Html segment"),
        }
    }

    #[test]
    fn script_tag_passthrough() {
        let source = "<script>\nconsole.log('client');\n</script>\n";
        let doc = p(source);
        assert_eq!(doc.segments.len(), 1);
        match &doc.segments[0] {
            Segment::Html(h) => {
                assert!(h.contains("<script>"));
                assert!(h.contains("console.log"));
            }
            _ => panic!("expected Html segment"),
        }
    }

    #[test]
    fn mixed_script_and_runtime() {
        let source = "\
<script>var x = 1;</script>
<python>
print('server')
</python>
<script>var y = 2;</script>
";
        let doc = p(source);
        let html_segments: Vec<&str> = doc
            .segments
            .iter()
            .filter_map(|s| match s {
                Segment::Html(h) => Some(h.as_str()),
                _ => None,
            })
            .collect();
        let all_html: String = html_segments.join("");
        assert!(all_html.contains("<script>var x = 1;</script>"));
        assert!(all_html.contains("<script>var y = 2;</script>"));
        assert!(!all_html.contains("print('server')"));
    }

    // -- Directive extraction --

    #[test]
    fn include_directive() {
        let doc = p("#include \"header.slt\"\n<p>body</p>\n");
        assert_eq!(doc.directives.len(), 1);
        assert_eq!(
            doc.directives[0].kind,
            DirectiveKind::Include {
                path: "header.slt".into()
            }
        );
        assert_eq!(doc.directives[0].line, 1);
    }

    #[test]
    fn status_directive() {
        let doc = p("#status 404\n<p>not found</p>\n");
        assert_eq!(doc.directives.len(), 1);
        assert_eq!(doc.directives[0].kind, DirectiveKind::Status { code: 404 });
    }

    #[test]
    fn content_type_directive() {
        let doc = p("#content-type application/json\n{}\n");
        assert_eq!(doc.directives.len(), 1);
        assert_eq!(
            doc.directives[0].kind,
            DirectiveKind::ContentType {
                mime: "application/json".into()
            }
        );
    }

    #[test]
    fn header_directive() {
        let doc = p("#header \"X-Custom\" \"value\"\n<p>hi</p>\n");
        assert_eq!(doc.directives.len(), 1);
        assert_eq!(
            doc.directives[0].kind,
            DirectiveKind::Header {
                name: "X-Custom".into(),
                value: "value".into()
            }
        );
    }

    #[test]
    fn cookie_directive() {
        let doc = p("#cookie \"session\" \"abc123\" httponly secure\n");
        assert_eq!(doc.directives.len(), 1);
        match &doc.directives[0].kind {
            DirectiveKind::Cookie { raw } => {
                assert!(raw.contains("session"));
                assert!(raw.contains("abc123"));
                assert!(raw.contains("httponly"));
            }
            _ => panic!("expected Cookie directive"),
        }
    }

    #[test]
    fn redirect_directive() {
        let doc = p("#redirect \"/other-page\"\n");
        assert_eq!(doc.directives.len(), 1);
        assert_eq!(
            doc.directives[0].kind,
            DirectiveKind::Redirect {
                location: "/other-page".into()
            }
        );
    }

    #[test]
    fn multiple_directives() {
        let source = "\
#status 200
#content-type text/html
#header \"X-Frame\" \"DENY\"
#header \"X-XSS\" \"1\"
#cookie \"sid\" \"xyz\" httponly
<p>hello</p>
";
        let doc = p(source);
        assert_eq!(doc.directives.len(), 5);
        assert!(matches!(
            doc.directives[0].kind,
            DirectiveKind::Status { code: 200 }
        ));
        assert!(matches!(
            doc.directives[1].kind,
            DirectiveKind::ContentType { .. }
        ));
        assert!(matches!(
            doc.directives[2].kind,
            DirectiveKind::Header { .. }
        ));
        assert!(matches!(
            doc.directives[3].kind,
            DirectiveKind::Header { .. }
        ));
        assert!(matches!(
            doc.directives[4].kind,
            DirectiveKind::Cookie { .. }
        ));
    }

    #[test]
    fn directives_not_in_html_output() {
        let source = "#status 404\n<p>body</p>\n";
        let doc = p(source);
        let html: String = doc
            .segments
            .iter()
            .filter_map(|s| match s {
                Segment::Html(h) => Some(h.as_str()),
                _ => None,
            })
            .collect();
        assert!(!html.contains("#status"));
        assert!(html.contains("<p>body</p>"));
    }

    #[test]
    fn invalid_status_directive() {
        let err = perr("#status abc\n");
        assert!(matches!(err, SalataError::DirectiveInvalid { .. }));
    }

    #[test]
    fn include_requires_quotes() {
        let err = perr("#include header.slt\n");
        assert!(matches!(err, SalataError::DirectiveInvalid { .. }));
    }

    // -- Scope attribute --

    #[test]
    fn scope_isolated_attribute() {
        let doc = p("<python scope=\"isolated\">\nprint('hi')\n</python>\n");
        match &doc.segments[0] {
            Segment::RuntimeBlock(b) => {
                assert_eq!(b.scope, Some(BlockScope::Isolated));
            }
            _ => panic!("expected RuntimeBlock"),
        }
    }

    #[test]
    fn no_scope_attribute() {
        let doc = p("<python>\nprint('hi')\n</python>\n");
        match &doc.segments[0] {
            Segment::RuntimeBlock(b) => {
                assert!(b.scope.is_none());
            }
            _ => panic!("expected RuntimeBlock"),
        }
    }

    // -- CSS color hashes not treated as directives --

    #[test]
    fn css_color_not_directive() {
        let source = "<p style=\"color: #fff\">#333 is gray</p>\n";
        let doc = p(source);
        assert!(doc.directives.is_empty());
        match &doc.segments[0] {
            Segment::Html(h) => assert!(h.contains("#333")),
            _ => panic!("expected Html"),
        }
    }

    // -- Multiline code block --

    #[test]
    fn multiline_code_preserved() {
        let source = "\
<python>
x = 1
y = 2
print(x + y)
</python>
";
        let doc = p(source);
        match &doc.segments[0] {
            Segment::RuntimeBlock(b) => {
                assert_eq!(b.code, "x = 1\ny = 2\nprint(x + y)");
            }
            _ => panic!("expected RuntimeBlock"),
        }
    }

    // -- Document position tracking --

    #[test]
    fn segment_ordering_preserved() {
        let source = "\
<h1>Title</h1>
<python>
print('a')
</python>
<p>middle</p>
<ruby>
puts 'b'
</ruby>
<footer>end</footer>
";
        let doc = p(source);
        assert_eq!(doc.segments.len(), 5);
        assert!(matches!(&doc.segments[0], Segment::Html(_)));
        assert!(matches!(&doc.segments[1], Segment::RuntimeBlock(_)));
        assert!(matches!(&doc.segments[2], Segment::Html(_)));
        assert!(matches!(&doc.segments[3], Segment::RuntimeBlock(_)));
        assert!(matches!(&doc.segments[4], Segment::Html(_)));
    }

    #[test]
    fn start_line_tracking() {
        let source = "\
<h1>Title</h1>
<python>
print('first')
</python>
<p>gap</p>
<ruby>
puts 'second'
</ruby>
";
        let doc = p(source);
        let blocks: Vec<&RuntimeBlock> = doc
            .segments
            .iter()
            .filter_map(|s| match s {
                Segment::RuntimeBlock(b) => Some(b),
                _ => None,
            })
            .collect();
        assert_eq!(blocks[0].start_line, 2);
        assert_eq!(blocks[1].start_line, 6);
    }

    // -- Helper function tests --

    #[test]
    fn parse_quoted_string_basic() {
        assert_eq!(parse_quoted_string("\"hello\""), Some("hello".to_string()));
        assert_eq!(parse_quoted_string("hello"), None);
        assert_eq!(parse_quoted_string("\"\""), Some(String::new()));
    }

    #[test]
    fn parse_two_quoted_strings_basic() {
        let result = parse_two_quoted_strings("\"name\" \"value\"");
        assert_eq!(result, Some(("name".to_string(), "value".to_string())));
    }

    #[test]
    fn parse_two_quoted_strings_missing_second() {
        assert_eq!(parse_two_quoted_strings("\"name\""), None);
    }

    // -- Dedent --

    #[test]
    fn dedent_no_indent() {
        assert_eq!(dedent("x = 1\ny = 2"), "x = 1\ny = 2");
    }

    #[test]
    fn dedent_uniform_indent() {
        assert_eq!(dedent("  x = 1\n  y = 2"), "x = 1\ny = 2");
    }

    #[test]
    fn dedent_mixed_indent_strips_common() {
        assert_eq!(
            dedent("    if True:\n        print('hi')"),
            "if True:\n    print('hi')"
        );
    }

    #[test]
    fn dedent_blank_lines_ignored() {
        assert_eq!(
            dedent("  x = 1\n\n  y = 2"),
            "x = 1\n\ny = 2"
        );
    }

    #[test]
    fn dedent_tabs() {
        assert_eq!(dedent("\t\tx = 1\n\t\ty = 2"), "x = 1\ny = 2");
    }

    #[test]
    fn dedent_single_line() {
        assert_eq!(dedent("    print('hi')"), "print('hi')");
    }

    // -- Indented code blocks in parsed documents --

    #[test]
    fn indented_python_block_dedented() {
        let source = "\
<div>
  <python>
    x = 1
    print(x)
  </python>
</div>
";
        let doc = p(source);
        let blocks: Vec<&RuntimeBlock> = doc
            .segments
            .iter()
            .filter_map(|s| match s {
                Segment::RuntimeBlock(b) => Some(b),
                _ => None,
            })
            .collect();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].code, "x = 1\nprint(x)");
    }

    #[test]
    fn indented_python_block_preserves_relative_indent() {
        let source = "\
  <python>
    if True:
      print('yes')
    else:
      print('no')
  </python>
";
        let doc = p(source);
        match &doc.segments[0] {
            Segment::RuntimeBlock(b) => {
                assert_eq!(b.code, "if True:\n  print('yes')\nelse:\n  print('no')");
            }
            _ => panic!("expected RuntimeBlock"),
        }
    }

    #[test]
    fn unindented_code_unchanged() {
        let source = "<python>\nx = 1\nprint(x)\n</python>\n";
        let doc = p(source);
        match &doc.segments[0] {
            Segment::RuntimeBlock(b) => {
                assert_eq!(b.code, "x = 1\nprint(x)");
            }
            _ => panic!("expected RuntimeBlock"),
        }
    }
}
