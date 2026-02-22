@echo off
REM Start the Salata interactive playground.

where docker >nul 2>&1
if %errorlevel% neq 0 (
    echo Error: docker is not installed or not in PATH.
    echo Install Docker: https://docs.docker.com/get-docker/
    exit /b 1
)

docker compose version >nul 2>&1
if %errorlevel% neq 0 (
    echo Error: 'docker compose' is not available.
    echo Install Docker Compose v2: https://docs.docker.com/compose/install/
    exit /b 1
)

REM Check if image already exists.
docker image inspect salata-playground >nul 2>&1
if %errorlevel% neq 0 (
    echo Building Salata Playground image (this only happens once^)...
    echo This takes a few minutes — installing runtimes and compiling salata.
    echo.
    docker compose -f "%~dp0docker-compose.playground.yml" build
    echo.
    echo Build complete!
    echo.
)

echo Starting Salata Playground...
echo.

docker compose -f "%~dp0docker-compose.playground.yml" run --rm --service-ports playground bash --login

echo.
echo Salata Playground exited.
