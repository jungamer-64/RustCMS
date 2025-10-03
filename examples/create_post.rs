//! Create Post Example
//!
//! Conceptual example showing post creation in `RustCMS`.

fn main() {
    println!("=== CMS Backend Create Post Example ===\n");

    println!("1. API Request");
    println!("   POST /api/posts");
    println!("   Content-Type: application/json");
    println!("   Authorization: Bearer <your-token>\n");

    println!("   Request: {{");
    println!("     \"title\": \"Getting Started with RustCMS\",");
    println!("     \"slug\": \"getting-started-rustcms\",");
    println!("     \"content\": \"# Welcome to RustCMS...\",");
    println!("     \"status\": \"published\"");
    println!("   }}\n");

    println!("2. Expected Response");
    println!("   HTTP/1.1 201 Created\n");

    println!("For actual implementation, see tests/content.rs");
}
