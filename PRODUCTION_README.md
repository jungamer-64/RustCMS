# Production CMS

**Enterprise-grade Content Management System built with Rust**

A high-performance, scalable CMS built with modern technologies designed for production environments that can handle large-scale access.

## 🏗️ Architecture

### Core Technologies
- **Backend**: Rust + Axum (async web framework)
- **Database**: PostgreSQL with Diesel ORM
- **Search**: Elasticsearch for full-text search
- **Cache**: Redis for session and data caching
- **Authentication**: 
  - biscuit-auth (token-based authentication)
  - WebAuthn/FIDO2 (passwordless authentication)

### Key Features
- 🚀 **High Performance**: Rust's zero-cost abstractions and memory safety
- 📈 **Scalable**: Built for large-scale access with connection pooling
- 🔒 **Secure**: Modern authentication with WebAuthn and biscuit tokens
- 🔍 **Full-text Search**: Elasticsearch integration for powerful search
- 📱 **REST API**: Complete API with OpenAPI documentation
- 🐳 **Containerized**: Full Docker deployment setup
- 📊 **Monitoring**: Built-in metrics and health checks

## 🚀 Quick Start

### Prerequisites
- Docker & Docker Compose
- Git

### 1. Clone and Setup
```bash
git clone <repository-url>
cd rust-backend
cp .env.example .env
# Edit .env with your production values
```

### 2. Deploy with Docker
```bash
# Windows
.\deploy.bat

# Linux/macOS
./deploy.sh
```

### 3. Access the Application
- **Main Application**: http://localhost:3000
- **API Documentation**: http://localhost:3000/api/docs
- **Health Check**: http://localhost:3000/health
- **pgAdmin**: http://localhost:5050 (admin@example.com / admin123)
- **Elasticsearch Head**: http://localhost:9100

## 📋 Manual Setup

### 1. Environment Configuration
Copy `.env.example` to `.env` and configure:

```env
# Database
DATABASE_URL=postgresql://cms_user:secure_password@localhost:5432/production_cms

# Authentication
JWT_SECRET=your-super-secret-jwt-key
SESSION_SECRET=your-session-secret-key

# Services
REDIS_URL=redis://localhost:6379
ELASTICSEARCH_URL=http://localhost:9200

# WebAuthn
WEBAUTHN_ORIGIN=http://localhost:3000
WEBAUTHN_RP_ID=localhost
```

### 2. Start Services
```bash
# Start database services
docker-compose up -d postgres elasticsearch redis

# Run database migrations
# Windows: .\migrate.bat
# Linux/macOS: ./migrate.sh

# Build and run application
cargo build --release
cargo run --release
```

## 🗄️ Database Schema

### Core Tables
- **users**: User authentication and profiles
- **posts**: Content management with full metadata
- **categories/tags**: Content organization
- **comments**: User engagement system
- **media**: File and asset management
- **sessions**: Session management
- **api_keys**: API access control
- **webauthn_credentials**: Passwordless authentication
- **settings**: System configuration

### Features
- ✅ Full referential integrity with foreign keys
- ✅ Performance indexes on all query columns
- ✅ Automatic `updated_at` triggers
- ✅ UUID primary keys for scalability
- ✅ JSONB support for flexible data

## 🔐 Authentication System

### Supported Methods
1. **Traditional**: Username/password with JWT tokens
2. **Passwordless**: WebAuthn/FIDO2 with hardware keys
3. **API Keys**: For programmatic access
4. **Sessions**: Secure session management

### Authorization
- Role-based access control (RBAC)
- Permission-based authorization with biscuit tokens
- API rate limiting and throttling

## 🔍 Search & Performance

### Elasticsearch Integration
- Full-text search across all content
- Advanced search with filters and facets
- Bulk indexing for performance
- Real-time index updates

### Performance Features
- Connection pooling for database efficiency
- Redis caching for frequently accessed data
- Optimized database queries with proper indexing
- Async request handling for high concurrency

## 📁 Project Structure

```
rust-backend/
├── src/
│   ├── main.rs              # Application entry point
│   ├── lib.rs               # Library exports
│   ├── models/              # Database models
│   ├── services/            # Business logic
│   ├── handlers/            # HTTP request handlers
│   ├── middleware/          # Custom middleware
│   ├── routes/              # Route definitions
│   ├── auth/                # Authentication modules
│   ├── cache/               # Cache management
│   ├── config/              # Configuration
│   └── utils/               # Utility functions
├── migrations/              # Database migrations
├── docker-compose.yml       # Production services
├── Dockerfile              # Application container
├── deploy.sh / deploy.bat   # Deployment scripts
└── migrate.sh / migrate.bat # Migration scripts
```

## 🐳 Docker Deployment

### Services
- **cms-backend**: Main Rust application
- **postgres**: PostgreSQL database
- **elasticsearch**: Search engine
- **redis**: Cache and sessions
- **pgadmin**: Database administration
- **elasticsearch-head**: Search administration

### Volumes
- PostgreSQL data persistence
- Elasticsearch data persistence
- Redis data persistence
- Application uploads

### Networks
- Internal network for service communication
- Exposed ports for external access

## 🛠️ Development

### Local Development
```bash
# Install dependencies
cargo build

# Run with hot reload
cargo watch -x run

# Run tests
cargo test

# Check code quality
cargo clippy
cargo fmt --check
```

### Database Management
```bash
# Create new migration
diesel migration generate <name>

# Run migrations
diesel migration run

# Revert migrations
diesel migration revert
```

## 📊 API Endpoints

### Authentication
- `POST /api/v1/auth/register` - User registration
- `POST /api/v1/auth/login` - Login with credentials
- `POST /api/v1/auth/logout` - Logout
- `POST /api/v1/auth/refresh` - Refresh JWT token

### WebAuthn
- `POST /api/v1/webauthn/register/start` - Start registration
- `POST /api/v1/webauthn/register/finish` - Complete registration
- `POST /api/v1/webauthn/login/start` - Start authentication
- `POST /api/v1/webauthn/login/finish` - Complete authentication

### Content Management
- `GET /api/v1/posts` - List posts with pagination
- `POST /api/v1/posts` - Create new post
- `GET /api/v1/posts/{id}` - Get specific post
- `PUT /api/v1/posts/{id}` - Update post
- `DELETE /api/v1/posts/{id}` - Delete post

### Search
- `GET /api/v1/search` - Search content
- `GET /api/v1/search/suggestions` - Search suggestions

### Admin
- `GET /api/v1/admin/users` - Manage users
- `GET /api/v1/admin/stats` - System statistics
- `GET /api/v1/admin/settings` - System settings

## 🔧 Configuration

### Environment Variables
See `.env.example` for all available configuration options.

### Key Settings
- **Database**: Connection pooling, timeouts
- **Authentication**: Token expiry, session timeout
- **File Upload**: Size limits, allowed types
- **Rate Limiting**: Request limits, burst capacity
- **CORS**: Allowed origins, credentials

## 📈 Monitoring & Logging

### Health Checks
- `GET /health` - Application health
- `GET /api/v1/health` - API health
- Database connectivity checks
- Service dependency checks

### Metrics
- Request/response metrics
- Database query performance
- Cache hit/miss ratios
- Authentication success rates

### Logging
- Structured JSON logging
- Request tracing
- Error tracking
- Performance monitoring

## 🔒 Security Features

### Data Protection
- Password hashing with bcrypt
- SQL injection prevention with prepared statements
- XSS protection with input validation
- CSRF protection with tokens

### Authentication Security
- JWT token with expiry
- Session management with Redis
- WebAuthn for passwordless authentication
- API key management

### Infrastructure Security
- HTTPS enforcement (in production)
- Rate limiting and DDoS protection
- Input validation and sanitization
- Secure headers configuration

## 📝 Default Admin Account

After deployment, use these credentials for initial access:

- **Username**: admin
- **Email**: admin@example.com
- **Password**: admin123

⚠️ **Important**: Change the default password immediately after first login!

## 🚀 Production Checklist

### Before Deployment
- [ ] Update all secrets in `.env`
- [ ] Configure backup strategy
- [ ] Set up monitoring alerts
- [ ] Configure SSL certificates
- [ ] Review security settings
- [ ] Test all functionality

### After Deployment
- [ ] Change default admin password
- [ ] Verify all services are running
- [ ] Test authentication flows
- [ ] Verify database connections
- [ ] Check search functionality
- [ ] Validate API endpoints

## 🔄 Backup & Recovery

### Automated Backups
- Daily PostgreSQL database backups
- Elasticsearch index snapshots
- File upload backups
- Configuration backups

### Manual Backup
```bash
# Database backup
docker-compose exec postgres pg_dump -U cms_user production_cms > backup.sql

# Restore from backup
docker-compose exec -T postgres psql -U cms_user production_cms < backup.sql
```

## 🎯 Performance Tuning

### Database Optimization
- Connection pooling configuration
- Query optimization with EXPLAIN
- Index usage monitoring
- Regular VACUUM and ANALYZE

### Application Optimization
- Async request handling
- Cache optimization
- Memory usage monitoring
- CPU profiling

## 🔧 Troubleshooting

### Common Issues
1. **Database Connection Failed**
   - Check PostgreSQL service status
   - Verify connection string in `.env`
   - Check network connectivity

2. **Search Not Working**
   - Verify Elasticsearch service status
   - Check index creation
   - Validate search configuration

3. **Authentication Issues**
   - Verify JWT secret configuration
   - Check session storage (Redis)
   - Validate WebAuthn configuration

### Debug Commands
```bash
# Check service status
docker-compose ps

# View logs
docker-compose logs -f cms-backend

# Test database connection
docker-compose exec postgres psql -U cms_user -d production_cms

# Test Elasticsearch
curl http://localhost:9200/_cluster/health
```

## 📞 Support

For issues and questions:
1. Check the troubleshooting section
2. Review application logs
3. Verify service status
4. Check configuration settings

## 📄 License

[License information here]
