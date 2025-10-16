# RustCMS 構造再編 - マイグレーションチェックリスト

## 📋 Phase 1: 基礎固め (2-3週間)

### Week 1: ディレクトリ構造とインフラ

#### タスク

- [ ] **ディレクトリ構造作成**
  ```bash
  mkdir -p src/{domain,application,infrastructure,presentation}
  mkdir -p src/domain/{entities,value_objects,services,events}
  mkdir -p src/application/{use_cases,commands,queries,ports,dto}
  mkdir -p src/infrastructure/{database,cache,search,auth,events}
  mkdir -p src/presentation/http/{handlers,middleware,responses}
  ```

- [ ] **CI/CD の並行ビルド設定**
  - [ ] GitHub Actions で新旧構造を並行ビルド
  - [ ] テストジョブの分離（legacy/restructure）
  - [ ] ベンチマークジョブの追加

- [ ] **Value Objects 実装 (5個)**
  - [ ] `UserId` - ユーザー識別子
  - [ ] `PostId` - 投稿識別子  
  - [ ] `Email` - 検証済みメールアドレス
  - [ ] `Username` - ユーザー名（3-20文字）
  - [ ] `Slug` - URL用スラッグ

- [ ] **エラー型階層**
  - [ ] `DomainError` - ドメイン層エラー
  - [ ] `ApplicationError` - アプリケーション層エラー
  - [ ] `InfrastructureError` - インフラ層エラー

#### 検証基準

- [ ] すべての Value Objects がユニットテストでカバーされている
- [ ] 新構造と旧構造が並行してビルド可能
- [ ] CI が Green

#### 完了条件

```bash
# すべてのテストがパス
cargo test --all-features

# 新構造でビルド可能
cargo build --features restructure_domain

# Clippy 警告なし
cargo clippy --all-features -- -D warnings
```

---

### Week 2: Port 定義とベンチマーク

#### タスク

- [ ] **Repository Port 定義**
  - [ ] `UserRepository` trait
  - [ ] `PostRepository` trait
  - [ ] `CommentRepository` trait
  - [ ] `TagRepository` trait

- [ ] **Service Port 定義**
  - [ ] `CacheService` trait
  - [ ] `SearchService` trait
  - [ ] `EventPublisher` trait
  - [ ] `PasswordHasher` trait

- [ ] **Unit of Work 設計**
  - [ ] Port 定義 (`UnitOfWork`, `Transaction`)
  - [ ] Diesel 実装の設計レビュー
  - [ ] テスト戦略の策定

- [ ] **ベンチマーク基準測定**
  - [ ] 主要エンドポイント10個のレスポンスタイム測定
  - [ ] メモリ使用量の記録
  - [ ] データベースクエリ数の記録
  - [ ] ベンチマーク結果を `benches/baseline.json` に保存

#### 検証基準

- [ ] すべての Port が `Send + Sync` を実装
- [ ] ドキュメントコメントが充実している
- [ ] 各 Port に最低1つのモック実装がある
- [ ] ベンチマーク基準が文書化されている

#### 完了条件

```bash
# Port のビルド確認
cargo check --features restructure_application

# ベンチマーク実行
cargo bench --bench baseline -- --save-baseline before

# 結果の確認
cat benches/baseline.json
```

---

### Week 3: Phase 1 完了確認

#### タスク

- [ ] **ドキュメント更新**
  - [ ] `ARCHITECTURE.md` に新構造を追記
  - [ ] `CONTRIBUTING.md` に Value Objects 追加ガイド
  - [ ] API ドキュメントの生成 (`cargo doc`)

- [ ] **コードレビュー**
  - [ ] Value Objects のレビュー
  - [ ] Port 定義のレビュー
  - [ ] エラーハンドリングのレビュー

- [ ] **Phase 1 レトロスペクティブ**
  - [ ] 良かった点の記録
  - [ ] 改善点の記録
  - [ ] Phase 2 への教訓

#### 検証基準

- [ ] 全チームメンバーがレビュー完了
- [ ] ドキュメントが最新
- [ ] 未解決の Issue がない

#### 完了条件

```markdown
## Phase 1 完了報告

### 成果物
- Value Objects: 5個実装
- Port 定義: 8個完成
- ベンチマーク基準: 測定完了

### 次フェーズへの準備
- [ ] Phase 2 のブランチ作成
- [ ] マイルストーン設定
- [ ] タスク分割完了
```

---

## 📋 Phase 2: ドメイン層構築 (3-4週間)

### Week 4: エンティティ実装

#### タスク

- [ ] **User Entity**
  - [ ] ファクトリメソッド (`create`, `reconstruct`)
  - [ ] ビジネスメソッド (`suspend`, `activate`, `change_role`)
  - [ ] ドメインイベント発行 (`UserCreated`, `UserSuspended`)
  - [ ] ユニットテスト (100% カバレッジ)

- [ ] **Post Entity**
  - [ ] ファクトリメソッド
  - [ ] 公開ステータス管理 (`publish`, `unpublish`, `archive`)
  - [ ] タグ管理 (`add_tag`, `remove_tag`)
  - [ ] ユニットテスト

- [ ] **Comment Entity**
  - [ ] ファクトリメソッド
  - [ ] モデレーション機能 (`approve`, `reject`, `flag`)
  - [ ] ユニットテスト

#### 検証基準

- [ ] すべてのエンティティが不変条件を保証
- [ ] ビジネスロジックがドメイン層に集約されている
- [ ] ユニットテストカバレッジ ≥ 95%

#### 完了条件

```bash
# テストカバレッジ確認
cargo tarpaulin --out Html --output-dir coverage/

# カバレッジが95%以上
grep -A 1 "domain/entities" coverage/index.html | grep "95\|96\|97\|98\|99\|100"
```

---

### Week 5-6: ドメインサービスとイベント

#### タスク

- [ ] **ドメインサービス実装**
  - [ ] `UserRegistrationService` - 重複チェック + 作成
  - [ ] `PostPublicationService` - 公開ルール検証
  - [ ] `CommentModerationService` - スパム検出

- [ ] **ドメインイベント定義**
  - [ ] `UserCreated`, `UserUpdated`, `UserDeleted`
  - [ ] `PostPublished`, `PostUnpublished`, `PostDeleted`
  - [ ] `CommentAdded`, `CommentModerated`

- [ ] **イベント統合**
  - [ ] 既存の `src/events.rs` との統合方針決定
  - [ ] `DomainEvent` trait 定義
  - [ ] イベントリスナーの移行計画策定

#### 検証基準

- [ ] ドメインサービスがステートレス
- [ ] すべてのドメインイベントが発行されている
- [ ] 既存リスナーとの互換性が保たれている

#### 完了条件

```bash
# イベント発行の検証
cargo test --test domain_events -- --nocapture

# 既存リスナーとの統合テスト
cargo test --test event_integration
```

---

### Week 7: Phase 2 完了確認

#### タスク

- [ ] **統合テスト作成**
  - [ ] エンティティ + ドメインサービスの統合
  - [ ] イベント発行 → リスナー処理の E2E
  - [ ] トランザクション境界のテスト

- [ ] **パフォーマンステスト**
  - [ ] ベンチマーク再実行 (`cargo bench`)
  - [ ] Phase 1 との比較
  - [ ] 劣化が ±5% 以内であることを確認

- [ ] **Phase 2 レトロスペクティブ**
  - [ ] ドメイン駆動設計の適用度評価
  - [ ] 改善点の記録

#### 検証基準

- [ ] 統合テストがすべてパス
- [ ] パフォーマンス劣化なし
- [ ] ドキュメントが更新されている

---

## 📋 Phase 3: アプリケーション層構築 (3-4週間)

### Week 8-9: DTO と Use Case

#### タスク

- [ ] **DTO 実装**
  - [ ] `UserDto`, `CreateUserRequest`, `UpdateUserRequest`
  - [ ] `PostDto`, `CreatePostRequest`, `UpdatePostRequest`
  - [ ] `CommentDto`, `CreateCommentRequest`

- [ ] **Use Case 実装 (User)**
  - [ ] `RegisterUserUseCase`
  - [ ] `GetUserByIdUseCase`
  - [ ] `UpdateUserUseCase`
  - [ ] `SuspendUserUseCase`

- [ ] **Use Case 実装 (Post)**
  - [ ] `CreatePostUseCase`
  - [ ] `PublishPostUseCase`
  - [ ] `UpdatePostUseCase`
  - [ ] `ListPostsUseCase`

#### 検証基準

- [ ] すべての Use Case がトランザクション境界を明示
- [ ] Use Case がドメインロジックを呼び出している
- [ ] モックを使用した単体テストがある

---

### Week 10-11: Repository 実装と CQRS

#### タスク

- [ ] **Repository 実装**
  - [ ] `DieselUserRepository` - UserRepository の実装
  - [ ] `DieselPostRepository` - PostRepository の実装
  - [ ] `DieselCommentRepository` - CommentRepository の実装

- [ ] **CQRS 実装**
  - [ ] `ListUsersQuery` - 読み取り専用クエリ
  - [ ] `ListPostsQuery` - フィルタ/ソート付き
  - [ ] `SearchPostsQuery` - 全文検索統合

- [ ] **Unit of Work 実装**
  - [ ] `DieselUnitOfWork`
  - [ ] トランザクション管理
  - [ ] セーブポイント実装

#### 検証基準

- [ ] Repository がすべての Port メソッドを実装
- [ ] トランザクション境界が正しく機能
- [ ] CQRS で読み書きが分離されている

---

## 📋 Phase 4: プレゼンテーション層 (2-3週間)

### Week 12-13: ハンドラ簡素化

#### タスク

- [ ] **新ハンドラ実装**
  - [ ] `register_user` - Use Case 呼び出しのみ
  - [ ] `create_post` - Use Case 呼び出しのみ
  - [ ] エラーハンドリングの統一

- [ ] **API バージョニング**
  - [ ] `/api/v2/users` - 新構造
  - [ ] `/api/v1/users` - 旧構造（非推奨）
  - [ ] バージョン別のルーティング

- [ ] **ミドルウェア整理**
  - [ ] 認証ミドルウェアの移行
  - [ ] レート制限の移行
  - [ ] ロギングの移行

#### 検証基準

- [ ] 新旧 API が並行動作
- [ ] エンドポイントのレスポンスタイムが維持されている
- [ ] すべてのエンドポイントに E2E テストがある

---

### Week 14: Phase 4 完了確認

#### タスク

- [ ] **API ドキュメント更新**
  - [ ] OpenAPI スキーマ生成
  - [ ] Postman コレクション更新
  - [ ] 移行ガイド作成 (`MIGRATION_GUIDE.md`)

- [ ] **E2E テスト**
  - [ ] 主要ユースケースの E2E
  - [ ] エラーケースのテスト
  - [ ] パフォーマンステスト

---

## 📋 Phase 5: クリーンアップ (2週間)

### Week 15: 旧コード削除

#### タスク

- [ ] **非推奨マーク**
  - [ ] 旧ハンドラに `#[deprecated]` 追加
  - [ ] 旧リポジトリに `#[deprecated]` 追加
  - [ ] ドキュメントに削除予定を明記

- [ ] **段階的削除**
  - [ ] `/api/v1` エンドポイントの削除
  - [ ] 旧 `handlers/` の削除
  - [ ] 旧 `repositories/` の削除

- [ ] **Feature Flag クリーンアップ**
  - [ ] `restructure_*` フラグの削除
  - [ ] `legacy_*` フラグの削除
  - [ ] デフォルトフラグの更新

#### 検証基準

- [ ] すべてのテストがパス
- [ ] ビルド警告なし
- [ ] デッドコード検出 (`cargo +nightly udeps`)

---

### Week 16: 最終確認

#### タスク

- [ ] **最終ベンチマーク**
  - [ ] Before/After 比較
  - [ ] パフォーマンス改善レポート作成

- [ ] **ドキュメント完成**
  - [ ] `README.md` 更新
  - [ ] `ARCHITECTURE.md` 完全版
  - [ ] `CHANGELOG.md` に移行記録

- [ ] **完了宣言**
  - [ ] チーム全体レビュー
  - [ ] ステークホルダー報告
  - [ ] 成功事例の文書化

#### 完了条件

```markdown
## ✅ 構造再編完了

### 成果
- 全 4000+ テストがパス
- テストカバレッジ: 82% → 95%
- パフォーマンス: +3% 改善
- Clippy 警告: 0件

### 効果
- 開発速度: +40% 向上
- バグ発生率: -70% 削減
- コードレビュー時間: -30% 短縮
```

---

## 📊 週次チェックポイント

各週の金曜日に以下を実施:

1. **進捗確認**
   - 完了タスク数 / 予定タスク数
   - 未完了タスクの理由分析

2. **品質確認**
   - テストカバレッジ
   - Clippy 警告数
   - CI ステータス

3. **リスク評価**
   - スケジュール遅延リスク
   - 技術的課題の有無
   - チームの負荷状況

4. **次週計画**
   - 優先タスクの確認
   - リソース配分の調整

---

## 🚨 ブロッカー発生時の対応

### トリガー条件

- **Red**: 2週連続でタスク完了率 < 70%
- **Red**: テストカバレッジが 5% 以上低下
- **Yellow**: パフォーマンス劣化 > 5%

### 対応フロー

1. **即座に停止**: 新規タスクの着手を停止
2. **原因分析**: ブロッカーの根本原因を特定
3. **対策協議**: チーム全体で対策を検討
4. **必要に応じてロールバック**: `ROLLBACK_PLAN.md` 参照

---

**作成日**: 2025年10月16日  
**最終更新**: 2025年10月16日  
**ステータス**: Phase 1 Week 1 開始準備中
