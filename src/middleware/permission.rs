use crate::{
    AppError,
    auth::{AuthContext, AuthError},
};
use axum::{
    extract::Request,
    response::{IntoResponse, Response},
};
use tower::{Layer, Service};

/// Checks if the `AuthContext` contains a specific permission.
///
/// # Errors
/// Returns `AuthError::InsufficientPermissions` if the permission is not found.
pub fn require_permission(auth_context: &AuthContext, permission: &str) -> crate::Result<()> {
    if auth_context.permissions.iter().any(|p| p == permission) {
        Ok(())
    } else {
        Err(AuthError::InsufficientPermissions.into())
    }
}

/// A layer that checks for a specific permission in the `AuthContext`.
#[derive(Clone)]
pub struct RequirePermissionLayer {
    permissions: Vec<String>,
}

impl RequirePermissionLayer {
    /// Creates a new layer that requires one of the given permissions.
    pub fn new(permissions: Vec<String>) -> Self {
        Self { permissions }
    }

    /// Creates a new layer that requires a single permission.
    pub fn one(permission: &str) -> Self {
        Self {
            permissions: vec![permission.to_string()],
        }
    }
}

impl<S> Layer<S> for RequirePermissionLayer {
    type Service = RequirePermissionService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequirePermissionService {
            inner,
            required_permissions: self.permissions.clone(),
        }
    }
}

#[derive(Clone)]
pub struct RequirePermissionService<S> {
    inner: S,
    required_permissions: Vec<String>,
}

impl<S, B> Service<Request<B>> for RequirePermissionService<S>
where
    S: Service<Request<B>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = futures::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let auth_context = req.extensions().get::<AuthContext>().cloned();
        let required = self.required_permissions.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            match auth_context {
                Some(ctx) => {
                    if required.iter().any(|p| ctx.permissions.contains(p)) {
                        inner.call(req).await
                    } else {
                        Ok(AppError::from(AuthError::InsufficientPermissions).into_response())
                    }
                }
                None => Ok(
                    AppError::Authentication("Authentication required".to_string()).into_response(),
                ),
            }
        })
    }
}
