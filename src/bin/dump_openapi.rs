//! Dump the `OpenAPI` spec as JSON.
//! Usage:
//!   cargo run --bin `dump_openapi` > openapi.json
//!   OPENAPI_OUT=./openapi-full.json cargo run --features "auth database search cache" --bin `dump_openapi`
//!   cargo run --bin `dump_openapi` ./openapi.json

use cms_backend::openapi::ApiDoc;
use std::{env, fs::File, io::Write, path::Path};
use utoipa::OpenApi;

/// Safe argument accessor returning first positional (non-flag) argument if present.
/// Skips argv[0] and any starting with '-'. Avoids assuming UTF-8 for os strings implicitly.
fn first_positional_arg() -> Option<String> {
    std::env::args().skip(1).find(|a| !a.starts_with('-'))
}

#[cfg(test)]
mod arg_tests {
    use super::*;

    // NOTE: We can't easily manipulate process-wide args reliably without spawning a subprocess.
    // This simple smoke test just ensures function executes without panicking given current args.
    #[test]
    fn first_positional_does_not_panic() {
        let _ = first_positional_arg();
    }
}

fn main() {
    let doc = ApiDoc::openapi();
    let json = serde_json::to_string_pretty(&doc).expect("failed to serialize openapi");

    // Determine output path from env or first non-flag arg.
    let mut out: Option<String> = env::var("OPENAPI_OUT").ok();
    if out.is_none() {
        if let Some(arg1) = first_positional_arg() {
            out = Some(arg1);
        }
    }

    if let Some(path) = out {
        let p = Path::new(&path);
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let mut f = File::create(p).expect("create file");
        f.write_all(json.as_bytes()).expect("write file");
        eprintln!("OpenAPI spec written to {path}");
    } else {
        println!("{json}");
    }
}
