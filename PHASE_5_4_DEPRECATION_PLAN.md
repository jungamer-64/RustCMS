# Phase 5-4: API v1 Deprecation 計画

**開始日**: 2025-01-17
**予定期間**: 2-3週間
**目標**: API v1 エンドポイントの非推奨化と v2 への段階的移行

---

## 📋 概要

Phase 5-4 では、API v1 エンドポイントを段階的に非推奨化し、クライアントを API v2 へ移行させます。

### 進捗目標

- ✅ Phase 5-3 完成 (HTTP E2E + Performance Benchmark)
- 🔄 Phase 5-4 開始: API v1 Deprecation
- ⏳ Phase 5-5: レガシーコード削除
- ⏳ Phase 6: パフォーマンス最適化 + 本番環境準備

---

## 🎯 非推奨化戦略（段階的）

### Week 1: 警告フェーズ

#### 1.1 Deprecation ヘッダー追加

すべての v1 エンドポイントに `Deprecation` ヘッダーを追加します。

**対象エンドポイント** (約 50個):

| カテゴリ | エンドポイント数 | 例 |
|---------|-----------------|-----|
| Users | 8 | GET /api/v1/users/{id}, POST /api/v1/users |
| Posts | 10 | GET /api/v1/posts, POST /api/v1/posts |
| Comments | 8 | GET /api/v1/comments, POST /api/v1/comments |
| Tags | 6 | GET /api/v1/tags, POST /api/v1/tags |
| Categories | 6 | GET /api/v1/categories, POST /api/v1/categories |
| Search | 4 | GET /api/v1/search |
| Analytics | 4 | GET /api/v1/analytics/* |
| Auth | 2 | POST /api/v1/auth/login, POST /api/v1/auth/logout |
| その他 | 2 | 管理系エンドポイント |

#### 1.2 ドキュメント更新

- `docs/API.md`: v1 は「非推奨」マークを追加
- `DEPRECATION_NOTICE.md`: 新しいドキュメント作成（移行ガイド）

#### 1.3 テスト更新

`tests/deprecation_warning_tests.rs` を作成し、全 v1 エンドポイントで Deprecation ヘッダーが付与されているか検証します。

### Week 2-3: 段階的削除フェーズ

#### 2.1 メトリクス収集

- v1 vs v2 アクセス数の比率を追跡
- ログレベルを `INFO` に上げる

#### 2.2 v1 → v2 自動リダイレクト (オプション)

リダイレクトミドルウェアを実装し、v1 アクセスを v2 にフォワード

#### 2.3 旧ハンドラーの段階的削除

**削除予定** (Phase 5-5):

```
rm -rf src/handlers/v1.rs
rm -rf src/handlers/legacy/
rm -rf src/repositories/legacy_*.rs
```

---

## 📊 タイムライン & マイルストーン

### Week 1 (1月17-23日)

- [ ] Deprecation ヘッダー実装 (5 日間)
  - [ ] 全 v1 エンドポイントに追加
  - [ ] テスト作成
  - [ ] ドキュメント更新
- [ ] メトリクス収集開始 (1 日間)
- [ ] CI/CD パイプライン確認 (1 日間)

### Week 2-3 (1月24-2月6日)

- [ ] クライアント移行状況の監視 (5 日間)
  - [ ] ログ分析
  - [ ] API 使用状況レポート
- [ ] v1 アクセス < 10% を確認 (3 日間)
  - [ ] 必要に応じて移行期間を延長
- [ ] Phase 5-5 準備 (4 日間)
  - [ ] ロールバック計画確認
  - [ ] テスト準備

---

## 🔧 実装チェックリスト

### Phase 5-4a: Deprecation ヘッダー追加

- [ ] **全 v1 エンドポイント**（~50個）に以下を追加:
  - `Deprecation: true`
  - `Sunset: Sun, 17 Mar 2025 00:00:00 GMT`
  - `Link: </api/v2/...>; rel="successor-version"`

- [ ] **テスト作成**:
  - `test_v1_deprecation_headers.rs` (50+ アサーション)
  - ヘッダー検証: Deprecation, Sunset, Link

- [ ] **ドキュメント作成**:
  - `DEPRECATION_NOTICE.md` (移行ガイド)
  - API ドキュメント更新

- [ ] **ログレベル調整**:
  - v1 アクセスを `warn!()` で記録

### Phase 5-4b: クライアント移行監視

- [ ] **メトリクス収集**:
  - v1 vs v2 アクセス数の比率
  - エンドポイント別アクセス統計

- [ ] **移行状況確認**:
  - [ ] 1週間後: v1 アクセス < 50%
  - [ ] 2週間後: v1 アクセス < 20%
  - [ ] 3週間後: v1 アクセス < 10% (Phase 5-5 開始条件)

- [ ] **通知発送** (オプション):
  - [ ] クライアント向けメール通知
  - [ ] 移行ガイド共有

### Phase 5-4c: レガシーコード準備

- [ ] **ロールバック計画確認**:
  - `ROLLBACK_PLAN.md` の Phase 5-4 セクション確認

- [ ] **依存関係分析**:
  - v1 ハンドラーに依存するコード特定
  - 削除前に移行完了確認

---

## 📈 成功指標

| 指標 | 目標 | 検証方法 |
|------|-----|---------|
| **v1 アクセス率** | < 10% | ログ分析 |
| **v2 アクセス率** | > 90% | ログ分析 |
| **API テスト合格率** | 100% | CI パス |
| **Deprecation ヘッダー** | 全 v1 で付与 | テスト検証 |
| **ドキュメント完成度** | 100% | マニュアルレビュー |

---

## ⚠️ リスク & 対応

| リスク | 確率 | 影響 | 対応 |
|------|------|------|-----|
| v1 使用率が 20% 以上残る | 中 | 高 | 非推奨期間を延長 (+1ヶ月) |
| v2 で互換性問題発見 | 低 | 高 | ホットフィックス (Phase 5-2 参照) |
| クライアント移行失敗 | 低 | 中 | サポート強化 + 技術支援チーム |

---

## 📚 参考ドキュメント

- `PHASE_5_3_COMPLETION_TRACKING.md` - Phase 5-3 完成記録
- `PHASE_5_3_HTTP_E2E_GUIDE.md` - HTTP E2E テスト実行ガイド
- `ROLLBACK_PLAN.md` - Phase 別ロールバック手順
- `RESTRUCTURE_PLAN.md` - 全体構造再編計画

---

**作成日**: 2025-01-17
**ステータス**: 計画中
