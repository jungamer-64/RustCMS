# Doppler統合 PowerShellスクリプト
# Dopplerから環境変数を読み込んでRustサーバーを起動

# 作業ディレクトリを設定
Set-Location "C:\Users\jumpe\Documents\Next..js\CMS\rust-backend"
Write-Host "Current directory: $(Get-Location)" -ForegroundColor Yellow
Write-Host ""

# Doppler CLI の確認
Write-Host "🔍 Checking Doppler CLI availability..." -ForegroundColor Cyan
try {
    $dopplerVersion = & doppler --version 2>$null
    Write-Host "✅ Doppler CLI found: $dopplerVersion" -ForegroundColor Green
} catch {
    Write-Host "❌ Doppler CLI not found in PATH" -ForegroundColor Red
    Write-Host "Please install Doppler CLI from: https://docs.doppler.com/docs/install-cli" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Alternative: Run without Doppler using simple server" -ForegroundColor Yellow
    Read-Host "Press Enter to exit"
    exit 1
}
Write-Host ""

# Doppler認証の確認
Write-Host "🔐 Checking Doppler authentication..." -ForegroundColor Cyan
try {
    & doppler me 2>$null | Out-Null
    Write-Host "✅ Doppler authentication verified" -ForegroundColor Green
} catch {
    Write-Host "❌ Not authenticated with Doppler" -ForegroundColor Red
    Write-Host "Please run: doppler login" -ForegroundColor Yellow
    Write-Host "Then re-run this script" -ForegroundColor Yellow
    Read-Host "Press Enter to exit"
    exit 1
}
Write-Host ""

# Dopplerプロジェクトの設定
Write-Host "📋 Setting up Doppler project..." -ForegroundColor Cyan
if (-not (Test-Path ".doppler.yaml")) {
    Write-Host "Configuring Doppler project 'cms' with config 'dev'..." -ForegroundColor Yellow
    try {
        & doppler setup --project cms --config dev --no-interactive
        Write-Host "✅ Doppler project configured" -ForegroundColor Green
    } catch {
        Write-Host "❌ Failed to setup Doppler project" -ForegroundColor Red
        Write-Host "Please ensure the 'cms' project exists in your Doppler account" -ForegroundColor Yellow
        Read-Host "Press Enter to exit"
        exit 1
    }
} else {
    Write-Host "✅ Doppler project already configured" -ForegroundColor Green
}
Write-Host ""

# Cargo.tomlの確認
Write-Host "🔍 Testing Cargo.toml existence..." -ForegroundColor Cyan
if (Test-Path "Cargo.toml") {
    Write-Host "✅ Cargo.toml found" -ForegroundColor Green
} else {
    Write-Host "❌ Cargo.toml not found" -ForegroundColor Red
    exit 1
}
Write-Host ""

# サーバー起動
Write-Host "🚀 Starting Rust server with Doppler configuration..." -ForegroundColor Green
Write-Host "Project: cms" -ForegroundColor Cyan
Write-Host "Config: dev" -ForegroundColor Cyan
Write-Host ""

# Dopplerでサーバーを実行
try {
    & doppler run -- cargo run
} catch {
    Write-Host "❌ Failed to start server with Doppler" -ForegroundColor Red
    Write-Host "Error: $_" -ForegroundColor Red
    Read-Host "Press Enter to exit"
    exit 1
}
