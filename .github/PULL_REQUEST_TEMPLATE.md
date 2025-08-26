## 概要

この PR は `gen_biscuit_keys` ユーティリティの強化を行います。

- `--max-backups` を追加してバックアップの保持数を制御
- `--backup-compress` を追加して作成したバックアップを gzip 圧縮
- バックアップ作成後の古いバックアップ削除（保持ポリシー）を実装
- テストを追加: バックアップ保持と圧縮の統合テスト
- CI ワークフローを追加して `cargo build` / `cargo test` を実行

## 変更点（詳細）

- ファイル: `src/bin/gen_biscuit_keys.rs`
  - CLI オプション: `--max-backups`, `--backup-compress`, `--backup-dir`
  - バックアップ作成ロジックのリファクタ
  - `compress_file` 実装（gzip 圧縮、元ファイル削除）
  - `enforce_backup_retention` 実装（最新 N 個のみ保持）

- テスト追加:
  - `tests/gen_biscuit_keys_retention.rs` (保持テスト)
  - `tests/gen_biscuit_keys_compress.rs` (圧縮テスト)

- ドキュメント:
  - `README.md` に使用例を追加

## テスト

ローカルで以下を実行済み:

```
cargo build
cargo test
```

すべてのテストが通っています。

## レビューチェックリスト

- [ ] セキュリティ: 秘密鍵が誤ってコミットされていないか
- [ ] 動作: `--max-backups` / `--backup-compress` の動作が期待通りか
- [ ] テスト: 新規追加テストが妥当で十分か
- [ ] CI: ワークフローが正しくトリガーされるか

## デプロイ / リリースメモ

- 開発ツール（`gen_biscuit_keys`）だけの変更です。本番 API には影響しません。

---
Please review and merge when ready.
