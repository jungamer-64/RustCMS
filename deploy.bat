@echo off
REM Production CMS Deployment Script - Windows版
REM Rust + PostgreSQL + Elasticsearch + WebAuthn + biscuit-auth

setlocal enabledelayedexpansion

echo 🚀 Production CMS Deployment Starting...
echo ========================================

REM 関数定義はラベルとして実装

REM メイン処理
set "command=%1"
if "%command%"=="" set "command=deploy"

if "%command%"=="check" goto check_env
if "%command%"=="db" goto start_database
if "%command%"=="build" goto build_application
if "%command%"=="deploy" goto full_deploy
if "%command%"=="stop" goto stop_services
if "%command%"=="restart" goto restart_services
if "%command%"=="logs" goto show_logs
if "%command%"=="status" goto show_status

echo Usage: %0 {deploy^|check^|db^|build^|stop^|restart^|logs^|status}
echo.
echo Commands:
echo   deploy  - Full deployment (default)
echo   check   - Check environment and prerequisites
echo   db      - Start database and run migrations
echo   build   - Build application only
echo   stop    - Stop all services
echo   restart - Restart all services
echo   logs    - Show application logs
echo   status  - Show service status
goto end

:check_env
echo 📋 Checking environment variables...
if "%DATABASE_URL%"=="" (
    echo ❌ Required environment variable DATABASE_URL is not set
    goto error
)
if "%JWT_SECRET%"=="" (
    echo ❌ Required environment variable JWT_SECRET is not set
    goto error
)
if "%SESSION_SECRET%"=="" (
    echo ❌ Required environment variable SESSION_SECRET is not set
    goto error
)
if "%ELASTICSEARCH_URL%"=="" (
    echo ❌ Required environment variable ELASTICSEARCH_URL is not set
    goto error
)
echo ✅ All required environment variables are set

echo 📋 Checking Docker environment...
docker --version >nul 2>&1
if errorlevel 1 (
    echo ❌ Docker is not installed
    goto error
)

docker-compose --version >nul 2>&1 || docker compose version >nul 2>&1
if errorlevel 1 (
    echo ❌ Docker Compose is not installed
    goto error
)
echo ✅ Docker environment is ready

echo 📋 Checking .env file...
if not exist ".env" (
    echo ⚠️  .env file not found, copying from .env.example
    if exist ".env.example" (
        copy ".env.example" ".env" >nul
        echo ✅ .env file created from .env.example
        echo ⚠️  Please edit .env file with your production values
    ) else (
        echo ❌ .env.example file not found
        goto error
    )
) else (
    echo ✅ .env file found
)
goto end

:start_database
echo 🗄️  Starting database services...
docker-compose up -d postgres elasticsearch redis
echo ⏳ Waiting for services to be ready...
timeout /t 30 /nobreak >nul

echo ⏳ Waiting for PostgreSQL...
:wait_postgres
docker-compose exec -T postgres pg_isready -U cms_user -d production_cms >nul 2>&1
if errorlevel 1 (
    timeout /t 5 /nobreak >nul
    goto wait_postgres
)

echo ⏳ Waiting for Elasticsearch...
:wait_elasticsearch
curl -f http://localhost:9200/_cluster/health >nul 2>&1
if errorlevel 1 (
    timeout /t 5 /nobreak >nul
    goto wait_elasticsearch
)

echo ✅ Database services are ready

echo 🔄 Running database migrations...
call migrate.bat
if errorlevel 1 (
    echo ❌ Database migration failed
    goto error
)
echo ✅ Database migrations completed
goto end

:build_application
echo 🔨 Building application...
cargo build --release
if errorlevel 1 (
    echo ❌ Application build failed
    goto error
)
echo ✅ Application build completed
goto end

:full_deploy
call :check_env
if errorlevel 1 goto error

call :start_database
if errorlevel 1 goto error

call :build_application
if errorlevel 1 goto error

echo 🚀 Starting Production CMS...
docker-compose up -d
echo ⏳ Waiting for application to start...
timeout /t 20 /nobreak >nul

echo ⏳ Waiting for application...
:wait_app
curl -f http://localhost:3000/health >nul 2>&1
if errorlevel 1 (
    timeout /t 5 /nobreak >nul
    goto wait_app
)

echo ✅ Production CMS is running!

echo 🔍 Running post-deployment checks...
echo ✅ http://localhost:3000/health is accessible
curl -f http://localhost:3000/docs >nul 2>&1 && echo ✅ http://localhost:3000/docs is accessible || echo ❌ http://localhost:3000/docs is not accessible
curl -f http://localhost:3000/api/v1/health >nul 2>&1 && echo ✅ http://localhost:3000/api/v1/health is accessible || echo ❌ http://localhost:3000/api/v1/health is not accessible

echo.
echo 📊 Service Status:
docker-compose ps

echo.
echo 🎉 Deployment completed successfully!
echo.
echo 🔗 Access URLs:
echo   • Main Application: http://localhost:3000
echo   • API Documentation: http://localhost:3000/docs
echo   • Health Check: http://localhost:3000/health
echo   • pgAdmin: http://localhost:5050 (admin@example.com / admin123)
echo   • Elasticsearch Head: http://localhost:9100
echo.
goto end

:stop_services
echo 🛑 Stopping Production CMS...
docker-compose down
echo ✅ Production CMS stopped
goto end

:restart_services
echo 🔄 Restarting Production CMS...
docker-compose restart
echo ✅ Production CMS restarted
goto end

:show_logs
docker-compose logs -f
goto end

:show_status
docker-compose ps
goto end

:error
echo ❌ Deployment failed
pause
exit /b 1

:end
echo.
pause
