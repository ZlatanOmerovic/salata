# Runtime Sandboxing

Salata runs all runtime code in separate child processes, never inside the salata process itself. Each runtime gets basic protections, but the level of sandboxing varies. Shell is the most restricted; all other runtimes receive a lighter set of protections.

---

## Process Isolation

Every runtime block is executed in a separate child process. Python code runs in a `python3` process, Ruby in a `ruby` process, JavaScript in a `node` process, and so on. This means:

- A crash in a runtime process does not crash salata itself.
- Runtime processes cannot access salata's internal memory or state.
- Each runtime process can be individually monitored and terminated.
- Runtimes are isolated from each other -- a Python process cannot see what a Ruby process is doing. Cross-runtime communication happens exclusively through the `#set`/`#get` macro system, with salata acting as the broker.

---

## Protections Applied to All Runtimes

The following protections apply to Python, Ruby, JavaScript, TypeScript, PHP, and Shell equally.

### Timeout

Every runtime process is subject to `max_execution_time` (default: 30 seconds). If a block takes longer than this to execute, the process is killed. This catches infinite loops, deadlocks, and unexpectedly long computations.

### Memory Limits

Memory usage of each runtime process is tracked against `max_memory_per_request` (default: 128MB). If a process exceeds this limit, it is killed immediately. This prevents a single block from consuming all available system memory.

### Output Size Limits

The stdout output of each runtime process is tracked against `max_response_size` (default: 50MB). If a process produces more output than this, it is killed. This prevents runaway output from filling up disk space or memory.

### Child Process Limits

The `max_child_processes` setting (default: 10) limits how many child processes a single runtime block can spawn. This is a defense against fork bombs, where code attempts to create an exponentially growing number of processes to crash the system.

---

## Shell: The Most Restricted Runtime

Shell is unique because shell code directly invokes system commands. A Python block that runs `os.system("rm -rf /")` requires the developer to explicitly import `os` and call a function. A shell block that runs `rm -rf /` does so directly -- there is no indirection.

Because of this, shell blocks receive the full three-phase sandbox described in the [Shell Sandbox](./shell-sandbox.md) chapter:

1. **Pre-execution static analysis** -- the code is scanned for blocked commands, blocked patterns, and blocked paths before it runs.
2. **Environment hardening** -- the process launches with a clean PATH, stripped environment variables, locked working directory, and ulimit enforcement.
3. **Runtime monitoring** -- timeout, memory, and output size are tracked continuously.

No other runtime receives the static analysis or environment hardening phases.

---

## Python, Ruby, JavaScript, TypeScript, PHP

These runtimes receive timeout, memory, output size, and child process protections, but they do **not** get:

- **Command-level scanning** -- there is no pre-execution analysis of the code. A Python block can call `subprocess.run(["rm", "-rf", "/"])` and salata will not catch it at parse time (though the OS may prevent it depending on permissions).
- **Environment stripping** -- these runtimes inherit a normal environment. They are not given a stripped-down PATH or cleared environment variables.
- **Blocked paths** -- there is no restriction on referencing `/dev`, `/proc`, `/sys`, or `/etc` from these runtimes.

This is a deliberate design choice. Python, Ruby, JavaScript, TypeScript, and PHP are general-purpose programming languages. Restricting them at the command level would cripple their usefulness -- nearly any library import or file operation could trigger a false positive. Instead, these runtimes rely on OS-level permissions and the process-level resource limits described above.

---

## Network Access

The `allow_outbound_network` setting in `config.toml` only affects the shell runtime. When set to `false`, `curl` and `wget` are added to the shell sandbox's blocked command list.

Python, Ruby, JavaScript, TypeScript, and PHP are unaffected by this setting. They can make outbound network requests using their native libraries (e.g., Python's `urllib`, Ruby's `net/http`, Node's `fetch`) regardless of the `allow_outbound_network` value.

---

## Summary

| Protection | Shell | Python | Ruby | JS | TS | PHP |
|------------|-------|--------|------|----|----|-----|
| Process isolation | Yes | Yes | Yes | Yes | Yes | Yes |
| Timeout enforcement | Yes | Yes | Yes | Yes | Yes | Yes |
| Memory limits | Yes | Yes | Yes | Yes | Yes | Yes |
| Output size limits | Yes | Yes | Yes | Yes | Yes | Yes |
| Child process limits | Yes | Yes | Yes | Yes | Yes | Yes |
| Command-level scanning | Yes | No | No | No | No | No |
| Environment stripping | Yes | No | No | No | No | No |
| Blocked paths | Yes | No | No | No | No | No |
| Network control | Yes | No | No | No | No | No |
