# detect-runtimes.ps1 — Detect available runtimes and generate config.toml
# Works on Windows PowerShell and PowerShell Core (Linux/macOS).

$Output = "config.toml"
$Found = 0
$Total = 0

function Find-Runtime {
    param([string[]]$Names)
    foreach ($name in $Names) {
        $cmd = Get-Command $name -ErrorAction SilentlyContinue
        if ($cmd) {
            return $cmd.Source
        }
    }
    return $null
}

function Get-RuntimeVersion {
    param([string]$Path)
    try {
        $ver = & $Path --version 2>&1 | Select-Object -First 1
        return "$ver"
    } catch {
        return "unknown"
    }
}

function Print-Found {
    param([string]$Label, [string]$Path, [string]$Version)
    Write-Host ("  {0,-14} {1,-40} {2}  OK" -f $Label, $Path, $Version)
}

function Print-Missing {
    param([string]$Label)
    Write-Host ("  {0,-14} NOT FOUND — will be disabled" -f $Label)
}

Write-Host "Salata Runtime Detection"
Write-Host "========================"
Write-Host ""

# --- Python -----------------------------------------------------------------
$Total++
$PythonPath = Find-Runtime "python3", "python"
$PythonEnabled = "false"
if ($PythonPath) {
    $PythonVersion = Get-RuntimeVersion $PythonPath
    Print-Found "python" $PythonPath $PythonVersion
    $PythonEnabled = "true"
    $Found++
} else {
    Print-Missing "python"
}

# --- Ruby -------------------------------------------------------------------
$Total++
$RubyPath = Find-Runtime "ruby"
$RubyEnabled = "false"
if ($RubyPath) {
    $RubyVersion = Get-RuntimeVersion $RubyPath
    Print-Found "ruby" $RubyPath $RubyVersion
    $RubyEnabled = "true"
    $Found++
} else {
    Print-Missing "ruby"
}

# --- JavaScript (node) -----------------------------------------------------
$Total++
$NodePath = Find-Runtime "node"
$NodeEnabled = "false"
if ($NodePath) {
    $NodeVersion = Get-RuntimeVersion $NodePath
    Print-Found "javascript" $NodePath $NodeVersion
    $NodeEnabled = "true"
    $Found++
} else {
    Print-Missing "javascript"
}

# --- TypeScript (ts-node, tsx, bun) -----------------------------------------
$Total++
$TsPath = Find-Runtime "ts-node", "tsx", "bun"
$TsEnabled = "false"
if ($TsPath) {
    $TsVersion = Get-RuntimeVersion $TsPath
    Print-Found "typescript" $TsPath $TsVersion
    $TsEnabled = "true"
    $Found++
} else {
    Print-Missing "typescript"
}

# --- PHP --------------------------------------------------------------------
$Total++
$PhpCliPath = Find-Runtime "php"
$PhpEnabled = "false"
if ($PhpCliPath) {
    $PhpVersion = Get-RuntimeVersion $PhpCliPath
    Print-Found "php" $PhpCliPath $PhpVersion
    $PhpEnabled = "true"
    $Found++
} else {
    Print-Missing "php"
}

# --- PHP-CGI ----------------------------------------------------------------
$PhpCgiPath = Find-Runtime "php-cgi"
if ($PhpCgiPath) {
    $PhpCgiVersion = Get-RuntimeVersion $PhpCgiPath
    Print-Found "php-cgi" $PhpCgiPath $PhpCgiVersion
} else {
    Print-Missing "php-cgi"
}

# --- Shells (whitelisted) ---------------------------------------------------
Write-Host ""
Write-Host "Shells (whitelisted):"

$ShellPath = $null
$ShellEnabled = "false"
$ShellCandidates = @("bash", "sh", "zsh", "fish", "dash", "ash")

foreach ($shell in $ShellCandidates) {
    $Total++
    $cmd = Get-Command $shell -ErrorAction SilentlyContinue
    if ($cmd) {
        $shellBin = $cmd.Source
        $shellVer = Get-RuntimeVersion $shellBin
        Print-Found $shell $shellBin $shellVer
        $Found++
        if (-not $ShellPath) {
            $ShellPath = $shellBin
            $ShellEnabled = "true"
        }
    } else {
        Print-Missing $shell
    }
}

# --- Default paths for missing runtimes ------------------------------------
if (-not $PythonPath)  { $PythonPath  = if ($IsLinux -or $IsMacOS) { "/usr/bin/python3" } else { "python3" } }
if (-not $RubyPath)    { $RubyPath    = if ($IsLinux -or $IsMacOS) { "/usr/bin/ruby" }    else { "ruby" } }
if (-not $NodePath)    { $NodePath    = if ($IsLinux -or $IsMacOS) { "/usr/bin/node" }    else { "node" } }
if (-not $TsPath)      { $TsPath      = if ($IsLinux -or $IsMacOS) { "/usr/bin/ts-node" } else { "ts-node" } }
if (-not $PhpCliPath)  { $PhpCliPath  = if ($IsLinux -or $IsMacOS) { "/usr/bin/php" }     else { "php" } }
if (-not $PhpCgiPath)  { $PhpCgiPath  = if ($IsLinux -or $IsMacOS) { "/usr/bin/php-cgi" } else { "php-cgi" } }
if (-not $ShellPath)   { $ShellPath   = if ($IsLinux -or $IsMacOS) { "/bin/bash" }        else { "bash" } }

# --- Generate config.toml --------------------------------------------------

Write-Host ""
Write-Host "Generating $Output ..."

$config = @"
[salata]
display_errors = true
default_content_type = "text/html; charset=utf-8"
encoding = "utf-8"

[server]
hot_reload = true

[logging]
directory = "./logs"
rotation_max_size = "50MB"
rotation_max_files = 10

[logging.server]
access_log = "access.log"
error_log = "error.log"
format = "combined"

[logging.runtimes]
python = "python.log"
ruby = "ruby.log"
javascript = "javascript.log"
typescript = "typescript.log"
php = "php.log"
shell = "shell.log"

[runtimes.python]
enabled = $PythonEnabled
path = "$PythonPath"
shared_scope = true
display_errors = true

[runtimes.ruby]
enabled = $RubyEnabled
path = "$RubyPath"
shared_scope = true

[runtimes.javascript]
enabled = $NodeEnabled
path = "$NodePath"
shared_scope = true

[runtimes.typescript]
enabled = $TsEnabled
path = "$TsPath"
shared_scope = true

[runtimes.php]
enabled = $PhpEnabled
mode = "cgi"
cli_path = "$PhpCliPath"
cgi_path = "$PhpCgiPath"
# fastcgi_socket = "/run/php/php-fpm.sock"
# fastcgi_host = "127.0.0.1:9000"
shared_scope = true

[runtimes.shell]
enabled = $ShellEnabled
path = "$ShellPath"
shared_scope = true

[cgi]
header_timeout = "5s"
body_timeout = "30s"
min_data_rate = "100b/s"
max_url_length = 2048
max_header_size = "8KB"
max_header_count = 50
max_query_string_length = 2048
max_body_size = "10MB"
max_connections_per_ip = 20
max_total_connections = 200
max_execution_time = "30s"
max_memory_per_request = "128MB"
max_response_size = "50MB"
response_timeout = "60s"
block_dotfiles = true
block_path_traversal = true
blocked_extensions = [".toml", ".env", ".git", ".log"]
block_null_bytes = true
block_non_printable_headers = true
validate_content_length = true
max_child_processes = 10
allow_outbound_network = true

[errors]
page_404 = "./errors/404.slt"
page_500 = "./errors/500.slt"
"@

$config | Out-File -FilePath $Output -Encoding utf8

Write-Host ""
Write-Host "Found $Found of $Total runtimes. Config written to $Output."
