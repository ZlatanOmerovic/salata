//! FastCGI daemon for Salata (stub -- not yet implemented).
//!
//! This binary will eventually provide a persistent FastCGI daemon that listens
//! on a Unix socket or TCP port, accepting requests from web servers (nginx,
//! Apache) and processing `.slt` files through salata-core. Currently it is a
//! placeholder that prints a "not yet implemented" message and exits.

mod listener;
mod worker;

/// Stub entry point. Prints a version/status message and exits.
fn main() {
    println!(
        "Salata FastCGI v{} — not yet implemented",
        env!("CARGO_PKG_VERSION")
    );
}
