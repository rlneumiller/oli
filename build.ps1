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
# SIG # Begin signature block
# MIIb4wYJKoZIhvcNAQcCoIIb1DCCG9ACAQExDzANBglghkgBZQMEAgEFADB5Bgor
# BgEEAYI3AgEEoGswaTA0BgorBgEEAYI3AgEeMCYCAwEAAAQQH8w7YFlLCE63JNLG
# KX7zUQIBAAIBAAIBAAIBAAIBADAxMA0GCWCGSAFlAwQCAQUABCCamHC15gP0MQkW
# vOp0ROyLfwRAstW7WwHqEYlFg0sVdqCCFicwggMgMIICCKADAgECAhAxbPyst2gh
# kUHSUELmIpjxMA0GCSqGSIb3DQEBCwUAMCgxJjAkBgNVBAMMHVlvdXIgQ29kZSBT
# aWduaW5nIENlcnRpZmljYXRlMB4XDTI1MDUyMzA4MzYwMVoXDTI2MDUyMzA4NTYw
# MVowKDEmMCQGA1UEAwwdWW91ciBDb2RlIFNpZ25pbmcgQ2VydGlmaWNhdGUwggEi
# MA0GCSqGSIb3DQEBAQUAA4IBDwAwggEKAoIBAQDCnbR2VwnUGG4ffGZasOx9K2tT
# Mukz96kIcVi+qC5rUZHKm3uEzbSU9cJlk90eRv8U8JNbJQ9juSWVYuhNndjvHZbC
# yyKMYDh8gxNx3x4OX5fTa9OA05HGGoZxrOo/RfTABuk5TPDoOQNP6llhCtpcZkd6
# 68VLVG7KgGN1/tBMkiiuF9Nzs4CN0Vu+3Hjp+9YEJruQpKqR63Hl3/CJMt2GVf53
# NKYXHrTqQHwX8B+9HnJq5qhcHJ0CO0Gq/M0K8bhCua1ELEpSI3ldCx1P1CofEZZr
# CtZPQIc7BL4KROjGPRg4zpllIdYu47JxZrYbWOzWSaFZpnbKgtTjS7gIV2jBAgMB
# AAGjRjBEMA4GA1UdDwEB/wQEAwIHgDATBgNVHSUEDDAKBggrBgEFBQcDAzAdBgNV
# HQ4EFgQUtujruP+HcdaAGEbapB5MHXXaaTkwDQYJKoZIhvcNAQELBQADggEBALt+
# iMdxYptJVFjaMnjYoZx9ag8kmCkbfQP50dfUbJBDXnhstMKJ91wkJg45xNyvwher
# sqP07Q0KRPT7sVOsmhqxjvEviYNLsfCfx8529sc17a0iZHMQMv79oKoA+83rphrY
# CdofgOgkDKyFvLMfb6yNUeA1SsO5jTZVTmwUZzV99KnlfvMkarXFy5uJ+6bAmT7L
# 3HJxg7w8NefmFpZB7vx+dwYskxsqAEkXTb7weuZORmb7fG74rMTZCeBVODzvNpMV
# +kdmchERhZ+gRwqV5w8OJkDba/e1cPgIz+DweWbZZR1HWoV/6ap9Ole117bKEpW5
# gc7yIOCZwnK9XLnw4s0wggWNMIIEdaADAgECAhAOmxiO+dAt5+/bUOIIQBhaMA0G
# CSqGSIb3DQEBDAUAMGUxCzAJBgNVBAYTAlVTMRUwEwYDVQQKEwxEaWdpQ2VydCBJ
# bmMxGTAXBgNVBAsTEHd3dy5kaWdpY2VydC5jb20xJDAiBgNVBAMTG0RpZ2lDZXJ0
# IEFzc3VyZWQgSUQgUm9vdCBDQTAeFw0yMjA4MDEwMDAwMDBaFw0zMTExMDkyMzU5
# NTlaMGIxCzAJBgNVBAYTAlVTMRUwEwYDVQQKEwxEaWdpQ2VydCBJbmMxGTAXBgNV
# BAsTEHd3dy5kaWdpY2VydC5jb20xITAfBgNVBAMTGERpZ2lDZXJ0IFRydXN0ZWQg
# Um9vdCBHNDCCAiIwDQYJKoZIhvcNAQEBBQADggIPADCCAgoCggIBAL/mkHNo3rvk
# XUo8MCIwaTPswqclLskhPfKK2FnC4SmnPVirdprNrnsbhA3EMB/zG6Q4FutWxpdt
# HauyefLKEdLkX9YFPFIPUh/GnhWlfr6fqVcWWVVyr2iTcMKyunWZanMylNEQRBAu
# 34LzB4TmdDttceItDBvuINXJIB1jKS3O7F5OyJP4IWGbNOsFxl7sWxq868nPzaw0
# QF+xembud8hIqGZXV59UWI4MK7dPpzDZVu7Ke13jrclPXuU15zHL2pNe3I6PgNq2
# kZhAkHnDeMe2scS1ahg4AxCN2NQ3pC4FfYj1gj4QkXCrVYJBMtfbBHMqbpEBfCFM
# 1LyuGwN1XXhm2ToxRJozQL8I11pJpMLmqaBn3aQnvKFPObURWBf3JFxGj2T3wWmI
# dph2PVldQnaHiZdpekjw4KISG2aadMreSx7nDmOu5tTvkpI6nj3cAORFJYm2mkQZ
# K37AlLTSYW3rM9nF30sEAMx9HJXDj/chsrIRt7t/8tWMcCxBYKqxYxhElRp2Yn72
# gLD76GSmM9GJB+G9t+ZDpBi4pncB4Q+UDCEdslQpJYls5Q5SUUd0viastkF13nqs
# X40/ybzTQRESW+UQUOsxxcpyFiIJ33xMdT9j7CFfxCBRa2+xq4aLT8LWRV+dIPyh
# HsXAj6KxfgommfXkaS+YHS312amyHeUbAgMBAAGjggE6MIIBNjAPBgNVHRMBAf8E
# BTADAQH/MB0GA1UdDgQWBBTs1+OC0nFdZEzfLmc/57qYrhwPTzAfBgNVHSMEGDAW
# gBRF66Kv9JLLgjEtUYunpyGd823IDzAOBgNVHQ8BAf8EBAMCAYYweQYIKwYBBQUH
# AQEEbTBrMCQGCCsGAQUFBzABhhhodHRwOi8vb2NzcC5kaWdpY2VydC5jb20wQwYI
# KwYBBQUHMAKGN2h0dHA6Ly9jYWNlcnRzLmRpZ2ljZXJ0LmNvbS9EaWdpQ2VydEFz
# c3VyZWRJRFJvb3RDQS5jcnQwRQYDVR0fBD4wPDA6oDigNoY0aHR0cDovL2NybDMu
# ZGlnaWNlcnQuY29tL0RpZ2lDZXJ0QXNzdXJlZElEUm9vdENBLmNybDARBgNVHSAE
# CjAIMAYGBFUdIAAwDQYJKoZIhvcNAQEMBQADggEBAHCgv0NcVec4X6CjdBs9thbX
# 979XB72arKGHLOyFXqkauyL4hxppVCLtpIh3bb0aFPQTSnovLbc47/T/gLn4offy
# ct4kvFIDyE7QKt76LVbP+fT3rDB6mouyXtTP0UNEm0Mh65ZyoUi0mcudT6cGAxN3
# J0TU53/oWajwvy8LpunyNDzs9wPHh6jSTEAZNUZqaVSwuKFWjuyk1T3osdz9HNj0
# d1pcVIxv76FQPfx2CWiEn2/K2yCNNWAcAgPLILCsWKAOQGPFmCLBsln1VWvPJ6ts
# ds5vIy30fnFqI2si/xK4VC0nftg62fC2h5b9W9FcrBjDTZ9ztwGpn1eqXijiuZQw
# ggauMIIElqADAgECAhAHNje3JFR82Ees/ShmKl5bMA0GCSqGSIb3DQEBCwUAMGIx
# CzAJBgNVBAYTAlVTMRUwEwYDVQQKEwxEaWdpQ2VydCBJbmMxGTAXBgNVBAsTEHd3
# dy5kaWdpY2VydC5jb20xITAfBgNVBAMTGERpZ2lDZXJ0IFRydXN0ZWQgUm9vdCBH
# NDAeFw0yMjAzMjMwMDAwMDBaFw0zNzAzMjIyMzU5NTlaMGMxCzAJBgNVBAYTAlVT
# MRcwFQYDVQQKEw5EaWdpQ2VydCwgSW5jLjE7MDkGA1UEAxMyRGlnaUNlcnQgVHJ1
# c3RlZCBHNCBSU0E0MDk2IFNIQTI1NiBUaW1lU3RhbXBpbmcgQ0EwggIiMA0GCSqG
# SIb3DQEBAQUAA4ICDwAwggIKAoICAQDGhjUGSbPBPXJJUVXHJQPE8pE3qZdRodbS
# g9GeTKJtoLDMg/la9hGhRBVCX6SI82j6ffOciQt/nR+eDzMfUBMLJnOWbfhXqAJ9
# /UO0hNoR8XOxs+4rgISKIhjf69o9xBd/qxkrPkLcZ47qUT3w1lbU5ygt69OxtXXn
# HwZljZQp09nsad/ZkIdGAHvbREGJ3HxqV3rwN3mfXazL6IRktFLydkf3YYMZ3V+0
# VAshaG43IbtArF+y3kp9zvU5EmfvDqVjbOSmxR3NNg1c1eYbqMFkdECnwHLFuk4f
# sbVYTXn+149zk6wsOeKlSNbwsDETqVcplicu9Yemj052FVUmcJgmf6AaRyBD40Nj
# gHt1biclkJg6OBGz9vae5jtb7IHeIhTZgirHkr+g3uM+onP65x9abJTyUpURK1h0
# QCirc0PO30qhHGs4xSnzyqqWc0Jon7ZGs506o9UD4L/wojzKQtwYSH8UNM/STKvv
# mz3+DrhkKvp1KCRB7UK/BZxmSVJQ9FHzNklNiyDSLFc1eSuo80VgvCONWPfcYd6T
# /jnA+bIwpUzX6ZhKWD7TA4j+s4/TXkt2ElGTyYwMO1uKIqjBJgj5FBASA31fI7tk
# 42PgpuE+9sJ0sj8eCXbsq11GdeJgo1gJASgADoRU7s7pXcheMBK9Rp6103a50g5r
# mQzSM7TNsQIDAQABo4IBXTCCAVkwEgYDVR0TAQH/BAgwBgEB/wIBADAdBgNVHQ4E
# FgQUuhbZbU2FL3MpdpovdYxqII+eyG8wHwYDVR0jBBgwFoAU7NfjgtJxXWRM3y5n
# P+e6mK4cD08wDgYDVR0PAQH/BAQDAgGGMBMGA1UdJQQMMAoGCCsGAQUFBwMIMHcG
# CCsGAQUFBwEBBGswaTAkBggrBgEFBQcwAYYYaHR0cDovL29jc3AuZGlnaWNlcnQu
# Y29tMEEGCCsGAQUFBzAChjVodHRwOi8vY2FjZXJ0cy5kaWdpY2VydC5jb20vRGln
# aUNlcnRUcnVzdGVkUm9vdEc0LmNydDBDBgNVHR8EPDA6MDigNqA0hjJodHRwOi8v
# Y3JsMy5kaWdpY2VydC5jb20vRGlnaUNlcnRUcnVzdGVkUm9vdEc0LmNybDAgBgNV
# HSAEGTAXMAgGBmeBDAEEAjALBglghkgBhv1sBwEwDQYJKoZIhvcNAQELBQADggIB
# AH1ZjsCTtm+YqUQiAX5m1tghQuGwGC4QTRPPMFPOvxj7x1Bd4ksp+3CKDaopafxp
# wc8dB+k+YMjYC+VcW9dth/qEICU0MWfNthKWb8RQTGIdDAiCqBa9qVbPFXONASIl
# zpVpP0d3+3J0FNf/q0+KLHqrhc1DX+1gtqpPkWaeLJ7giqzl/Yy8ZCaHbJK9nXzQ
# cAp876i8dU+6WvepELJd6f8oVInw1YpxdmXazPByoyP6wCeCRK6ZJxurJB4mwbfe
# Kuv2nrF5mYGjVoarCkXJ38SNoOeY+/umnXKvxMfBwWpx2cYTgAnEtp/Nh4cku0+j
# Sbl3ZpHxcpzpSwJSpzd+k1OsOx0ISQ+UzTl63f8lY5knLD0/a6fxZsNBzU+2QJsh
# IUDQtxMkzdwdeDrknq3lNHGS1yZr5Dhzq6YBT70/O3itTK37xJV77QpfMzmHQXh6
# OOmc4d0j/R0o08f56PGYX/sr2H7yRp11LB4nLCbbbxV7HhmLNriT1ObyF5lZynDw
# N7+YAN8gFk8n+2BnFqFmut1VwDophrCYoCvtlUG3OtUVmDG0YgkPCr2B2RP+v6TR
# 81fZvAT6gt4y3wSJ8ADNXcL50CN/AAvkdgIm2fBldkKmKYcJRyvmfxqkhQ/8mJb2
# VVQrH4D6wPIOK+XW+6kvRBVK5xMOHds3OBqhK/bt1nz8MIIGvDCCBKSgAwIBAgIQ
# C65mvFq6f5WHxvnpBOMzBDANBgkqhkiG9w0BAQsFADBjMQswCQYDVQQGEwJVUzEX
# MBUGA1UEChMORGlnaUNlcnQsIEluYy4xOzA5BgNVBAMTMkRpZ2lDZXJ0IFRydXN0
# ZWQgRzQgUlNBNDA5NiBTSEEyNTYgVGltZVN0YW1waW5nIENBMB4XDTI0MDkyNjAw
# MDAwMFoXDTM1MTEyNTIzNTk1OVowQjELMAkGA1UEBhMCVVMxETAPBgNVBAoTCERp
# Z2lDZXJ0MSAwHgYDVQQDExdEaWdpQ2VydCBUaW1lc3RhbXAgMjAyNDCCAiIwDQYJ
# KoZIhvcNAQEBBQADggIPADCCAgoCggIBAL5qc5/2lSGrljC6W23mWaO16P2RHxjE
# iDtqmeOlwf0KMCBDEr4IxHRGd7+L660x5XltSVhhK64zi9CeC9B6lUdXM0s71EOc
# Re8+CEJp+3R2O8oo76EO7o5tLuslxdr9Qq82aKcpA9O//X6QE+AcaU/byaCagLD/
# GLoUb35SfWHh43rOH3bpLEx7pZ7avVnpUVmPvkxT8c2a2yC0WMp8hMu60tZR0Cha
# V76Nhnj37DEYTX9ReNZ8hIOYe4jl7/r419CvEYVIrH6sN00yx49boUuumF9i2T8U
# uKGn9966fR5X6kgXj3o5WHhHVO+NBikDO0mlUh902wS/Eeh8F/UFaRp1z5SnROHw
# SJ+QQRZ1fisD8UTVDSupWJNstVkiqLq+ISTdEjJKGjVfIcsgA4l9cbk8Smlzddh4
# EfvFrpVNnes4c16Jidj5XiPVdsn5n10jxmGpxoMc6iPkoaDhi6JjHd5ibfdp5uzI
# Xp4P0wXkgNs+CO/CacBqU0R4k+8h6gYldp4FCMgrXdKWfM4N0u25OEAuEa3Jyidx
# W48jwBqIJqImd93NRxvd1aepSeNeREXAu2xUDEW8aqzFQDYmr9ZONuc2MhTMizch
# NULpUEoA6Vva7b1XCB+1rxvbKmLqfY/M/SdV6mwWTyeVy5Z/JkvMFpnQy5wR14GJ
# cv6dQ4aEKOX5AgMBAAGjggGLMIIBhzAOBgNVHQ8BAf8EBAMCB4AwDAYDVR0TAQH/
# BAIwADAWBgNVHSUBAf8EDDAKBggrBgEFBQcDCDAgBgNVHSAEGTAXMAgGBmeBDAEE
# AjALBglghkgBhv1sBwEwHwYDVR0jBBgwFoAUuhbZbU2FL3MpdpovdYxqII+eyG8w
# HQYDVR0OBBYEFJ9XLAN3DigVkGalY17uT5IfdqBbMFoGA1UdHwRTMFEwT6BNoEuG
# SWh0dHA6Ly9jcmwzLmRpZ2ljZXJ0LmNvbS9EaWdpQ2VydFRydXN0ZWRHNFJTQTQw
# OTZTSEEyNTZUaW1lU3RhbXBpbmdDQS5jcmwwgZAGCCsGAQUFBwEBBIGDMIGAMCQG
# CCsGAQUFBzABhhhodHRwOi8vb2NzcC5kaWdpY2VydC5jb20wWAYIKwYBBQUHMAKG
# TGh0dHA6Ly9jYWNlcnRzLmRpZ2ljZXJ0LmNvbS9EaWdpQ2VydFRydXN0ZWRHNFJT
# QTQwOTZTSEEyNTZUaW1lU3RhbXBpbmdDQS5jcnQwDQYJKoZIhvcNAQELBQADggIB
# AD2tHh92mVvjOIQSR9lDkfYR25tOCB3RKE/P09x7gUsmXqt40ouRl3lj+8QioVYq
# 3igpwrPvBmZdrlWBb0HvqT00nFSXgmUrDKNSQqGTdpjHsPy+LaalTW0qVjvUBhcH
# zBMutB6HzeledbDCzFzUy34VarPnvIWrqVogK0qM8gJhh/+qDEAIdO/KkYesLyTV
# OoJ4eTq7gj9UFAL1UruJKlTnCVaM2UeUUW/8z3fvjxhN6hdT98Vr2FYlCS7Mbb4H
# v5swO+aAXxWUm3WpByXtgVQxiBlTVYzqfLDbe9PpBKDBfk+rabTFDZXoUke7zPgt
# d7/fvWTlCs30VAGEsshJmLbJ6ZbQ/xll/HjO9JbNVekBv2Tgem+mLptR7yIrpaid
# RJXrI+UzB6vAlk/8a1u7cIqV0yef4uaZFORNekUgQHTqddmsPCEIYQP7xGxZBIhd
# mm4bhYsVA6G2WgNFYagLDBzpmk9104WQzYuVNsxyoVLObhx3RugaEGru+SojW4dH
# PoWrUhftNpFC5H7QEY7MhKRyrBe7ucykW7eaCuWBsBb4HOKRFVDcrZgdwaSIqMDi
# CLg4D+TPVgKx2EgEdeoHNHT9l3ZDBD+XgbF+23/zBjeCtxz+dL/9NWR6P2eZRi7z
# cEO1xwcdcqJsyz/JceENc2Sg8h3KeFUCS7tpFk7CrDqkMYIFEjCCBQ4CAQEwPDAo
# MSYwJAYDVQQDDB1Zb3VyIENvZGUgU2lnbmluZyBDZXJ0aWZpY2F0ZQIQMWz8rLdo
# IZFB0lBC5iKY8TANBglghkgBZQMEAgEFAKCBhDAYBgorBgEEAYI3AgEMMQowCKAC
# gAChAoAAMBkGCSqGSIb3DQEJAzEMBgorBgEEAYI3AgEEMBwGCisGAQQBgjcCAQsx
# DjAMBgorBgEEAYI3AgEVMC8GCSqGSIb3DQEJBDEiBCBa8cUjnRfYtw71i9kkVMRp
# u6MZrQHpxSQhJhcvJFToEDANBgkqhkiG9w0BAQEFAASCAQAEVNMwSJjse45Cgl7Q
# SP4NO+zo3Db4dUVgSjENFcGyqp0LcpS8zSKdFNgT0VbW/U/gn1m8swdXmA8HDA9m
# TSTKYQmb/wn5E0x0rizSf146R5gA/mrMLY8+jAcEprMkuicBQ1/henCWBFbnCnJn
# 83SLe2Zk4R1s4s/nAlF2r0eEcMycR/ysDxLsu21FbWiyLDjQl5SMXg9P2Yx1V9BL
# VzWDrlnroQTp4gxIhXxohXHsO/n8bdiftq8lNRfyn8Orjbmvx3jjOKcAJKo2+OXw
# +8UdXmAAP/ERIwQDfwcKVCPRQtYOnvxFKDKqTzZqtruZBJV9MRsLuK5fplwCHiNh
# h9rjoYIDIDCCAxwGCSqGSIb3DQEJBjGCAw0wggMJAgEBMHcwYzELMAkGA1UEBhMC
# VVMxFzAVBgNVBAoTDkRpZ2lDZXJ0LCBJbmMuMTswOQYDVQQDEzJEaWdpQ2VydCBU
# cnVzdGVkIEc0IFJTQTQwOTYgU0hBMjU2IFRpbWVTdGFtcGluZyBDQQIQC65mvFq6
# f5WHxvnpBOMzBDANBglghkgBZQMEAgEFAKBpMBgGCSqGSIb3DQEJAzELBgkqhkiG
# 9w0BBwEwHAYJKoZIhvcNAQkFMQ8XDTI1MDYyMTEyMzc0N1owLwYJKoZIhvcNAQkE
# MSIEII8qouRh7bNoY3FqfTFu9wdKJ1u4TbMl05U+ojrTlxi3MA0GCSqGSIb3DQEB
# AQUABIICAH1cbkPUdjz9HD5ecNSGKs+fJcMRvLKe8hirRBdBodQJcY/XCxYX8l7e
# N3DJlaU/J+aLWWTh3VIfIkjH0m4F7wX5ysDuibThuEQUvChnF1lmhQWo7aBOD5r3
# PnSJVJX9pU8iDRGvWnZy6amXvn8oG0ggJbRgt3bvp8+u55Sn35DUui1op3578ikh
# buCxa+cZEMM6gs5jNz4lWgOcX0KWJ16owUZIm3QKegBO8R4ZZvvJPBWgtjSKADlD
# PApYAJWeP5r3NZ+bzBDzcoOkUdOsZQqagQQkZKKrRiT4TD/N5XtfVRXqU3QRr7N+
# tHb35qYS8A3vWaB4ScPvFHq3BXAGxT48yy5HCybwBTFHBfVTYz6NvJlSsLSJqAfw
# mglPqlrEc53KF4N+jhmcWsOVfCqyHeHNJG4v/Nszi62a9eIhn9VikqJ6NmmafwwL
# 9Vc9L9R2VZ1i7tbqHi7OhI9PYh5PlQxZCNJC02iTmafunk2b7rcACOI1CfX5GTTm
# HZ6iEQDZ0V2YTWngIOme7DVaco0NQ8ijLp99r6a++wFajwYVcORponNrDqdJAFZf
# ZlBT4lMS45GS2OEb+EOU4le/gTHeKL9okZBk8ECs9nMBsEEVIyHKEHVQXr3Uudr5
# CLveC1C0jU4XN8SUh3Ww/XRn8MR/UZNfRESa4wGvB/qoBOJoTUCy
# SIG # End signature block
