//! Dump the OpenAPI spec as JSON.
//! Usage:
//!   cargo run --bin dump_openapi > openapi.json
//!   OPENAPI_OUT=./openapi-full.json cargo run --features "auth database search cache" --bin dump_openapi
//!   cargo run --bin dump_openapi ./openapi.json

use cms_backend::openapi::ApiDoc;
use utoipa::OpenApi;
use std::{env, fs::File, io::Write, path::Path};

fn main() {
    let doc = ApiDoc::openapi();
    let json = serde_json::to_string_pretty(&doc).expect("failed to serialize openapi");

    // Determine output path from env or first non-flag arg.
    let mut out: Option<String> = env::var("OPENAPI_OUT").ok();
    if out.is_none() {
        if let Some(arg1) = env::args().nth(1) {
            if !arg1.starts_with('-') { out = Some(arg1); }
        }
    }

    if let Some(path) = out {
        let p = Path::new(&path);
        if let Some(parent) = p.parent() { let _ = std::fs::create_dir_all(parent); }
    let mut f = File::create(p).expect("create file");
        f.write_all(json.as_bytes()).expect("write file");
        eprintln!("OpenAPI spec written to {path}");
    } else {
        println!("{json}");
    }
}
