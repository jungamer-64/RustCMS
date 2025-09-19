use std::future::Future;
use std::pin::Pin;

use uuid::Uuid;

use crate::models::User;
use crate::Result;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait UserRepository: Send + Sync {
    fn get_user_by_email(&self, email: &str) -> BoxFuture<'_, Result<User>>;
    fn get_user_by_id(&self, id: Uuid) -> BoxFuture<'_, Result<User>>;
    fn update_last_login(&self, id: Uuid) -> BoxFuture<'_, Result<()>>;
}
