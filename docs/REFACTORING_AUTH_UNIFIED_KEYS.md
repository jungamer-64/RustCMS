# 認証システム統合鍵管理リファクタリング

## 概要

Phase 5.7（JWT署名のEdDSA化）完了後、認証システムの統合性と保守性を向上させるためのリファクタリングを実施しました。

## 主な変更点

### 1. 統合鍵管理システム (`unified_key_management.rs`)

**目的**: JWT認証とBiscuit認可で使用する鍵を一元管理

**主な機能**:
- `UnifiedKeyPair`構造体: JWT用とBiscuit用の鍵ペアを統合
- 環境変数からの鍵ロード: `ED25519_PRIVATE_KEY_B64`
- 鍵ロード優先順位:
  1. 環境変数 (`ED25519_PRIVATE_KEY_B64`)
  2. ファイル (`./secrets/unified_ed25519.key`)
  3. 自動生成（開発環境のみ、`is_production=false`）
- 鍵のフィンガープリント: SHA256ハッシュでログ記録

**セキュリティ強化**:
- 本番環境では`is_production=true`設定で鍵の自動生成を禁止
- 鍵が存在しない場合はエラーを返す

**後方互換性**:
- 既存の`ed25519_keys.rs`と同じインターフェース
- 段階的な移行が可能

### 2. JWT EdDSA専用化 (`jwt.rs`)

**目的**: HS256サポートを廃止し、EdDSA（Ed25519）に統一

**主な変更**:
- `UnifiedKeyPair`を使用して鍵管理を統合
- `JwtService::new()`が`Result<Self, AuthError>`を返す
- `JwtClaims::user_id()`が`Result<Uuid, AuthError>`を返す
- HS256関連のコードを完全削除

**破壊的変更**:
- HS256で署名されたトークンは検証不可
- 既存トークンは再発行が必要

### 3. エラー型の詳細化 (`error.rs`)

**目的**: デバッグとエラーハンドリングの改善

**新しいエラーバリアント**:
```rust
InvalidTokenFormat,           // トークン形式エラー
InvalidTokenSignature,        // 署名検証エラー
TokenTypeMismatch { expected: String, actual: String },
BiscuitError(String),         // Biscuit関連エラー
PasswordHashError(String),    // パスワードハッシュエラー
```

**後方互換性**:
```rust
#[deprecated]
InvalidToken,                 // -> InvalidTokenFormat | InvalidTokenSignature
#[deprecated]
Biscuit(String),              // -> BiscuitError(String)
```

**追加メソッド**:
- `is_safe_to_expose(&self) -> bool`: ユーザーに公開可能か
- `user_message(&self) -> String`: ユーザー向けメッセージ
- `http_status_code(&self) -> u16`: HTTPステータスコード
- `log_level(&self) -> tracing::Level`: ログレベル

### 4. パスワードサービス分離 (`password_service.rs`)

**目的**: パスワード関連機能を独立モジュール化

**主な機能**:
- `hash_password()`: Argon2でハッシュ化
- `verify_password()`: タイミング攻撃対策付き検証
- `validate_password_policy()`: ポリシー検証
  - 最小8文字、最大128文字
  - 大文字、小文字、数字を各1文字以上含む
- `calculate_password_strength()`: 強度スコア（0-100）
  - 長さ: 最大40点
  - 文字種の多様性: 最大28点
  - 繰り返しチェック: 最大20点
  - 一般的なパターン: 最大10点

**セキュリティ機能**:
- Argon2id アルゴリズム
- タイミング攻撃対策（ダミー処理実行）
- 詳細なログ記録（エラー時も情報漏洩しない）

### 5. AuthService改善 (`service.rs`)

**目的**: 統合鍵管理システムの活用

**主な変更**:
- `UnifiedKeyPair`ベースに変更
- `new_with_repo()`が`Result<Self>`を返す
- `verify_user_password()`実装（TODO: Phase 9でUser entity拡張）
- パスワード関連メソッド追加

### 6. AuthConfig拡張 (`config/mod.rs`)

**新フィールド**:
```rust
pub is_production: bool,  // 本番環境フラグ
```

**用途**:
- 本番環境での鍵自動生成を禁止
- 開発環境での利便性を維持

## テスト結果

### パスワードサービス (5テスト)
```
✅ test_hash_and_verify_password
✅ test_password_policy_validation
✅ test_password_strength
✅ test_invalid_hash_format
✅ test_timing_attack_resistance
```

### JWT (5テスト)
```
✅ test_generate_and_verify_token_pair
✅ test_invalid_token_format
✅ test_tampered_token
✅ test_remember_me_expiry
✅ test_token_type_mismatch
```

### 統合鍵管理 (4テスト)
```
✅ test_generate_keypair
✅ test_save_and_load
✅ test_fingerprint
✅ test_biscuit_keypair_compatibility
```

### 全体結果
```
✅ 41 passed; 0 failed
```

## 移行ガイド

### 1. エラーハンドリングの更新

**旧コード**:
```rust
match auth_error {
    AuthError::InvalidToken => { /* ... */ }
    AuthError::Biscuit(msg) => { /* ... */ }
    _ => { /* ... */ }
}
```

**新コード**:
```rust
match auth_error {
    AuthError::InvalidTokenFormat => { /* ... */ }
    AuthError::InvalidTokenSignature => { /* ... */ }
    AuthError::BiscuitError(msg) => { /* ... */ }
    #[allow(deprecated)]
    AuthError::InvalidToken => { /* 後方互換 */ }
    #[allow(deprecated)]
    AuthError::Biscuit(msg) => { /* 後方互換 */ }
    _ => { /* ... */ }
}
```

### 2. JwtClaims::user_id()の更新

**旧コード**:
```rust
let user_id = claims.user_id();
```

**新コード**:
```rust
let user_id = claims.user_id().map_err(|e| {
    error!("Failed to parse user ID: {:?}", e);
    AppError::Authentication("Invalid user ID in token".to_string())
})?;
```

### 3. 環境変数の設定（本番環境）

```bash
# Ed25519秘密鍵をBase64でエクスポート
export ED25519_PRIVATE_KEY_B64="base64_encoded_private_key_here"
```

### 4. is_productionフラグの設定

**config/default.toml**:
```toml
[auth]
is_production = false  # 開発環境
```

**config/production.toml**:
```toml
[auth]
is_production = true  # 本番環境
```

## 今後の計画 (Phase 9)

### User entityの拡張
```rust
pub struct User {
    // ... 既存フィールド
    password_hash: Option<String>,  // 追加
}
```

### verify_user_password()の完全実装
```rust
async fn verify_user_password(&self, user: &User, password: &str) -> Result<()> {
    let hash = user.password_hash
        .as_ref()
        .ok_or(AuthError::InvalidPassword)?;
    
    self.password_service.verify_password(password, hash)
}
```

## 破壊的変更のまとめ

1. **HS256廃止**: 既存のHS256トークンは無効
2. **エラーバリアント**: `InvalidToken` → `Format/Signature`、`Biscuit` → `BiscuitError`
3. **Result返却**: `JwtService::new()`、`JwtClaims::user_id()`がResultを返す
4. **本番環境**: `is_production=true`で鍵自動生成不可

## 利点

### セキュリティ
- Ed25519への統一で署名強度向上
- 環境変数での鍵注入対応
- 本番環境での鍵自動生成禁止
- タイミング攻撃対策

### 保守性
- 鍵管理の一元化
- エラー型の詳細化でデバッグ容易
- パスワードサービスの独立化
- 後方互換性の維持

### パフォーマンス
- Ed25519の高速署名・検証
- Argon2idの適切なパラメータ設定

## 参考資料

- [EdDSA (Ed25519)](https://ed25519.cr.yp.to/)
- [Argon2](https://github.com/P-H-C/phc-winner-argon2)
- [Biscuit Token](https://www.biscuitsec.org/)
- [JWT Best Practices](https://datatracker.ietf.org/doc/html/rfc8725)

## 作成日

2025-01-XX

## 変更履歴

- 2025-01-XX: 初版作成（Phase 5.7完了後）
