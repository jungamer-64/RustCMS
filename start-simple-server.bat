@echo off
cd /d "C:\Users\jumpe\Documents\Next..js\CMS\rust-backend"
echo Current directory: %CD%
echo Testing Cargo.toml existence...
if exist Cargo.toml (
    echo ✅ Cargo.toml found
) else (
    echo ❌ Cargo.toml not found
    exit /b 1
)

echo.
echo 🚀 Starting simple Rust server on port 3001...
cargo run --bin simple-server
