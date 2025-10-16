//! プレゼンテーション層 (Presentation Layer)
//!
//! HTTP リクエスト/レスポンス処理を担うレイヤーです。
//! - HTTP Handlers: 薄いリクエスト/レスポンス変換層
//! - Middleware: クロスカッティングな関心事（認証、CORS等）
//! - OpenAPI: API スキーマ定義
//!
//! # 設計原則
//! - ハンドラーは薄く保つ（ビジネスロジックはアプリケーション層に）
//! - HTTP 詳細（ステータスコード、ヘッダー）のみ扱う
//! - DTOのシリアライズ/デシリアライズのみ責務

#[cfg(feature = "restructure_presentation")]
pub mod http;

/// Presentation Layer prelude: 共通型のインポート
pub mod prelude {
    // Phase 4 で実装予定
}
