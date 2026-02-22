# Shell Sandbox

The shell runtime is the most restricted runtime in Salata. Because shell code has direct access to system commands and the filesystem, Salata applies a three-phase security model: static analysis before execution, environment hardening at launch, and continuous monitoring during execution. All of these protections are baked into the salata binary itself -- no external sandboxing tools are required.

## Shell Whitelist

Only the following shell interpreters are allowed. This list is **hardcoded** and cannot be changed via configuration:

- `/bin/sh`
- `/bin/bash`
- `/bin/zsh`
- `/usr/bin/sh`
- `/usr/bin/bash`
- `/usr/bin/zsh`
- `/usr/bin/fish`
- `/usr/bin/dash`
- `/usr/bin/ash`

The shell path must be absolute. Relative paths like `bash` or `./sh` are rejected.

---

## Phase 1: Pre-Execution Static Analysis

Before any shell code runs, Salata scans the entire block for blocked commands, blocked patterns, and blocked path references. If any match is found, the block is rejected with an error and never executed.

### Blocked Commands

These commands are always blocked and cannot be unblocked via configuration.

**System commands:**

`rm`, `rmdir`, `shred`, `wipefs`, `mkfs`, `dd`, `fdisk`, `mount`, `umount`, `reboot`, `shutdown`, `halt`, `poweroff`, `init`, `systemctl`, `service`, `ln`

**Process and user management:**

`kill`, `killall`, `pkill`, `su`, `sudo`, `doas`, `chown`, `chmod`, `chgrp`, `chroot`, `useradd`, `userdel`, `usermod`, `groupadd`, `passwd`

**Network tools:**

`nc`, `ncat`, `netcat`, `nmap`, `telnet`, `ssh`, `scp`, `sftp`, `ftp`, `rsync`, `socat`

**Code execution:**

`python`, `python3`, `perl`, `ruby`, `node`, `php`, `lua`, `gdb`, `strace`, `ltrace`, `nohup`, `screen`, `tmux`, `at`, `batch`, `crontab`

**Package management:**

`apt`, `apt-get`, `yum`, `dnf`, `pacman`, `brew`, `pip`, `npm`, `gem`

**Disk and filesystem:**

`losetup`, `lvm`, `parted`, `mkswap`, `swapon`, `swapoff`

**Kernel modules:**

`insmod`, `rmmod`, `modprobe`, `dmesg`, `sysctl`

**Container runtimes:**

`docker`, `podman`, `kubectl`, `lxc`

**Network downloads (`curl` and `wget`):**

These two commands are the exception. They are controlled by the `allow_outbound_network` setting in the `[cgi]` section of `config.toml`. When `allow_outbound_network = true` (the default), `curl` and `wget` are allowed. When set to `false`, they are blocked like any other network tool.

### Blocked Patterns

These patterns are detected anywhere in the shell block and cause immediate rejection.

| Pattern | Reason |
|---------|--------|
| `&` (single ampersand) | Blocks backgrounding of processes. Note that `&&` (logical AND) is allowed. |
| `\| bash`, `\| sh`, `\| zsh`, `\| dash`, `\| fish` | Prevents piping output into a shell interpreter. |
| `eval` | Blocks dynamic code evaluation. |
| `exec` | Blocks process replacement. |
| `source` | Blocks sourcing external scripts. |
| `. /` | Blocks the dot-source shorthand. |
| `/dev/tcp/`, `/dev/udp/` | Blocks Bash network redirects (e.g., `/dev/tcp/host/port`). |
| `base64 -d`, `base64 --decode` | Blocks decoding of obfuscated payloads. |
| `xxd -r` | Blocks hex-to-binary conversion. |
| `\x`, `\u00`, `$'\x` | Blocks encoding-based bypass attempts. |
| `history`, `HISTFILE` | Blocks shell history access and manipulation. |
| `export PATH`, `export LD_` | Blocks PATH and dynamic linker variable manipulation. |
| `LD_PRELOAD`, `LD_LIBRARY_PATH` | Blocks library injection attacks. |
| `:()`, `bomb()` | Blocks fork bomb function definitions. |
| `while true; do`, `while :; do` | Blocks infinite loop patterns. |

### Blocked Paths

Any reference to the following paths causes the block to be rejected:

- `/dev` -- device files (includes `/dev/null`, `/dev/zero`, `/dev/random`, `/dev/tcp`, `/dev/udp`)
- `/proc` -- process information filesystem
- `/sys` -- kernel/device configuration filesystem
- `/etc` -- system configuration files

This is a substring match, so even harmless uses like `>/dev/null` or `cat /etc/hostname` are blocked. This is an intentional trade-off: blocking `/dev/null` is the cost of preventing access to `/dev/tcp` and other dangerous device files. Similarly, `2>&1` is blocked because the `&` character triggers the backgrounding check.

---

## Phase 2: Environment Setup

If the shell block passes static analysis, Salata prepares a hardened execution environment before launching the shell process.

### Clean PATH

The `PATH` environment variable is set to a minimal list of safe directories only. System directories containing administrative tools are excluded.

### Stripped Environment Variables

The shell process inherits only a whitelisted set of environment variables. Sensitive variables from the parent process (such as credentials, tokens, or library paths) are stripped.

### Locked Working Directory

The shell process's working directory is locked to the document root. The shell block cannot `cd` to arbitrary locations outside the project.

### ulimit Enforcement

Resource limits are applied via `ulimit` before the shell code runs:

- **Max memory** -- prevents the shell process from consuming excessive RAM
- **Max processes** -- limits the number of child processes (prevents fork bombs at the OS level)
- **Max file size** -- limits the size of files the shell can create
- **Max open files** -- limits the number of file descriptors

---

## Phase 3: Runtime Monitoring

While the shell block is executing, Salata actively monitors the process and will terminate it if limits are exceeded.

### Timeout

The shell process is killed if it exceeds `max_execution_time` (default: 30 seconds, configurable in the `[cgi]` section). This prevents infinite loops that were not caught by static analysis, long-running commands, and hung processes.

### Memory Tracking

Salata tracks the memory usage of the shell process. If it exceeds `max_memory_per_request` (default: 128MB), the process is killed immediately.

### Output Size Tracking

The stdout output of the shell process is tracked. If it exceeds `max_response_size` (default: 50MB), the process is killed. This prevents a shell block from flooding the output buffer with an enormous amount of data.

---

## Practical Implications

The shell sandbox is strict by design, and some common shell idioms are not available inside `<shell>` blocks:

```text
# These will NOT work in Salata shell blocks:

command > /dev/null       # /dev path blocked
command 2>&1              # & character blocked
command &                 # backgrounding blocked
rm temp_file              # rm command blocked
sudo apt install foo      # sudo and apt blocked
curl http://example.com   # allowed only if allow_outbound_network = true
```

Shell blocks are best suited for read-only tasks like gathering system information, text processing with `awk`/`sed`/`grep`, and generating output from existing data. For tasks that require more system access, consider using Python or Ruby runtime blocks instead, which have fewer restrictions.
