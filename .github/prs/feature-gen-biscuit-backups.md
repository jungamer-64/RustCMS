# PR: gen_biscuit_keys - backups, retention, compression

## 概要

この PR は開発者ユーティリティ `gen_biscuit_keys` を強化します。

- `--max-backups` によるバックアップ保持数の指定
- `--backup-compress` による gzip 圧縮（作成後、未圧縮ファイルを削除）
- バックアップ作成後の古いバックアップ削除（保持ポリシー）
- 統合テストを追加（保持/圧縮の検証）
- CI ワークフローを追加（`cargo build` / `cargo test`）

 
## 目的
開発者が Biscuit 鍵を安全に更新できるように、既存ファイルの自動バックアップと簡単な保持/圧縮ポリシーを提供します。これにより誤上書きを防ぎ、ディスク使用量の管理が容易になります。

 
## 変更点のハイライト

- `src/bin/gen_biscuit_keys.rs`
- 新しい CLI オプション: `--max-backups`, `--backup-compress`, `--backup-dir`
- `maybe_backup_file` と `maybe_backup_env` を拡張して保持と圧縮をサポート
- `enforce_backup_retention` を実装（最新 N 個を保持）
- `compress_file` 実装（gzip）

## テスト

- `tests/gen_biscuit_keys_retention.rs`：複数回実行後、`--max-backups` が働くことを確認
- `tests/gen_biscuit_keys_compress.rs`：`--backup-compress` により .gz ファイルが作られることを確認

## ドキュメント

- `README.md` に使用例を追加

## ローカル検証

以下はローカルで実行したコマンドと結果の要約です（主要な抜粋）：

```bash
cargo build  # 成功
cargo test   # 全テスト成功
```

 
## レビュー時チェックリスト

- [ ] セキュリティ: 秘密鍵（private key）がリポジトリやテスト結果に残っていないこと
- [ ] 動作: `--max-backups` のパラメータが意図した通り動作すること
- [ ] 動作: `--backup-compress` 実行時に .gz ファイルが生成され、元ファイルが削除されること
- [ ] テスト: 追加テストが CI で安定してパスすること
- [ ] ドキュメント: README の例が適切で理解可能であること

 
## PR を作るときのテンプレート
タイトル: gen_biscuit_keys: add backup retention and compression

本文に上記の「概要」「変更点のハイライト」「ローカル検証」を貼り付けてください。CI 結果が出たら、下のチェックリストをチェックしてマージを依頼してください。
