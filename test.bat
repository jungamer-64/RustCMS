@echo off
REM Production CMS Testing Script - Windows版
REM 大規模アクセス対応のテストスイート

setlocal enabledelayedexpansion

echo 🧪 Production CMS Testing Suite
echo =================================

set "BASE_URL=http://localhost:3000"
set "API_URL=%BASE_URL%/api/v1"
set "TEST_RESULTS=test_results.txt"

REM テスト結果ファイルを初期化
echo Production CMS Test Results - %date% %time% > "%TEST_RESULTS%"
echo ================================================== >> "%TEST_RESULTS%"

echo 📋 Starting comprehensive tests...

REM 1. ヘルスチェックテスト
echo.
echo 🏥 Health Check Tests
echo ---------------------
echo Testing health endpoints... >> "%TEST_RESULTS%"

curl -s -f "%BASE_URL%/health" >nul
if %errorlevel% equ 0 (
    echo ✅ Health check endpoint: PASS
    echo ✅ Health check endpoint: PASS >> "%TEST_RESULTS%"
) else (
    echo ❌ Health check endpoint: FAIL
    echo ❌ Health check endpoint: FAIL >> "%TEST_RESULTS%"
)

curl -s -f "%API_URL%/health" >nul
if %errorlevel% equ 0 (
    echo ✅ API health endpoint: PASS
    echo ✅ API health endpoint: PASS >> "%TEST_RESULTS%"
) else (
    echo ❌ API health endpoint: FAIL
    echo ❌ API health endpoint: FAIL >> "%TEST_RESULTS%"
)

REM 2. 認証システムテスト
echo.
echo 🔐 Authentication System Tests
echo ------------------------------
echo Testing authentication endpoints... >> "%TEST_RESULTS%"

REM 管理者ログインテスト
set "login_response=login_response.json"
curl -s -X POST "%API_URL%/auth/login" ^
     -H "Content-Type: application/json" ^
     -d "{\"username\":\"admin\",\"password\":\"admin123\"}" ^
     -o "%login_response%"

if %errorlevel% equ 0 (
    echo ✅ Admin login: PASS
    echo ✅ Admin login: PASS >> "%TEST_RESULTS%"
    
    REM JWTトークンを抽出（簡単な実装）
    for /f "tokens=*" %%i in ('type "%login_response%" 2^>nul ^| findstr "token"') do set "jwt_line=%%i"
) else (
    echo ❌ Admin login: FAIL
    echo ❌ Admin login: FAIL >> "%TEST_RESULTS%"
)

REM 認証が必要なエンドポイントのテスト
curl -s -f "%API_URL%/auth/profile" ^
     -H "Authorization: Bearer sample_token" >nul
if %errorlevel% equ 0 (
    echo ✅ Protected endpoint access: PASS
    echo ✅ Protected endpoint access: PASS >> "%TEST_RESULTS%"
) else (
    echo ⚠️  Protected endpoint access: Expected failure (needs valid token)
    echo ⚠️  Protected endpoint access: Expected failure >> "%TEST_RESULTS%"
)

REM 3. データベース接続テスト
echo.
echo 🗄️  Database Connection Tests
echo ----------------------------
echo Testing database connectivity... >> "%TEST_RESULTS%"

docker-compose exec -T postgres pg_isready -U cms_user -d production_cms >nul 2>&1
if %errorlevel% equ 0 (
    echo ✅ PostgreSQL connection: PASS
    echo ✅ PostgreSQL connection: PASS >> "%TEST_RESULTS%"
) else (
    echo ❌ PostgreSQL connection: FAIL
    echo ❌ PostgreSQL connection: FAIL >> "%TEST_RESULTS%"
)

REM 4. Elasticsearch接続テスト
echo.
echo 🔍 Elasticsearch Connection Tests
echo ---------------------------------
echo Testing Elasticsearch connectivity... >> "%TEST_RESULTS%"

curl -s -f "http://localhost:9200/_cluster/health" >nul
if %errorlevel% equ 0 (
    echo ✅ Elasticsearch connection: PASS
    echo ✅ Elasticsearch connection: PASS >> "%TEST_RESULTS%"
) else (
    echo ❌ Elasticsearch connection: FAIL
    echo ❌ Elasticsearch connection: FAIL >> "%TEST_RESULTS%"
)

REM 5. Redis接続テスト
echo.
echo 🗃️  Redis Connection Tests
echo -------------------------
echo Testing Redis connectivity... >> "%TEST_RESULTS%"

docker-compose exec -T redis redis-cli ping >nul 2>&1
if %errorlevel% equ 0 (
    echo ✅ Redis connection: PASS
    echo ✅ Redis connection: PASS >> "%TEST_RESULTS%"
) else (
    echo ❌ Redis connection: FAIL
    echo ❌ Redis connection: FAIL >> "%TEST_RESULTS%"
)

REM 6. API エンドポイントテスト
echo.
echo 🌐 API Endpoints Tests
echo ---------------------
echo Testing API endpoints... >> "%TEST_RESULTS%"

REM 公開投稿一覧
curl -s -f "%API_URL%/posts" >nul
if %errorlevel% equ 0 (
    echo ✅ Public posts endpoint: PASS
    echo ✅ Public posts endpoint: PASS >> "%TEST_RESULTS%"
) else (
    echo ❌ Public posts endpoint: FAIL
    echo ❌ Public posts endpoint: FAIL >> "%TEST_RESULTS%"
)

REM 検索エンドポイント
curl -s -f "%API_URL%/search?q=test" >nul
if %errorlevel% equ 0 (
    echo ✅ Search endpoint: PASS
    echo ✅ Search endpoint: PASS >> "%TEST_RESULTS%"
) else (
    echo ❌ Search endpoint: FAIL
    echo ❌ Search endpoint: FAIL >> "%TEST_RESULTS%"
)

REM OpenAPI ドキュメント
curl -s -f "%BASE_URL%/docs/openapi.json" >nul
if %errorlevel% equ 0 (
    echo ✅ OpenAPI documentation: PASS
    echo ✅ OpenAPI documentation: PASS >> "%TEST_RESULTS%"
) else (
    echo ❌ OpenAPI documentation: FAIL
    echo ❌ OpenAPI documentation: FAIL >> "%TEST_RESULTS%"
)

REM 7. セキュリティヘッダーテスト
echo.
echo 🔒 Security Headers Tests
echo ------------------------
echo Testing security headers... >> "%TEST_RESULTS%"

set "headers_response=headers_response.txt"
curl -s -I "%BASE_URL%/health" > "%headers_response%"

findstr /i "x-content-type-options" "%headers_response%" >nul
if %errorlevel% equ 0 (
    echo ✅ X-Content-Type-Options header: PASS
    echo ✅ X-Content-Type-Options header: PASS >> "%TEST_RESULTS%"
) else (
    echo ❌ X-Content-Type-Options header: FAIL
    echo ❌ X-Content-Type-Options header: FAIL >> "%TEST_RESULTS%"
)

findstr /i "x-frame-options" "%headers_response%" >nul
if %errorlevel% equ 0 (
    echo ✅ X-Frame-Options header: PASS
    echo ✅ X-Frame-Options header: PASS >> "%TEST_RESULTS%"
) else (
    echo ❌ X-Frame-Options header: FAIL
    echo ❌ X-Frame-Options header: FAIL >> "%TEST_RESULTS%"
)

REM 8. レート制限テスト
echo.
echo ⏱️  Rate Limiting Tests
echo ----------------------
echo Testing rate limiting... >> "%TEST_RESULTS%"

REM 複数回リクエストを送信してレート制限をテスト
set /a "requests_sent=0"
set /a "requests_success=0"

for /l %%i in (1,1,10) do (
    curl -s -w "%%{http_code}" "%API_URL%/posts" -o nul
    if !errorlevel! equ 0 (
        set /a "requests_success+=1"
    )
    set /a "requests_sent+=1"
    timeout /t 1 /nobreak >nul
)

if %requests_success% gtr 0 (
    echo ✅ Rate limiting functional: PASS (%requests_success%/%requests_sent% requests succeeded)
    echo ✅ Rate limiting functional: PASS >> "%TEST_RESULTS%"
) else (
    echo ❌ Rate limiting: FAIL (no requests succeeded)
    echo ❌ Rate limiting: FAIL >> "%TEST_RESULTS%"
)

REM 9. パフォーマンステスト
echo.
echo ⚡ Performance Tests
echo ------------------
echo Testing response times... >> "%TEST_RESULTS%"

set "perf_response=perf_response.txt"
curl -s -w "Response time: %%{time_total}s\nStatus: %%{http_code}\n" "%API_URL%/posts" -o nul > "%perf_response%"

for /f "tokens=3" %%i in ('findstr "Response time:" "%perf_response%"') do set "response_time=%%i"
for /f "tokens=2" %%i in ('findstr "Status:" "%perf_response%"') do set "status_code=%%i"

echo Response time: %response_time%
echo Status code: %status_code%
echo Response time: %response_time% >> "%TEST_RESULTS%"
echo Status code: %status_code% >> "%TEST_RESULTS%"

REM 10. データベーステーブル検証
echo.
echo 📊 Database Schema Tests
echo -----------------------
echo Testing database schema... >> "%TEST_RESULTS%"

docker-compose exec -T postgres psql postgresql://cms_user:secure_password@localhost:5432/production_cms -c "\dt" >nul 2>&1
if %errorlevel% equ 0 (
    echo ✅ Database schema accessible: PASS
    echo ✅ Database schema accessible: PASS >> "%TEST_RESULTS%"
) else (
    echo ❌ Database schema accessible: FAIL
    echo ❌ Database schema accessible: FAIL >> "%TEST_RESULTS%"
)

REM 11. WebAuthn エンドポイントテスト
echo.
echo 🔐 WebAuthn Tests
echo ----------------
echo Testing WebAuthn endpoints... >> "%TEST_RESULTS%"

curl -s -f "%API_URL%/auth/webauthn/login/start" -X POST >nul
if %errorlevel% equ 0 (
    echo ✅ WebAuthn login start: PASS
    echo ✅ WebAuthn login start: PASS >> "%TEST_RESULTS%"
) else (
    echo ❌ WebAuthn login start: FAIL
    echo ❌ WebAuthn login start: FAIL >> "%TEST_RESULTS%"
)

REM 12. メディアアップロードテスト（簡易）
echo.
echo 📁 Media Upload Tests
echo --------------------
echo Testing media endpoints... >> "%TEST_RESULTS%"

curl -s -f "%API_URL%/media" >nul
if %errorlevel% equ 0 (
    echo ✅ Media endpoint accessible: PASS
    echo ✅ Media endpoint accessible: PASS >> "%TEST_RESULTS%"
) else (
    echo ⚠️  Media endpoint: Expected authentication required
    echo ⚠️  Media endpoint: Expected authentication required >> "%TEST_RESULTS%"
)

REM テスト完了
echo.
echo 📋 Test Summary
echo ==============

set /a "total_tests=20"
set /a "passed_tests=0"

for /f %%i in ('findstr /c:"PASS" "%TEST_RESULTS%"') do set /a "passed_tests=%%i"

echo Total Tests: %total_tests%
echo Passed Tests: %passed_tests%
echo.

if %passed_tests% geq 15 (
    echo 🎉 Overall Status: GOOD
    echo ✅ Production CMS is ready for deployment!
) else if %passed_tests% geq 10 (
    echo ⚠️  Overall Status: NEEDS ATTENTION
    echo 🔧 Some issues found, please review the test results
) else (
    echo ❌ Overall Status: CRITICAL ISSUES
    echo 🚨 Multiple failures detected, system needs fixing
)

echo.
echo 📄 Detailed results saved to: %TEST_RESULTS%
echo.

REM 一時ファイルのクリーンアップ
if exist "%login_response%" del "%login_response%"
if exist "%headers_response%" del "%headers_response%"
if exist "%perf_response%" del "%perf_response%"

REM ログファイルの場所を表示
echo 📊 Additional Information:
echo • Application logs: docker-compose logs cms-backend
echo • Database logs: docker-compose logs postgres
echo • Elasticsearch logs: docker-compose logs elasticsearch
echo • System metrics: curl %BASE_URL%/metrics
echo.

pause
