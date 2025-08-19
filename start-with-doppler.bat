@echo off
cd /d "C:\Users\jumpe\Documents\Next..js\CMS\rust-backend"
echo Current directory: %CD%
echo.

:: Doppler CLI availability check
echo 🔍 Checking Doppler CLI availability...
doppler --version >nul 2>&1
if %errorlevel% neq 0 (
    echo ❌ Doppler CLI not found in PATH
    echo Please install Doppler CLI from: https://docs.doppler.com/docs/install-cli
    echo.
    echo Alternative: Run without Doppler using start-simple-server.bat
    pause
    exit /b 1
)

echo ✅ Doppler CLI found
echo.

:: Check if already logged in to Doppler
echo 🔐 Checking Doppler authentication...
doppler me >nul 2>&1
if %errorlevel% neq 0 (
    echo ❌ Not authenticated with Doppler
    echo Please run: doppler login
    echo Then re-run this script
    pause
    exit /b 1
)

echo ✅ Doppler authentication verified
echo.

:: Setup Doppler project if not already configured
echo 📋 Setting up Doppler project...
if not exist .doppler.yaml (
    echo Configuring Doppler project 'cms' with config 'dev'...
    doppler setup --project cms --config dev --no-interactive
    if %errorlevel% neq 0 (
        echo ❌ Failed to setup Doppler project
        echo Please ensure the 'cms' project exists in your Doppler account
        pause
        exit /b 1
    )
)

echo ✅ Doppler project configured
echo.

:: Test Doppler secrets access
echo 🧪 Testing Doppler secrets access...
doppler secrets --only-names --project cms --config dev >nul 2>&1
if %errorlevel% neq 0 (
    echo ❌ Cannot access Doppler secrets
    echo Please ensure you have access to the 'cms' project
    echo And that secrets are configured for the 'dev' environment
    echo.
    echo Continuing with .env fallback...
)

:: Verify Cargo.toml exists
echo 🔍 Testing Cargo.toml existence...
if exist Cargo.toml (
    echo ✅ Cargo.toml found
) else (
    echo ❌ Cargo.toml not found
    exit /b 1
)

echo.
echo 🚀 Starting Rust server with Doppler configuration...
echo Project: cms
echo Config: dev
echo.

:: Run the server with Doppler
doppler run -- cargo run --bin cms-backend
