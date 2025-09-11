use clap::{Parser, Subcommand};
use cms_backend::Result;

#[derive(Parser)]
#[command(name = "dev-tools", about = "Development helper tools for CMS")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run DB checks (counts, list posts, delete)
    DbCheck {
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Limit for listing recent posts
        #[arg(long, default_value_t = 10)]
        limit: i64,
        /// Delete a post by id (UUID)
        #[arg(long)]
        delete_post: Option<String>,
    },

    /// Add a sample post to the database
    AddSamplePost {
        /// Title for the sample
        #[arg(long, default_value = "Sample Post")]
        title: String,
    },
    /// Print environment helpful values
    EnvCheck,

    /// Dump OpenAPI JSON to stdout
    DumpOpenapi,

    /// Dump docs and template summary
    DumpDocs,

    /// Run a small docs-only server (blocks)
    RunDocs,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::DbCheck {
            json,
            limit,
            delete_post,
        } => {
            // Delegate to existing db_check implementation where possible
            // We'll reuse logic from src/bin/db_check.rs but inline minimal code here
            let state = cms_backend::utils::init::init_app_state().await?;
            use serde_json::json;

            if let Some(id_str) = delete_post {
                match uuid::Uuid::parse_str(&id_str) {
                    Ok(uuid) => {
                        state.db_admin_delete_post(uuid).await?;

                        if json {
                            println!("{}", json!({"deleted": 1}));
                        } else {
                            println!("Deleted 1 rows for post id {}", uuid);
                        }
                        return Ok(());
                    }
                    Err(_) => {
                        eprintln!("Invalid UUID provided for --delete-post");
                        std::process::exit(1);
                    }
                }
            }

            let users_count: i64 = state.db_admin_users_count().await?;

            let admin_user = state.db_admin_find_admin_user().await?;

            let posts_count: i64 = state.db_admin_posts_count().await?;

            let recent = state.db_admin_list_recent_posts(limit).await?;

            if json {
                let admin = admin_user.map(|u| json!({"username": u.username, "email": u.email}));
                let posts_json: Vec<_> = recent
                    .into_iter()
                    .map(|p| {
                        json!({
                            "id": p.id,
                            "title": p.title,
                            "author_id": p.author_id,
                            "status": p.status,
                            "created_at": p.created_at,
                        })
                    })
                    .collect();

                println!(
                    "{}",
                    json!({"users_count": users_count, "admin": admin, "posts_count": posts_count, "recent_posts": posts_json})
                );
            } else {
                println!("Users count: {}", users_count);
                match admin_user {
                    Some(u) => println!("Found admin user: {} <{}>", u.username, u.email),
                    None => println!("No admin user found"),
                }
                println!("Posts count: {}", posts_count);

                if recent.is_empty() {
                    println!("No posts found.");
                } else {
                    println!("Recent posts:");
                    for p in recent {
                        println!(
                            "- {} | {} | author={} | {} | {}",
                            p.id, p.title, p.author_id, p.status, p.created_at
                        );
                    }
                }
            }
        }

        Commands::AddSamplePost { title } => {
            let state = cms_backend::utils::init::init_app_state().await?;
            let sample_post = cms_backend::models::post::CreatePostRequest {
                title: title.clone(),
                content: "This is a sample post added by dev-tools".to_string(),
                excerpt: Some("Dev tools sample post".to_string()),
                slug: None,
                published: Some(true),
                tags: Some(vec!["dev".to_string(), "sample".to_string()]),
                category: None,
                featured_image: None,
                meta_title: None,
                meta_description: None,
                published_at: None,
                status: Some(cms_backend::models::PostStatus::Published),
            };

            state.db_create_post(sample_post).await?;
            println!("Sample post created");
        }
        Commands::EnvCheck => {
            cms_backend::utils::bin_utils::print_env_summary();
        }

        Commands::DumpOpenapi => {
            // Delegate to ApiDoc openapi and pretty print
            use cms_backend::openapi::ApiDoc;
            use utoipa::OpenApi;
            let openapi = ApiDoc::openapi();
            let json = serde_json::to_string_pretty(&openapi).expect("failed to serialize openapi");
            println!("{}", json);
        }

        Commands::DumpDocs => {
            // Inline dump_docs.rs behavior
            use cms_backend::openapi::ApiDoc;
            use serde_json::Value;
            use std::fs;
            use utoipa::OpenApi;

            if let Ok(html) = fs::read_to_string("templates/swagger-ui.html") {
                println!(
                    "---- templates/swagger-ui.html (truncated 1k) ----\n{}\n",
                    &html.chars().take(1000).collect::<String>()
                );
            }

            let doc = ApiDoc::openapi();
            let v: Value = serde_json::to_value(&doc).unwrap_or(Value::Null);

            println!("---- openapi top-level keys ----");
            if let Value::Object(map) = &v {
                for k in map.keys() {
                    println!("- {}", k);
                }
            }

            if let Some(components) = v.get("components")
                && let Some(schemas) = components.get("schemas")
            {
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

        Commands::RunDocs => {
            use std::net::SocketAddr;
            let addr: SocketAddr = "127.0.0.1:3003"
                .parse()
                .map_err(|e| cms_backend::AppError::Config(format!("Addr parse error: {}", e)))?;
            cms_backend::utils::bin_utils::run_docs_server(addr).await?;
            return Ok(());
        }
    }

    Ok(())
}
