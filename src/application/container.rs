// Application-level DI container for holding shared concrete adapters and use-cases.
// This is intentionally minimal: it holds only the things the application currently
// needs to avoid premature generalization. The container is feature-gated so
// that builds without the `database` feature do not carry unnecessary code.

use std::sync::Arc;

#[cfg(feature = "database")]
use crate::application::use_cases::{CreateUserUseCase, GetUserByIdUseCase, UpdateUserUseCase};
#[cfg(feature = "database")]
use crate::infrastructure::repositories::DieselUserRepository;

/// Simple app-level container that holds concrete repository adapters and
/// constructed use-case instances to be shared across handlers and background
/// tasks. Instances are stored in Arcs so they can be cheaply cloned into
/// request handlers.
#[cfg(feature = "database")]
#[derive(Clone)]
pub struct AppContainer {
    /// Concrete Diesel-backed user repository adapter
    pub user_repo: Arc<DieselUserRepository>,

    /// Use-cases constructed against the concrete adapter
    pub get_user_by_id: Arc<GetUserByIdUseCase<DieselUserRepository>>,
    pub create_user: Arc<CreateUserUseCase<DieselUserRepository>>,
    pub update_user: Arc<UpdateUserUseCase<DieselUserRepository>>,
}

#[cfg(feature = "database")]
impl AppContainer {
    /// Create a new container from already-initialized infrastructure pieces.
    ///
    /// `database` is cloned into the concrete repository adapter; `event_bus`
    /// is accepted for completeness and may be used to construct other
    /// components in the future.
    #[must_use]
    pub fn new(database: crate::database::Database, _event_bus: crate::events::EventBus) -> Self {
        let repo = DieselUserRepository::new(database.clone());
        let repo_arc = Arc::new(repo);

        let get_user_uc = Arc::new(GetUserByIdUseCase::new(repo_arc.clone()));
        let create_user_uc = Arc::new(CreateUserUseCase::new(repo_arc.clone()));
        let update_user_uc = Arc::new(UpdateUserUseCase::new(repo_arc.clone()));

        Self {
            user_repo: repo_arc,
            get_user_by_id: get_user_uc,
            create_user: create_user_uc,
            update_user: update_user_uc,
        }
    }
}
