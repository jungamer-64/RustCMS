use axum::extract::Request;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::Service;

// Boxed future type alias reused by middleware Service impls
pub type BoxServiceFuture<T, E> = Pin<Box<dyn Future<Output = Result<T, E>> + Send>>;

// Forward poll_ready to the inner service to avoid repeating boilerplate
pub fn forward_poll_ready<S, B>(service: &mut S, cx: &mut Context<'_>) -> Poll<Result<(), S::Error>>
where
    S: Service<Request<B>>,
{
    service.poll_ready(cx)
}
