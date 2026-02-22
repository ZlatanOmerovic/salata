# Security — Salata

## CGI Attack Protection (Built into salata-cgi)

All configurable in `[cgi]` section of config.toml.

### Slowloris Protection
- `header_timeout` (5s) — kill if headers not received in time
- `body_timeout` (30s) — kill if body not complete in time
- `min_data_rate` (100b/s) — drop if rate drops below threshold

### Request Limits
- `max_url_length` (2048), `max_header_size` (8KB), `max_header_count` (50)
- `max_query_string_length` (2048), `max_body_size` (10MB)

### Process Limits
- `max_connections_per_ip` (20), `max_total_connections` (200)
- `max_execution_time` (30s), `max_memory_per_request` (128MB)
- `max_response_size` (50MB), `response_timeout` (60s)

### Path Security
- `block_path_traversal` — block `../`
- `block_dotfiles` — block `.env`, `.git`, etc.
- `blocked_extensions` — `.toml`, `.env`, `.git`, `.log`

### Input Sanitization
- `block_null_bytes` — %00 injection prevention
- `block_non_printable_headers`
- `validate_content_length` — match actual body

### Runtime Sandboxing
- `max_child_processes` (10) — prevent fork bombs
- `allow_outbound_network` (true) — configurable

## Shell Sandbox (Built into salata binary)

All protections baked in. No external tools (no nsjail, firejail). The interpreter IS the sandbox.

### Phase 1: Pre-Execution Static Analysis

**Blocked commands** (always blocked, hardcoded):
- *System:* `rm`, `rmdir`, `shred`, `wipefs`, `mkfs`, `dd`, `fdisk`, `mount`, `umount`, `reboot`, `shutdown`, `halt`, `poweroff`, `init`, `systemctl`, `service`, `ln`
- *Process/user:* `kill`, `killall`, `pkill`, `su`, `sudo`, `doas`, `chown`, `chmod`, `chgrp`, `chroot`, `useradd`, `userdel`, `usermod`, `groupadd`, `passwd`
- *Network:* `nc`, `ncat`, `netcat`, `nmap`, `telnet`, `ssh`, `scp`, `sftp`, `ftp`, `rsync`, `socat`
- *Code execution:* `python`, `python3`, `perl`, `ruby`, `node`, `php`, `lua`, `gdb`, `strace`, `ltrace`, `nohup`, `screen`, `tmux`, `at`, `batch`, `crontab`
- *Package management:* `apt`, `apt-get`, `yum`, `dnf`, `pacman`, `brew`, `pip`, `npm`, `gem`
- *Disk/filesystem:* `losetup`, `lvm`, `parted`, `mkswap`, `swapon`, `swapoff`
- *Kernel:* `insmod`, `rmmod`, `modprobe`, `dmesg`, `sysctl`
- *Container:* `docker`, `podman`, `kubectl`, `lxc`

`curl`/`wget` configurable via `allow_outbound_network` (allowed by default, blocked when set to `false`).

**Blocked patterns:**
- `&` (any lone ampersand — blocks backgrounding but also `2>&1` and `>/dev/null 2>&1`; `&&` is allowed)
- `| bash`, `| sh`, `| zsh`, `| dash`, `| fish` (pipe to shell)
- `eval`, `exec`, `source`, `. /` (shell code execution)
- `/dev/tcp/`, `/dev/udp/` (network via redirects)
- `base64 -d`, `base64 --decode`, `xxd -r`, `\x`, `\u00`, `$'\x` (encoding bypass)
- `history`, `HISTFILE`, `export PATH`, `export LD_`, `LD_PRELOAD`, `LD_LIBRARY_PATH` (env manipulation)
- Fork bomb patterns (`:()`, `bomb()`, `while true; do`, `while :; do`)
- **Blocked paths:** `/dev`, `/proc`, `/sys`, `/etc` (any reference to these paths, including `/dev/null`)

**Note:** The `/dev` path block means shell code cannot use `>/dev/null` or `2>/dev/null` redirects. The `&` backgrounding check means `2>&1` is also blocked. Shell examples should avoid these patterns.

### Phase 2: Environment Setup
- Clean PATH with only safe directories
- Stripped env vars, only whitelisted ones
- Working directory locked to document root
- ulimit: max memory, max processes, max file size, max open files

### Phase 3: Runtime Monitoring
- Timeout kills on `max_execution_time`
- Memory tracked, killed if exceeded
- Output size tracked, killed if exceeded

### Other Runtimes
Python, Ruby, JS, TS, PHP get basic protections (timeout, memory, output limits) but NOT command-level scanning.

## Shell Whitelist (Hardcoded)
```
/bin/sh, /bin/bash, /bin/zsh
/usr/bin/sh, /usr/bin/bash, /usr/bin/zsh, /usr/bin/fish, /usr/bin/dash, /usr/bin/ash
```
Not configurable. Modify source + recompile to change.
