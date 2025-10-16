# Phase 5-4 詳細実装スケジュール

**開始日**: 2025-01-24
**終了日**: 2025-02-06 (2 週間)
**目標**: API v1 Deprecation 完全実装
**成功指標**: v1 アクセス < 10%, 全 Deprecation ヘッダー 100%

---

## 📅 詳細スケジュール

### Week 1 (2025-01-24 ~ 2025-01-30): ヘッダー実装 & テスト

#### Day 1-2 (金-土): 設計 & ミドルウェア実装

**タスク**:

- [ ] Deprecation ヘッダーミドルウェア実装
- [ ] RFC 8594 準拠の確認
- [ ] 単体テスト作成

**成果物**:

```bash
src/middleware/deprecation.rs  (150-200 行)
tests/deprecation_headers_test.rs  (50+ テスト)
```

**コード例**:

```rust
pub async fn deprecation_middleware<B>(
    req: Request<B>,
    next: Next,
) -> impl IntoResponse {
    let path = req.uri().path().to_string();
    let mut response = next.run(req).await;

    if path.contains("/api/v1/") {
        // Deprecation: true
        // Sunset: Sun, 17 Mar 2025 00:00:00 GMT
        // Link: rel="successor-version"
        // Warning: 299 - "Deprecation..."
    }

    response
}
```

#### Day 3-4 (日-月): ルーティング統合 & テスト

**タスク**:

- [ ] `src/main.rs` に middleware 追加
- [ ] 全エンドポイントでテスト
- [ ] ホストヘッダー検証

**テスト例**:

```rust
#[tokio::test]
async fn test_v1_users_get_has_deprecation() {
    let response = client
        .get("http://localhost:3000/api/v1/users/123")
        .send()
        .await;

    assert_eq!(
        response.headers().get("Deprecation"),
        Some("true")
    );
}
```

#### Day 5 (火): ドキュメント & PR 作成

**タスク**:

- [ ] PHASE_5_4_IMPLEMENTATION_GUIDE.md 更新
- [ ] 実装例追加
- [ ] PR 作成

**PR チェックリスト**:

- [ ] CI/CD パス
- [ ] テスト 100% パス
- [ ] Codacy スコア良好
- [ ] コードレビュー 2 人承認

---

### Week 2 (2025-01-31 ~ 2025-02-06): 監視 & クライアント通知

#### Day 6-7 (水-木): v1 アクセス監視

**タスク**:

- [ ] ログ監視ダッシュボード設定
- [ ] Grafana メトリクス作成
- [ ] v1 vs v2 アクセス比率追跡

**メトリクス例**:

```prometheus
api_v1_requests_total{endpoint="/api/v1/users"}
api_v2_requests_total{endpoint="/api/v2/users"}
deprecation_header_hit_rate
```

#### Day 8-9 (金-土): クライアント通知

**タスク**:

- [ ] クライアント移行ガイド公開
- [ ] 既存クライアントに通知メール送信
- [ ] Slack チャネルで案内

**通知テンプレート**:

```markdown
# API v1 削除予定のお知らせ

API v1 は **2025-03-17** に削除されます。

## 影響を受けるエンドポイント
- /api/v1/users (及び 49 他)

## 移行手順
1. ベース URL を `/api/v1` から `/api/v2` に変更
2. エラーレスポンス形式を更新 (`error` → `errors`)
3. ページネーション を更新 (`page` → `offset`)

## サポート
- 移行ガイド: https://docs.example.com/migration
- 質問: support@example.com
```

#### Day 10 (日): 監視 & 報告

**タスク**:

- [ ] v1 アクセス率レポート
- [ ] クライアント移行状況確認
- [ ] 問題の早期対応

**報告内容**:

```
v1 アクセス率: 45% (初日)
v2 アクセス率: 55%

移行済みクライアント: 12/20 (60%)

問題:
- 3 クライアントが v1 に固定
  → 直接連絡予定
```

---

## 🎯 実装チェックリスト

### Phase 5-4a: Deprecation ヘッダー (完全)

#### Infrastructure Layer

- [ ] `src/middleware/deprecation.rs` 作成
  - [x] ヘッダー追加ロジック
  - [x] RFC 8594 準拠
  - [x] パス置換ロジック
  - [ ] エラーハンドリング

#### Test Layer

- [ ] `tests/deprecation_headers_test.rs` (50+ テスト)
  - [x] ユーザーエンドポイント (8)
  - [x] ポストエンドポイント (10)
  - [x] コメントエンドポイント (8)
  - [x] タグエンドポイント (6)
  - [x] カテゴリエンドポイント (6)
  - [x] サーチエンドポイント (4)
  - [x] 分析エンドポイント (4)
  - [x] 認証エンドポイント (2)
  - [x] 管理エンドポイント (2)
  - [x] ヘッダー形式検証

#### Documentation

- [ ] クライアント移行ガイド
  - [x] 主要変更点説明
  - [x] 言語別ガイド (JS/Python/Go/Rust)
  - [x] テスト例
  - [x] トラブルシューティング

#### Monitoring

- [ ] Grafana ダッシュボード
- [ ] ログ追跡スクリプト
- [ ] アラート設定

---

## 📊 成功指標

### Week 1 終了時

| 指標 | 目標 | 達成度 |
|------|------|--------|
| ヘッダー実装完了 | 100% | 0% |
| テスト成功率 | 100% | 0% |
| CI/CD パス | 100% | 0% |
| ドキュメント完成 | 100% | 0% |

### Week 2 終了時

| 指標 | 目標 | 達成度 |
|------|------|--------|
| v1 アクセス < 50% | Yes | 待機中 |
| クライアント通知 | 100% | 待機中 |
| クライアント移行 > 60% | Yes | 待機中 |
| サポートチケット解決 | 100% | 待機中 |

---

## 🔧 詳細なタスク分解

### Task 1: ミドルウェア実装 (8 時間)

**依存関係**: なし
**リスク**: ミディアム (RFC 8594 準拠の複雑さ)

**ステップ**:

1. ヘッダー定数定義 (30 分)
2. ミドルウェア構造実装 (2 時間)
3. ヘッダー追加ロジック (1.5 時間)
4. パス置換ロジック (1 時間)
5. テスト作成 (3 時間)

**検証方法**:

```bash
curl -v http://localhost:3000/api/v1/users/123 \
  | grep -E "Deprecation|Sunset|Link|Warning"
```

### Task 2: ルーティング統合 (4 時間)

**依存関係**: Task 1 完了

**ステップ**:

1. `src/main.rs` 確認 (30 分)
2. middleware 登録 (1 時間)
3. Feature flag 設定 (1 時間)
4. エンドツーエンドテスト (1.5 時間)

### Task 3: テスト作成 (16 時間)

**依存関係**: Task 2 完了

**ステップ**:

1. テスト構造設計 (1 時間)
2. ユーザーエンドポイント (2 時間)
3. ポストエンドポイント (2 時間)
4. コメント・タグ・カテゴリ (3 時間)
5. 検索・分析・認証 (2 時間)
6. ヘッダー形式検証 (2 時間)
7. 統合テスト (2 時間)

### Task 4: ドキュメント (8 時間)

**依存関係**: Task 1-3 完了

**ステップ**:

1. API ドキュメント更新 (2 時間)
2. クライアント移行ガイド (4 時間)
3. トラブルシューティング (2 時間)

---

## 💻 実装コード例

### 1. ミドルウェア登録

```rust
// src/main.rs
use crate::middleware::deprecation::deprecation_middleware;

#[tokio::main]
async fn main() {
    let app = Router::new()
        // Routes
        .route("/api/v1/users", get(list_users_v1))
        .route("/api/v2/users", get(list_users_v2))
        // Middleware (v1 に対してのみ適用)
        .layer(axum::middleware::from_fn(|req, next| {
            deprecation_middleware(req, next)
        }));

    // ...
}
```

### 2. メトリクス収集

```rust
// src/handlers/metrics.rs
use prometheus::{Counter, Registry};

pub struct DeprecationMetrics {
    v1_requests: Counter,
    v2_requests: Counter,
    deprecation_header_hits: Counter,
}

impl DeprecationMetrics {
    pub fn record_v1_request(&self) {
        self.v1_requests.inc();
    }

    pub fn record_v2_request(&self) {
        self.v2_requests.inc();
    }
}
```

### 3. クライアント通知ルック

```python
# scripts/notify_clients.py
import smtplib
from datetime import datetime

clients = [
    "client1@example.com",
    "client2@example.com",
    # ...
]

message = """
API v1 削除予定のお知らせ

削除予定日: 2025-03-17

対象: /api/v1/users 他 50 エンドポイント

移行手順: https://docs.example.com/migration
"""

for email in clients:
    # メール送信
    pass
```

---

## 🚀 ロールアウト戦略

### 段階的ロールアウト

**Phase 5-4a**: Deprecation ヘッダー追加 (2025-01-24)

- v1 & v2 両方で動作
- クライアント影響なし
- 移行期間開始

**Phase 5-4b**: ログ監視 (2025-01-31)

- v1 vs v2 アクセス比率追跡
- クライアント通知送信

**Phase 5-4c**: 強制移行準備 (2025-02-21)

- v1 アクセス < 5% なら Phase 5-5 へ
- v1 アクセス > 5% なら期間延長

**Phase 5-5**: v1 削除 (2025-03-17)

- v1 エンドポイント完全削除
- v2 のみ提供

---

## 📞 コミュニケーション計画

### Week 1: 静かな準備

- 内部テストのみ
- 開発チームに通知

### Week 2: 公開準備

- ドキュメント公開
- ブログ投稿
- メール通知

### Week 3-4: 監視期間

- 日次レポート
- クライアント向けサポート
- 問題解決

---

## 🎯 リスク管理

### リスク 1: クライアント側で更新が遅れる

**確率**: ミディアム
**インパクト**: ハイ
**対策**:

- 迅速なクライアント通知
- 電話/Slack での直接連絡
- 期間延長検討

### リスク 2: ヘッダー形式エラー

**確率**: ロー
**インパクト**: ミディアム
**対策**:

- RFC 8594 厳密準拠テスト
- Chrome/Firefox でヘッダー検証
- Curl デバッグ

### リスク 3: パフォーマンス低下

**確率**: ロー
**インパクト**: ハイ
**対策**:

- ベンチマーク前後比較
- 負荷テスト
- ローリングデプロイ

---

## ✅ 完了チェックリスト

### Phase 5-4 完全完了

- [ ] Deprecation ヘッダー実装 (50+ テスト)
- [ ] クライアント移行ガイド公開
- [ ] v1 アクセス < 50%
- [ ] ドキュメント完成
- [ ] 監視ダッシュボード稼働
- [ ] サポート体制整備
- [ ] 全テスト 100% パス
- [ ] PR マージ & 本番デプロイ

**実装予定終了**: 2025-02-06
**本番反映予定**: 2025-02-07

---

**作成日**: 2025-01-17
**最終更新**: 2025-01-17
**ステータス**: 計画中 → 実装準備中
