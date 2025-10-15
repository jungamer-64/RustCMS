# GitHub Copilot / AI 開発者向け 指示 — RustCMS

目的: このリポジトリでAI支援のコード作成やリファクタリングを行う際に、すぐに生産的になれる必須の知識と明確な守るべきルールをまとめます。

---

## 1) 大局 (Big picture)
- 単一クレートで複数バイナリを持つ（`cms-server` が default-run、`cms-migrate`、`cms-admin`、`dump_openapi` 等は `src/bin/*`）。バイナリごとに責務が分かれています（実行用／管理用／マイグレーション等）。
- 機能フラグで機能群を切り替える設計（`auth`, `database`, `cache`, `search`, ...）。CIは複数の feature セットでビルド/テストします（`--all-features`, `--no-default-features` 等）。
- 中核は `AppState`（`src/app.rs`）で、サービス（DB／Auth／Cache／Search）・イベントバス・メトリクスなどを集約。初期化は `AppStateBuilder` 経由。
- イベント駆動: `src/events.rs` の `AppEvent`（単一 enum） と `create_event_bus(capacity)` を使う。`spawn_event_listeners`（`src/listeners.rs`）でリスナーを立ち上げ、`tokio::spawn` で背景タスクとして実行。

## 2) 変更・実装時に最初に確認するファイル（優先）
- `src/app.rs` — AppState / AppStateBuilder（機能を追加する際はここを更新）
- `src/events.rs` — イベント定義と EventBus 型
- `src/listeners.rs` — リスナーの spawn／イベント処理方針（必ず再読）
- `src/error.rs` — `AppError` と HTTP 変換ルール（エラー表現はここで統一）
- `src/handlers/`, `src/repositories/`, `src/models/` — 既存のレイヤー構造を素早く理解するため
- `Cargo.toml` と `.github/workflows/ci.yml` — ビルド/テスト matrix と feature ポリシーを確認する
- `config/` フォルダ（`default.toml` / `production.toml`）— 実行時設定のキー名や既定値

## 3) 具体的なコード規約・パターン（このリポジトリ固有）
- イベントの発行は Fire-and-Forget: ほとんどの箇所で `let _ = event_bus.send(AppEvent::...);` を採用。リスナーは冪等／再実行可能に実装すること。
  - リスナー側では "最新の正しい状態をDBから取得する" 方針が採用されています（例: `state.db_get_user_by_id(data.id)`）。イベントは軽量データに留める。
- `AppStateBuilder::build` は feature-初期化不足をパニックで検出する設計。ビルド時に `cfg(feature = "...")` に合わせてフィールドを追加すること。
- trait 設計スタイルが混在しています（例: `SessionStore` は `async_trait`、`UserRepository` は `BoxFuture` を返す非-async_traitパターン）。新しい trait を追加する際は、**既存モジュールのスタイルに合わせる**（一貫性を重視）。
- エラーは `AppError` に集約して `IntoResponse` で HTTP へ変換します。ハンドラ内では `crate::Result<T>` を返す慣習を踏襲してください。
- 型安全化のため NewType / 値オブジェクトが増えてきています（例: `domain/value_objects/*` の計画）。識別子や検証済み文字列は専用型で扱う方針を優先。

## 4) ビルド / テスト / ローカル実行の必須コマンド（開発者向け）
- 形式チェック: `cargo fmt --all -- --check` と `cargo clippy --workspace --all-targets --all-features -- -D warnings`（CI と同じ clippy ポリシー）
- 全ビルド（CI と同等）: `cargo build --workspace --all-targets --locked --all-features`（もしくは matrix の feature セットに合わせる）
- テスト（ローカルで CI を模す）:
  - DB/Redisを必要とする場合は環境変数を設定（例: `DATABASE_URL=postgres://postgres:REPLACE_ME@localhost:5432/cms_test`）。
  - マイグレーション: `cargo run --bin cms-migrate -- migrate --no-seed`（CIの実行例を参照）
  - テスト実行（CIスタイル）: `cargo test --workspace --no-fail-fast <feature-args>`
- スナップショット: `cargo insta test`（CI で実行されるため、スナップショットを更新する場合は慎重に）
- OpenAPI 出力: `OPENAPI_OUT=./openapi-full.json cargo run --features "auth database search cache" --bin dump_openapi`
- 統合テスト: CI の `integration-tests` ジョブを参照（BISCUIT鍵の扱い・DBマイグレーション手順あり）。

## 5) CI の重要な前提（守るべきこと）
- CI は `RUSTFLAGS: -D warnings` で警告をエラー化しているため、警告が出ないように修正すること。
- CI matrix は複数の feature セット（`--all-features` / `--no-default-features` / カスタム）でビルド/テストします。ローカルで変更の影響範囲を確認するには各 feature セットでのビルドを推奨。
- 依存関係追加時は `cargo-deny` / `cargo-audit` のチェックが存在するので、新しい crate の導入は CI での警告を確認してからマージする。

## 6) インテグレーション・外部依存とリソース
- PostgreSQL（Diesel）、Redis、Tantivy（ローカルインデックス）、Biscuit-auth/WebAuthn、rustls 等が統合ポイント。関連実装は `infrastructure/` 以下にまとまる想定。
- Integration テストや CI は DB/Redis コンテナを用いるため、ローカル実行時には同等のサービスを立ち上げること。
- BISCUIT 秘密鍵は CI では secrets 経由で与えられます。ローカルで不足する場合は CI に倣って `gen_biscuit_keys` バイナリ（`src/bin/gen_biscuit_keys.rs`）で一時生成可能。

## 7) 変更時のチェックリスト（AI がコードを生成/変更する際）
- 変更箇所に対応する feature gate（`cfg(feature = "...")`）の追加/更新を忘れないこと。
- `AppState` にサービスを追加する場合は `AppStateBuilder` に optional フィールドを追加し、`build()` で検査・panic を維持する。
- `AppEvent` を拡張する際は軽量データにし、既存リスナーの挙動と互換性を確認する。リスナーは必ず冪等であること。
- エンドポイントの変更は OpenAPI (dump_openapi) と insta スナップショットに反映させること。
- テストを追加したら、該当する feature セットで `cargo test --workspace` を実行して CI マトリクスと同等の検証を行う。

## 8) 参考（必読）
- `src/app.rs` — AppState と初期化ロジック（重要）
- `src/events.rs` — AppEvent / EventBus（イベント設計の単一の出発点）
- `src/listeners.rs` — イベントリスナーの起動と実装方針
- `src/error.rs` — エラーの一元化と HTTP マッピング
- `.github/workflows/ci.yml` — CI の実行手順と feature matrix（ローカル検証はここを参照）
- `RESTRUCTURE_PLAN.md` と `RESTRUCTURE_EXAMPLES.md` — 現在の再編計画と実装例（方針確認用）
- `.github/instructions/codacy.instructions.md` — Codacy 連携ルール（ファイル編集後はコマンド実行が必須なルールあり）

---

このドキュメントを基に自動生成や修正を行います。内容に不備や追加して欲しいリスト（例: 他の重要なファイル、よくある失敗例、開発者ごとの運用慣習）があれば教えてください。
