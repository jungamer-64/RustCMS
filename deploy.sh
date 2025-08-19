#!/bin/bash

# Production CMS Deployment Script
# Rust + PostgreSQL + Elasticsearch + WebAuthn + biscuit-auth

set -e  # エラー時に停止

echo "🚀 Production CMS Deployment Starting..."
echo "========================================"

# 環境変数のチェック
check_env() {
    echo "📋 Checking environment variables..."
    
    required_vars=(
        "DATABASE_URL"
        "JWT_SECRET"
        "SESSION_SECRET"
        "ELASTICSEARCH_URL"
    )
    
    for var in "${required_vars[@]}"; do
        if [ -z "${!var}" ]; then
            echo "❌ Required environment variable $var is not set"
            exit 1
        fi
    done
    
    echo "✅ All required environment variables are set"
}

# Docker環境の確認
check_docker() {
    echo "📋 Checking Docker environment..."
    
    if ! command -v docker &> /dev/null; then
        echo "❌ Docker is not installed"
        exit 1
    fi
    
    if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
        echo "❌ Docker Compose is not installed"
        exit 1
    fi
    
    echo "✅ Docker environment is ready"
}

# .envファイルの確認
check_env_file() {
    echo "📋 Checking .env file..."
    
    if [ ! -f ".env" ]; then
        echo "⚠️  .env file not found, copying from .env.example"
        if [ -f ".env.example" ]; then
            cp .env.example .env
            echo "✅ .env file created from .env.example"
            echo "⚠️  Please edit .env file with your production values"
        else
            echo "❌ .env.example file not found"
            exit 1
        fi
    else
        echo "✅ .env file found"
    fi
}

# データベース起動
start_database() {
    echo "🗄️  Starting database services..."
    
    # PostgreSQL と Elasticsearch を起動
    docker-compose up -d postgres elasticsearch redis
    
    echo "⏳ Waiting for services to be ready..."
    sleep 30
    
    # PostgreSQL接続確認
    until docker-compose exec -T postgres pg_isready -U cms_user -d production_cms; do
        echo "⏳ Waiting for PostgreSQL..."
        sleep 5
    done
    
    # Elasticsearch接続確認
    until curl -f http://localhost:9200/_cluster/health; do
        echo "⏳ Waiting for Elasticsearch..."
        sleep 5
    done
    
    echo "✅ Database services are ready"
}

# マイグレーション実行
run_migrations() {
    echo "🔄 Running database migrations..."
    
    # Windowsの場合
    if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]]; then
        ./migrate.bat
    else
        chmod +x migrate.sh
        ./migrate.sh
    fi
    
    echo "✅ Database migrations completed"
}

# アプリケーションビルド
build_application() {
    echo "🔨 Building application..."
    
    # Rustアプリケーションのビルド
    cargo build --release
    
    echo "✅ Application build completed"
}

# アプリケーション起動
start_application() {
    echo "🚀 Starting Production CMS..."
    
    # 全サービスを起動
    docker-compose up -d
    
    echo "⏳ Waiting for application to start..."
    sleep 20
    
    # ヘルスチェック
    until curl -f http://localhost:3000/health; do
        echo "⏳ Waiting for application..."
        sleep 5
    done
    
    echo "✅ Production CMS is running!"
}

# デプロイメント後の確認
post_deployment_check() {
    echo "🔍 Running post-deployment checks..."
    
    # APIエンドポイントの確認
    endpoints=(
        "http://localhost:3000/health"
        "http://localhost:3000/docs"
        "http://localhost:3000/api/v1/health"
    )
    
    for endpoint in "${endpoints[@]}"; do
        if curl -f "$endpoint" &> /dev/null; then
            echo "✅ $endpoint is accessible"
        else
            echo "❌ $endpoint is not accessible"
        fi
    done
    
    # サービス状態の確認
    echo ""
    echo "📊 Service Status:"
    docker-compose ps
    
    echo ""
    echo "🎉 Deployment completed successfully!"
    echo ""
    echo "🔗 Access URLs:"
    echo "  • Main Application: http://localhost:3000"
    echo "  • API Documentation: http://localhost:3000/docs"
    echo "  • Health Check: http://localhost:3000/health"
    echo "  • pgAdmin: http://localhost:5050 (admin@example.com / admin123)"
    echo "  • Elasticsearch Head: http://localhost:9100"
    echo ""
}

# メイン実行
main() {
    case "${1:-deploy}" in
        "check")
            check_env
            check_docker
            check_env_file
            ;;
        "db")
            start_database
            run_migrations
            ;;
        "build")
            build_application
            ;;
        "deploy")
            check_env
            check_docker
            check_env_file
            start_database
            run_migrations
            build_application
            start_application
            post_deployment_check
            ;;
        "stop")
            echo "🛑 Stopping Production CMS..."
            docker-compose down
            echo "✅ Production CMS stopped"
            ;;
        "restart")
            echo "🔄 Restarting Production CMS..."
            docker-compose restart
            echo "✅ Production CMS restarted"
            ;;
        "logs")
            docker-compose logs -f
            ;;
        "status")
            docker-compose ps
            ;;
        *)
            echo "Usage: $0 {deploy|check|db|build|stop|restart|logs|status}"
            echo ""
            echo "Commands:"
            echo "  deploy  - Full deployment (default)"
            echo "  check   - Check environment and prerequisites"
            echo "  db      - Start database and run migrations"
            echo "  build   - Build application only"
            echo "  stop    - Stop all services"
            echo "  restart - Restart all services"
            echo "  logs    - Show application logs"
            echo "  status  - Show service status"
            exit 1
            ;;
    esac
}

# スクリプト実行
main "$@"
