// Inert test runner: imports removed because this binary is disabled by default.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // This test runner is intentionally inert in CI/dev builds because
    // constructing a full `AppState` requires environment-specific config
    // (DB, secrets, etc.). Instead, we print a short message and exit so
    // the binary remains in the repo but does not block building.
    eprintln!("run_with_router is disabled by default - AppState must be constructed manually to run this binary.");
    Ok(())
}
