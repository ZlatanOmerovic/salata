//! # salata-core
//!
//! Core library for the Salata polyglot text templating engine.
//!
//! This crate provides the shared components used by all Salata binaries:
//! parsing `.slt` files, executing embedded runtime blocks (`<python>`, `<ruby>`,
//! `<javascript>`, `<typescript>`, `<php>`, `<shell>`), resolving directives
//! (`#include`, `#status`, `#content-type`, `#header`, `#cookie`, `#redirect`),
//! and expanding cross-runtime `#set`/`#get` macros.
//!
//! ## Processing Pipeline
//!
//! ```text
//! .slt file → parse → resolve #include → resolve directives
//!           → expand macros → execute runtime blocks → splice outputs
//! ```
//!
//! ## Key Modules
//!
//! - [`parser`] — Parses `.slt` source into segments and directives
//! - [`runtime`] — Executes code blocks in Python, Ruby, JS, TS, PHP, Shell
//! - [`directives`] — Resolves `#include`, `#status`, `#header`, etc.
//! - [`macros`] — Expands `#set`/`#get` cross-runtime data bridge
//! - [`config`] — TOML configuration parsing and validation
//! - [`security`] — Shell sandbox with static analysis and runtime monitoring
//! - [`context`] — Execution context (CLI, CGI, FastCGI, Server)
//! - [`scope`] — Shared vs isolated scope management
//! - [`cache`] — Parsed file caching by path + mtime
//! - [`logging`] — Per-runtime log files with rotation
//! - [`error`] — Error types using `thiserror`

pub mod cache;
pub mod config;
pub mod context;
pub mod directives;
pub mod error;
pub mod logging;
pub mod macros;
pub mod parser;
pub mod runtime;
pub mod scope;
pub mod security;
pub mod uniform_ast;

use std::collections::HashMap;
use std::path::Path;

use config::SalataConfig;
use context::ExecutionContext;
use directives::ResolvedDirectives;
use error::{SalataError, SalataResult};
use macros::MacroDataStore;
use parser::Segment;
use runtime::CgiEnv;

// ---------------------------------------------------------------------------
// Pipeline result
// ---------------------------------------------------------------------------

/// The result of processing a `.slt` file through the full pipeline.
#[derive(Debug)]
pub struct ProcessResult {
    /// The final HTML output (runtime blocks replaced with their output).
    pub html: String,
    /// Resolved directives (status, headers, cookies, redirect, content-type).
    pub directives: ResolvedDirectives,
    /// Whether any runtime errors occurred during block execution.
    /// When `display_errors` handling is active, errors are caught per-block
    /// rather than propagated, and this flag indicates if any were caught.
    pub had_runtime_errors: bool,
}

// ---------------------------------------------------------------------------
// Full execution pipeline
// ---------------------------------------------------------------------------

/// Process a `.slt` file through the full Salata pipeline:
///
/// 1. Read the source file
/// 2. Parse into segments and directives
/// 3. Resolve `#include` directives (recursive text substitution)
/// 4. Validate and resolve directives (`#status`, `#header`, etc.)
/// 5. Expand `#set`/`#get` macros into native code per language
/// 6. Build runtime executors from config
/// 7. Execute runtime blocks (shared + isolated scope)
/// 8. Splice outputs back into the document
///
/// Returns the final HTML and resolved directives.
pub fn process_file(
    file: &Path,
    config: &SalataConfig,
    env: &CgiEnv,
    ctx: ExecutionContext,
) -> SalataResult<ProcessResult> {
    // 1. Read the source file.
    let source = std::fs::read_to_string(file).map_err(SalataError::Io)?;

    process_source(&source, file, config, env, ctx)
}

/// Process an in-memory `.slt` source string through the full pipeline.
/// The `file` path is used for error messages and include resolution.
pub fn process_source(
    source: &str,
    file: &Path,
    config: &SalataConfig,
    env: &CgiEnv,
    ctx: ExecutionContext,
) -> SalataResult<ProcessResult> {
    let base_dir = file.parent().unwrap_or(Path::new("."));

    // 2. Parse into segments and directives.
    let doc = parser::parse(source, file)?;

    // 3. Resolve #include directives.
    let doc = directives::resolve_includes(doc, base_dir)?;

    // 4. Validate and resolve directives.
    let resolved_directives = directives::resolve_directives(&doc)?;

    // 5. Check if any block uses macros and set up the macro data store.
    let any_macros = doc
        .segments
        .iter()
        .any(|seg| matches!(seg, Segment::RuntimeBlock(b) if macros::has_macros(&b.code)));

    let macro_store = if any_macros {
        Some(MacroDataStore::new()?)
    } else {
        None
    };

    // Set up env with macro dir if needed.
    let mut exec_env = env.clone();
    if let Some(ref store) = macro_store {
        exec_env.macro_data_dir = Some(store.path_str().to_string());
    }

    // Expand macros in runtime block code.
    let mut segments = doc.segments;
    for seg in &mut segments {
        if let Segment::RuntimeBlock(block) = seg {
            if macros::has_macros(&block.code) {
                block.code = macros::expand_macros(&block.code, &block.language);
            }
        }
    }

    // 6. Build runtime executors from config (skips disabled runtimes).
    let executors = build_executors(config, ctx);

    // Check that all runtime blocks reference enabled runtimes.
    for seg in &segments {
        if let Segment::RuntimeBlock(block) = seg {
            if !executors.contains_key(block.language.as_str())
                && !config.is_runtime_enabled(&block.language)
            {
                return Err(SalataError::RuntimeDisabled {
                    runtime: block.language.clone(),
                });
            }
        }
    }

    // 7. Build shared scope config.
    let shared_scope_config = scope::shared_scope_map(config);

    // 8. Execute runtime blocks.
    // When macros are in use, execute sequentially (document order) so that
    // cross-runtime #set/#get works correctly. Otherwise, use grouped execution
    // for shared-scope batching.
    // Pass config to enable per-block error handling based on display_errors.
    let (outputs, had_runtime_errors) = runtime::execute_blocks(
        &segments,
        &executors,
        &shared_scope_config,
        &exec_env,
        file,
        any_macros,
        Some(config),
    )?;

    // 9. Splice outputs back into the document.
    let html = runtime::splice_outputs(&segments, &outputs);

    // macro_store is dropped here, cleaning up the temp directory.

    Ok(ProcessResult {
        html,
        directives: resolved_directives,
        had_runtime_errors,
    })
}

// ---------------------------------------------------------------------------
// Executor construction
// ---------------------------------------------------------------------------

/// Build runtime executors from the config, skipping disabled runtimes.
/// The `ctx` determines which PHP binary to use.
fn build_executors(
    config: &SalataConfig,
    ctx: ExecutionContext,
) -> HashMap<String, Box<dyn runtime::RuntimeExecutor>> {
    let mut executors: HashMap<String, Box<dyn runtime::RuntimeExecutor>> = HashMap::new();

    if let Some(ref r) = config.runtimes.python {
        if r.enabled {
            executors.insert(
                "python".into(),
                Box::new(runtime::python::PythonRuntime::new(&r.path)),
            );
        }
    }
    if let Some(ref r) = config.runtimes.ruby {
        if r.enabled {
            executors.insert(
                "ruby".into(),
                Box::new(runtime::ruby::RubyRuntime::new(&r.path)),
            );
        }
    }
    if let Some(ref r) = config.runtimes.javascript {
        if r.enabled {
            executors.insert(
                "javascript".into(),
                Box::new(runtime::javascript::JavaScriptRuntime::new(&r.path)),
            );
        }
    }
    if let Some(ref r) = config.runtimes.typescript {
        if r.enabled {
            executors.insert(
                "typescript".into(),
                Box::new(runtime::typescript::TypeScriptRuntime::new(&r.path)),
            );
        }
    }
    if let Some(ref r) = config.runtimes.php {
        if r.enabled {
            let executor = runtime::php::create_php_runtime(
                ctx,
                r.cli_path.as_deref(),
                r.cgi_path.as_deref(),
                r.fastcgi_socket.as_deref(),
                r.fastcgi_host.as_deref(),
            );
            executors.insert("php".into(), executor);
        }
    }
    if let Some(ref r) = config.runtimes.shell {
        if r.enabled {
            executors.insert(
                "shell".into(),
                Box::new(runtime::shell::ShellRuntime::new(&r.path)),
            );
        }
    }

    executors
}
