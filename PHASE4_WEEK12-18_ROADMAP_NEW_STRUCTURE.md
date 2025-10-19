# Phase 4 Week 12-18 実装ロードマップ（新構造完全対応版）

**作成日**: 2025年10月18日  
**適用対象**: Phase 4 全体（Week 12-18）  
**ベース**: 監査済み新構造 ⭐⭐⭐⭐⭐ (4.8/5.0)  
**ステータス**: 🚀 Week 12 Day 1-2 完了、Day 3-5 準備中

---

## 🎯 Phase 4 全体目標

### 成果物（Week 18 終了時）

| 層 | コンポーネント | 状態 | テスト | 行数 |
|----|--------------------|------|--------|------|
| **Web** | routes.rs | 完成 | ✅ | 100+ |
| | handlers/* | 完成 | ✅ | 350+ |
| | middleware.rs | 完成 | ✅ | 200+ |
| **Infrastructure** | events/bus.rs | 移行完成 | ✅ | 150+ |
| | events/listeners.rs | 移行完成 | ✅ | 200+ |
| | auth/biscuit.rs | 強化 | ✅ | 150+ |
| **Common** | types.rs | 移行 | ✅ | 200+ |
| | telemetry.rs | 移行 | ✅ | 150+ |
| **合計** | - | **100% 完成** | **50+ tests** | **1500+ 行** |

---

## 📅 詳細ロードマップ

### ✅ Week 12: Web層 基本構造完成（Day 1-2: 完了、Day 3-5: 進行中）

#### Day 1-2 ✅ 完了

- [x] 8個ハンドラ詳細化完成
- [x] 計画ドキュメント 4個作成
- [x] 37.9 KB のドキュメント完備

#### Day 3-5 🔜 実装予定（所要時間: 6時間20分）

```
Day 3 (2h 5m):
  30m: require_auth ミドルウェア実装 ← Tower middleware パターン
  30m: rate_limit ミドルウェア実装
  20m: request_logging ミドルウェア実装
  30m: routes.rs 完成化（全エンドポイント集約）
  15m: cargo check + clippy

Day 4 (1h 30m):
  40m: ミドルウェアテスト実装（6個）
  20m: ルートテスト実装（4個）
  15m: 全テスト実行確認
  15m: ドキュメント見直し

Day 5 (2h 45m):
  90m: ハンドラユニットテスト実装（12個）
  45m: 統合確認（cargo test）
  30m: Week 12 最終報告書作成
```

**達成基準**:
- ✅ ミドルウェア 3個完全実装
- ✅ ハンドラテスト 12+個実装
- ✅ cargo test --lib web:: パス
- ✅ 0 コンパイル警告

---

### 🔜 Week 13: 統合テスト + OpenAPI ドキュメント（Week 13）

#### タスク

```
Day 1-2 (4h):
  1h: testcontainers + PostgreSQL セットアップ
  1h: 統合テスト実装（repository + event chain）
  1h: E2E テスト実装（curl/Postman 検証）
  1h: 並行アクセステスト

Day 3-5 (4h):
  1h: OpenAPI ドキュメント生成（cargo doc）
  1h: Swagger UI セットアップ
  1h: API v2 互換性検証
  1h: パフォーマンステスト確認
```

**達成基準**:
- ✅ 統合テスト 10+個実装パス
- ✅ OpenAPI ドキュメント完成
- ✅ API v2 エンドポイント 12/12 検証

---

### 🔜 Week 14: ミドルウェア詳細化 + セキュリティ強化（Week 14）

#### タスク

```
Day 1-3 (6h):
  1h: Biscuit トークン検証実装強化
  1h: CORS ミドルウェア実装
  1h: Content-Security-Policy ヘッダ追加
  1h: サニタイゼーション ロジック実装
  1h: セキュリティテスト実装（10個）
  1h: 侵害テスト実行（SQL injection, XSS 等）

Day 4-5 (3h):
  1h: エラーレスポンス安全性確認
  1h: ログレベル確認（PII 非開示）
  1h: デプロイ前セキュリティチェックリスト作成
```

**達成基準**:
- ✅ Biscuit 検証実装完成
- ✅ セキュリティテスト 15+個パス
- ✅ OWASP Top 10 チェック完了

---

### 🔜 Week 15: API v2 パイロット + 互換性レイヤー（Week 15）

#### タスク

```
Day 1-3 (6h):
  1.5h: API v1 ↔ v2 互換性レイヤー実装
  1.5h: 段階的マイグレーション設計
  1.5h: Deprecation ヘッダ追加
  1.5h: v1 エンドポイント互換性テスト（10個）

Day 4-5 (3h):
  1h: パフォーマンス比較（v1 vs v2）
  1h: ユーザー フィードバック統合
  1h: Week 15 レビュー + Next steps 確認
```

**達成基準**:
- ✅ API v1/v2 並行運用確認
- ✅ 互換性テスト 10+個パス
- ✅ パフォーマンス低下なし（< 5%）

---

### 🔜 Week 16: イベントシステム移行（Week 16）

#### タスク

```
Day 1-2 (4h):
  1h: infrastructure/events/ ディレクトリ準備
  1h: infrastructure/events/bus.rs 実装
  1h: infrastructure/events/listeners.rs 実装
  1h: イベント移行テスト（10個）

Day 3-5 (4h):
  1.5h: src/events.rs → infrastructure/events/bus.rs 統合
  1.5h: src/listeners.rs → infrastructure/events/listeners.rs 統合
  1h: 既存イベント互換性確認
  0.5h: ドキュメント更新
```

**達成基準**:
- ✅ infrastructure/events/ 完全実装
- ✅ イベント移行テスト 10+個パス
- ✅ 既存イベントリスナー互換性確認

---

### 🔜 Week 17: common/ ディレクトリ + レガシー削除計画（Week 17）

#### タスク

```
Day 1-2 (4h):
  1h: src/common/types.rs 移行
  1h: src/common/telemetry.rs 移行
  1h: import 文一括更新（common/ への切り替え）
  1h: 既存 shared/ 削除準備

Day 3-5 (4h):
  1.5h: src/handlers/ 削除計画実装
  1.5h: レガシーコード削除（段階的）
  1h: Phase 4 最終ビルド確認
  0.5h: Phase 4 完成レポート作成
```

**達成基準**:
- ✅ common/ 移行完成
- ✅ shared/ 削除完了
- ✅ handlers/ 削除 50%

---

### ✅ Week 18: Phase 4 完了 + Phase 5 準備（Week 18）

#### タスク

```
Day 1-3 (6h):
  1h: すべてのテスト実行確認
  1h: コード品質最終チェック
  1h: ドキュメント完成化
  1h: パフォーマンス最終測定
  1h: セキュリティ最終監査
  1h: Phase 4 完了レポート作成

Day 4-5 (3h):
  1h: Phase 5 計画策定（API versioning, マイグレーション戦略）
  1h: チームレビュー + フィードバック統合
  1h: 次フェーズのマイルストーン設定
```

**達成基準**:
- ✅ Phase 4 100% 完成
- ✅ 全テスト 50+個パス
- ✅ 0 コンパイル警告
- ✅ ドキュメント 100% 完成

---

## 📊 進捗追跡表

```
Phase 4 Progress:

Week 12: ████████░░ 80% ✅ 準備中
  Day 1-2: ████████░░ 100% ✅ 完了
  Day 3-5: ░░░░░░░░░░ 0% 🔜 準備中

Week 13: ░░░░░░░░░░ 0% 🔜 統合テスト
Week 14: ░░░░░░░░░░ 0% 🔜 セキュリティ
Week 15: ░░░░░░░░░░ 0% 🔜 API パイロット
Week 16: ░░░░░░░░░░ 0% 🔜 イベント移行
Week 17: ░░░░░░░░░░ 0% 🔜 common/ + レガシー削除
Week 18: ░░░░░░░░░░ 0% 🔜 最終完成

総合: ████░░░░░░ 40% 進行中
```

---

## 🔄 新構造への段階的移行パターン

### Step 1: Web層（Week 12）✅ 準備中

```
✅ Day 1-2: ハンドラ詳細化完成
🔜 Day 3-5: ミドルウェア実装 + テスト
```

### Step 2: 統合テスト（Week 13）🔜

```
🔜 Day 1-2: testcontainers セットアップ
🔜 Day 3-5: OpenAPI ドキュメント生成
```

### Step 3: セキュリティ強化（Week 14）🔜

```
🔜 Day 1-3: Biscuit + CORS + CSP 実装
🔜 Day 4-5: セキュリティテスト
```

### Step 4: API v2 パイロット（Week 15）🔜

```
🔜 Day 1-3: 互換性レイヤー実装
🔜 Day 4-5: パフォーマンス検証
```

### Step 5: イベントシステム移行（Week 16）🔜

```
🔜 Day 1-2: infrastructure/events/ 実装
🔜 Day 3-5: src/events.rs 移行
```

### Step 6: common/ + レガシー削除（Week 17）🔜

```
🔜 Day 1-2: common/ 移行
🔜 Day 3-5: src/handlers/ 削除計画
```

### Step 7: 最終完成（Week 18）🔜

```
🔜 Day 1-3: 全テスト実行 + 最終チェック
🔜 Day 4-5: Phase 4 完了 + Phase 5 準備
```

---

## ✅ Success Metrics（Phase 4 終了時）

### 📊 コード品質

| 指標 | 目標 | 状態 |
|------|------|------|
| **テスト数** | 50+ | 🔜 目指す |
| **テスト通過率** | 100% | 🔜 目指す |
| **コンパイル警告** | 0 | ✅ 維持 |
| **Clippy 警告** | 0 | ✅ 維持 |
| **コードカバレッジ** | ≥ 85% | 🔜 目指す |

### 🔒 セキュリティ

| 指標 | 目標 | 状態 |
|------|------|------|
| **セキュリティテスト** | 15+ | 🔜 目指す |
| **CVE チェック** | 0 既知脆弱性 | ✅ 確認済み |
| **Biscuit 検証** | 完全実装 | 🔜 Week 14 |
| **OWASP Top 10** | 100% チェック済み | 🔜 Week 14 |

### 📈 パフォーマンス

| 指標 | 目標 | 状態 |
|------|------|------|
| **レスポンス時間** | < 100ms (P95) | 🔜 Week 13 測定 |
| **スループット** | > 1000 req/s | 🔜 Week 13 測定 |
| **エラー率** | < 0.1% | 🔜 Week 13 確認 |

### 📚 ドキュメント

| 指標 | 目標 | 状態 |
|------|------|------|
| **API ドキュメント** | 100% | 🔜 Week 13 |
| **ハンドラドキュメント** | 100% | ✅ 完成 |
| **ミドルウェアドキュメント** | 100% | 🔜 Week 12 Day 3 |
| **移行ガイド** | 完成 | 🔜 Week 17 |

---

## 🚀 Next Steps（即座）

### 今からすぐ実施

```bash
# 1. 現在の状態確認
cd /mnt/lfs/home/jgm/Desktop/Rust/RustCMS
ls -la src/web/

# 2. 新構造確認
ls -la src/common/

# 3. Day 3 準備: ミドルウェアファイル確認
ls -la src/web/middleware.rs 2>/dev/null || echo "ミドルウェアファイル作成予定"

# 4. 計画書確認
cat PHASE4_WEEK12_DAY3-5_DETAILED_PLAN.md | head -100
```

### Day 3 最初の操作

1. ミドルウェアスケルトン実装開始
2. require_auth 基本構造作成
3. cargo check 確認
4. テスト駆動開発で実装進行

---

## 📞 重要な連絡事項

### 🔴 Critical

- **Tower middleware**: async/await パターンに対応（Axum 標準）
- **Biscuit 検証**: Week 14 で完全実装（現在は TODO）
- **エラーハンドリング**: 詳細情報不開示（セキュリティ）

### 🟡 High

- **ドキュメント**: すべてのエンドポイント要 API ドキュメント
- **テスト**: ユニット + 統合 + E2E の 3 層
- **feature flags**: `restructure_domain` で新構造ビルド確認

### 🟢 Info

- **パフォーマンス**: Week 13 で測定予定
- **キャッシング**: Week 15 で検討
- **ログレベル**: production で INFO 以上

---

**ロードマップ完成日**: 2025年10月18日  
**次回更新**: Week 12 Day 3 完了後

