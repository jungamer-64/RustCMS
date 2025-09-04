use std::env;

fn main() {
    println!("Environment variables for CMS:");

    // 現在の環境変数を表示
    println!(
        "DATABASE_URL: {}",
        env::var("DATABASE_URL").unwrap_or_else(|_| "Not set".to_string())
    );
    println!(
        "DATABASE_NAME: {}",
        env::var("DATABASE_NAME").unwrap_or_else(|_| "Not set".to_string())
    );
    println!(
        "SERVER_HOST: {}",
        env::var("SERVER_HOST").unwrap_or_else(|_| "Not set".to_string())
    );
    println!(
        "SERVER_PORT: {}",
        env::var("SERVER_PORT").unwrap_or_else(|_| "Not set".to_string())
    );

    // 推奨設定を表示
    println!("\n推奨設定を表示");
    println!("DATABASE_URL=postgres://user:pass@localhost:5432/rust_cms");
    println!("DATABASE_NAME=rust_cms");
    println!("SERVER_HOST=127.0.0.1");
    println!("SERVER_PORT=3001");
    println!("BISCUIT_ROOT_KEY=base64_or_path_to_keydir");

    println!("\n✅ To run the CMS, use:");
    println!("cargo run --bin cms-simple");
}
