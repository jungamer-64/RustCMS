use crate::middleware::common::{BoxServiceFuture, forward_poll_ready};
use axum::{extract::Request, http::HeaderValue, response::Response};
use tower::{Layer, Service};
use uuid::Uuid;

/// Request ID middleware for distributed tracing
#[derive(Clone)]
pub struct RequestIdLayer;

impl RequestIdLayer {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for RequestIdLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for RequestIdLayer {
    type Service = RequestIdService<S>;

    fn layer(&self, service: S) -> Self::Service {
        RequestIdService { service }
    }
}

#[derive(Clone)]
pub struct RequestIdService<S> {
    service: S,
}

impl<S, B> Service<Request<B>> for RequestIdService<S>
where
    S: Service<Request<B>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxServiceFuture<Self::Response, Self::Error>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        forward_poll_ready(&mut self.service, cx)
    }

    fn call(&mut self, mut request: Request<B>) -> Self::Future {
        let request_id = Uuid::new_v4().to_string();

        // Add request ID to headers
        let header_value = HeaderValue::from_str(&request_id)
            .unwrap_or_else(|_| HeaderValue::from_static("unknown"));
        request.headers_mut().insert("X-Request-ID", header_value);

        let mut service = self.service.clone();
        Box::pin(async move {
            let mut response = service.call(request).await?;

            // Add request ID to response headers
            response.headers_mut().insert(
                "X-Request-ID",
                HeaderValue::from_str(&request_id)
                    .unwrap_or_else(|_| HeaderValue::from_static("unknown")),
            );

            Ok(response)
        })
    }
}
