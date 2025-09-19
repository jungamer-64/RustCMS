use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    cms_backend::utils::init::init_env();

    let addr: SocketAddr = "127.0.0.1:3003".parse()?;
    cms_backend::utils::bin_utils::run_docs_server(addr).await?;
    Ok(())
}
