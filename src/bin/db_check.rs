use clap::Parser;
use cms_backend::{config::Config, database::Database, Result};
use diesel::prelude::*;
use serde_json::json;

#[derive(Parser, Debug)]
#[command(about = "DB check tool for CMS (counts, list posts, delete)")]
struct Args {
    /// Output as JSON
    #[arg(long)]
    json: bool,

    /// Limit for listing recent posts
    #[arg(long, default_value_t = 10)]
    limit: i64,

    /// Delete a post by id (UUID)
    #[arg(long)]
    delete_post: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let config = Config::from_env()?;
    let database = Database::new(&config.database).await?;
    let mut conn = database.get_connection()?;

    use cms_backend::database::schema::users::dsl as users_dsl;
    use cms_backend::database::schema::posts::dsl as posts_dsl;
    use cms_backend::models::User;

    // If delete_post provided, attempt deletion and exit
    if let Some(id_str) = args.delete_post {
        match uuid::Uuid::parse_str(&id_str) {
            Ok(uuid) => {
                let deleted = diesel::delete(posts_dsl::posts.filter(posts_dsl::id.eq(uuid)))
                    .execute(&mut conn)
                    .map_err(|e| cms_backend::AppError::Database(e))?;

                if args.json {
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

    // Count users
    let users_count: i64 = users_dsl::users
        .count()
        .get_result(&mut conn)
        .map_err(|e| cms_backend::AppError::Database(e))?;

    // Check for admin user
    let admin_user: Option<User> = users_dsl::users
        .filter(users_dsl::username.eq("admin"))
        .first::<User>(&mut conn)
        .optional()
        .map_err(|e| cms_backend::AppError::Database(e))?;

    // Count posts
    let posts_count: i64 = posts_dsl::posts
        .count()
        .get_result(&mut conn)
        .map_err(|e| cms_backend::AppError::Database(e))?;

    // Recent posts via query
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
        args.limit
    );

    let recent: Vec<PostRow> = diesel::sql_query(q)
        .load(&mut conn)
        .map_err(|e| cms_backend::AppError::Database(e))?;

    if args.json {
        let admin = admin_user.map(|u| json!({"username": u.username, "email": u.email}));
        let posts_json: Vec<_> = recent
            .into_iter()
            .map(|p| json!({
                "id": p.id.to_string(),
                "title": p.title,
                "author_id": p.author_id.to_string(),
                "status": p.status,
                "created_at": p.created_at.to_rfc3339(),
            }))
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
                    p.id,
                    p.title,
                    p.author_id,
                    p.status,
                    p.created_at
                );
            }
        }
    }

    Ok(())
}
