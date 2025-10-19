# Phase 4 レガシーコード削除計画

> **作成日**: 2025年10月19日  
> **目的**: 監査済みディレクトリ構造への完全移行  
> **Phase**: Phase 4 - Presentation層改善 + レガシー削除

---

## 📋 削除対象ファイル一覧

### 1. infrastructure/repositories/ (旧構造) - 7ファイル

**理由**: Phase 3 Week 10で `infrastructure/database/repositories.rs` に統合済み

| ファイル | 行数 | 削除理由 | 移行先 |
|---------|------|---------|--------|
| `diesel_user_repository.rs` | ~300 | 旧User Repository実装 | `infrastructure/database/repositories.rs` (341行) |
| `diesel_post_repository.rs` | ~300 | 旧Post Repository実装 | `infrastructure/database/repositories.rs` (370行) |
| `diesel_comment_repository.rs` | ~300 | 旧Comment Repository実装 | `infrastructure/database/repositories.rs` (373行) |
| `diesel_tag_repository.rs` | ~250 | 旧Tag Repository（未使用） | 削除のみ（新版未実装） |
| `diesel_category_repository.rs` | ~250 | 旧Category Repository（未使用） | 削除のみ（新版未実装） |
| `error_helpers.rs` | ~50 | 旧エラーヘルパー | `common/error_types.rs` に統合済み |
| `mod.rs` | ~100 | モジュール定義 | 削除 |
| **合計** | **~1,550行** | **削除対象** | - |

### 2. application/use_cases/ (統合予定) - 部分削除

**理由**: Phase 3 Week 8-9で `application/*.rs` (CQRS統合) に移行済み

| ファイル/ディレクトリ | 行数 | 削除理由 | 移行先 |
|---------------------|------|---------|--------|
| `user/` ディレクトリ | ~600 | 旧User Use Cases | `application/user.rs` (CQRS統合) |
| `post/` ディレクトリ | ~800 | 旧Post Use Cases | `application/post.rs` (CQRS統合) |
| `comment/` ディレクトリ | ~500 | 旧Comment Use Cases | `application/comment.rs` (CQRS統合) |
| `category.rs` | ~200 | 旧Category Use Case（未完成） | 削除のみ（新版未実装） |
| `tag.rs` | ~200 | 旧Tag Use Case（未完成） | 削除のみ（新版未実装） |
| `examples_unit_of_work.rs` | ~150 | サンプルコード | 保持（教育目的） |
| `mod.rs` | ~50 | モジュール定義（一部保持） | 修正（examples_unit_of_work のみ export） |
| **合計（削除）** | **~2,300行** | **削除対象** | - |
| **合計（保持）** | **~200行** | **保持** | - |

### 3. その他のレガシーファイル

| ファイル | 行数 | 削除理由 | 代替 |
|---------|------|---------|------|
| `src/events.rs` | ~500 | 旧イベント定義 | `infrastructure/events/bus.rs` + `domain/events.rs` |
| `src/listeners.rs` | ~400 | 旧リスナー | `infrastructure/events/listeners.rs` |
| `presentation/http/handlers.rs` | 0 | 既に削除済み ✅ | `web/handlers/*.rs` |

**Note**: `events.rs` と `listeners.rs` は Phase 4後半で移行予定（bin/の依存関係整理後）

---

## 📊 削除統計サマリー

| カテゴリ | ファイル数 | 行数 | ステータス |
|---------|----------|------|----------|
| **infrastructure/repositories/** | 7 | ~1,550 | 🔜 削除予定 |
| **application/use_cases/** | 5 dirs/files | ~2,300 | 🔜 削除予定 |
| **その他** | 2 | ~900 | ⏰ Phase 4後半 |
| **合計（今回）** | **12** | **~3,850行** | **🔜 削除予定** |

---

## 🎯 削除前チェックリスト

### 1. infrastructure/repositories/ 削除前確認

- [ ] `infrastructure/database/repositories.rs` が全機能を実装していることを確認
  - [x] DieselUserRepository ✅ (341行, 5 tests)
  - [x] DieselPostRepository ✅ (370行, 4 tests)
  - [x] DieselCommentRepository ✅ (373行, 5 tests)
  - [ ] DieselTagRepository ⚠️ (未実装 - 必要に応じてPhase 5で実装)
  - [ ] DieselCategoryRepository ⚠️ (未実装 - 必要に応じてPhase 5で実装)

- [ ] 旧Repositoryへの参照がないことを確認
  ```bash
  grep -r "infrastructure::repositories" src/ --exclude-dir=target
  # 結果: 0件であるべき
  ```

### 2. application/use_cases/ 削除前確認

- [ ] `application/*.rs` (CQRS統合) が全Use Casesを実装していることを確認
  - [x] User Use Cases ✅ (4個, 14 tests)
  - [x] Post Use Cases ✅ (4個, 20 tests)
  - [x] Comment Use Cases ✅ (2個, 9 tests)
  - [ ] Category Use Cases ⚠️ (未実装)
  - [ ] Tag Use Cases ⚠️ (未実装)

- [ ] 旧Use Casesへの参照がないことを確認
  ```bash
  grep -r "application::use_cases::" src/ --exclude-dir=target
  # bin/admin.rs の参照のみ（Phase 4で修正予定）
  ```

---

## 🚀 削除実行計画

### Phase 4.1: infrastructure/repositories/ 削除（今回）

**優先度**: High ✅  
**リスク**: Low（新Repositoryで完全置換済み）

**手順**:
1. 参照確認: `grep -r "infrastructure::repositories" src/`
2. バックアップ: Git commit 状態確認
3. 削除実行: `rm -rf src/infrastructure/repositories/`
4. mod.rs 修正: `src/infrastructure/mod.rs` から repositories モジュール削除
5. ビルド確認: `cargo check --lib --features restructure_domain,database`
6. テスト確認: `cargo test --lib --features restructure_domain,database`

### Phase 4.2: application/use_cases/ 削除（今回）

**優先度**: Medium ⚠️  
**リスク**: Medium（bin/admin.rs が依存）

**手順**:
1. 参照確認: `grep -r "application::use_cases" src/`
2. bin/admin.rs 修正: 新Use Cases (application::user, etc.) に移行
3. 削除実行:
   ```bash
   rm -rf src/application/use_cases/user/
   rm -rf src/application/use_cases/post/
   rm -rf src/application/use_cases/comment/
   rm src/application/use_cases/category.rs
   rm src/application/use_cases/tag.rs
   ```
4. mod.rs 修正: examples_unit_of_work のみ export
5. ビルド確認: `cargo check --all-features`（bin含む）
6. テスト確認: `cargo test --all-features`

### Phase 4.3: events.rs/listeners.rs 移行（Phase 4後半）

**優先度**: Low ⏰  
**リスク**: High（bin/の広範な依存）

**手順**:
1. bin/全バイナリの events.rs/listeners.rs 依存を確認
2. infrastructure/events/bus.rs に機能統合
3. bin/リファクタリング完了後に削除

---

## 📝 削除後の検証

### 必須チェック

1. **ビルド成功**:
   ```bash
   cargo check --lib --features restructure_domain,database
   # ✅ 0 errors
   ```

2. **テスト成功**:
   ```bash
   cargo test --lib --features restructure_domain,database
   # ✅ 398+ tests passing
   ```

3. **参照確認**:
   ```bash
   grep -r "infrastructure::repositories" src/
   grep -r "use_cases::" src/
   # ✅ 0件（またはbin/のみ）
   ```

### オプションチェック

4. **bin/ビルド**（Phase 4.2後）:
   ```bash
   cargo check --all-features
   # ✅ 0 errors（bin含む）
   ```

5. **統合テスト**（DB起動後）:
   ```bash
   cargo test --test integration_*
   # ✅ All passing
   ```

---

## 🎯 期待される成果

### 削除統計

| 項目 | Before | After | 削減 |
|------|--------|-------|------|
| infrastructure/repositories/ | 7 files, ~1,550行 | 0 files | -1,550行 |
| application/use_cases/ | 5 dirs, ~2,300行 | 1 file, ~200行 | -2,100行 |
| **合計** | **~3,850行** | **~200行** | **-3,650行** ✅ |

### ファイル数削減

| カテゴリ | Before | After | 削減率 |
|---------|--------|-------|--------|
| Repositoryファイル | 7 | 1 | **-85.7%** |
| Use Caseファイル | 12+ | 3+examples | **-75%** |

### 監査適合率向上

| 指標 | Before | After | 改善 |
|------|--------|-------|------|
| 監査適合率 | 95% | **98%** ✅ | +3% |
| レガシーコード | ~3,850行 | 0行 | **-100%** ✅ |
| ファイル数（新構造） | 34 | **30** ✅ | -11.7% |

---

## ⚠️ リスク管理

### High Risk

1. **bin/admin.rs の依存**
   - リスク: use_cases 削除で admin CLI が動作不能
   - 対策: Phase 4.2で新Use Cases (application::user等) に移行
   - 検証: `cargo run --bin cms-admin -- user list`

### Medium Risk

2. **Tag/Category Repository未実装**
   - リスク: 将来的に必要になる可能性
   - 対策: Phase 5で必要に応じて実装
   - 現状: 使用されていないため問題なし

### Low Risk

3. **events.rs/listeners.rs 移行**
   - リスク: bin/の広範な依存
   - 対策: Phase 4後半で慎重に移行
   - 現状: 並行稼働可能

---

## 📅 実施タイムライン

| Phase | タスク | 所要時間 | ステータス |
|-------|--------|----------|-----------|
| **4.1** | infrastructure/repositories/ 削除 | 30分 | 🔜 次 |
| **4.2** | application/use_cases/ 削除 | 1時間 | 🔜 次 |
| **4.3** | bin/リファクタリング | 2時間 | ⏰ 後 |
| **4.4** | events/listeners移行 | 1時間 | ⏰ 後 |
| **Total** | - | **4.5時間** | - |

---

**Status**: 🔜 Phase 4.1 開始準備完了  
**Next**: infrastructure/repositories/ 削除実行  
**Goal**: レガシーコード -3,650行達成 + 監査適合率98%達成
