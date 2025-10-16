# RustCMS 構造再編 - ロールバック計画

## 🚨 ロールバック判断基準

### 自動トリガー (CI で検出)

| 条件 | しきい値 | 対応 |
|-----|---------|-----|
| **テスト失敗率** | ≥ 10% | 即座にマージブロック |
| **カバレッジ低下** | < 70% | Phase を1つ戻す |
| **パフォーマンス劣化** | > 10% | 詳細調査 → ロールバック検討 |
| **ビルド時間増加** | > 20% | 原因特定 → ロールバック検討 |
| **Clippy 警告** | ≥ 50件 | マージブロック |

### 手動トリガー (チーム判断)

1. **スケジュール遅延**
   - 2週連続でタスク完了率 < 70%
   - 見積もり誤差が累積で +30% 以上

2. **品質問題**
   - Production で Critical バグが週2件以上
   - セキュリティ脆弱性の発見

3. **チーム負荷**
   - チームメンバーの50%以上が過負荷を報告
   - バーンアウトリスクの兆候

---

## 📋 Phase 別ロールバック手順

### Phase 1: 基礎固め (Week 1-3)

#### ロールバック条件

- Value Objects のテストカバレッジ < 95%
- Port 定義に曖昧さが残る
- CI ビルドが通らない状態が2日以上継続

#### ロールバック手順

```bash
# 1. Phase 1 ブランチの特定
git log --oneline --grep="Phase 1"

# 2. Phase 1 開始前のコミットを確認
git log --oneline main | grep "Phase 1 開始直前"

# 3. ロールバック実行
git checkout main
git revert <phase1-start-commit>..<phase1-end-commit>

# 4. ロールバックブランチ作成
git checkout -b rollback/phase1-$(date +%Y%m%d)
git push origin rollback/phase1-$(date +%Y%m%d)

# 5. PR 作成
gh pr create --title "ロールバック: Phase 1" \
  --body "理由: [ここに理由を記載]"
```

#### 影響範囲

- **削除対象**:
  - `src/domain/value_objects/` 配下のすべて
  - `src/application/ports/` 配下のすべて
  - 新規追加された CI ジョブ

- **復元対象**:
  - 旧 `src/models/` の使用
  - 既存のハンドラー実装

#### リスク

- **低**: Value Objects/Ports は既存コードに影響しない設計

---

### Phase 2: ドメイン層構築 (Week 4-7)

#### ロールバック条件

- エンティティのバグが3件以上未解決
- ドメインイベント統合で既存リスナーが動作不良
- パフォーマンステストで > 10% 劣化

#### ロールバック手順

```bash
# 1. Phase 2 の変更内容を確認
git log --oneline main --grep="Phase 2"

# 2. Phase 1 完了時点まで戻す
git checkout main
git revert <phase2-start>..<phase2-end>

# 3. Phase 1 の成果物は保持
git checkout <phase1-end-commit> -- src/domain/value_objects/
git checkout <phase1-end-commit> -- src/application/ports/

# 4. コミットとプッシュ
git commit -m "ロールバック: Phase 2 (Phase 1 成果物は保持)"
git push origin rollback/phase2-$(date +%Y%m%d)
```

#### 影響範囲

- **削除対象**:
  - `src/domain/entities/` 配下のすべて
  - `src/domain/services/` 配下のすべて
  - `src/domain/events/` の新規イベント

- **復元対象**:
  - 既存の `src/models/user.rs` などの直接使用
  - 旧イベントシステム (`src/events.rs`) の継続使用

- **保持**:
  - Phase 1 の Value Objects と Ports (他に影響なし)

#### リスク

- **中**: ドメインイベントが既存リスナーと統合されている場合、リスナーの修正が必要

#### リスク軽減策

- Phase 2 開始時に `feature = "restructure_domain"` フラグを導入
- 既存リスナーは旧イベントを継続使用
- 新イベントは feature flag で隔離

---

### Phase 3: アプリケーション層構築 (Week 8-11)

#### ロールバック条件

- Use Case のトランザクション管理でデータ不整合が発生
- Repository 実装のバグが5件以上未解決
- Unit of Work でデッドロックが頻発

#### ロールバック手順

```bash
# 1. Phase 2 完了時点まで戻す
git revert <phase3-start>..<phase3-end>

# 2. Phase 1-2 の成果物を保持
git checkout <phase2-end> -- src/domain/

# 3. Application Layer のみ削除
rm -rf src/application/use_cases/
rm -rf src/application/dto/
git checkout <phase2-end> -- src/infrastructure/

# 4. コミット
git commit -m "ロールバック: Phase 3 (Domain Layer は保持)"
```

#### 影響範囲

- **削除対象**:
  - `src/application/use_cases/` 配下のすべて
  - `src/application/dto/` 配下のすべて
  - `src/infrastructure/repositories/` の新実装
  - `DieselUnitOfWork` 実装

- **復元対象**:
  - 既存ハンドラー内の直接的な DB 操作
  - 旧リポジトリパターン (`src/repositories/`)

- **保持**:
  - Phase 1-2 のドメイン層 (独立しているため影響なし)

#### リスク

- **高**: トランザクション境界の変更は既存コードに影響大

#### リスク軽減策

- Phase 3 開始前に `feature = "restructure_application"` フラグを導入
- 新旧コードを並行稼働させる期間を2週間設定
- 旧ハンドラーを `/api/v1` に残し、新ハンドラーを `/api/v2` に配置

---

### Phase 4: プレゼンテーション層 (Week 12-14)

#### ロールバック条件

- API レスポンスタイムが > 15% 劣化
- エンドポイントの E2E テスト失敗率 > 5%
- クライアント側の統合エラーが多発

#### ロールバック手順

```bash
# 1. ルーティングを旧ハンドラーに戻す
git checkout <phase3-end> -- src/routes/

# 2. 新ハンドラーを削除
rm -rf src/presentation/http/handlers/

# 3. 旧ハンドラーを復元
git checkout <phase3-end> -- src/handlers/

# 4. main.rs のルーティング設定を戻す
git checkout <phase3-end> -- src/main.rs

# 5. コミット
git commit -m "ロールバック: Phase 4 (API を旧ハンドラーに戻す)"
```

#### 影響範囲

- **削除対象**:
  - `src/presentation/http/handlers/` 配下のすべて
  - `/api/v2` エンドポイント
  - 新しいミドルウェア実装

- **復元対象**:
  - `/api/v1` エンドポイントを default に
  - 既存ハンドラー (`src/handlers/`) の使用継続

- **保持**:
  - Phase 1-3 の成果物 (ドメイン/アプリケーション層)
  - Use Case は内部で使用可能なまま

#### リスク

- **低〜中**: API バージョニングで旧エンドポイントが残っているため影響小

#### リスク軽減策

- Phase 4 開始前に `/api/v2` を試験運用 (production traffic の 10% のみ)
- カナリアリリースで段階的に切り替え
- `/api/v1` を3ヶ月間維持 (非推奨マーク付き)

---

### Phase 5: クリーンアップ (Week 15-16)

#### ロールバック条件

- 旧コード削除後に未検出の依存関係が発覚
- Production で Critical バグが発生

#### ロールバック手順

```bash
# 1. 削除されたファイルを復元
git log --diff-filter=D --summary | grep "src/handlers/"
git checkout <phase4-end> -- src/handlers/

# 2. 非推奨マークを再度追加
# (手動で #[deprecated] を追加)

# 3. コミット
git commit -m "ロールバック: Phase 5 (旧コードを一時復元)"
```

#### 影響範囲

- **削除対象**: なし (Phase 5 のクリーンアップを保留)
- **復元対象**: 旧ハンドラー/旧リポジトリの一時復元

#### リスク

- **低**: Phase 5 は純粋なコード削除のため、復元は容易

---

## 🔄 段階的ロールバック (Gradual Rollback)

### 全面ロールバックが必要な場合

```bash
# 1. main ブランチの Phase 0 (再編開始前) タグを確認
git tag | grep "before-restructure"

# 2. 新しいロールバックブランチを作成
git checkout -b full-rollback/$(date +%Y%m%d)

# 3. Phase 0 の状態を復元
git reset --hard before-restructure

# 4. 強制プッシュ (注意!)
git push origin full-rollback/$(date +%Y%m%d) --force-with-lease

# 5. PR 作成とレビュー
gh pr create --title "全面ロールバック: 構造再編の中止" \
  --body "理由: [詳細な理由を記載]"
```

### Feature Flag による部分ロールバック

Phase 3 以降は feature flag で新旧を並行稼働できるため、**段階的なロールバック**が可能:

```toml
# Cargo.toml
[features]
default = ["database", "cache", "search", "auth"]

# 再編関連フラグ
restructure_domain = []         # Phase 1-2
restructure_application = []    # Phase 3
restructure_presentation = []   # Phase 4

# レガシー維持フラグ
legacy_handlers = []            # 旧ハンドラーを残す
legacy_repositories = []        # 旧リポジトリを残す
```

**ロールバック手順**:

```bash
# 1. feature flag を無効化してビルド
cargo build --no-default-features \
  --features "database,cache,search,auth,legacy_handlers"

# 2. Production で旧コードにルーティング
# (環境変数で切り替え)
export USE_LEGACY_HANDLERS=true
```

---

## 📊 ロールバック後の再評価プロセス

### 1. 原因分析 (1週間以内)

- [ ] **技術的原因の特定**
  - どの設計判断が問題だったか
  - テストで見逃された箇所はどこか

- [ ] **プロセス的原因の特定**
  - 見積もりが甘かったか
  - レビュープロセスに問題があったか

### 2. 対策立案 (1週間以内)

- [ ] **技術的対策**
  - 設計の見直し
  - テストカバレッジの強化

- [ ] **プロセス的対策**
  - タスク分割の細分化
  - レビュー基準の厳格化

### 3. 再開判断 (2週間以内)

#### 再開条件

- [ ] 原因が完全に特定されている
- [ ] 対策が文書化されている
- [ ] チーム全員の合意が得られている
- [ ] スケジュールが再調整されている

#### 中止条件

- [ ] 原因が不明瞭
- [ ] 対策コストが > 予想効果
- [ ] チームリソースが不足

---

## 🛠️ ロールバック補助ツール

### 自動ロールバックスクリプト

```bash
#!/bin/bash
# scripts/rollback.sh

PHASE=$1
DATE=$(date +%Y%m%d)

if [ -z "$PHASE" ]; then
  echo "Usage: ./scripts/rollback.sh <phase_number>"
  exit 1
fi

echo "🚨 Phase $PHASE のロールバックを開始します..."

# Phase 開始前のコミットを検索
START_COMMIT=$(git log --oneline --grep="Phase $PHASE 開始" --format="%H" | head -1)
END_COMMIT=$(git log --oneline --grep="Phase $PHASE 完了" --format="%H" | head -1)

if [ -z "$START_COMMIT" ]; then
  echo "❌ Phase $PHASE の開始コミットが見つかりません"
  exit 1
fi

# ロールバック実行
git checkout main
git pull origin main
git revert $START_COMMIT..$END_COMMIT --no-commit

# ロールバックブランチ作成
git checkout -b "rollback/phase$PHASE-$DATE"
git commit -m "ロールバック: Phase $PHASE"

echo "✅ ロールバック完了。以下のコマンドでプッシュしてください:"
echo "  git push origin rollback/phase$PHASE-$DATE"
```

### 影響範囲確認スクリプト

```bash
#!/bin/bash
# scripts/check_rollback_impact.sh

PHASE=$1

echo "📊 Phase $PHASE のロールバック影響範囲を確認中..."

# 変更されたファイル一覧
git diff --name-only <phase-start>..<phase-end>

# テストカバレッジの変化
cargo tarpaulin --out Json --output-dir /tmp/before
# ... ロールバック実行 ...
cargo tarpaulin --out Json --output-dir /tmp/after

echo "カバレッジ変化:"
jq '.coverage' /tmp/before/tarpaulin.json /tmp/after/tarpaulin.json
```

---

## 📋 ロールバック後チェックリスト

### 即座に実施

- [ ] **CI が Green になることを確認**
  ```bash
  cargo test --workspace --all-features
  cargo clippy --workspace --all-features -- -D warnings
  ```

- [ ] **ベンチマークが Phase 前と同等**
  ```bash
  cargo bench --bench baseline
  ```

- [ ] **Production デプロイ前の Staging 検証**
  - [ ] E2E テストがすべてパス
  - [ ] 主要エンドポイントの動作確認

### 1週間以内

- [ ] **ドキュメント更新**
  - [ ] `CHANGELOG.md` にロールバックを記録
  - [ ] `ARCHITECTURE.md` から該当 Phase の記述を削除

- [ ] **チームレトロスペクティブ**
  - [ ] 良かった点 (ロールバック判断の迅速さなど)
  - [ ] 改善点 (見逃したリスク、テスト不足など)

- [ ] **ステークホルダー報告**
  - [ ] ロールバック理由の説明
  - [ ] 再開計画 or 中止判断の共有

---

**作成日**: 2025年10月16日  
**最終更新**: 2025年10月16日  
**ステータス**: Phase 1 開始前
