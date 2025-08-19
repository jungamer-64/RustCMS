use axum::{
    extract::Request,
    http::{HeaderMap, HeaderValue},
    middleware::Next,
    response::Response,
};
use std::time::Duration;
use tower::{Layer, Service};
use uuid::Uuid;

/// Request ID middleware for distributed tracing
#[derive(Clone)]
pub struct RequestIdLayer;

impl RequestIdLayer {
    pub fn new() -> Self {
        Self
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
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, mut request: Request<B>) -> Self::Future {
        let request_id = Uuid::new_v4().to_string();
        
        // Add request ID to headers
        request.headers_mut().insert(
            "X-Request-ID",
            HeaderValue::from_str(&request_id).unwrap(),
        );

        let mut service = self.service.clone();
        Box::pin(async move {
            let mut response = service.call(request).await?;
            
            // Add request ID to response headers
            response.headers_mut().insert(
                "X-Request-ID",
                HeaderValue::from_str(&request_id).unwrap(),
            );
            
            Ok(response)
        })
    }
}
