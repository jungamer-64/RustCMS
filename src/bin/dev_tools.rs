use clap::{Parser, Subcommand};
use cms_backend::Result;
// Diesel helpers: prelude brings QueryDsl, RunQueryDsl, ExpressionMethods, etc.
use diesel::prelude::*;
// Derive macro for QueryableByName used below
use diesel::QueryableByName;

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
        Commands::DbCheck { json, limit, delete_post } => {
            // Delegate to existing db_check implementation where possible
            // We'll reuse logic from src/bin/db_check.rs but inline minimal code here
            let state = cms_backend::utils::init::init_app_state().await?;
            let mut conn = state.get_conn()?;

            use cms_backend::database::schema::posts::dsl as posts_dsl;
            use cms_backend::database::schema::users::dsl as users_dsl;
            use cms_backend::models::User;
            use serde_json::json;

            if let Some(id_str) = delete_post {
                match uuid::Uuid::parse_str(&id_str) {
                    Ok(uuid) => {
                        let deleted = diesel::delete(posts_dsl::posts.filter(posts_dsl::id.eq(uuid)))
                            .execute(&mut conn)
                            .map_err(|e| cms_backend::AppError::Database(e))?;

                        if json {
                            println!("{}", json!({"deleted": deleted}));
                        } else {
                            println!("Deleted {} rows for post id {}", deleted, uuid);
                        }
                        return Ok(());
                    }
                    Err(_) => {
                        eprintln!("Invalid UUID provided for --delete-post");
                        std::process::exit(1);
                    }
                }
            }

            let users_count: i64 = users_dsl::users
                .count()
                .get_result(&mut conn)
                .map_err(|e| cms_backend::AppError::Database(e))?;

            let admin_user: Option<User> = users_dsl::users
                .filter(users_dsl::username.eq("admin"))
                .first::<User>(&mut conn)
                .optional()
                .map_err(|e| cms_backend::AppError::Database(e))?;

            let posts_count: i64 = posts_dsl::posts
                .count()
                .get_result(&mut conn)
                .map_err(|e| cms_backend::AppError::Database(e))?;

            #[derive(QueryableByName, Debug)]
            struct PostRow {
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                id: uuid::Uuid,
                #[diesel(sql_type = diesel::sql_types::Text)]
                title: String,
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                author_id: uuid::Uuid,
                #[diesel(sql_type = diesel::sql_types::Text)]
                status: String,
                #[diesel(sql_type = diesel::sql_types::Timestamptz)]
                created_at: chrono::DateTime<chrono::Utc>,
            }

            let q = format!(
                "SELECT id, title, author_id, status, created_at FROM posts ORDER BY created_at DESC LIMIT {}",
                limit
            );

            let recent: Vec<PostRow> = diesel::sql_query(q)
                .load(&mut conn)
                .map_err(|e| cms_backend::AppError::Database(e))?;

            if json {
                let admin = admin_user.map(|u| json!({"username": u.username, "email": u.email}));
                let posts_json: Vec<_> = recent
                    .into_iter()
                    .map(|p| {
                        json!({
                            "id": p.id.to_string(),
                            "title": p.title,
                            "author_id": p.author_id.to_string(),
                            "status": p.status,
                            "created_at": p.created_at.to_rfc3339(),
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

            state.database.create_post(sample_post).await?;
            println!("Sample post created");
        }
        Commands::EnvCheck => {
            // Inline env-check.rs behavior
            println!("Environment variables for CMS:");
            use std::env;

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

            println!("\n推奨設定を表示");
            println!("DATABASE_URL=postgres://user:pass@localhost:5432/rust_cms");
            println!("DATABASE_NAME=rust_cms");
            println!("SERVER_HOST=127.0.0.1");
            println!("SERVER_PORT=3001");
            println!("JWT_SECRET=your_secure_jwt_secret_here_at_least_32_characters");

            println!("\n✅ To run the CMS, use:");
            println!("cargo run --bin cms-simple");
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

            if let Some(components) = v.get("components") {
                if let Some(schemas) = components.get("schemas") {
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
        }

        Commands::RunDocs => {
            // Start lightweight docs server (blocks)
            use axum::{routing::get, Router};
            use cms_backend::handlers;
            use std::net::SocketAddr;

            let app = Router::new()
                .route("/api/docs", get(handlers::docs_ui))
                .route("/api/docs/openapi.json", get(handlers::openapi_json));

            let addr: SocketAddr = "127.0.0.1:3003"
                .parse()
                .map_err(|e| cms_backend::AppError::Config(format!("Addr parse error: {}", e)))?;
            println!(
                "Docs server running on http://{} (endpoints: /api/docs, /api/docs/openapi.json)",
                addr
            );

            let listener = tokio::net::TcpListener::bind(addr).await?;
            axum::serve(listener, app).await?;
            return Ok(());
        }
    }

    Ok(())
}
