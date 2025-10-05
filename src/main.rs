
use axum::{routing::get, Router};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

// Make our modules available
mod error;
mod pagination;
mod routes;

// Define a simple AppState
#[derive(Clone)]
pub struct AppState;

#[tokio::main]
async fn main() {
    // Define the OpenAPI documentation.
    #[derive(OpenApi)]
    #[openapi(
        paths(
            routes::posts::get_posts,
        ),
        components(
            schemas(routes::posts::Post)
        ),
        // The `Pagination` struct needs to be listed here so `utoipa`
        // knows about its parameters.
        params(
            pagination::Pagination
        ),
        tags(
            (name = "posts", description = "Post management API")
        )
    )]
    struct ApiDoc;

    let app_state = AppState;

    // Build the Axum router.
    let app = Router::new()
        // Merge the Swagger UI router into our main router.
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/posts", get(routes::posts::get_posts))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    println!("Swagger UI available at /swagger-ui");
    axum::serve(listener, app).await.unwrap();
}
