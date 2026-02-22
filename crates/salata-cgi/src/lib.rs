//! Shared library for the `salata-cgi` crate, exposing CGI protection
//! utilities that are also consumed by `salata-server`.

/// CGI attack protections: request validation, rate limiting, and input sanitization.
pub mod protection;
