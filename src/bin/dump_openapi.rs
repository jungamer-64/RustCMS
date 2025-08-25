use cms_backend::openapi::ApiDoc;
use utoipa::OpenApi;

fn main() {
    let openapi = ApiDoc::openapi();
    // Pretty-print OpenAPI JSON to stdout for inspection
    let json = serde_json::to_string_pretty(&openapi).expect("failed to serialize openapi");
    println!("{}", json);
}
