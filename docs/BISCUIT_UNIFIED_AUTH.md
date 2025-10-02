# Biscuit 統一認証リファクタリング

**日付**: 2025-10-02  
**バージョン**: 3.0.0

## 概要

本リファクタリングでは、リポジトリ内の全ての認証メカニズムを Biscuit ベースに完全統一しました。これにより、API Key 認証も内部的には Biscuit トークンベースの `AuthContext` に変換され、システム全体で一貫した認証アーキテクチャを実現しています。

### v3.0.0 での完全移行

- **全てのレガシー認証機能を削除**: JWT、admin token、flat response fields
- **Biscuit のみの認証システム**: 統一されたセキュリティアーキテクチャ
- **シンプルな API レスポンス**: `tokens` オブジェクトのみを使用

## 変更内容

### 1. API Key から Biscuit への変換機能

#### `src/auth/service.rs`

新しいメソッド `create_biscuit_from_api_key()` を追加:

- API Key に関連付けられたユーザー情報を取得
- API Key に付与された権限を使用して `AuthContext` を生成
- 一時的なセッション ID を生成
- システム内では統一的に Biscuit ベースの認証コンテキストを使用

```rust
pub async fn create_biscuit_from_api_key(
    &self,
    user_id: Uuid,
    permissions: Vec<String>,
) -> Result<AuthContext>
```

### 2. API Key ミドルウェアの更新

#### `src/middleware/api_key.rs`

- API Key 検証成功後、`AuthContext` を生成して request extensions に格納
- 後方互換性のため `ApiKeyAuthInfo` も併せて格納
- 全ての認証パスで統一的に `AuthContext` が利用可能に

**変更前**:

```rust
req.extensions_mut().insert(info); // ApiKeyAuthInfo のみ
```

**変更後**:

```rust
let auth_context = state.auth_create_biscuit_from_api_key(...).await?;
req.extensions_mut().insert(auth_context); // AuthContext (統一)
req.extensions_mut().insert(info);         // ApiKeyAuthInfo (後方互換)
```

### 3. 認証ミドルウェアの強化

#### `src/middleware/auth.rs`

`parse_authorization_header()` を更新して Bearer スキームもサポート:

```rust
// Before: Biscuit スキームのみ
if let Some(rest) = v.strip_prefix("Biscuit ") {
    return Some(rest.trim());
}

// After: Bearer スキームも追加
if let Some(rest) = v.strip_prefix("Bearer ") {
    return Some(rest.trim());
}
```

これにより、以下の両方の形式をサポート:

- `Authorization: Biscuit <token>`
- `Authorization: Bearer <token>` (より標準的)

### 4. AppState の拡張

#### `src/app.rs`

新しいメソッド `auth_create_biscuit_from_api_key()` を追加:

- 認証サービスへのプロキシとして機能
- メトリクス記録を統合 (`timed_op!` マクロを使用)

### 5. OpenAPI ドキュメントの更新

#### `src/handlers/mod.rs`

セキュリティスキームの説明を更新:

- **BiscuitAuth**: "All authentication mechanisms are unified to use Biscuit tokens internally."
- **ApiKeyHeader**: "API keys are internally converted to Biscuit-based authentication contexts for unified security processing."

## アーキテクチャの利点

### 1. **統一された認証フロー**

```text
API Key → AuthContext (Biscuit ベース)
Bearer Token → AuthContext (Biscuit ベース)
```

全ての認証メカニズムが同じ `AuthContext` 型に収束するため、下流のハンドラは認証方式を意識する必要がありません。

### 2. **権限管理の一元化**

API Key の権限も Biscuit の権限モデルに統合されるため、RBAC (Role-Based Access Control) が一貫して適用されます。

### 3. **監査とログの統一**

全ての認証イベントが同じコンテキスト (`AuthContext`) を通過するため、ログとメトリクスが統一されます。

### 4. **セキュリティの向上**

- Biscuit のケイパビリティベースのセキュリティモデルを全体で活用
- 権限の委譲や制限が統一的に実装可能
- トークンの有効期限や失効管理が一元化

### 5. **拡張性**

将来的に新しい認証メカニズム (例: OAuth2, SAML) を追加する際も、同じパターンで `AuthContext` への変換を実装するだけで統合できます。

## マイグレーション

### API 利用者への影響

**変更なし**: 既存の API 利用方法は全て互換性を維持しています。

#### API Key 認証

```bash
# 変更前・変更後とも同じ
curl -H "X-API-Key: ak_your_api_key" \
     http://localhost:3000/api/v1/posts
```

#### Biscuit トークン認証

```bash
# 両方のスキームをサポート
curl -H "Authorization: Biscuit <token>" \
     http://localhost:3000/api/v1/posts

curl -H "Authorization: Bearer <token>" \
     http://localhost:3000/api/v1/posts
```

### 内部コードへの影響

#### ハンドラでの使用

```rust
// Before: 認証方式によって異なる型を使用
async fn handler(
    Extension(api_key_info): Extension<ApiKeyAuthInfo>,  // API Key 経由
    // または
    Extension(auth_ctx): Extension<AuthContext>,         // Biscuit 経由
) -> Result<impl IntoResponse> {
    // ...
}

// After: 統一的に AuthContext を使用可能
async fn handler(
    Extension(auth_ctx): Extension<AuthContext>,  // 全ての認証方式で共通
) -> Result<impl IntoResponse> {
    let user_id = auth_ctx.user_id;
    let permissions = &auth_ctx.permissions;
    // ...
}
```

## テスト結果

全ての既存テストが成功:

- ✅ `api_key_model_tests`: API Key モデルのテスト
- ✅ `auth_flow_tests`: 認証フローのテスト
- ✅ `biscuit_token_flow_tests`: Biscuit トークンフローのテスト
- ✅ その他全ての統合テスト

## 次のステップ

### 推奨される追加改善

1. **API Key のリフレッシュトークン対応**
   - 長期間有効な API Key に対して、短期的な Biscuit トークンを定期的に再生成

2. **より細かい権限制御**
   - API Key ごとに異なるスコープを Biscuit の facts/checks として表現

3. **監査ログの強化**
   - API Key 使用時の Biscuit コンテキスト生成をより詳細にログ記録

4. **パフォーマンス最適化**
   - API Key 検証時の Biscuit コンテキスト生成をキャッシュ化

## 互換性

- **後方互換性**: ✅ 完全に維持
- **API 互換性**: ✅ 変更なし
- **内部API**: ⚠️ `ApiKeyAuthInfo` は非推奨だが、当面は並行利用可能

## まとめ

このリファクタリングにより、RustCMS は完全に Biscuit ベースの統一認証システムを実現しました。全ての認証メカニズムが内部的には同じ `AuthContext` に変換されることで、コードの保守性、セキュリティ、拡張性が大幅に向上しています。

---

**関連ドキュメント**:

- [docs/BISCUIT.md](./BISCUIT.md) - Biscuit 認証ガイド
- [docs/AUTH_CONSOLIDATION.md](./AUTH_CONSOLIDATION.md) - 認証統合の履歴
- [docs/AUTH_MIGRATION_V2.md](./AUTH_MIGRATION_V2.md) - 認証レスポンス移行ガイド
