# Unit of Work パターン設計書

## 概要

複数のリポジトリ操作を1つのトランザクション内で実行するためのパターン実装。

## 目的

- **ACID 保証**: 複数の操作が原子的に実行されることを保証
- **整合性維持**: 部分的な失敗時の自動ロールバック
- **明示的な境界**: トランザクション境界を明確化

## Diesel での実装

### 基本設計

```rust
// src/application/ports/unit_of_work.rs

use async_trait::async_trait;
use std::future::Future;

/// トランザクション管理のポート
#[async_trait]
pub trait UnitOfWork: Send + Sync {
    /// トランザクションを開始
    async fn begin(&self) -> Result<Box<dyn Transaction>, TransactionError>;
}

/// トランザクションインターフェース
#[async_trait]
pub trait Transaction: Send + Sync {
    /// トランザクションをコミット
    async fn commit(self: Box<Self>) -> Result<(), TransactionError>;

    /// トランザクションをロールバック
    async fn rollback(self: Box<Self>) -> Result<(), TransactionError>;

    /// トランザクション内でリポジトリを取得
    fn user_repository(&self) -> Box<dyn UserRepository>;
    fn post_repository(&self) -> Box<dyn PostRepository>;
}

#[derive(Debug, thiserror::Error)]
pub enum TransactionError {
    #[error("Failed to begin transaction: {0}")]
    BeginFailed(String),

    #[error("Failed to commit transaction: {0}")]
    CommitFailed(String),

    #[error("Failed to rollback transaction: {0}")]
    RollbackFailed(String),
}
```

### Diesel 実装

```rust
// src/infrastructure/database/unit_of_work_impl.rs

use crate::application::ports::{Transaction, UnitOfWork};
use crate::infrastructure::database::DbPool;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::PgConnection;
use std::sync::Arc;

pub struct DieselUnitOfWork {
    pool: Arc<DbPool>,
}

impl DieselUnitOfWork {
    pub fn new(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UnitOfWork for DieselUnitOfWork {
    async fn begin(&self) -> Result<Box<dyn Transaction>, TransactionError> {
        let conn = self.pool
            .get()
            .await
            .map_err(|e| TransactionError::BeginFailed(e.to_string()))?;

        Ok(Box::new(DieselTransaction::new(conn)))
    }
}

/// Diesel トランザクション実装
pub struct DieselTransaction {
    connection: PooledConnection<ConnectionManager<PgConnection>>,
    committed: bool,
    rolled_back: bool,
}

impl DieselTransaction {
    fn new(connection: PooledConnection<ConnectionManager<PgConnection>>) -> Self {
        // Diesel では connection.begin_test_transaction() を使用
        // または手動で BEGIN を実行
        connection.execute("BEGIN").expect("Failed to begin transaction");

        Self {
            connection,
            committed: false,
            rolled_back: false,
        }
    }
}

#[async_trait]
impl Transaction for DieselTransaction {
    async fn commit(mut self: Box<Self>) -> Result<(), TransactionError> {
        if self.rolled_back {
            return Err(TransactionError::CommitFailed(
                "Transaction already rolled back".to_string()
            ));
        }

        self.connection
            .execute("COMMIT")
            .map_err(|e| TransactionError::CommitFailed(e.to_string()))?;

        self.committed = true;
        Ok(())
    }

    async fn rollback(mut self: Box<Self>) -> Result<(), TransactionError> {
        if self.committed {
            return Err(TransactionError::RollbackFailed(
                "Transaction already committed".to_string()
            ));
        }

        self.connection
            .execute("ROLLBACK")
            .map_err(|e| TransactionError::RollbackFailed(e.to_string()))?;

        self.rolled_back = true;
        Ok(())
    }

    fn user_repository(&self) -> Box<dyn UserRepository> {
        Box::new(DieselUserRepository::with_connection(&self.connection))
    }

    fn post_repository(&self) -> Box<dyn PostRepository> {
        Box::new(DieselPostRepository::with_connection(&self.connection))
    }
}

impl Drop for DieselTransaction {
    fn drop(&mut self) {
        // 自動ロールバック: commit も rollback もされていない場合
        if !self.committed && !self.rolled_back {
            let _ = self.connection.execute("ROLLBACK");
        }
    }
}
```

### Use Case での使用例

```rust
// src/application/use_cases/user/transfer_post_ownership.rs

use crate::application::ports::{UnitOfWork, UserRepository, PostRepository};
use crate::domain::value_objects::{UserId, PostId};

pub struct TransferPostOwnershipUseCase<U: UnitOfWork> {
    unit_of_work: Arc<U>,
}

impl<U: UnitOfWork> TransferPostOwnershipUseCase<U> {
    pub async fn execute(
        &self,
        post_id: PostId,
        new_owner_id: UserId,
    ) -> Result<(), ApplicationError> {
        // トランザクション開始
        let tx = self.unit_of_work.begin().await?;

        // トランザクション内でリポジトリ取得
        let user_repo = tx.user_repository();
        let post_repo = tx.post_repository();

        // 複数の操作を実行
        let new_owner = user_repo.find_by_id(new_owner_id).await?
            .ok_or(ApplicationError::UserNotFound)?;

        let mut post = post_repo.find_by_id(post_id).await?
            .ok_or(ApplicationError::PostNotFound)?;

        // ビジネスロジック
        post.change_author(new_owner_id)?;

        // 保存
        post_repo.save(&post).await?;

        // コミット（失敗時は自動ロールバック）
        tx.commit().await?;

        Ok(())
    }
}
```

## ネストされたトランザクションの扱い

### セーブポイント実装

```rust
// Diesel の SAVEPOINT サポート
impl Transaction for DieselTransaction {
    async fn savepoint(&self, name: &str) -> Result<(), TransactionError> {
        self.connection
            .execute(&format!("SAVEPOINT {}", name))
            .map_err(|e| TransactionError::SavepointFailed(e.to_string()))?;
        Ok(())
    }

    async fn rollback_to_savepoint(&self, name: &str) -> Result<(), TransactionError> {
        self.connection
            .execute(&format!("ROLLBACK TO SAVEPOINT {}", name))
            .map_err(|e| TransactionError::RollbackFailed(e.to_string()))?;
        Ok(())
    }
}
```

### ネストされた Use Case の例

```rust
pub async fn execute(&self) -> Result<(), ApplicationError> {
    let tx = self.unit_of_work.begin().await?;

    // 外側の操作
    self.operation_a(&tx).await?;

    // ネストされた操作（セーブポイント使用）
    tx.savepoint("nested").await?;
    match self.operation_b(&tx).await {
        Ok(_) => {},
        Err(e) => {
            // 部分的なロールバック
            tx.rollback_to_savepoint("nested").await?;
            // エラーをログに記録して続行
            tracing::warn!("Operation B failed, but continuing: {}", e);
        }
    }

    tx.commit().await?;
    Ok(())
}
```

## ロールバック時の副作用処理

### イベント発行との統合

```rust
// src/infrastructure/events/transactional_event_bus.rs

pub struct TransactionalEventBus {
    event_bus: Arc<EventBus>,
    pending_events: Arc<Mutex<Vec<DomainEvent>>>,
}

impl TransactionalEventBus {
    /// トランザクション内でイベントをバッファリング
    pub fn publish_pending(&self, event: DomainEvent) {
        self.pending_events.lock().unwrap().push(event);
    }

    /// コミット成功後に一括発行
    pub async fn flush(&self) -> Result<(), EventError> {
        let events = self.pending_events.lock().unwrap().drain(..).collect::<Vec<_>>();
        for event in events {
            self.event_bus.publish(event).await?;
        }
        Ok(())
    }

    /// ロールバック時にバッファをクリア
    pub fn clear(&self) {
        self.pending_events.lock().unwrap().clear();
    }
}

// Use Case での使用
impl UseCase {
    pub async fn execute(&self) -> Result<(), ApplicationError> {
        let tx = self.unit_of_work.begin().await?;
        let event_bus = TransactionalEventBus::new(self.event_bus.clone());

        // 操作実行
        self.do_work(&tx, &event_bus).await?;

        // コミット
        tx.commit().await?;

        // イベント発行（コミット後）
        event_bus.flush().await?;

        Ok(())
    }
}
```

### キャッシュ無効化との統合

```rust
pub struct TransactionalCacheInvalidator {
    cache: Arc<dyn CacheService>,
    pending_invalidations: Arc<Mutex<Vec<String>>>,
}

impl TransactionalCacheInvalidator {
    /// 無効化をスケジュール
    pub fn schedule_invalidation(&self, key: String) {
        self.pending_invalidations.lock().unwrap().push(key);
    }

    /// コミット後に実行
    pub async fn execute(&self) -> Result<(), CacheError> {
        let keys = self.pending_invalidations.lock().unwrap().drain(..).collect::<Vec<_>>();
        for key in keys {
            self.cache.delete(&key).await?;
        }
        Ok(())
    }

    /// ロールバック時はクリア
    pub fn clear(&self) {
        self.pending_invalidations.lock().unwrap().clear();
    }
}
```

## テスト戦略

### Unit Test

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_transaction_commit() {
        let uow = DieselUnitOfWork::new(test_pool());
        let tx = uow.begin().await.unwrap();

        let user_repo = tx.user_repository();
        user_repo.save(&test_user()).await.unwrap();

        tx.commit().await.unwrap();

        // 検証: データが保存されている
    }

    #[tokio::test]
    async fn test_transaction_rollback() {
        let uow = DieselUnitOfWork::new(test_pool());
        let tx = uow.begin().await.unwrap();

        let user_repo = tx.user_repository();
        user_repo.save(&test_user()).await.unwrap();

        tx.rollback().await.unwrap();

        // 検証: データが保存されていない
    }

    #[tokio::test]
    async fn test_auto_rollback_on_drop() {
        let uow = DieselUnitOfWork::new(test_pool());
        {
            let tx = uow.begin().await.unwrap();
            let user_repo = tx.user_repository();
            user_repo.save(&test_user()).await.unwrap();
            // tx が Drop される
        }

        // 検証: 自動ロールバックされている
    }
}
```

## 実装スケジュール

| Week | タスク | 成果物 |
|------|--------|--------|
| Week 1 | Port 定義 | `application/ports/unit_of_work.rs` |
| Week 2 | Diesel 実装 | `infrastructure/database/unit_of_work_impl.rs` |
| Week 3 | イベント/キャッシュ統合 | TransactionalEventBus, CacheInvalidator |
| Week 4 | テスト作成 | 統合テスト 30+ |

## 参考資料

- [Diesel Transaction Docs](https://docs.diesel.rs/master/diesel/connection/trait.Connection.html#method.transaction)
- [Unit of Work Pattern](https://martinfowler.com/eaaCatalog/unitOfWork.html)
- [PostgreSQL Savepoints](https://www.postgresql.org/docs/current/sql-savepoint.html)

---

**作成日**: 2025年10月16日
**ステータス**: 設計完了
**次回レビュー**: Phase 2 開始前
