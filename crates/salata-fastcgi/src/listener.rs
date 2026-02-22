// TODO: FastCGI listener — accepts connections on a Unix socket or TCP port.
//
// Responsibilities:
// - Parse config.toml for bind address (unix socket path or host:port)
// - Listen for incoming FastCGI connections from nginx/Apache
// - Accept connections and dispatch to the worker pool
// - Handle SIGTERM/SIGINT for graceful shutdown: stop accepting new
//   connections, finish in-flight requests, kill runtime processes, flush
//   logs, then exit cleanly
// - Support both Unix domain sockets and TCP sockets
