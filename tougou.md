
以下は、認証ロジックの重複を最小化し、無駄のない状態へ効率的に到達するための統合計画（Sequential Thinking + Codacy 対応）です。実際の重複箇所は調査フェーズで確定させ、設計・実装・検証を最短経路で回す前提の計画です。

目的

認証ロジックの単一実装化（Single Source of Truth）
仕様・実装・ドキュメントの不一致解消
CI/Codacy による重複再発防止
成果物

単一の認証抽出・検証コンポーネント（ヘッダ抽出/トークン検証/ユーザ解決）
ルーティング層での認証適用一元化（公開/保護ルートの重複表現を撤廃）
OpenAPI/README の統一
単体/統合テストの網羅
Codacy/CI 設定（重複・品質ゲート）
ゴール指標（達成条件）

Authorization ヘッダのパース処理が1カ所のみ
トークン検証のコア関数/サービスが1カ所のみ
公開ルート判定がルータ構成から自動的に決まる（定数列挙を撤廃）
 
## 認証ロジック統合計画（Sequential Thinking + Codacy 対応）

以下は、認証ロジックの重複を最小化し、無駄のない状態へ効率的に到達するための統合計画です。実際の重複箇所は調査フェーズで確定させ、設計・実装・検証を最短経路で回す前提です。

## 目的

- 認証ロジックの単一実装化（Single Source of Truth）
- 仕様・実装・ドキュメントの不一致解消
- CI/Codacy による重複再発防止

## 成果物

- 単一の認証抽出・検証コンポーネント（ヘッダ抽出/トークン検証/ユーザ解決）
- ルーティング層での認証適用一元化（公開/保護ルートの重複表現を撤廃）
- OpenAPI/README の統一
- 単体/統合テストの網羅
- Codacy/CI 設定（重複・品質ゲート）

## ゴール指標（達成条件）

- Authorization ヘッダのパース処理が 1 カ所のみ
- トークン検証のコア関数/サービスが 1 カ所のみ
- 公開ルート判定がルータ構成から自動的に決まる（定数列挙を撤廃）
- 認証関連の関数は公開 API として 2 系統以内（例: verify→AuthContext / validate→User）
- Codacy の重複検出で認証関連の重複が 0 件
- テストで認証 OK/NG/権限/期限切れ/スキーム差異を網羅

## 進め方（Sequential Thinking）

### S1. インベントリ作成（検出）

- 目的: 重複候補を網羅的に抽出
- VS Code 検索クエリ（正規表現ON推奨）
  - Authorization|Bearer|Biscuit|token|apikey|api[-_ ]key|login|session|admin
  - verify|validate|auth|middleware|extract|parse|is_public|public[_-]route
- ripgrep（Linux）

```bash
rg -n "Authorization|Bearer|Biscuit|api[_-]?key|verify|validate|is_public" src
```

- 出力: ファイル→関数→責務のマップ（抽出/検証/権限/公開判定/ユーザ解決）

### S2. 重複の型分類

- 同一責務の多重 API（例: verify_xxx/validate_xxx が複数）
- 実装重複（同じヘッダ抽出/トークン検証のコピー）
- 知識重複（公開ルート定義、スキーム表現の分散）
- バックフィル/互換ラッパによる段階的重複

### S3. 目標アーキテクチャ定義（最小構成）

- 共通抽出: Authorization ヘッダ → トークン抽象型 TokenKind (Bearer/Biscuit/APIKey/…)
- 認証サービス: AuthService（verify→AuthContext, validate→User）
- ルーティングでの適用: 認証レイヤは保護ルートにのみ適用（is_public_route 撤廃）
- スキーム方針: Bearer のみ or Biscuit/Bearer 両対応のどちらかに統一（仕様=実装）

### S4. マイグレーション方針

- 既存 API を段階的に非推奨化（deprecated）し一時ラッパに集約 → 呼び先を新実装へ置換 → 削除
- ルート保護はルータ側でミドルウェア適用範囲を切る（公開ルート列挙を解体）
- ドキュメントと OpenAPI を最後に同期

### S5. 実装（小さく安全に差分を刻む）

1. 共通抽出関数/トークン型導入
2. AuthService 導入（コア検証 1 カ所化）
3. 既存ミドルウェアの呼び先を AuthService 経由に切替
4. ルータへ認証レイヤ適用（公開ルート判定撤廃）
5. 古い API を deprecate → 削除

### S6. 検証

- 単体テスト: ヘッダ抽出、トークン検証、期限/権限、エラーメッセージ
- 統合テスト: 公開/保護ルート、良/悪トークン、API キー、管理者トークン
- 回帰: 主要エンドポイントの E2E スモーク

### S7. ドキュメント/CI/Codacy

- README/OpenAPI の用語・スキーム統一
- Codacy 設定追加（重複検知・品質ゲート）
- CI に clippy/format/test/coverage/deny を追加

## 設計（提案サンプル）

- トークン抽象と共通抽出
- 認証サービスの単一化
- ミドルウェアの簡素化

### ルーティング適用例（方針）

- ルータ構築時に「保護ルートのルートグループ」にのみ auth_middleware（layer）を適用
- ヘルスチェック/メトリクス/ログイン/ドキュメント系は「公開グループ」で層を付けない
- これにより is_public_route の手動管理を撤廃

## 作業計画（スプリント別）

### Sprint 0（0.5日）

- S1〜S2 実施、重複マップと対象スコープ確定
- リスク・互換要件の合意（Bearer/Biscuit 方針）

### Sprint 1（1〜1.5日）

- 共通抽出・AuthService 導入（新規コード＋テスト）
- 既存ミドルウェアの呼び先切替（ラッパ経由）

### Sprint 2（1日）

- ルータ側でのレイヤ適用に変更、is_public_route 撤廃
- 古い検証関数群の deprecate

### Sprint 3（0.5〜1日）

- OpenAPI/README 整理、命名統一
- 統合テスト/E2E スモーク

### Sprint 4（0.5日）

- 不要 API/ラッパ削除、最終リント/フォーマット
- Codacy/CI 最終化

## VS Code/ターミナル手順（例）

### 重複探索

- VS Code: Ctrl+Shift+F で正規表現検索（前述クエリ）
- ターミナル:

```bash
rg -n "Authorization|Bearer|Biscuit" src
rg -n "is_public|public[_-]route" src
rg -n "verify|validate" src | sort
```

### 影響範囲の可視化

- `code --goto path:line` でジャンプ
- GitLens で呼び出し関係を確認

### Codacy/CI 設定（重複再発防止）

- codacy-analysis.yml（例）
- CI ジョブ（GitHub Actions の一例）

## レビューと合意ポイント

- 認証スキームの最終方針（Bearer のみ vs Biscuit 併用）
- 公開ルートの確定（ルータ上の適用範囲）
- 既存 API の非推奨化の段階・期間
- バックフィル/レガシー互換の扱い（必要最小限に圧縮）

## 受け入れ基準（Definition of Done）

- 認証の抽出・検証・ユーザ解決が単一実装に集約
- 公開ルート管理の重複が 0
- OpenAPI/README が実装と一致
- テスト成功率 100%、主要フローのカバレッジ確保
- Codacy の重複検出 0、品質ゲート通過

## 次アクション

- Sprint 0 の調査を開始して重複マップを提示
- その結果に基づき、上記スケジュールの確定と着手順序の最終合意

---

## 意思決定ログ（Decision Log）

- 認証スキーム: Bearer を標準。必要時のみ Biscuit スキーム文字列も許容（実装と OpenAPI を一致）。
- 公開ルート管理: ルータ構成側のみで管理。is_public_route の列挙は廃止。
- 認証 API の表面: 2 系統に統一
  - verify(token) -> AuthContext
  - validate(token) -> User
- API キー: ミドルウェアは抽出のみ。検証は AppState/Service に集約。
- ドキュメント: README と OpenAPI は実装と完全同期。

## 具体的な変更一覧（Change List）

1. Authorization 抽出の一本化
   - 新規: `parse_authorization_header(allow_biscuit: bool)` を導入
   - 統合: 既存のヘッダ抽出コードはこの関数を呼ぶように変更
2. 認証サービス（AuthService）の単一実装
   - 新規: `AuthService::verify(TokenKind) -> AuthContext`
   - 新規: `AuthService::validate_user(TokenKind) -> User`
   - 既存エイリアス整理:
     - `verify_jwt` → 非推奨（内部移行後削除）
     - `verify_biscuit` → `verify` に統合
     - `verify_biscuit_with_user` → `validate_user` に統合
     - `validate_token` → `validate_user` に名称統一
3. ルータ構成でのレイヤ適用
   - 公開ルートグループ: health/metrics/login/docs など（認証レイヤ未適用）
   - 保護ルートグループ: それ以外（認証レイヤ適用）
   - `is_public_route` の削除
4. API キー認証の責務整理
   - ミドルウェア: `X-API-Key` 抽出のみ
   - AppState/Service: ルックアップ、ハッシュ照合、期限検査、タッチ更新、レガシー回収
   - バックフィル関数を Service 側に寄せて一元管理
5. ドキュメント/仕様の同期
   - OpenAPI: securitySchemes を実装に合わせて Bearer（必要なら Biscuit も）に同期
   - README: 認証手順・ヘッダ例を実装と一致させる
6. 非推奨 API の段階的削除
   - 段階 1: `@deprecated` アノテーションと警告コメント
   - 段階 2: 呼び出し置換完了後に削除

## Deprecated API マップ（置換表）

- `verify_jwt(...)` → `verify(...)`
- `verify_biscuit(...)` → `verify(...)`
- `verify_biscuit_with_user(...)` → `validate_user(...)`
- `validate_token(...)` → `validate_user(...)`
- `is_public_route(path)` → ルータ構成のレイヤ適用に移行（削除）

## ルータ再構成手順（例: axum 方針）

1) 公開ルートを公開グループに集約
2) 認証レイヤ（auth_layer）を保護グループにのみ適用
3) ルートの移設によって `is_public_route` 参照を完全に排除
4) 統合テストで公開/保護の適用確認

## OpenAPI/README 同期手順

- OpenAPI:
  - components.securitySchemes:
    - BearerAuth（type: http, scheme: bearer, bearerFormat: JWT or Biscuit）
    - BiscuitAuth を残す場合は scheme: "Biscuit" の説明を注記（実装が許容する場合のみ）
  - グローバル/個別エンドポイントの security を整理
- README:
  - Authorization: `Bearer <token>` を基本形として提示
  - Biscuit ヘッダは実装が許容時のみ明記
  - 公開/保護ルートの説明は「ルータ構成で管理」に更新

## テストマトリクス（最小セット）

- Authorization 抽出
  - なし/空白/未知スキーム/大小文字差/余分スペース
  - Bearer 正常 / Biscuit 許容設定時の正常
- トークン検証
  - 正常/期限切れ/署名不正/スコープ不足/セッション不整合
- API キー
  - 正常/不一致/期限切れ/レガシー回収経路
- 公開/保護ルート
  - 公開ルートは未認証で 200
  - 保護ルートは未認証 401、誤トークン 401、スコープ不足 403
- 回帰
  - 主要エンドポイントの E2E スモーク

## 品質/運用（Codacy/CI）

- CI:
  - `cargo fmt --check`
  - `cargo clippy -- -D warnings`
  - `cargo test --all`
- Codacy:
  - 重複・品質ゲートを有効化
  - 認証関連の重複検出は 0 を維持
- 運用ルール:
  - 認証に関するロジック追加は AuthService と共通抽出にのみ置く
  - ルータ側以外で公開/保護の分岐を作らない

## リスクと緩和

- 互換性崩れ（API 名称変更）
  - 段階的非推奨と型/戻り値互換のラッパを短期提供
- ドキュメント不一致
  - PR を Docs 同期込みで必須化
- バックフィル経路の取りこぼし
  - 統合テストにレガシーキーのテストケースを追加

## ロールバック計画

- ルータ適用の切替は専用 PR に分割
- 問題時はレイヤ適用コミットのみ revert
- 非推奨 API 削除前のタグを残す（hotfix 可能化）

## 実行計画（PR 分割）

- PR-1: 共通抽出 + AuthService 導入 + 単体テスト
- PR-2: 既存ミドルウェアの呼び先切替 + 統合テスト
- PR-3: ルータ再構成（is_public_route 撤廃）
- PR-4: OpenAPI/README 同期
- PR-5: 非推奨 API 削除 + 最終クリーニング

## 実装スケルトン（適用順）

### 1) 共通抽出とトークン抽象

```rust
// filepath: /home/jungamer/Desktop/RustCMS/src/auth/service.rs
pub enum TokenKind {
    Bearer(String),
    Biscuit(String),
    ApiKey(String),
}

pub struct ParsedAuthHeader {
    pub kind: TokenKind,
}

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("missing authorization header")]
    MissingAuthorization,
    #[error("unsupported authorization scheme")]
    InvalidScheme,
    #[error("invalid token")]
    InvalidToken,
    #[error("expired token")]
    Expired,
    #[error("forbidden")]
    Forbidden,
    #[error("internal error")]
    Internal,
}

pub fn parse_authorization_header(value: &str, allow_biscuit: bool) -> Result<ParsedAuthHeader, AuthError> {
    let v = value.trim();
    if let Some(rest) = v.strip_prefix("Bearer ") {
        return Ok(ParsedAuthHeader { kind: TokenKind::Bearer(rest.trim().to_owned()) });
    }
    if allow_biscuit {
        if let Some(rest) = v.strip_prefix("Biscuit ") {
            return Ok(ParsedAuthHeader { kind: TokenKind::Biscuit(rest.trim().to_owned()) });
        }
    }
    Err(AuthError::InvalidScheme)
}
```

### 2) AuthService の単一窓口

```rust
// filepath: /home/jungamer/Desktop/RustCMS/src/auth/service.rs
// ...existing code...
pub struct AuthContext {
    pub subject: String,
    pub scopes: Vec<String>,
    // 必要に応じて user_id, tenant_id, session_id 等を拡張
}

pub trait AuthService {
    fn verify(&self, token: &TokenKind) -> Result<AuthContext, AuthError>;
    fn validate_user(&self, token: &TokenKind) -> Result<crate::models::user::User, AuthError>;
}

pub struct DefaultAuthService {
    // 署名鍵、DB 接続、設定など
    // pub db: DbPool, pub verifier: Verifier, ...
}

impl Default for DefaultAuthService {
    fn default() -> Self { Self { /* 初期化 */ } }
}

impl AuthService for DefaultAuthService {
    fn verify(&self, token: &TokenKind) -> Result<AuthContext, AuthError> {
        match token {
            TokenKind::Bearer(t) | TokenKind::Biscuit(t) => {
                // 署名検証/期限/スコープを一元実装（Biscuit でもここへ集約）
                // Ok(AuthContext { subject, scopes })
                Err(AuthError::InvalidToken) // TODO: 実装
            }
            TokenKind::ApiKey(_) => Err(AuthError::Forbidden),
        }
    }

    fn validate_user(&self, token: &TokenKind) -> Result<crate::models::user::User, AuthError> {
        let _ctx = self.verify(token)?;
        // DB から User 解決
        // Ok(user)
        Err(AuthError::Internal) // TODO: 実装
    }
}
```

### 3) ミドルウェアは抽出→サービス呼び出しのみ

```rust
// filepath: /home/jungamer/Desktop/RustCMS/src/middleware/auth_layer.rs
use axum::{http::Request, extract::State};
use tower::{Layer, Service};
use std::task::{Context, Poll};
use crate::auth::service::{parse_authorization_header, DefaultAuthService, AuthError};

#[derive(Clone)]
pub struct AuthLayer {
    pub allow_biscuit: bool,
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthMiddleware<S>;
    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddleware { inner, allow_biscuit: self.allow_biscuit }
    }
}

#[derive(Clone)]
pub struct AuthMiddleware<S> {
    inner: S,
    allow_biscuit: bool,
}

impl<B, S> Service<Request<B>> for AuthMiddleware<S>
where
    S: Service<Request<B>, Response = axum::response::Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        let header = req.headers().get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok());
        let header = match header {
            Some(h) => h,
            None => {
                // 401 を返すハンドリングに差し替え可
                return self.inner.call(req);
            }
        };

        // State からサービスを取り出す設計にするなら Extension/State を利用
        let _ = parse_authorization_header(header, self.allow_biscuit);
        // 成功時に拡張へ context を注入する設計が可能

        self.inner.call(req)
    }
}
```

### 4) ルータ構成（公開/保護をレイヤで分離）

```rust
// filepath: /home/jungamer/Desktop/RustCMS/src/routes/mod.rs
// ...existing code...
use axum::{Router, routing::get, Extension};
use crate::middleware::auth_layer::AuthLayer;
use crate::auth::service::DefaultAuthService;

pub fn app_router() -> Router {
    let svc = DefaultAuthService::default();

    let public = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .route("/api/auth/login", get(login))
        .route("/api/docs", get(docs));

    let protected = Router::new()
        .route("/api/v1/posts", get(list_posts))
        .route("/api/v1/posts/:id", get(get_post))
        .layer(AuthLayer { allow_biscuit: false })
        .layer(Extension(svc));

    public.merge(protected)
}
```

### 5) 非推奨 API の一時ラッパ

```rust
// filepath: /home/jungamer/Desktop/RustCMS/src/auth/mod.rs
// ...existing code...
#[deprecated(note = "Use AuthService::verify instead")]
pub fn verify_biscuit(token: &str) -> Result<AuthContext, AuthError> {
    // 新実装を呼ぶ
    // DefaultAuthService::default().verify(&TokenKind::Bearer(token.to_owned()))
    unimplemented!()
}

#[deprecated(note = "Use AuthService::validate_user instead")]
pub fn validate_token(token: &str) -> Result<User, AuthError> {
    // DefaultAuthService::default().validate_user(&TokenKind::Bearer(token.to_owned()))
    unimplemented!()
}
```

### 6) OpenAPI と README の同期テンプレ

```yaml
# filepath: /home/jungamer/Desktop/RustCMS/openapi/partials/security.yml
components:
  securitySchemes:
    BearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT
security:
  - BearerAuth: []
```

```md
# filepath: /home/jungamer/Desktop/RustCMS/README.md
## 認証
デフォルトは Authorization: Bearer <token> を使用します。
Biscuit スキームを許容する場合は、設定で allow_biscuit=true を有効化してください。
```

### 7) 最小テスト雛形

```rust
// filepath: /home/jungamer/Desktop/RustCMS/tests/auth_header_parse.rs
use rustcms::auth::service::{parse_authorization_header, AuthError};

#[test]
fn bearer_ok() {
    let p = parse_authorization_header("Bearer abc", false).unwrap();
    match p.kind { rustcms::auth::service::TokenKind::Bearer(s) => assert_eq!(s, "abc"), _ => panic!() }
}

#[test]
fn biscuit_rejected_when_not_allowed() {
    let e = parse_authorization_header("Biscuit xyz", false).unwrap_err();
    matches!(e, AuthError::InvalidScheme);
}
```

## 適用チェックリスト

- [ ] parse_authorization_header を導入し既存抽出箇所を置換
- [ ] AuthService を導入し verify/validate_user を 1 カ所化
- [ ] ルータで保護ルートにのみ AuthLayer を適用（is_public_route 削除）
- [ ] 非推奨 API に #[deprecated] を付与し呼び先を移行
- [ ] OpenAPI/README を同期
- [ ] 単体/統合テストを追加し CI で緑化