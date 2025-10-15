# RustCMS 構造再編 - エグゼクティブサマリー

## 📌 概要

RustCMSをよりRustらしい、安全で、保守しやすい構造に再編するための計画です。

## 🎯 主な目的

| 目的 | 説明 | 期待効果 |
|------|------|----------|
| **型安全性の向上** | NewTypeパターンと検証済み値オブジェクトの導入 | コンパイル時エラー検出、バグの事前防止 |
| **ドメイン駆動設計** | ビジネスロジックのドメイン層への集約 | 変更影響範囲の最小化、整合性の維持 |
| **レイヤード分離** | プレゼンテーション、アプリケーション、ドメイン、インフラの明確化 | 関心の分離、テスタビリティの向上 |
| **依存性の逆転** | Port/Adapter パターンの適用 | モックの容易化、技術スタック変更の柔軟性 |

## 📊 現状と課題

### 現在の構造

```text
src/
├── handlers/      # HTTPハンドラ + ビジネスロジック混在
├── repositories/  # データアクセス層
├── models/        # ドメインモデル（貧血）
├── auth/          # 認証機能
├── cache/         # キャッシュ機能
├── search/        # 検索機能
└── utils/         # 28個のユーティリティモジュール
```

### 主な課題

1. **ビジネスロジックの分散** → handlers, repositories, modelsに散在
2. **肥大化したファイル** → app.rs (2080行)
3. **型安全性の不足** → 文字列ベースの識別子
4. **密結合** → ハンドラがDB実装詳細に直接依存

## 🏗️ 提案する新構造

```text
src/
├── domain/               # ドメイン層（ビジネスロジック）
│   ├── entities/        # エンティティ
│   ├── value_objects/   # 値オブジェクト（検証済み）
│   ├── services/        # ドメインサービス
│   └── events/          # ドメインイベント
│
├── application/          # アプリケーション層（ユースケース）
│   ├── use_cases/       # ユースケース実装
│   ├── commands/        # コマンド（書き込み）
│   ├── queries/         # クエリ（読み取り）
│   ├── ports/           # インターフェース定義
│   └── dto/             # Data Transfer Objects
│
├── infrastructure/       # インフラストラクチャ層
│   ├── database/        # DB実装（Diesel）
│   ├── cache/           # キャッシュ実装（Redis）
│   ├── search/          # 検索実装（Tantivy）
│   ├── auth/            # 認証実装（biscuit-auth）
│   └── events/          # イベントバス実装
│
├── presentation/         # プレゼンテーション層
│   └── http/            # Web API
│       ├── handlers/    # HTTPハンドラ（薄い層）
│       ├── middleware/  # ミドルウェア
│       └── responses/   # レスポンス型
│
└── shared/              # 共有ユーティリティ
    ├── types/           # 共通型
    ├── telemetry/       # 監視・ロギング
    └── utils/           # 純粋関数
```

## 🎨 主要パターン

### 1. NewType パターン

**Before:**

```rust
fn get_user(id: Uuid) -> Result<User>
```

**After:**

```rust
fn get_user(id: UserId) -> Result<User>
```

✅ 型レベルでの安全性保証

### 2. 検証済み値オブジェクト

**Before:**

```rust
// バリデーションがハンドラ層に散在
if email.is_empty() { return Err(...) }
if !email.contains('@') { return Err(...) }
```

**After:**

```rust
// 型作成時に自動検証
let email = Email::new(raw_email)?; // 検証完了
// 以降は検証済みとして扱える
```

✅ 不正データの混入を防止

### 3. Repository パターン

**Before:**

```rust
// ハンドラがDB詳細に依存
state.database.pool.get().await?
```

**After:**

```rust
// インターフェースを通じてアクセス
user_repo.find_by_id(user_id).await?
```

✅ テストでのモック化が容易

### 4. CQRS パターン

```rust
// 書き込み（コマンド）
CreatePostCommand → CreatePostHandler

// 読み取り（クエリ）
ListPostsQuery → ListPostsHandler
```

✅ 読み書きの最適化を独立して実行

## 📅 移行スケジュール

| フェーズ | 期間 | 主な作業 | 成果物 |
|---------|------|---------|--------|
| **Phase 1** | 1-2週間 | 基礎固め | 新ディレクトリ構造、値オブジェクト、Port定義 |
| **Phase 2** | 2-3週間 | ドメイン層構築 | エンティティ、ドメインサービス、ドメインイベント |
| **Phase 3** | 2-3週間 | アプリケーション層構築 | DTO、Use Case、リポジトリ実装 |
| **Phase 4** | 1-2週間 | プレゼンテーション層 | ハンドラ簡素化、ミドルウェア整理 |
| **Phase 5** | 1週間 | クリーンアップ | 旧コード削除、ドキュメント更新 |

**合計期間**: 7-11週間

## ✨ 期待される効果

### 1. 開発生産性の向上

- **変更の影響範囲が明確** → 修正時間 -30%
- **新機能追加が容易** → 開発速度 +40%
- **バグの早期発見** → デバッグ時間 -50%

### 2. コード品質の向上

- **型安全性** → ランタイムエラー -70%
- **テストカバレッジ** → 80% → 95%
- **Clippy警告** → 0件維持

### 3. 保守性の向上

- **理解しやすい構造** → オンボーディング時間 -40%
- **明確な責任分離** → コードレビュー時間 -30%
- **技術スタック変更の柔軟性** → 依存ライブラリ変更時の影響範囲 -60%

## 🚨 リスクと対策

| リスク | 影響 | 対策 |
|--------|------|------|
| 移行期間中の開発停滞 | 高 | 機能追加を一時凍結、各フェーズで動作確認 |
| パフォーマンスの劣化 | 中 | 各フェーズでベンチマーク実行 |
| テストカバレッジの低下 | 中 | 移行前のカバレッジを基準に維持 |

## 📝 アクションプラン

### 即座に実行

1. ✅ **計画の共有とレビュー** → チーム全体での合意形成
2. ✅ **Phase 1の着手** → 新ディレクトリ構造の作成

### 1週間以内

1. 📋 **マイルストーン設定** → 各フェーズの具体的なタスク分割
2. 📋 **ブランチ戦略の決定** → feature/restructure-phase-X

### 2週間以内

1. 🔄 **Phase 1の完了** → 新旧構造での並行ビルド確認
2. 📊 **進捗ダッシュボード** → GitHub Projects でタスク管理

## 📚 関連ドキュメント

- **詳細計画**: [`RESTRUCTURE_PLAN.md`](./RESTRUCTURE_PLAN.md)
  - 完全な設計思想と段階的移行計画

- **実装例**: [`RESTRUCTURE_EXAMPLES.md`](./RESTRUCTURE_EXAMPLES.md)
  - 各パターンの具体的なコード例

- **現在のアーキテクチャ**: [`ARCHITECTURE.md`](./ARCHITECTURE.md)
  - 現行システムの構造とイベント駆動設計

## 🎓 学習リソース

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Domain-Driven Design](https://martinfowler.com/bliki/DomainDrivenDesign.html)
- [Hexagonal Architecture](https://alistair.cockburn.us/hexagonal-architecture/)
- [Zero To Production In Rust](https://www.zero2prod.com/)

## ✅ 成功基準

移行完了の条件:

- [ ] 全テストがパスする（4000+テスト）
- [ ] テストカバレッジ ≥ 移行前のカバレッジ
- [ ] ベンチマークテストで性能劣化なし（±5%以内）
- [ ] Clippy警告 = 0
- [ ] ドキュメントが完全に更新されている
- [ ] 既存APIの互換性が保たれている

## 🚀 開始宣言

この再編計画は、RustCMSを次のレベルに引き上げるための重要なステップです。段階的なアプローチにより、リスクを最小化しながら、モダンなRustのベストプラクティスを適用します。

**準備はできています。Phase 1を開始しましょう！**

---

**作成日**: 2025年10月16日
**バージョン**: 1.0
**ステータス**: 📋 計画中
**次回レビュー**: Phase 1完了後
