use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let addr: SocketAddr = "127.0.0.1:3003".parse()?;
    cms_backend::utils::bin_utils::run_docs_server(addr).await?;
    Ok(())
}
