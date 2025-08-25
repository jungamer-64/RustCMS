use cms_backend::openapi::ApiDoc;
use serde_json::Value;
use std::fs;
use utoipa::OpenApi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Print first 1k of the swagger html template
    let html = fs::read_to_string("templates/swagger-ui.html")?;
    println!(
        "---- templates/swagger-ui.html (truncated 1k) ----\n{}\n",
        &html.chars().take(1000).collect::<String>()
    );

    // Generate OpenAPI doc via ApiDoc
    let doc = ApiDoc::openapi();
    let v: Value = serde_json::to_value(&doc)?;

    println!("---- openapi top-level keys ----");
    if let Value::Object(map) = &v {
        for k in map.keys() {
            println!("- {}", k);
        }
    }

    if let Some(components) = v.get("components") {
        if let Some(schemas) = components.get("schemas") {
            println!(
                "\nFound {} schemas",
                schemas.as_object().map(|o| o.len()).unwrap_or(0)
            );
            println!("ApiResponse: {}", schemas.get("ApiResponse").is_some());
            println!(
                "PaginatedResponse: {}",
                schemas.get("PaginatedResponse").is_some()
            );
        }
    }

    Ok(())
}
