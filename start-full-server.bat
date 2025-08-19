@echo off
cd /d "C:\Users\jumpe\Documents\Next..js\CMS\rust-backend"
echo Current directory: %CD%

echo.
echo 🚀 Starting full Rust CMS backend on port 3002...

echo Environment variables:
echo   DATABASE_URL=postgres://user:pass@localhost:5432/cms
echo   DATABASE_NAME=cms
echo   RUST_PORT=3002

set RUST_LOG=info
set DATABASE_URL=postgres://user:pass@localhost:5432/cms
set RUST_DATABASE_NAME=cms
set RUST_JWT_SECRET=test-jwt-secret
set RUST_PORT=3002

cargo run --bin cms-backend
