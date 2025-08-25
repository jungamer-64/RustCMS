# 🚀 Enterprise CMS Backend

A high-performance, production-ready Content Management System API built with Rust and Axum, designed to handle large-scale traffic with enterprise-grade features.

## 🚀 Features

### Performance & Scalability

- **High-Performance Architecture**: Built with Rust and Axum for maximum performance
- **Database Connection Pooling**: PostgreSQL with optimized connection management
- **Redis Caching**: Multi-layer caching strategy for optimal response times
- **Rate Limiting**: Intelligent rate limiting to prevent abuse
- **Load Balancer Ready**: Stateless design for horizontal scaling

### Security

- **JWT Authentication**: Secure token-based authentication
- **Role-Based Access Control**: Granular permission system
- **Input Validation**: Comprehensive request validation
- **CORS Protection**: Configurable cross-origin resource sharing
- **SQL Injection Prevention**: Parameterized queries and type safety

### Monitoring & Observability

- **Prometheus Metrics**: Comprehensive metrics collection
- **Structured Logging**: Detailed logging with tracing support
- **Health Checks**: Endpoint and service health monitoring
- **Performance Tracking**: Request timing and performance analytics

### Developer Experience

- **OpenAPI Documentation**: Auto-generated Swagger documentation
- **Type Safety**: Rust's type system prevents runtime errors
- **Modern Async**: Tokio-based async runtime
- **Docker Support**: Production-ready containerization

## 📊 Architecture

```text
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Load Balancer │    │     Redis       │    │   PostgreSQL    │
│     (Nginx)     │◄──►│     Cache       │    │    Database     │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                 │                       ▲                       ▲
                 ▼                       │                       │
┌─────────────────┐              │                       │
│   CMS Backend   │              │                       │
│   (Rust/Axum)   │──────────────┼───────────────────────┘
│                 │              │
│  ┌─────────────┐│              │
│  │Rate Limiter ││──────────────┘
│  └─────────────┘│
│  ┌─────────────┐│
│  │   Auth      ││
│  └─────────────┘│
│  ┌─────────────┐│
│  │  Metrics    ││
│  └─────────────┘│
└─────────────────┘
```

## 🛠️ Quick Start

### Prerequisites

- Rust 1.75+
- PostgreSQL 13+
- Redis 6+
- Docker (optional)

### Local Development

1. **Clone the repository**

```bash
git clone <repository-url>
cd Rust-CMS  # or your chosen folder name
```

1. **Configure environment**

Config は `config/default.toml` を基点に環境変数で上書きできます。最低限:

```bash
cp .env.example .env   # （存在する場合）
# set DATABASE_URL=postgres://user:pass@localhost:5432/cms_db
```

1. **(Optional) Start external services**

```bash
docker compose up -d postgres redis  # Redis / search を使わないなら省略可
```

1. **Migrations (Diesel)**

Diesel を使用する構成では（feature `database` 有効時）:

```bash
cargo run --bin cms-migrate  # 実装されている簡易マイグレーションバイナリ
```

1. **Run server**

```bash
cargo run --bin cms-server
```

削減構成（外部サービス最小）でビルドしたい場合例:

```bash
cargo build --no-default-features --features "dev-tools,auth,database"
cargo run --no-default-features --features "dev-tools,auth,database" --bin cms-server
```

補助バイナリ:

- `cms-admin` : 管理/運用 CLI 操作（ユーザ作成等）
- `cms-migrate` : DB マイグレーション実行

デフォルトバイナリは `cms-server`（`Cargo.toml` の `default-run`）。

### Docker Deployment

```bash
# Build and start all services
docker-compose up -d

# View logs
docker-compose logs -f cms-backend

# Scale the backend
docker-compose up -d --scale cms-backend=3
```

## 📚 API Documentation / Routes

ベースパス: `http://localhost:3000/api/v1`

- API Info: `GET /api/v1` または `/api/v1/info`
- Health: `GET /api/v1/health`（`/liveness`, `/readiness` サブパスあり）
- Auth (feature=auth): `POST /api/v1/auth/register`, `POST /api/v1/auth/login`, `POST /api/v1/auth/logout`, `GET /api/v1/auth/profile`, `POST /api/v1/auth/refresh`
- Posts (feature=database): CRUD under `/api/v1/posts`
- Users (feature=database): CRUD under `/api/v1/users`
- Admin (feature=database): `/api/v1/admin/posts` (list/create), `/api/v1/admin/posts/:id` (delete)
- Search (feature=search): `/api/v1/search`, `/suggest`, `/stats`, `/reindex`, `/health`
- OpenAPI UI: `GET /api/docs`
- OpenAPI JSON: `GET /api/docs/openapi.json`

ヘルスチェック（ルート直下 `/health`）は簡易化された別実装が残る可能性がありますが、標準は `/api/v1/health` を利用してください。

### Authentication

All protected endpoints require a JWT token in the Authorization header:

```http
Authorization: Bearer <your-jwt-token>
```

### Example Requests (current routes)

#### Login and Get Token

```bash
curl -X POST http://localhost:3000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "demo_user",
    "password": "password"
  }'
```

#### Create a Post

```bash
curl -X POST http://localhost:3000/api/v1/posts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{
    "title": "My First Post",
    "content": "This is the content of my first post.",
    "published": true
  }'
```

#### Get Posts (with pagination)

```bash
curl "http://localhost:3000/api/v1/posts?page=1&limit=10"
```

#### Search (feature=search)

```bash
curl "http://localhost:3000/api/v1/search?q=rust"
```

drill --benchmark benchmark.yml
 
## 詳細: 設定・パフォーマンス・監視・テスト等

多くの本番運用向けの詳細（環境変数の項目、パフォーマンスチューニング、監視・ヘルスチェック、ロードテスト、セキュリティ設定など）は `README_PRODUCTION.md` にまとめてあります。

トップ README では開発・ビルドの最小限手順と参照先を示します。運用や本番デプロイ手順が必要な場合は次を参照してください:

- `README_PRODUCTION.md` — 本番用の詳細手順、チェックリスト、スクリプト、API エンドポイント一覧

必要なら、`README_PRODUCTION.md` の特定セクション（例: デプロイ手順、監視設定）をトップ README に抜粋して簡潔に表示します。希望があればどのセクションを抜粋するか教えてください。

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Write tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🆘 Support

- **Documentation**: Check the `/api/docs` endpoint for API documentation
- **Issues**: Report bugs via GitHub issues
- **Performance**: Use the `/metrics` endpoint for performance insights
- **Monitoring**: Set up Prometheus for production monitoring

---

## 🎯 Recent Improvements

This enterprise CMS backend has been completely refactored with the following improvements:

### ✅ Performance Enhancements

- **Database Connection Pooling**: Implemented with SQLx for PostgreSQL
- **Redis Caching**: Multi-layer caching strategy with automatic invalidation
- **Rate Limiting**: Intelligent, per-endpoint rate limiting
- **Async Processing**: Full async/await implementation with Tokio

### ✅ Security Improvements

- **JWT Authentication**: Secure token-based authentication system
- **Input Validation**: Comprehensive request validation with custom error handling
- **SQL Injection Prevention**: Parameterized queries and type safety
- **CORS Protection**: Configurable cross-origin resource sharing

### ✅ Scalability Features

- **Horizontal Scaling**: Stateless design for load balancer compatibility
- **Connection Management**: Optimized database and cache connections
- **Memory Efficiency**: Efficient data structures and memory management
- **Resource Optimization**: Fine-tuned resource usage for high throughput

### ✅ Developer Experience

- **OpenAPI Documentation**: Auto-generated Swagger documentation
- **Type Safety**: Rust's type system prevents runtime errors
- **Error Handling**: Comprehensive error handling with custom error types
- **Testing**: Unit and integration tests for reliability

### ✅ Monitoring & Observability

- **Prometheus Metrics**: Comprehensive metrics collection
- **Health Checks**: Detailed health monitoring for all services
- **Structured Logging**: Detailed logging with tracing support
- **Performance Tracking**: Request timing and performance analytics

This refactored CMS backend is now ready for enterprise-level deployments and can handle large-scale traffic with confidence.
