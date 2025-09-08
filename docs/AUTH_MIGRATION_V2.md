# Auth Response Migration (V2)

日付: 2025-09-09  
対象バージョン: 2.x 系 (後方互換段階) → 3.0.0 で旧型削除予定

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
| Phase 4 | メジャー (3.0.0) でフラットフィールド + feature 削除 | 予定 |

### Phase 4 計画 (3.0.0 予定)

目的: 最終的にレスポンス構造を `success + tokens + user (+ 将来 extra)` の最小形へ確定し、保守負荷と誤用リスクを除去します。

削除予定項目:

1. Cargo feature `legacy-auth-flat`
2. フラットフィールド: `access_token`, `refresh_token`, `biscuit_token`, `expires_in`, `session_id`, `token`
3. (安全確認後) 旧 `LoginResponse` / その OpenAPI スキーマ (feature gate 下でのみ存在)

スキーマ移行後の最終形 (3.0.0+):

```jsonc
{
  "success": true,
  "tokens": {
    "access_token": "...",
    "refresh_token": "...",
    "biscuit_token": "...",
    "expires_in": 3600,
    "session_id": "..."
  },
  "user": { /* UserInfo */ }
  // optional: "extra": { ... }
}
```

クライアント移行ガイドライン (3.0.0 へのアップグレード時):

- フラットトークン参照をすべて `response.tokens.*` へ置換
- `token` (access_token エイリアス) を除去
- OpenAPI 再生成: 旧フィールドがないことを前提に型を再生成
- 異常系ハンドリングに変更は無し (HTTP ステータス / エラー JSON 不変)

互換性ウィンドウ & タイムライン:

- 2.x 系: Deprecated フィールドは残存 (警告ベースで移行促進)
- 3.0.0: 削除 (コンパイルエラーとして顕在化し、早期に気付ける)

レポジトリ上のガード (推奨 CI 強化案):

1. `cargo build --no-default-features --features auth,database` (最小構成) でビルド継続確認
2. `cargo doc --features legacy-auth-flat` (2.x 系のみ) で警告数をメトリク化 (過剰増加を検知)
3. 3.0.0 ブランチ作成時に `grep -R "legacy-auth-flat"` が空であることを CI チェック

リスク評価 & 緩和:

| リスク | 内容 | 緩和策 |
|--------|------|--------|
| 未移行クライアント破損 | フラットフィールド直接参照でビルド失敗 | 2.x 期間に警告周知 + 明確な CHANGELOG 記載 |
| OpenAPI 生成キャッシュ | 旧スキーマが生成物に残留 | 3.0.0 移行ガイドでキャッシュクリア明示 |
| サードパーティ SDK ラグ | SDK 側更新遅延 | 早期アナウンス (2.x 内 README + CHANGELOG) |

作業チェックリスト (3.0.0 ブランチ時):

- [ ] `legacy-auth-flat` feature セクション削除 (Cargo.toml)
- [ ] `AuthSuccessResponse` の deprecated 属性除去 + フラットフィールド削除
- [ ] `LoginResponse` 削除 (`cfg` ブロック含む)
- [ ] OpenAPI の components から `LoginResponse` 除外コード削除
- [ ] テスト: フラットフィールド参照を tokens.* へ完全置換
- [ ] 生成されたドキュメント差分 (run_docs, dump_openapi) のレビュー
- [ ] CHANGELOG (Breaking Changes) へ明記

ステータス表更新条件: 上記チェックリスト完了後、Phase 4 を「完了」に書き換える。

## アクションアイテム

| ID | 項目 | 優先度 |
|----|------|--------|
| A1 | refresh エンドポイントも統一レスポンスへ移行 (オプション: `AuthSuccessResponse`) | 完了 |
| A2 | `token` フィールドに `#[deprecated]` 属性付与 (警告拡散検証後) | 完了 |
| A4 | `legacy-auth-flat` feature の最終削除 (メジャー) | Low |
| A3 | `AuthSuccess<T>::extra` 利用シナリオ検討 (2FA, 拡張 claims) | Low |
| A5 | Phase 4 (3.0.0) 削除計画ドキュメント化 | 完了 |

## 互換性とテスト

- 既存テストは `AuthSuccessResponse` へ更新済み (login/register)
- 追加回帰テスト案: refresh を統一形式に拡張した際の後方互換 JSON スナップショット

---

このガイドは認証レスポンス統一によるクライアント移行を迅速にするためのものです。質問や改善提案は ISSUE へ。
