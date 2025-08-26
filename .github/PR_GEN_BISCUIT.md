
# PR: feature/gen-biscuit-backups → main

この PR は `feature/gen-biscuit-backups` ブランチの変更を `main` にマージするためのものです。

主な変更点:

- `src/bin/gen_biscuit_keys.rs`: Biscuit キー生成の CLI（ファイル・env出力、バックアップ、保持、gzip 圧縮）
- テスト: `tests/gen_biscuit_keys_*.rs`（smoke / retention / compress）および関連テストの安定化
- `src/auth/mod.rs` 関連: Biscuit キーの永続化と検証ロジックの改善（base64 エンジンの正しい使用を含む）
- CI (`.github/workflows/rust.yml`): nasm インストール（rav1e ビルド依存対策）とバイナリの確実なビルド追加

検証:

- すべてのユニット・統合テストを GitHub Actions で実行・確認済み（最新の CI run が成功）

注意（セキュリティ）:

- リポジトリの過去コミットにシークレットが含まれていた場合、必ずキーをローテートしてください。履歴書き換え（BFG / git filter-repo）は破壊的です。管理者と調整のうえ実行してください。
- CI は `BISCUIT_PRIVATE_KEY_B64`, `BISCUIT_PUBLIC_KEY_B64`, `DATABASE_URL` を GitHub Secrets から取得する想定です。マージ前にリポジトリの Secrets を設定してください。

推奨のレビューポイント:

1. `src/bin/gen_biscuit_keys.rs` の CLI オプションとエラーメッセージ（ユーザビリティ）
2. `src/auth/mod.rs` のキー永続化ロジック（安全性・エッジケース）
3. CI の変更（ビルド時間増加の懸念がないか）

マージ後の次の作業:

- 必要ならシークレットのローテーションと（合意があれば）履歴書き換え手順の実行
- README に `gen_biscuit_keys` の簡単な使い方を追加

---

自動生成された PR 本文です。レビューをお願いします。
