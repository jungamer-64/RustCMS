# Cargo Feature ガイド（RustCMS）

本プロジェクトは Cargo feature により機能を切り替えます。ビルドや挙動に影響するため、以下の方針に従ってください。

## 概要

- feature はコンパイル単位で有効/無効が決まります（実行時切替ではありません）。
- 互換性維持の一時的 feature は削除予定バージョンを明記します。

## 主な feature

- `auth`: 認証機能（argon2 でパスワードハッシュ、biscuit-auth でトークン）。
- `auth-flat-fields`: 旧フラットトークン互換フィールドをAPIレスポンスに追加（3.0.0で削除予定）。
- `cache`: Redis ベースのキャッシュ機能を有効化。
- `database`: Diesel/PostgreSQL を有効化（接続プール/マイグレーション）。
- `email`: メール送信を有効化。
- `legacy-auth-flat`: 歴史的 `LoginResponse` を OpenAPI スキーマに含めます（互換目的、3.0.0で削除予定）。
- `monitoring`: メトリクス/Prometheusなどの監視機能。
- `search`: Tantivy による全文検索。

## ロギング/テレメトリの方針

- `cms_backend::telemetry::TelemetryHandle` はプロセス全体のログ・メトリクス配管を維持します。ハンドルを `drop` しても即座に flush はされず、プロセス終了時に内部の `WorkerGuard` が確実に書き出しを行います。
- `LOG_OUTPUT=file:<path>` を選ぶと、ローテーションなし(`never`)の場合でも `rolling::never` の挙動によりファイルへ追記され続けます。外部の logrotate などでサイズ制御するか、`daily` / `hourly` トークンを付与してください。
- 現在の `monitoring` feature はスタブ実装で、今後 OpenTelemetry/metrics エクスポータを `TelemetryHandle` に統合することで API 安定性を維持する予定です。
- テストでは `serial_test` を用いてテレメトリ初期化を直列化しています。追加のテレメトリテストを作成する際は `#[serial]` アトリビュートの付与を忘れないでください。

### 今後の検討事項

- サイズベースのローテーションは未実装です。長期運用で必要になった場合は別バックエンドや外部ローテーションツールの導入を検討してください。
- 非推奨の `init_telemetry_with_verbose` は WorkerGuard を保持しないため、将来のメジャーバージョンで削除予定です。
- `LOG_FORMAT` に不正値が指定された場合は現在エラーになります。フォールバック戦略や警告ログによる運用改善も検討可能です。

## 方針と注意

- `legacy-*` / `*-flat*` は移行期間の互換向けです。コードコメントに削除予定を明記し、OpenAPI でも差分を説明します。
- feature によりパスやスキーマが変化する場合は、ハンドラのRustdocに明記します。
- 開発時は `development`、本番は `production`（`default`）を推奨。`minimal` は検証用。

## OpenAPI との整合

- `legacy-auth-flat` 有効時: 参考用に `LoginResponse` スキーマを含めます。
- 無効時: `AuthSuccessResponse` を統一スキーマとして使用し、`LoginResponse` は含めません。

