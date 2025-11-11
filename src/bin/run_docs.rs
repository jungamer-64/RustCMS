use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_env();

    let addr: SocketAddr = "127.0.0.1:3003".parse()?;
    cms_backend::utils::bin_utils::run_docs_server(addr).await?;
    Ok(())
}

fn init_env() {
    if let Err(e) = dotenvy::dotenv() {
        eprintln!("Warning: Could not load .env file: {}", e);
    }
}
