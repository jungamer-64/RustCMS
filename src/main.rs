//! 統合CMSサーバーのエントリポイント
//!
//! cms-lightweight / cms-simple / cms-unified の機能を統合した単一バイナリです。
//! - 本番モード: データベース有効。安定運用向け設定で起動します。
//! - 開発モード: インメモリで軽量起動（featureや設定により挙動が変わります）。
//!
//! 起動フローの概略:
//! 1. 設定の読み込み（環境変数や設定ファイル）
//! 2. 依存サービスの初期化（DB/認証/キャッシュ/検索など、featureに応じて）
//! 3. ルータの構築と状態(AppState)の注入
//! 4. HTTPサーバーの待受開始
//!
//! English:
//! This server supports both production mode (with database) and development mode (in-memory).
//! It serves as the main unified entry point for the `RustCMS` backend.

use axum::Router as AxumRouter;
use std::net::SocketAddr;
use tracing::info;

use thiserror::Error;

use cms_backend::error::AppError as BackendAppError;
use cms_backend::routes::create_router;
use hyper::Error as HyperError;

/// Unified CMS server entrypoint（統合CMSサーバー起動）
///
/// Integrates functionality from:
/// - cms-lightweight: Initialization and config loading
/// - cms-simple: In-memory development mode and web interface  
/// - cms-unified: Consolidated API endpoints
///
/// This replaces the need for separate CMS binaries by providing a single,
/// unified entry point that can operate in different modes.
#[derive(Debug, Error)]
enum ServerError {
    #[error("Failed to initialize application state: {0}")]
    Init(#[from] BackendAppError),

    #[error("Invalid socket address: {0}")]
    AddrParse(#[from] std::net::AddrParseError),

    #[error("Failed to bind TCP listener: {0}")]
    Bind(#[from] std::io::Error),

    #[error("Server error: {0}")]
    Serve(#[from] HyperError),
}

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    // 概要: アプリケーション状態を初期化し、アドレスへバインドしてHTTPサーバーを起動します。
    // 入力: 環境変数/設定ファイル（bind host/port、有効化featureに依存）
    // 返り値: 起動成功で Ok(())、初期化やバインドに失敗すると Err
    // 副作用: DB接続/キャッシュ接続/検索インデックス準備などの外部IO
    // 注意: 非同期ランタイム上でブロッキング処理を避けること。
    // Initialize full AppState using shared helper
    let state = cms_backend::utils::init::init_app_state().await?;

    info!("🚀 Starting Unified CMS Server");
    info!("   Integrating cms-lightweight + cms-simple + cms-unified functionality");

    // Compute address from config before moving state
    let addr: SocketAddr =
        format!("{}:{}", state.config.server.host, state.config.server.port).parse()?;

    // Build router and attach state (we keep a clone to call shutdown later)
    let state_clone_for_router = state.clone();
    let router: AxumRouter = create_router().with_state(state_clone_for_router);

    // Actually start the HTTP server (this was missing in the original implementation)
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("🌐 CMS Server listening on http://{}", addr);
    info!("📚 API Documentation: http://{}/api/docs", addr);
    info!("🔍 Health Check: http://{}/api/v1/health", addr);
    info!("📈 Metrics: http://{}/api/v1/metrics", addr);

    // Log available endpoints based on enabled features
    #[cfg(feature = "auth")]
    info!("🔐 Authentication endpoints available at /api/v1/auth/*");

    #[cfg(feature = "database")]
    info!("💾 Database-backed endpoints available");

    #[cfg(not(feature = "database"))]
    {
        use tracing::warn;
        warn!("⚠️  Running in lightweight mode - no database features available");
    }

    #[cfg(feature = "search")]
    info!("🔍 Search endpoints available at /api/v1/search/*");

    // Start the server with graceful shutdown handling
    let server = axum::serve(listener, router).with_graceful_shutdown(shutdown_signal());
    server.await?;

    // TODO: Phase 5+ で AppState に shutdown メソッドを実装
    // After server returns (graceful shutdown triggered), run AppState shutdown
    // state.shutdown().await;

    info!("✅ Server shutdown complete");
    Ok(())
}

/// Listens for shutdown signals (Ctrl+C and SIGTERM on Unix) and returns
/// once one is received so the server can start graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    info!("🔌 Signal received, starting graceful shutdown");
}
