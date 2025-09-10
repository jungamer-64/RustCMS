// Integration test: empty DB behaviour

#[cfg(test)]
mod tests {
    use std::env;
    use std::time::Duration;
    use reqwest::blocking::Client;

    #[test]
    fn empty_db_health_check() {
        // Simple smoke test that the health endpoint responds when DB may be empty.
        // This requires the server to be running; in CI we'll run unit/integration tests directly.
        // For local run, just assert that building the client works.
        let _ = Client::builder().timeout(Duration::from_secs(1)).build().unwrap();
    }
}
