# Shell Runtime

| Property       | Value                              |
|----------------|------------------------------------|
| Tag            | `<shell>`                          |
| Output method  | `echo`, `printf`                   |
| Default binary | `/bin/bash`                        |
| Shared scope   | `true` (default)                   |

## Overview

The Shell runtime executes code in a sandboxed shell environment. It is the most restricted runtime in Salata. While the other runtimes have relatively open access to their language's full capabilities, the Shell runtime enforces a strict security sandbox with pre-execution analysis, environment lockdown, and runtime monitoring. This sandbox is baked into Salata itself -- it does not rely on external tools.

## Output

Use `echo` or `printf` to produce output:

```html
<shell>
echo "<h1>System Report</h1>"
echo "<p>Hostname: $(hostname)</p>"
echo "<p>Date: $(date '+%Y-%m-%d %H:%M:%S')</p>"
echo "<p>Kernel: $(uname -sr)</p>"
</shell>
```

## Shell Whitelist

The allowed shells are hardcoded into Salata. This is a security boundary and cannot be changed through configuration:

| Shell Path          |
|---------------------|
| `/bin/sh`           |
| `/bin/bash`         |
| `/bin/zsh`          |
| `/usr/bin/sh`       |
| `/usr/bin/bash`     |
| `/usr/bin/zsh`      |
| `/usr/bin/fish`     |
| `/usr/bin/dash`     |
| `/usr/bin/ash`      |

The shell path configured in `config.toml` must be an absolute path and must match one of these entries. Relative paths are rejected.

## The Three-Phase Sandbox

Shell execution goes through three security phases before and during execution.

### Phase 1: Pre-Execution Static Analysis

Before the shell code runs, Salata scans it for dangerous patterns. If any are found, execution is blocked with an error.

**Blocked commands** -- commands that could damage the system or escape the sandbox:

Examples include `rm`, `sudo`, `su`, `kill`, `killall`, `shutdown`, `reboot`, `mkfs`, `dd`, `mount`, `umount`, `chown`, `chmod`, `python`, `ruby`, `node`, `perl`, `docker`, `kubectl`, and others.

**Blocked patterns** -- syntax patterns that enable evasion or background execution:

| Pattern               | Reason                                      |
|-----------------------|---------------------------------------------|
| `&`                   | Background execution / job control           |
| `\| bash`, `\| sh`   | Piping into a shell                          |
| `eval`                | Arbitrary code execution                     |
| `exec`                | Process replacement                          |
| Fork bombs            | Denial of service                            |
| `/dev/tcp`            | Network access via bash pseudo-devices       |

**Blocked paths** -- filesystem paths that should not be accessed from templates:

| Path     | Reason                                     |
|----------|--------------------------------------------|
| `/dev`   | Device files (includes `/dev/null`)        |
| `/proc`  | Process information filesystem             |
| `/sys`   | Kernel and hardware interface              |
| `/etc`   | System configuration files                 |

Note that `/dev/null` is blocked because it falls under the `/dev` path restriction. This means common patterns like `command > /dev/null` will not work in shell blocks.

### Phase 2: Environment Setup

If the code passes static analysis, Salata sets up a restricted execution environment:

- **Clean PATH** -- only essential directories are on the PATH, preventing access to arbitrary binaries.
- **Stripped environment variables** -- sensitive environment variables are removed. The shell process starts with a minimal, sanitized environment.
- **Locked working directory** -- the shell process runs in a controlled working directory.
- **ulimit enforcement** -- resource limits are applied to prevent runaway processes (CPU time, memory, file descriptors, file size).

### Phase 3: Runtime Monitoring

During execution, Salata monitors the shell process:

- **Timeout** -- if the shell block takes too long, it is terminated.
- **Memory tracking** -- excessive memory usage triggers termination.
- **Output size tracking** -- if stdout grows beyond the configured limit, the process is stopped.

## Known Limitations

Because of the sandbox, some common shell idioms do not work:

| Pattern                | Why It Fails                                     |
|------------------------|--------------------------------------------------|
| `command > /dev/null`  | `/dev` is a blocked path                         |
| `command 2>&1`         | `&` is a blocked pattern                         |
| `#set("key", value)`  | Macro syntax produces invalid shell code         |
| `#get("key")`         | Macro syntax produces invalid shell code         |
| `command &`            | `&` is a blocked pattern (no backgrounding)      |
| `eval "$var"`          | `eval` is a blocked command                      |

The `#set` and `#get` macro limitation means Shell cannot participate in the cross-runtime data bridge. Use other runtimes (Python, Ruby, JavaScript) for data sharing, and use Shell for system information and text processing tasks.

## Example: System Information Report

```html
<shell>
echo "<h2>Server Status</h2>"
echo "<table>"
echo "  <tr><th>Property</th><th>Value</th></tr>"
echo "  <tr><td>Hostname</td><td>$(hostname)</td></tr>"
echo "  <tr><td>Date</td><td>$(date '+%Y-%m-%d %H:%M:%S %Z')</td></tr>"
echo "  <tr><td>Uptime</td><td>$(uptime -p 2>/dev/null || uptime)</td></tr>"
echo "  <tr><td>Kernel</td><td>$(uname -sr)</td></tr>"
echo "  <tr><td>Architecture</td><td>$(uname -m)</td></tr>"
echo "  <tr><td>Shell</td><td>$BASH_VERSION</td></tr>"
echo "</table>"
</shell>
```

## Example: Text Processing

Shell is effective for text processing with tools like `awk`, `sed`, and `grep`:

```html
<shell>
echo "<h2>Disk Usage</h2>"
echo "<pre>"
df -h | head -10
echo "</pre>"
</shell>
```

```html
<shell>
echo "<h2>Environment</h2>"
echo "<dl>"
echo "  <dt>User</dt><dd>$(whoami)</dd>"
echo "  <dt>Home</dt><dd>$HOME</dd>"
echo "  <dt>PWD</dt><dd>$PWD</dd>"
echo "</dl>"
</shell>
```

## Shared Scope

With shared scope enabled (the default), all `<shell>` blocks share a single shell process. Variables set in one block are available in later blocks:

```html
<shell>
SITE_NAME="My Website"
BUILD_DATE=$(date '+%Y-%m-%d')
</shell>

<shell>
echo "<footer>"
echo "  <p>$SITE_NAME - Built on $BUILD_DATE</p>"
echo "</footer>"
</shell>
```

## Configuration

```toml
[runtimes.shell]
enabled = true
path = "/bin/bash"
shared_scope = true
```

| Field            | Type   | Default            | Description                                     |
|------------------|--------|--------------------|-------------------------------------------------|
| `enabled`        | bool   | `true`             | Enable or disable the Shell runtime             |
| `path`           | string | `/bin/bash`        | Absolute path to the shell (must be whitelisted)|
| `shared_scope`   | bool   | `true`             | All blocks share one process per page            |
| `display_errors` | bool   | (global fallback)  | Override the global `display_errors` setting     |

## Isolated Scope

To run a block in its own shell process:

```html
<shell scope="isolated">
echo "This runs in its own shell process."
</shell>
```

Or set `shared_scope = false` in the config for all Shell blocks.

## When to Use Shell

Shell is best suited for:

- System information (hostname, date, uname, whoami, uptime)
- Environment variable access
- Text processing with `awk`, `sed`, `grep`
- Reading files with `cat`
- Simple string manipulation

For anything involving data processing, cross-runtime communication, or complex logic, use Python, Ruby, or JavaScript instead.
