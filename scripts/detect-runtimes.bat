@echo off
REM detect-runtimes.bat — Detect available runtimes and generate config.toml
REM Works on Windows CMD.

setlocal EnableDelayedExpansion

set OUTPUT=config.toml
set FOUND=0
set TOTAL=0

echo Salata Runtime Detection
echo ========================
echo.

REM --- Python ---------------------------------------------------------------
set /a TOTAL+=1
set PYTHON_PATH=
set PYTHON_ENABLED=false

where python3 >nul 2>&1
if !errorlevel! == 0 (
    for /f "delims=" %%i in ('where python3 2^>nul') do (
        if "!PYTHON_PATH!" == "" set "PYTHON_PATH=%%i"
    )
)
if "!PYTHON_PATH!" == "" (
    where python >nul 2>&1
    if !errorlevel! == 0 (
        for /f "delims=" %%i in ('where python 2^>nul') do (
            if "!PYTHON_PATH!" == "" set "PYTHON_PATH=%%i"
        )
    )
)
if not "!PYTHON_PATH!" == "" (
    for /f "delims=" %%v in ('"!PYTHON_PATH!" --version 2^>^&1') do set "PYTHON_VERSION=%%v"
    echo   python         !PYTHON_PATH!    !PYTHON_VERSION!  OK
    set PYTHON_ENABLED=true
    set /a FOUND+=1
) else (
    echo   python         NOT FOUND — will be disabled
)

REM --- Ruby -----------------------------------------------------------------
set /a TOTAL+=1
set RUBY_PATH=
set RUBY_ENABLED=false

where ruby >nul 2>&1
if !errorlevel! == 0 (
    for /f "delims=" %%i in ('where ruby 2^>nul') do (
        if "!RUBY_PATH!" == "" set "RUBY_PATH=%%i"
    )
)
if not "!RUBY_PATH!" == "" (
    for /f "delims=" %%v in ('"!RUBY_PATH!" --version 2^>^&1') do set "RUBY_VERSION=%%v"
    echo   ruby           !RUBY_PATH!    !RUBY_VERSION!  OK
    set RUBY_ENABLED=true
    set /a FOUND+=1
) else (
    echo   ruby           NOT FOUND — will be disabled
)

REM --- JavaScript (node) ----------------------------------------------------
set /a TOTAL+=1
set NODE_PATH=
set NODE_ENABLED=false

where node >nul 2>&1
if !errorlevel! == 0 (
    for /f "delims=" %%i in ('where node 2^>nul') do (
        if "!NODE_PATH!" == "" set "NODE_PATH=%%i"
    )
)
if not "!NODE_PATH!" == "" (
    for /f "delims=" %%v in ('"!NODE_PATH!" --version 2^>^&1') do set "NODE_VERSION=%%v"
    echo   javascript     !NODE_PATH!    !NODE_VERSION!  OK
    set NODE_ENABLED=true
    set /a FOUND+=1
) else (
    echo   javascript     NOT FOUND — will be disabled
)

REM --- TypeScript (ts-node, tsx, bun) ---------------------------------------
set /a TOTAL+=1
set TS_PATH=
set TS_ENABLED=false

where ts-node >nul 2>&1
if !errorlevel! == 0 (
    for /f "delims=" %%i in ('where ts-node 2^>nul') do (
        if "!TS_PATH!" == "" set "TS_PATH=%%i"
    )
)
if "!TS_PATH!" == "" (
    where tsx >nul 2>&1
    if !errorlevel! == 0 (
        for /f "delims=" %%i in ('where tsx 2^>nul') do (
            if "!TS_PATH!" == "" set "TS_PATH=%%i"
        )
    )
)
if "!TS_PATH!" == "" (
    where bun >nul 2>&1
    if !errorlevel! == 0 (
        for /f "delims=" %%i in ('where bun 2^>nul') do (
            if "!TS_PATH!" == "" set "TS_PATH=%%i"
        )
    )
)
if not "!TS_PATH!" == "" (
    for /f "delims=" %%v in ('"!TS_PATH!" --version 2^>^&1') do set "TS_VERSION=%%v"
    echo   typescript     !TS_PATH!    !TS_VERSION!  OK
    set TS_ENABLED=true
    set /a FOUND+=1
) else (
    echo   typescript     NOT FOUND — will be disabled
)

REM --- PHP ------------------------------------------------------------------
set /a TOTAL+=1
set PHP_CLI_PATH=
set PHP_ENABLED=false

where php >nul 2>&1
if !errorlevel! == 0 (
    for /f "delims=" %%i in ('where php 2^>nul') do (
        if "!PHP_CLI_PATH!" == "" set "PHP_CLI_PATH=%%i"
    )
)
if not "!PHP_CLI_PATH!" == "" (
    for /f "delims=" %%v in ('"!PHP_CLI_PATH!" --version 2^>^&1') do set "PHP_VERSION=%%v"
    echo   php            !PHP_CLI_PATH!    !PHP_VERSION!  OK
    set PHP_ENABLED=true
    set /a FOUND+=1
) else (
    echo   php            NOT FOUND — will be disabled
)

REM --- PHP-CGI --------------------------------------------------------------
set PHP_CGI_PATH=
where php-cgi >nul 2>&1
if !errorlevel! == 0 (
    for /f "delims=" %%i in ('where php-cgi 2^>nul') do (
        if "!PHP_CGI_PATH!" == "" set "PHP_CGI_PATH=%%i"
    )
)
if not "!PHP_CGI_PATH!" == "" (
    for /f "delims=" %%v in ('"!PHP_CGI_PATH!" --version 2^>^&1') do set "PHP_CGI_VERSION=%%v"
    echo   php-cgi        !PHP_CGI_PATH!    !PHP_CGI_VERSION!  OK
) else (
    echo   php-cgi        NOT FOUND — will be disabled
)

REM --- Shells ---------------------------------------------------------------
echo.
echo Shells (whitelisted):

set SHELL_PATH=
set SHELL_ENABLED=false

for %%s in (bash sh zsh fish dash ash) do (
    set /a TOTAL+=1
    set "CURR_SHELL="
    where %%s >nul 2>&1
    if !errorlevel! == 0 (
        for /f "delims=" %%i in ('where %%s 2^>nul') do (
            if "!CURR_SHELL!" == "" set "CURR_SHELL=%%i"
        )
    )
    if not "!CURR_SHELL!" == "" (
        for /f "delims=" %%v in ('"!CURR_SHELL!" --version 2^>^&1') do set "SHELL_VER=%%v"
        echo   %%s            !CURR_SHELL!    !SHELL_VER!  OK
        set /a FOUND+=1
        if "!SHELL_PATH!" == "" (
            set "SHELL_PATH=!CURR_SHELL!"
            set SHELL_ENABLED=true
        )
    ) else (
        echo   %%s            NOT FOUND — will be disabled
    )
)

REM --- Default paths for missing runtimes -----------------------------------
if "!PYTHON_PATH!" == "" set "PYTHON_PATH=python3"
if "!RUBY_PATH!" == "" set "RUBY_PATH=ruby"
if "!NODE_PATH!" == "" set "NODE_PATH=node"
if "!TS_PATH!" == "" set "TS_PATH=ts-node"
if "!PHP_CLI_PATH!" == "" set "PHP_CLI_PATH=php"
if "!PHP_CGI_PATH!" == "" set "PHP_CGI_PATH=php-cgi"
if "!SHELL_PATH!" == "" set "SHELL_PATH=bash"

REM --- Generate config.toml ------------------------------------------------

echo.
echo Generating %OUTPUT% ...

(
echo [salata]
echo display_errors = true
echo default_content_type = "text/html; charset=utf-8"
echo encoding = "utf-8"
echo.
echo [server]
echo hot_reload = true
echo.
echo [logging]
echo directory = "./logs"
echo rotation_max_size = "50MB"
echo rotation_max_files = 10
echo.
echo [logging.server]
echo access_log = "access.log"
echo error_log = "error.log"
echo format = "combined"
echo.
echo [logging.runtimes]
echo python = "python.log"
echo ruby = "ruby.log"
echo javascript = "javascript.log"
echo typescript = "typescript.log"
echo php = "php.log"
echo shell = "shell.log"
echo.
echo [runtimes.python]
echo enabled = !PYTHON_ENABLED!
echo path = "!PYTHON_PATH!"
echo shared_scope = true
echo display_errors = true
echo.
echo [runtimes.ruby]
echo enabled = !RUBY_ENABLED!
echo path = "!RUBY_PATH!"
echo shared_scope = true
echo.
echo [runtimes.javascript]
echo enabled = !NODE_ENABLED!
echo path = "!NODE_PATH!"
echo shared_scope = true
echo.
echo [runtimes.typescript]
echo enabled = !TS_ENABLED!
echo path = "!TS_PATH!"
echo shared_scope = true
echo.
echo [runtimes.php]
echo enabled = !PHP_ENABLED!
echo mode = "cgi"
echo cli_path = "!PHP_CLI_PATH!"
echo cgi_path = "!PHP_CGI_PATH!"
echo # fastcgi_socket = "/run/php/php-fpm.sock"
echo # fastcgi_host = "127.0.0.1:9000"
echo shared_scope = true
echo.
echo [runtimes.shell]
echo enabled = !SHELL_ENABLED!
echo path = "!SHELL_PATH!"
echo shared_scope = true
echo.
echo [cgi]
echo header_timeout = "5s"
echo body_timeout = "30s"
echo min_data_rate = "100b/s"
echo max_url_length = 2048
echo max_header_size = "8KB"
echo max_header_count = 50
echo max_query_string_length = 2048
echo max_body_size = "10MB"
echo max_connections_per_ip = 20
echo max_total_connections = 200
echo max_execution_time = "30s"
echo max_memory_per_request = "128MB"
echo max_response_size = "50MB"
echo response_timeout = "60s"
echo block_dotfiles = true
echo block_path_traversal = true
echo blocked_extensions = [".toml", ".env", ".git", ".log"]
echo block_null_bytes = true
echo block_non_printable_headers = true
echo validate_content_length = true
echo max_child_processes = 10
echo allow_outbound_network = true
echo.
echo [errors]
echo page_404 = "./errors/404.slt"
echo page_500 = "./errors/500.slt"
) > %OUTPUT%

echo.
echo Found !FOUND! of !TOTAL! runtimes. Config written to %OUTPUT%.

endlocal
