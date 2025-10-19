# RustCMS レガシーコード削除計画（Phase 6）

**作成日**: 2025年10月19日  
**Phase**: Phase 6 - レガシーコード完全削除  
**前提**: Phase 1-5 完了済み（Domain, Application, Infrastructure, Presentation Layer実装済み）

---

## 📊 現状分析

### 新構造（DDD: Phase 1-5完了）

```
src/
├── domain/                    ✅ Phase 1-2完了（7ファイル, 3,200行）
│   ├── user.rs, post.rs, comment.rs, category.rs, tag.rs
│   ├── events.rs, services/
│
├── application/              ✅ Phase 3完了（40ファイル, 5,454行）
│   ├── user.rs, post.rs, comment.rs, category.rs
│   ├── dto/, use_cases/, queries/, ports/
│
├── infrastructure/           ✅ Phase 3完了（13ファイル）
│   ├── database/, repositories/, events/
│
└── web/                      ✅ Phase 4完了（13ファイル, 1,335行）
    ├── routes_v2.rs
    ├── handlers/*_v2.rs (5ファイル)
```

**新構造合計**: 73ファイル, ~10,000行

---

### レガシー構造（削除対象候補）

#### 🔴 優先度: 高（即削除可能）

これらは新構造で完全に置き換え済み：

```
src/
├── handlers/                  🔴 Phase 4で web/handlers/*_v2.rs に置き換え
│   ├── admin.rs              → web/handlers/ で実装予定
│   ├── api_keys.rs           → web/handlers/ で実装予定
│   ├── auth.rs               ❌ v1 API用（保持）
│   ├── health.rs             ❌ v1 API用（保持）
│   ├── metrics.rs            ❌ 共通（保持）
│   ├── posts.rs              ❌ v1 API用（保持）
│   ├── search.rs             → web/handlers/ で実装予定
│   └── users.rs              ❌ v1 API用（保持）
│
├── models/                   🔴 Phase 3で application/dto/ + infrastructure/database/models.rs に置き換え
│   ├── api_key.rs            → 削除可能（infrastructure/database/models.rs に統合済み）
│   ├── mod.rs
│   ├── pagination.rs         → 削除可能（application/queries/pagination.rs に移行済み）
│   ├── post.rs               → 削除可能（domain/post.rs + infrastructure/database/models.rs）
│   └── user.rs               → 削除可能（domain/user.rs + infrastructure/database/models.rs）
│
└── repositories/             🔴 Phase 3で infrastructure/repositories/ に置き換え
    ├── mod.rs
    ├── post.rs               → 削除可能（infrastructure/repositories/diesel_post_repository.rs）
    └── user_repository.rs    → 削除可能（infrastructure/repositories/diesel_user_repository.rs）
```

#### 🟡 優先度: 中（慎重に削除）

v1 API互換性のため保持中、将来的に削除：

```
src/
├── handlers/                 🟡 v1 API用（Phase 7で廃止予定）
│   ├── auth.rs              ❌ 保持（v1認証エンドポイント）
│   ├── health.rs            ❌ 保持（v1ヘルスチェック）
│   ├── posts.rs             ❌ 保持（v1投稿API）
│   └── users.rs             ❌ 保持（v1ユーザーAPI）
│
└── presentation/             🟡 使用状況不明（確認必要）
    └── http/handlers.rs     → 使用確認後に削除判断
```

#### 🟢 優先度: 低（調査後に判断）

他モジュールへの依存関係が不明：

```
src/
├── web/handlers/            🟢 Phase 4で作成、Phase 5で整理済み
│   ├── admin.rs             → v1用（保持）
│   ├── api_keys.rs          → v1用（保持）
│   ├── auth.rs              → v1用（保持）
│   ├── health.rs            → v1用（保持）
│   ├── metrics.rs           → 共通（保持）
│   ├── posts.rs             → v1用（保持）
│   ├── search.rs            → v1用（保持）
│   └── users.rs             → v1用（保持）
```

---

## 🎯 削除戦略

### Phase 6-1: 即削除可能（リスク: 低）

**対象**: src/models/, src/repositories/（新構造で完全置き換え済み）

#### 削除対象ファイル（5個）

```bash
src/models/
├── api_key.rs       # infrastructure/database/models.rs に統合済み
├── mod.rs
├── pagination.rs    # application/queries/pagination.rs に移行済み
├── post.rs          # domain/post.rs + infrastructure/database/models.rs に移行済み
└── user.rs          # domain/user.rs + infrastructure/database/models.rs に移行済み

src/repositories/
├── mod.rs
├── post.rs          # infrastructure/repositories/diesel_post_repository.rs に移行済み
└── user_repository.rs # infrastructure/repositories/diesel_user_repository.rs に移行済み
```

#### 削除手順

```bash
# 1. src/models/ 削除
rm -rf src/models/

# 2. src/repositories/ 削除
rm -rf src/repositories/

# 3. src/lib.rs から mod 宣言削除
# pub mod models;        → 削除
# pub mod repositories;  → 削除

# 4. ビルド確認
cargo build --lib --all-features
cargo test --lib --all-features
```

---

### Phase 6-2: v1 Handlers保持（リスク: 中）

**対象**: src/handlers/（v1 API用ハンドラ）

#### 保持するハンドラ（v1 API用）

```
src/handlers/
├── auth.rs      ❌ 保持（v1認証）
├── health.rs    ❌ 保持（v1ヘルスチェック）
├── posts.rs     ❌ 保持（v1投稿API）
└── users.rs     ❌ 保持（v1ユーザーAPI）
```

#### 削除可能なハンドラ（新構造で実装済みor未使用）

```
src/handlers/
├── admin.rs     → 削除可能（web/handlers/admin.rs で実装）
├── api_keys.rs  → 削除可能（web/handlers/api_keys.rs で実装）
├── metrics.rs   → 削除可能（web/handlers/metrics.rs で実装）
└── search.rs    → 削除可能（web/handlers/search.rs で実装）
```

#### 削除手順

```bash
# 1. 使用状況確認
grep -r "handlers::admin" src/
grep -r "handlers::api_keys" src/
grep -r "handlers::metrics" src/
grep -r "handlers::search" src/

# 2. 未使用なら削除
rm src/handlers/admin.rs
rm src/handlers/api_keys.rs
rm src/handlers/metrics.rs
rm src/handlers/search.rs

# 3. ビルド確認
cargo build --lib --all-features
```

---

### Phase 6-3: 依存関係整理（リスク: 高）

**対象**: src/lib.rs モジュール宣言整理

#### 現在の宣言（レガシー）

```rust
// src/lib.rs
pub mod handlers;      // レガシー（Phase 6-2で整理）
pub mod models;        // 🔴 Phase 6-1で削除
pub mod repositories;  // 🔴 Phase 6-1で削除
```

#### 整理後の宣言（Phase 6完了時）

```rust
// src/lib.rs
// レガシーモジュール（Phase 7で完全削除予定）
#[cfg(not(feature = "restructure_domain"))]
pub mod handlers;      // v1 API用（保持）

// 新構造モジュール（Phase 1-5完了）
#[cfg(feature = "restructure_domain")]
pub mod domain;        // ✅ Phase 1-2
#[cfg(feature = "restructure_domain")]
pub mod application;   // ✅ Phase 3
#[cfg(feature = "restructure_domain")]
pub mod infrastructure; // ✅ Phase 3
#[cfg(feature = "restructure_domain")]
pub mod web;           // ✅ Phase 4
```

---

## 📋 削除チェックリスト

### Phase 6-1: models/ と repositories/ 削除

- [ ] **依存関係確認**
  - [ ] `grep -r "use.*models::" src/` → 使用箇所リスト
  - [ ] `grep -r "use.*repositories::" src/` → 使用箇所リスト
  - [ ] 使用箇所がある場合は新構造へ移行

- [ ] **ファイル削除**
  - [ ] `rm -rf src/models/`
  - [ ] `rm -rf src/repositories/`

- [ ] **lib.rs 更新**
  - [ ] `pub mod models;` 削除
  - [ ] `pub mod repositories;` 削除

- [ ] **ビルド確認**
  - [ ] `cargo build --lib --all-features`
  - [ ] `cargo test --lib --all-features`
  - [ ] エラーが出た場合は修正

### Phase 6-2: handlers/ 整理

- [ ] **使用状況確認**
  - [ ] admin.rs の参照確認
  - [ ] api_keys.rs の参照確認
  - [ ] metrics.rs の参照確認
  - [ ] search.rs の参照確認

- [ ] **未使用ハンドラ削除**
  - [ ] 確認済みの未使用ファイル削除

- [ ] **ビルド確認**
  - [ ] `cargo build --lib --all-features`

### Phase 6-3: lib.rs 整理

- [ ] **feature flag 追加**
  - [ ] レガシーモジュールに `#[cfg(not(feature = "restructure_domain"))]`
  - [ ] 新構造モジュールに `#[cfg(feature = "restructure_domain")]`

- [ ] **ビルド確認**
  - [ ] `cargo build --lib --all-features`
  - [ ] `cargo build --lib --no-default-features`
  - [ ] `cargo build --lib --features "restructure_domain"`

---

## 🚨 ロールバック手順

Phase 6で問題が発生した場合：

### 即座のロールバック

```bash
# Git で直前のコミットに戻す
git reset --hard HEAD~1

# または特定のファイルを復元
git checkout HEAD -- src/models/
git checkout HEAD -- src/repositories/
git checkout HEAD -- src/handlers/
```

### 段階的ロールバック

```bash
# 削除したファイルを一時的に復元
git show HEAD:src/models/user.rs > src/models/user.rs
git show HEAD:src/repositories/user_repository.rs > src/repositories/user_repository.rs

# lib.rs のモジュール宣言を復元
git checkout HEAD -- src/lib.rs

# ビルド確認
cargo build --lib --all-features
```

---

## 📊 削除予測

### 削除予定ファイル数

| カテゴリ | ファイル数 | 行数（推定） | 削除優先度 |
|---------|-----------|-------------|-----------|
| src/models/ | 5個 | ~400行 | 🔴 高（即削除） |
| src/repositories/ | 3個 | ~300行 | 🔴 高（即削除） |
| src/handlers/ (一部) | 4個 | ~300行 | 🟡 中（確認後） |
| src/presentation/ | 1個 | ~100行 | 🟢 低（調査後） |
| **合計** | **13個** | **~1,100行** | - |

### 削減効果

```
削除前: 73ファイル（新構造）+ 13ファイル（レガシー）= 86ファイル
削除後: 73ファイル（新構造）+ 4-8ファイル（v1 API保持）= 77-81ファイル

削減数: 5-9ファイル（-6% ~ -10%）
削減行数: ~1,100行
```

---

## ✅ Phase 6 完了条件

### 必須条件

- [ ] src/models/ 削除完了
- [ ] src/repositories/ 削除完了
- [ ] src/lib.rs モジュール宣言整理完了
- [ ] 全feature flagsでビルド成功
- [ ] 全テストパス（--all-features）

### 推奨条件

- [ ] 依存関係ドキュメント更新
- [ ] Phase 6完了レポート作成
- [ ] MIGRATION_CHECKLIST.md 更新（Phase 6完了マーク）

---

## 🔜 Phase 7（将来計画）

### v1 API完全廃止

Phase 6完了後、Phase 7で以下を削除：

```
src/handlers/
├── auth.rs      # v1認証（廃止予定）
├── health.rs    # v1ヘルスチェック（廃止予定）
├── posts.rs     # v1投稿API（廃止予定）
└── users.rs     # v1ユーザーAPI（廃止予定）
```

**条件**:
- v2 API が安定稼働（6ヶ月以上）
- 既存クライアントがv2に完全移行
- v1 API使用率 < 5%

---

## 📚 参考ドキュメント

- **Phase 1-5 完了報告**:
  - `PHASE2_COMPLETION_REPORT.md` (Domain Layer)
  - `PHASE3_COMPLETION_REPORT.md` (Application Layer)
  - `PHASE4_PRESENTATION_LAYER_IMPLEMENTATION.md` (Presentation Layer)
  - `PHASE5_COMPLETION_REPORT.md` (Legacy削除 核心部分)

- **計画書**:
  - `RESTRUCTURE_PLAN.md` (全体計画)
  - `ROLLBACK_PLAN.md` (ロールバック手順)
  - `MIGRATION_CHECKLIST.md` (Phase 1-5チェックリスト)

- **実装例**:
  - `RESTRUCTURE_EXAMPLES.md` (新構造実装パターン)
