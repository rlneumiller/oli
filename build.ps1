# This script builds the hybrid Rust/React 'oli' application on Windows.
# It requires PowerShell 7+, Rust (cargo), and Node.js (npm) to be installed and in the PATH.

# Stop on the first error
$ErrorActionPreference = 'Stop'

# Color definitions for console output
$ColorGreen = "Green"
$ColorBlue = "Cyan"
$ColorRed = "Red"
$ColorYellow = "Yellow"
$ColorDefault = $Host.UI.RawUI.ForegroundColor

# Configuration
$SERVER_BINARY_NAME = "oli-server.exe"  # Make configurable
$BUILD_CONFIG = "release"  # Could be 'debug' for development

# --- Script Start ---

Write-Host "======================================`n" -ForegroundColor $ColorBlue
Write-Host "Building oli (Hybrid Rust/React Application)" -ForegroundColor $ColorGreen
Write-Host "======================================`n" -ForegroundColor $ColorBlue

# Get the script's directory (where this .ps1 file is located)
$ScriptDir = $PSScriptRoot

# Verify project structure
Write-Host "=== Verifying project structure ===" -ForegroundColor $ColorBlue
$requiredPaths = @(
    "Cargo.toml",
    "package.json",
    "app"
)

foreach ($path in $requiredPaths) {
    $fullPath = Join-Path $ScriptDir $path
    if (-not (Test-Path $fullPath)) {
        Write-Host "Error: Required path '$path' not found. Are you running from the project root?" -ForegroundColor $ColorRed
        exit 1
    }
}
Write-Host "Project structure verified." -ForegroundColor $ColorGreen

# 1. Check for required tools
Write-Host "`n=== Checking for required tools ===" -ForegroundColor $ColorBlue

# Check for Rust/Cargo
Write-Host "Checking for Rust toolchain..." -ForegroundColor $ColorYellow
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Error: cargo is not installed or not in PATH." -ForegroundColor $ColorRed
    Write-Host "Please install Rust from: https://rustup.rs/" -ForegroundColor $ColorRed
    exit 1
}
$cargoVersion = cargo --version 2>$null
Write-Host "✓ Cargo found: $cargoVersion" -ForegroundColor $ColorGreen

# Check for Node.js/npm
Write-Host "Checking for Node.js and npm..." -ForegroundColor $ColorYellow
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    Write-Host "Error: node is not installed or not in PATH." -ForegroundColor $ColorRed
    Write-Host "Please install Node.js from: https://nodejs.org/" -ForegroundColor $ColorRed
    exit 1
}
$nodeVersion = node --version 2>$null
Write-Host "✓ Node.js found: $nodeVersion" -ForegroundColor $ColorGreen

if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
    Write-Host "Error: npm is not installed or not in PATH." -ForegroundColor $ColorRed
    Write-Host "npm should come with Node.js. Please reinstall Node.js or check your PATH." -ForegroundColor $ColorRed
    exit 1
}
$npmVersion = npm --version 2>$null
Write-Host "✓ npm found: v$npmVersion" -ForegroundColor $ColorGreen

# Verify npm can run properly
try {
    $npmConfig = npm config get registry 2>$null
    Write-Host "✓ npm registry: $npmConfig" -ForegroundColor $ColorGreen
}
catch {
    Write-Host "Warning: npm may not be configured properly." -ForegroundColor $ColorYellow
}

# 2. Clean previous build
Write-Host "`n=== Cleaning previous build ===" -ForegroundColor $ColorBlue
$distDir = Join-Path $ScriptDir "dist"
if (Test-Path -Path $distDir) {
    Write-Host "Removing previous build in 'dist' directory..." -ForegroundColor $ColorYellow
    try {
        Remove-Item -Path $distDir -Recurse -Force
        Write-Host "Previous build cleaned successfully." -ForegroundColor $ColorGreen
    }
    catch {
        Write-Host "Warning: Could not fully clean previous build: $($_.Exception.Message)" -ForegroundColor $ColorYellow
    }
}

# Clean Rust build cache if requested
if ($args -contains "--clean-all") {
    Write-Host "Cleaning Rust build cache..." -ForegroundColor $ColorYellow
    cargo clean
}

Write-Host "Clean complete." -ForegroundColor $ColorGreen

# 3. Build the Rust backend
Write-Host "`n=== Building Rust backend ===" -ForegroundColor $ColorBlue
try {
    cargo build --$BUILD_CONFIG
    
    # Verify the binary was created
    $serverBinarySource = Join-Path $ScriptDir "target/$BUILD_CONFIG/$SERVER_BINARY_NAME"
    if (-not (Test-Path $serverBinarySource)) {
        Write-Host "Error: Expected binary '$SERVER_BINARY_NAME' not found after build." -ForegroundColor $ColorRed
        exit 1
    }
    
    $binarySize = (Get-Item $serverBinarySource).Length
    Write-Host "Rust backend build complete. Binary size: $([math]::Round($binarySize/1MB, 2)) MB" -ForegroundColor $ColorGreen
}
catch {
    Write-Host "Error building Rust backend: $($_.Exception.Message)" -ForegroundColor $ColorRed
    exit 1
}

# 4. Build the React/Ink frontend
Write-Host "`n=== Building React frontend ===" -ForegroundColor $ColorBlue
$appDir = Join-Path $ScriptDir "app"
if (-not (Test-Path -Path $appDir -PathType Container)) {
    Write-Host "Error: 'app' directory not found at $appDir" -ForegroundColor $ColorRed
    exit 1
}

try {
    Push-Location $ScriptDir
    
    # Check if node_modules exists, if not run npm install
    $nodeModulesPath = Join-Path $ScriptDir "node_modules"
    if (-not (Test-Path $nodeModulesPath)) {
        Write-Host "Installing npm dependencies..." -ForegroundColor $ColorYellow
        npm install
    } else {
        Write-Host "Dependencies already installed, checking for updates..." -ForegroundColor $ColorYellow
        npm ci
    }
    
    Write-Host "Building UI..." -ForegroundColor $ColorYellow
    npm run build
    
    # Verify UI build output
    $uiDistPath = Join-Path $appDir "dist"
    if (-not (Test-Path $uiDistPath)) {
        Write-Host "Error: UI build output not found at $uiDistPath" -ForegroundColor $ColorRed
        exit 1
    }
    
    Pop-Location
    Write-Host "React frontend build complete." -ForegroundColor $ColorGreen
}
catch {
    Pop-Location
    Write-Host "Error building React frontend: $($_.Exception.Message)" -ForegroundColor $ColorRed
    exit 1
}

# 5. Create combined CLI package
Write-Host "`n=== Creating combined CLI package ===" -ForegroundColor $ColorBlue

# Create the package directory structure
$packageDir = Join-Path $distDir "oli"
$packageAppDistDir = Join-Path $packageDir "app/dist"
New-Item -Path $packageAppDistDir -ItemType Directory -Force | Out-Null

try {
    # Copy the Rust binary
    $serverBinarySource = Join-Path $ScriptDir "target/$BUILD_CONFIG/$SERVER_BINARY_NAME"
    if (Test-Path $serverBinarySource) {
        Write-Host "Copying Rust binary..." -ForegroundColor $ColorYellow
        Copy-Item -Path $serverBinarySource -Destination $packageDir
    } else {
        Write-Host "Error: Rust binary '$serverBinarySource' not found." -ForegroundColor $ColorRed
        exit 1
    }

    # Copy UI build output
    $uiDistSource = Join-Path $appDir "dist"
    if (Test-Path $uiDistSource) {
        Write-Host "Copying UI build output..." -ForegroundColor $ColorYellow
        Copy-Item -Path (Join-Path $uiDistSource "*") -Destination $packageAppDistDir -Recurse
    } else {
        Write-Host "Error: UI dist directory '$uiDistSource' not found." -ForegroundColor $ColorRed
        exit 1
    }

    # Copy essential files only from node_modules (or recreate with npm ci)
    Write-Host "Setting up production dependencies... (This may take a while)" -ForegroundColor $ColorYellow
    Push-Location $packageDir
    # Use --ignore-scripts to prevent the postinstall hook from running during the build
    npm ci --only=production --ignore-scripts
    Pop-Location

    # Remove the package-lock.json as it's not needed in the final package
    $packageLockPath = Join-Path $packageDir "package-lock.json"
    if (Test-Path $packageLockPath) {
        Remove-Item -Path $packageLockPath -Force
    }

    Write-Host "Package creation complete." -ForegroundColor $ColorGreen
}
catch {
    if ((Get-Location).Path -ne $ScriptDir) { Pop-Location }
    Write-Host "Error creating package: $($_.Exception.Message)" -ForegroundColor $ColorRed
    exit 1
}

# 6. Create the PowerShell wrapper script
Write-Host "Creating PowerShell wrapper script..." -ForegroundColor $ColorYellow
$wrapperPath = Join-Path $packageDir "oli.ps1"

$wrapperContent = @'
# This script is a wrapper to run the oli application.
# It starts the Rust backend server and then launches the Node.js UI.

# Stop on the first error
$ErrorActionPreference = 'Stop'

# Get the directory where this script is located.
$ScriptDir = $PSScriptRoot

# Check for the server executable.
$serverPath = Join-Path $ScriptDir "oli-server.exe"
if (-not (Test-Path $serverPath)) {
  Write-Error "Error: oli-server.exe binary not found at '$serverPath'!"
  exit 1
}

# Get a random free port for the server to listen on.
# This avoids port conflicts if multiple instances are run.
$listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
$listener.Start()
$port = $listener.LocalEndpoint.Port
$listener.Stop()

# Start the Rust backend server.
# The server will run in the background and redirect its output to a log file.
$logFile = Join-Path $ScriptDir "oli-server.log"
Write-Host "Starting backend server on port $port... Logging to $logFile"
$serverProcess = Start-Process -FilePath $serverPath -ArgumentList "--port $port" -PassThru -NoNewWindow -RedirectStandardError $logFile

# Give the server a moment to start up.
Start-Sleep -Seconds 2

try {
    # Start the UI, passing through any arguments from the wrapper script
    Write-Host "Starting UI..."
    & node (Join-Path $ScriptDir 'app/dist/cli.js') $args
}
finally {
    # After the UI process exits, stop the backend server if it's still running.
    if ($serverProcess -and (Get-Process -Id $serverProcess.Id -ErrorAction SilentlyContinue)) {
        Write-Host "Stopping backend server..."
        Stop-Process -Id $serverProcess.Id -Force
    }
    Write-Host "Backend server stopped. Check $logFile for any errors."
}
'@

$wrapperContent | Set-Content -Path $wrapperPath -Encoding UTF8

# Create a simple batch file for easier execution from cmd
$batchWrapperPath = Join-Path $packageDir "oli.bat"
$batchContent = @"
@echo off
powershell -ExecutionPolicy Bypass -File "%~dp0oli.ps1" %*
"@
Set-Content -Path $batchWrapperPath -Value $batchContent -Encoding ascii

# Calculate package size
$packageSize = (Get-ChildItem $packageDir -Recurse | Measure-Object -Property Length -Sum).Sum
$packageSizeMB = [math]::Round($packageSize / 1MB, 2)

# --- Script End ---

Write-Host "`n======================================" -ForegroundColor $ColorBlue
Write-Host "=== Build complete! ===" -ForegroundColor $ColorGreen
Write-Host "======================================" -ForegroundColor $ColorBlue
Write-Host "Package location: $packageDir" -ForegroundColor $ColorDefault
Write-Host "Package size: $packageSizeMB MB" -ForegroundColor $ColorDefault
Write-Host "`nTo run the application:" -ForegroundColor $ColorDefault
Write-Host "  PowerShell: $(Join-Path $packageDir "oli.ps1")" -ForegroundColor $ColorBlue
Write-Host "  Command Prompt: $(Join-Path $packageDir "oli.bat")" -ForegroundColor $ColorBlue
