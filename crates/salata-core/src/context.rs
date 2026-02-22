//! Execution context — determines how salata was invoked and affects
//! runtime behavior (especially PHP binary selection).

/// Execution context — determines how salata was invoked.
///
/// Each binary sets its context before calling into salata-core.
/// The context affects runtime behavior, particularly PHP binary selection:
/// - `Cli`: PHP uses the `php` binary (`cli_path`)
/// - `Cgi`: PHP uses `php-cgi` (`cgi_path`)
/// - `FastCgi` / `Server`: PHP uses `php-fpm` via socket/TCP
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionContext {
    /// salata CLI — direct file processing, no HTTP.
    Cli,
    /// salata-cgi — traditional CGI bridge.
    Cgi,
    /// salata-fastcgi — persistent FastCGI daemon.
    FastCgi,
    /// salata-server — standalone HTTP server.
    Server,
}

impl std::fmt::Display for ExecutionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cli => write!(f, "cli"),
            Self::Cgi => write!(f, "cgi"),
            Self::FastCgi => write!(f, "fastcgi"),
            Self::Server => write!(f, "server"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_display() {
        assert_eq!(ExecutionContext::Cli.to_string(), "cli");
        assert_eq!(ExecutionContext::Cgi.to_string(), "cgi");
        assert_eq!(ExecutionContext::FastCgi.to_string(), "fastcgi");
        assert_eq!(ExecutionContext::Server.to_string(), "server");
    }

    #[test]
    fn context_equality() {
        assert_eq!(ExecutionContext::Cli, ExecutionContext::Cli);
        assert_ne!(ExecutionContext::Cli, ExecutionContext::Cgi);
    }

    #[test]
    fn context_is_copy() {
        let ctx = ExecutionContext::Server;
        let ctx2 = ctx; // Copy
        assert_eq!(ctx, ctx2);
    }
}
