# 認証システム リファクタリング マイグレーションガイド

## 📋 概要

このドキュメントは、既存の認証システムから改善版（Phase 5.7完了版）への移行手順を説明します。

## 🔴 重大な変更点

### 1. パスワード検証の実装（緊急）

**現状**: パスワード検証が完全にスキップされている  
**対応**: すぐに実装が必要

#### Phase 9で対応すべきこと

**User entityの拡張**:

```rust
// src/domain/user.rs または src/models/user.rs

pub struct User {
    // 既存フィールド...
    password_hash: Option<String>,           // 新規追加
    last_login: Option<DateTime<Utc>>,       // 新規追加
}

impl User {
    pub fn password_hash(&self) -> Option<&str> {
        self.password_hash.as_deref()
    }
    
    pub fn update_last_login(&mut self) {
        self.last_login = Some(Utc::now());
    }
}
```

**UserRepositoryの拡張**:

```rust
#[async_trait]
pub trait UserRepository {
    // 既存メソッド...
    
    // 新規追加
    async fn update_password_hash(
        &self, 
        user_id: UserId, 
        password_hash: String
    ) -> Result<(), RepositoryError>;
    
    async fn update_last_login(
        &self, 
        user_id: UserId
    ) -> Result<(), RepositoryError>;
}
```

**データベースマイグレーション**:

```sql
-- migrations/XXXX_add_password_fields.sql

-- password_hash フィールドを追加
ALTER TABLE users ADD COLUMN password_hash VARCHAR(255);

-- last_login フィールドを追加
ALTER TABLE users ADD COLUMN last_login TIMESTAMP WITH TIME ZONE;

-- インデックスの追加（パフォーマンス最適化）
CREATE INDEX idx_users_last_login ON users(last_login);
```

### 2. 鍵管理の統一

**現状**: JWT用とBiscuit用で別々の鍵管理  
**改善**: 統合Ed25519鍵ペア（`UnifiedKeyPair`）

#### 移行手順

```bash
# 1. 既存の鍵をバックアップ
mkdir -p ./secrets/backup
cp ./secrets/ed25519.key ./secrets/backup/ed25519.key.$(date +%Y%m%d)
cp ./secrets/biscuit_private.b64 ./secrets/backup/biscuit_private.b64.$(date +%Y%m%d)

# 2. 新しい統合鍵を生成（開発環境）
# アプリケーション起動時に自動生成される
# ./secrets/unified_ed25519.key が作成される

# 3. 本番環境では環境変数で設定
export ED25519_PRIVATE_KEY_B64="<base64_encoded_key>"
export IS_PRODUCTION=true
```

**環境変数の設定例**:

```bash
# 開発環境 (.env.development)
IS_PRODUCTION=false
# ED25519_PRIVATE_KEY_B64は未設定（自動生成）

# 本番環境 (.env.production)
IS_PRODUCTION=true
ED25519_PRIVATE_KEY_B64="your_base64_encoded_private_key_here"
```

### 3. JWT署名アルゴリズムの統一

**現状**: HS256とEdDSAが混在  
**改善**: EdDSA (Ed25519) のみ

#### 既存トークンの扱い

```rust
// 移行期間中は両方をサポート（オプション）
// ただし、新規発行はすべてEdDSAを使用

pub struct AuthService {
    unified_keypair: Arc<UnifiedKeyPair>,
    jwt_service: Arc<JwtService>,
    old_jwt_secret: Option<String>,  // 移行期間用（オプション）
}

impl AuthService {
    pub async fn verify_access_token(&self, token: &str) -> Result<AuthContext> {
        // 新しいEdDSA鍵で試行
        match self.jwt_service.verify_access_token(token) {
            Ok(claims) => Ok(self.claims_to_context(claims)),
            Err(_) if self.old_jwt_secret.is_some() => {
                // 旧HS256トークンを検証（移行期間のみ）
                warn!("Legacy HS256 token detected, please refresh");
                self.verify_legacy_token(token).await
            }
            Err(e) => Err(e),
        }
    }
}
```

**推奨移行戦略**:

1. **即座移行**: 既存トークンを無効化し、ユーザーに再ログインを要求
2. **段階的移行**: 移行期間中（例: 1週間）は両方をサポート、その後旧形式を廃止

## 📦 新しいモジュール構成

```
src/auth/
├── mod.rs                      # モジュールエントリポイント
├── error.rs                    # 詳細なエラー型（改善版）
├── unified_key_management.rs   # 統合鍵管理（新規）
├── password_service.rs         # パスワードサービス（新規）
├── jwt.rs                      # JWTサービス（EdDSA統一版）
├── service.rs                  # 認証サービス（改善版）
├── session.rs                  # セッション管理
├── unified_context.rs          # 統合認証コンテキスト
└── biscuit.rs                  # Biscuit認可（既存）
```

## 🔄 段階的移行手順

### Phase 1: 準備（1-2日）

#### 1. 依存関係の確認

```toml
[dependencies]
# 既存の依存関係...
sha2 = "0.10"           # 鍵フィンガープリント用
hex = "0.4"             # 16進数エンコード用
argon2 = "0.5"          # パスワードハッシュ
ed25519-dalek = "2.1"   # EdDSA署名
```

#### 2. 設定の更新

```toml
# config/default.toml
[auth]
jwt_secret = "${JWT_SECRET}"
biscuit_root_key = "${BISCUIT_ROOT_KEY}"
bcrypt_cost = 12
session_timeout = 86400
access_token_ttl_secs = 3600
refresh_token_ttl_secs = 86400
remember_me_access_ttl_secs = 86400
is_production = false  # 開発環境

# config/production.toml
[auth]
# ... 同じ設定
is_production = true   # 本番環境
```

#### 3. テストの実行

```bash
# 認証関連のテストを実行
cargo test --package cms-backend --lib auth

# 期待される結果: 41 passed
```

### Phase 2: 鍵管理の移行（2-3日）

#### 1. 新しい鍵管理モジュールの確認

`unified_key_management.rs`が追加されていることを確認。

#### 2. 統合テスト

```rust
#[tokio::test]
async fn test_key_compatibility() {
    let keypair = UnifiedKeyPair::generate().unwrap();
    
    // JWT署名テスト
    let message = b"test message";
    let signature = keypair.signing_key().sign(message);
    assert!(keypair.verifying_key().verify(message, &signature).is_ok());
    
    // Biscuit鍵テスト
    let biscuit_kp = keypair.biscuit_keypair();
    assert_eq!(biscuit_kp.public().to_bytes().len(), 32);
    
    // フィンガープリント
    let fingerprint = keypair.fingerprint();
    assert_eq!(fingerprint.len(), 64); // SHA256 hex = 64文字
}
```

#### 3. 段階的置き換え

- [x] JWTサービスを`UnifiedKeyPair`使用に更新済み
- [x] Biscuitサービスは`UnifiedKeyPair`から鍵を取得
- [x] 既存の`ed25519_keys.rs`を削除

### Phase 3: エラーハンドリングの改善（1-2日）

#### 1. 新しいエラー型の確認

```rust
// src/auth/error.rs

pub enum AuthError {
    // 詳細化されたエラー
    InvalidTokenFormat,
    InvalidTokenSignature,
    TokenTypeMismatch { expected: String, actual: String },
    BiscuitError(String),
    PasswordHashError(String),
    
    // 後方互換性（非推奨）
    #[deprecated]
    InvalidToken,
    #[deprecated]
    Biscuit(String),
}
```

#### 2. エラーログの改善

```rust
// Before
.map_err(|_| AuthError::InvalidToken)?

// After
.map_err(|e| {
    error!("Token verification failed: {}", e);
    AuthError::InvalidTokenSignature
})?
```

#### 3. エラーハンドリングのテスト

```rust
#[test]
fn test_error_messages() {
    let err = AuthError::InvalidCredentials;
    assert!(err.is_safe_to_expose());
    assert_eq!(err.http_status_code(), 401);
    assert_eq!(err.user_message(), "Invalid credentials");
}
```

### Phase 4: パスワードサービスの実装（2-3日）

#### 1. PasswordServiceの確認

`password_service.rs`が追加されていることを確認。

```rust
let service = PasswordService::new();

// パスワードハッシュ化
let hash = service.hash_password("SecurePass123")?;

// パスワード検証
service.verify_password("SecurePass123", &hash)?;

// ポリシー検証
service.validate_password_policy("SecurePass123")?;

// 強度計算
let strength = service.calculate_password_strength("SecurePass123");
// strength: 0-100のスコア
```

#### 2. User entityの拡張（TODO: Phase 9）

```rust
// src/domain/user.rs

pub struct User {
    id: UserId,
    username: Username,
    email: Email,
    role: UserRole,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    
    // 新規追加
    password_hash: Option<String>,
    last_login: Option<DateTime<Utc>>,
}
```

#### 3. マイグレーションスクリプトの作成

```bash
# dieselマイグレーションの作成
diesel migration generate add_password_fields

# migrations/XXXX_add_password_fields/up.sql
cat > migrations/XXXX_add_password_fields/up.sql << 'EOF'
ALTER TABLE users ADD COLUMN password_hash VARCHAR(255);
ALTER TABLE users ADD COLUMN last_login TIMESTAMP WITH TIME ZONE;
CREATE INDEX idx_users_last_login ON users(last_login);
EOF

# migrations/XXXX_add_password_fields/down.sql
cat > migrations/XXXX_add_password_fields/down.sql << 'EOF'
DROP INDEX IF EXISTS idx_users_last_login;
ALTER TABLE users DROP COLUMN IF EXISTS last_login;
ALTER TABLE users DROP COLUMN IF EXISTS password_hash;
EOF
```

#### 4. 既存ユーザーのマイグレーション

```rust
// ユーティリティスクリプト: src/bin/migrate_passwords.rs

use cms_backend::auth::PasswordService;

#[tokio::main]
async fn main() -> Result<()> {
    let service = PasswordService::new();
    let users = load_all_users().await?;
    
    for user in users {
        if user.password_hash().is_none() {
            // 一時パスワードを生成してメール送信
            let temp_password = generate_temp_password();
            let hash = service.hash_password(&temp_password)?;
            
            update_user_password_hash(user.id(), hash).await?;
            send_password_reset_email(user.email(), temp_password).await?;
            
            info!("Migrated user: {}", user.username());
        }
    }
    
    Ok(())
}
```

### Phase 5: 認証サービスの更新（3-4日）

#### 1. 新しいAuthServiceの確認

`service.rs`が`UnifiedKeyPair`ベースに更新されていることを確認。

#### 2. エンドポイントの確認

```rust
// src/web/handlers/auth_v2.rs

pub async fn login(
    State(auth): State<Arc<AuthService>>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<AuthResponse>> {
    // ユーザー認証
    let user = auth.user_repo
        .find_by_email(&Email::new(&request.email)?)
        .await?
        .ok_or_else(|| AuthError::InvalidCredentials)?;
    
    // パスワード検証（TODO: Phase 9で実装）
    auth.verify_user_password(&user, &request.password).await?;
    
    // 認証レスポンス作成
    let response = auth.create_auth_response(user, request.remember_me.unwrap_or(false)).await?;
    
    Ok(Json(response))
}
```

#### 3. 統合テスト

```rust
#[tokio::test]
async fn test_full_auth_flow() {
    let config = AuthConfig::default();
    let repo = Arc::new(MockUserRepository::new());
    let service = AuthService::new_with_repo(&config, repo).unwrap();
    
    // ユーザー作成（Phase 9で実装）
    let password = "SecurePass123";
    let hash = service.password_service.hash_password(password).unwrap();
    
    // TODO: User entityにpassword_hashを追加後に有効化
    /*
    let user = create_test_user_with_password(hash);
    
    // ログインリクエスト
    let request = LoginRequest {
        email: "test@example.com".to_string(),
        password: password.to_string(),
        remember_me: Some(false),
    };
    
    // 認証
    let response = service.authenticate_and_create_response(request).await.unwrap();
    
    // トークン検証
    let ctx = service.verify_access_token(&response.tokens.access_token)
        .await
        .unwrap();
    
    assert_eq!(ctx.username, "testuser");
    */
}
```

### Phase 6: 本番環境への展開（1週間）

#### 1. ステージング環境でのテスト

```bash
# ステージング環境のセットアップ
export ENVIRONMENT=staging
export IS_PRODUCTION=true
export ED25519_PRIVATE_KEY_B64="$(cat /path/to/staging/key.b64)"

# データベースマイグレーション
diesel migration run

# アプリケーション起動
cargo run --release
```

**確認項目**:

- [ ] すべての機能が正常に動作
- [ ] パフォーマンステスト通過
- [ ] セキュリティ監査完了
- [ ] ログとメトリクスが正常

#### 2. 本番環境の準備

```bash
# 環境変数の設定（本番）
export ENVIRONMENT=production
export IS_PRODUCTION=true
export ED25519_PRIVATE_KEY_B64="$(cat /secure/path/production/key.b64)"

# データベースマイグレーション（本番）
diesel migration run --locked-schema

# バックアップの確認
pg_dump -h prod-db -U user cms_db > backup_$(date +%Y%m%d_%H%M%S).sql
```

#### 3. 段階的ロールアウト

**カナリアデプロイメント**:

```yaml
# kubernetes/deployment.yaml (例)
apiVersion: apps/v1
kind: Deployment
metadata:
  name: cms-backend-canary
spec:
  replicas: 1  # トラフィックの10%
  selector:
    matchLabels:
      app: cms-backend
      version: v2-auth-refactor
```

**トラフィック割合**:

1. 10% → 監視24時間
2. 50% → 監視12時間
3. 100% → 全面展開

**メトリクスの監視**:

```rust
// Prometheusメトリクス
auth_login_attempts_total
auth_login_failures_total
auth_token_verifications_total
auth_session_count
auth_password_strength_avg
```

#### 4. ロールバック計画

```rust
// フィーチャーフラグで制御
pub struct AppConfig {
    use_new_auth_system: bool,  // デフォルト: true
}

// 起動時の分岐
if config.use_new_auth_system {
    info!("Using new unified auth system");
    AuthService::new_with_repo(&config.auth, user_repo)?
} else {
    warn!("Using legacy auth system (rollback mode)");
    // 旧システム（必要に応じて実装）
    todo!("Implement legacy auth fallback")
}
```

## 📊 検証チェックリスト

### セキュリティ

- [x] Ed25519鍵が安全に生成・保存されている
- [x] タイミング攻撃対策が実装されている（パスワード検証）
- [x] エラーメッセージが機密情報を漏らさない
- [x] セッション管理が適切に実装されている
- [ ] パスワード検証が正しく実装されている（Phase 9）

### 機能

- [x] トークン生成が正常に動作する
- [x] トークンリフレッシュが正常に動作する
- [x] エラーハンドリングが詳細化されている
- [x] パスワードポリシーが実装されている
- [ ] ログインが正常に動作する（Phase 9: password_hash追加後）
- [ ] ログアウトが正常に動作する

### パフォーマンス

- [x] トークン検証が高速（< 10ms）
- [x] Ed25519署名が高速
- [x] セッションストアのメモリ使用量が適切
- [ ] ログイン時間が許容範囲内（< 500ms）（Phase 9で測定）

### 運用

- [x] ログが適切に出力されている
- [x] エラートラッキングが設定されている
- [ ] メトリクスが収集されている（TODO）
- [ ] ヘルスチェックが動作する（TODO）

## 🚨 よくある問題と解決策

### 問題1: 既存トークンが無効になる

**原因**: 鍵の変更または署名アルゴリズムの変更

**解決策**:

```rust
// 移行期間中は両方の鍵をサポート（オプション）
pub struct AuthService {
    unified_keypair: Arc<UnifiedKeyPair>,
    jwt_service: Arc<JwtService>,
    old_jwt_secret: Option<String>,  // 移行期間用
}

impl AuthService {
    pub async fn verify_access_token(&self, token: &str) -> Result<AuthContext> {
        // 新しいEdDSA鍵で試行
        match self.jwt_service.verify_access_token(token) {
            Ok(claims) => Ok(self.claims_to_context(claims)),
            Err(_) if self.old_jwt_secret.is_some() => {
                // 旧HS256トークンを検証（移行期間のみ）
                warn!("Legacy HS256 token detected");
                self.verify_legacy_hs256_token(token).await
            }
            Err(e) => Err(e),
        }
    }
}
```

### 問題2: パスワードハッシュの移行

**原因**: User entityに`password_hash`フィールドがない

**解決策**:

```rust
// 初回ログイン時に再ハッシュ化
async fn handle_first_login_after_migration(
    service: &AuthService,
    email: &str,
    password: &str,
) -> Result<User> {
    let user = service.user_repo.find_by_email(email).await?
        .ok_or(AuthError::InvalidCredentials)?;
    
    if user.password_hash().is_none() {
        warn!("User {} has no password hash, initiating migration", user.username());
        
        // レガシーシステムでパスワードを検証（必要に応じて）
        // verify_legacy_password(&user, password)?;
        
        // 新形式でハッシュ化
        let new_hash = service.password_service.hash_password(password)?;
        
        // ユーザーを更新
        service.user_repo.update_password_hash(user.id(), new_hash).await?;
        
        info!("Migrated password for user {}", user.username());
    }
    
    Ok(user)
}
```

### 問題3: セッションの不整合

**原因**: セッションバージョン管理の変更

**解決策**:

```rust
// セッションストアのクリア（開発環境のみ）
#[cfg(not(production))]
pub async fn clear_all_sessions_for_migration(service: &AuthService) {
    service.session_store.clear_all_sessions().await;
    warn!("All sessions cleared for migration (development only)");
}

// 本番環境では段階的にセッションを失効
#[cfg(production)]
pub async fn expire_old_sessions(service: &AuthService, cutoff: DateTime<Utc>) {
    // cutoff より古いセッションを削除
    service.session_store.cleanup_sessions_before(cutoff).await;
}
```

### 問題4: 鍵の読み込みエラー

**原因**: 環境変数が設定されていない、または鍵ファイルが存在しない

**解決策**:

```rust
// エラーメッセージの改善
match UnifiedKeyPair::load_or_generate(&key_config) {
    Ok(keypair) => keypair,
    Err(AuthError::BiscuitError(msg)) if config.is_production => {
        error!("Failed to load key in production: {}", msg);
        error!("Please set ED25519_PRIVATE_KEY_B64 environment variable");
        return Err(AppError::Configuration(
            "Ed25519 private key required in production".to_string()
        ));
    }
    Err(e) => return Err(e.into()),
}
```

## 📈 監視とメトリクス

### 重要な指標

```rust
// Prometheusメトリクス定義（TODO）
pub struct AuthMetrics {
    login_attempts: Counter,
    login_failures: Counter,
    token_verifications: Counter,
    token_refresh_count: Counter,
    session_count: Gauge,
    password_strength_avg: Gauge,
    auth_errors: Counter,
}

impl AuthService {
    pub fn collect_metrics(&self) -> AuthMetrics {
        AuthMetrics {
            session_count: self.session_store.active_count() as f64,
            // ...
        }
    }
}
```

### アラート設定（推奨）

| メトリクス | 閾値 | アクション |
|-----------|------|-----------|
| ログイン失敗率 | > 10% | 調査 |
| トークン検証エラー | > 5% | 調査 |
| セッション数 | 急激な増加 | スケーリング |
| パスワード強度平均 | < 50 | ポリシー見直し |
| 認証エラー率 | > 2% | システム確認 |

## 📚 参考資料

- [OWASP Authentication Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)
- [Ed25519 Signature Scheme](https://ed25519.cr.yp.to/)
- [JWT Best Practices (RFC 8725)](https://datatracker.ietf.org/doc/html/rfc8725)
- [Argon2 Password Hashing](https://github.com/P-H-C/phc-winner-argon2)
- [Biscuit Token Specification](https://www.biscuitsec.org/)

## 🎯 Phase 9実装チェックリスト

### 必須実装

- [ ] User entityに`password_hash: Option<String>`フィールド追加
- [ ] User entityに`last_login: Option<DateTime<Utc>>`フィールド追加
- [ ] UserRepositoryに`update_password_hash()`メソッド追加
- [ ] UserRepositoryに`update_last_login()`メソッド追加
- [ ] データベースマイグレーションスクリプト作成
- [ ] AuthServiceの`verify_user_password()`実装
- [ ] ログインエンドポイントでのパスワード検証有効化
- [ ] 既存ユーザーのパスワードマイグレーションスクリプト

### テスト

- [ ] パスワード検証の単体テスト
- [ ] ログインフローの統合テスト
- [ ] パスワードマイグレーションのテスト
- [ ] エンドツーエンドテスト

### ドキュメント

- [ ] API仕様書の更新
- [ ] READMEの更新
- [ ] デプロイ手順書の更新

---

**作成日**: 2025-10-20  
**最終更新**: 2025-10-20  
**ステータス**: Phase 5.7完了、Phase 9準備中
