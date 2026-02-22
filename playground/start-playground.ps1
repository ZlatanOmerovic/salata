# Start the Salata interactive playground.

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ComposeFile = Join-Path $ScriptDir "docker-compose.playground.yml"
$ImageName = "salata-playground"

if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
    Write-Error "docker is not installed or not in PATH."
    Write-Host "Install Docker: https://docs.docker.com/get-docker/"
    exit 1
}

$composeCheck = docker compose version 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Error "'docker compose' is not available."
    Write-Host "Install Docker Compose v2: https://docs.docker.com/compose/install/"
    exit 1
}

# Build the image if it doesn't exist yet.
docker image inspect $ImageName *>&1 | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Host "Building Salata Playground image (this only happens once)..."
    Write-Host "This takes a few minutes — installing runtimes and compiling salata."
    Write-Host ""
    docker compose -f $ComposeFile build
    Write-Host ""
    Write-Host "Build complete!"
    Write-Host ""
}

Write-Host "Starting Salata Playground..."
Write-Host ""

docker compose -f $ComposeFile `
    run --rm --service-ports playground bash --login

Write-Host ""
Write-Host "Salata Playground exited."
