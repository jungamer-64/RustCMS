# Auth Response Migration (V2)

日付: 2025-09-09  
対象バージョン: 次回マイナーバージョン (後方互換段階) → 将来メジャーで旧型削除予定

## Feature Flag (`legacy-auth-flat`)

旧 `LoginResponse` 型（およびその OpenAPI スキーマ露出）は Cargo feature `legacy-auth-flat` 有効時のみコンパイル/ドキュメント化されるようになりました。デフォルトでは無効であり、新規クライアントは統一スキーマ `AuthSuccessResponse` のみを参照します。

```bash
# 旧 LoginResponse を含めたい場合のみ
cargo build --features legacy-auth-flat
```

これにより、後方互換を維持しつつ警告 (deprecated フィールド) を利用した移行促進が可能です。将来のメジャーでは feature 自体を削除予定です。

## 概要

`login` / `register` エンドポイントの認証レスポンスが旧 `LoginResponse` から **統一スキーマ `AuthSuccessResponse`** に移行しました。新スキーマは `tokens` コンテナを中心とした構造を提供しつつ、旧クライアントを壊さないためフラットなトークンフィールドも併置しています。

```text
AuthSuccessResponse {
  success: true,
  tokens: { access_token, refresh_token, biscuit_token, expires_in, session_id },
  user: { ...UserInfo },
  access_token, refresh_token, biscuit_token, expires_in, session_id, token
}
```

`token` フィールドは `access_token` のエイリアス (旧クライアント互換) です。`refresh` エンドポイントも現在は直接 `AuthSuccessResponse` 互換構造を返し、内部で `AuthTokens` を一貫利用します (旧 `RefreshResponse` は削除済み)。

## 目的

1. トークン関連フィールド重複の排除 (内部では `AuthTokens` へ集約)
2. OpenAPI スキーマ統一でクライアント生成を簡素化
3. 将来追加される可能性のあるメタ (`extra`) / セッション属性拡張に備えた拡張点確保

## クライアント移行手順

| ステップ | 旧 | 新 | 備考 |
|----------|----|----|------|
| 1 | `response.access_token` | `response.tokens.access_token` | 旧フィールドは当面存続 |
| 2 | `response.refresh_token` | `response.tokens.refresh_token` | 同上 |
| 3 | `response.biscuit_token` | `response.tokens.biscuit_token` | Refreshでは空文字列の場合あり |
| 4 | - | `response.tokens.session_id` | 新規取得推奨 |
| 5 | `response.token` | `response.tokens.access_token` | `token` は非推奨 |

### 推奨コード (TypeScript例)

```ts
interface AuthTokens { access_token: string; refresh_token: string; biscuit_token: string; expires_in: number; session_id: string; }
interface AuthSuccessResponse { success: boolean; tokens: AuthTokens; user: any; access_token: string; refresh_token: string; biscuit_token: string; expires_in: number; session_id: string; token: string; }

function extractTokens(r: AuthSuccessResponse) {
  return r.tokens ?? {
    access_token: r.access_token,
    refresh_token: r.refresh_token,
    biscuit_token: r.biscuit_token,
    expires_in: r.expires_in,
    session_id: r.session_id,
  };
}
```

## API 変更一覧

| エンドポイント | 変更 | 互換性 |
|----------------|------|--------|
| POST /api/v1/auth/login | レスポンス型を AuthSuccessResponse に変更 | 旧フィールド残存 |
| POST /api/v1/auth/register | 同上 | 同上 |
| POST /api/v1/auth/refresh | レスポンスを AuthSuccessResponse 互換形式に統一 | 完全互換 |

## サーバ内部実装ポイント

| コンポーネント | 内容 |
|---------------|------|
| `utils/auth_response.rs` | `AuthTokens`, `AuthSuccess<T>`, `AuthSuccessResponse` を定義 |
| `handlers/auth.rs` | `AuthResponse` から `AuthSuccessResponse` へ `From` で変換 |
| `openapi.rs` | 新スキーマを components に登録 |

## 段階的廃止ポリシー

| フェーズ | 内容 | 状態 |
|---------|------|------|
| Phase 1 | 新スキーマ導入＋旧フィールド温存 | 完了 |
| Phase 2 | クライアントガイド/ドキュメント公開 (本ファイル) | 完了 |
| Phase 3 | `token` / フラットフィールド deprecate + feature gate (`legacy-auth-flat`) | 完了 |
| Phase 4 | メジャーリリースでフラットフィールド削除 & feature 削除 | 予定 |

## アクションアイテム

| ID | 項目 | 優先度 |
|----|------|--------|
| A1 | refresh エンドポイントも統一レスポンスへ移行 (オプション: `AuthSuccessResponse`) | 完了 |
| A2 | `token` フィールドに `#[deprecated]` 属性付与 (警告拡散検証後) | 完了 |
| A4 | `legacy-auth-flat` feature の最終削除 (メジャー) | Low |
| A3 | `AuthSuccess<T>::extra` 利用シナリオ検討 (2FA, 拡張 claims) | Low |

## 互換性とテスト

- 既存テストは `AuthSuccessResponse` へ更新済み (login/register)
- 追加回帰テスト案: refresh を統一形式に拡張した際の後方互換 JSON スナップショット

---

このガイドは認証レスポンス統一によるクライアント移行を迅速にするためのものです。質問や改善提案は ISSUE へ。
