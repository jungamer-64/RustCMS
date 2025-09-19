use clap::Parser;
use cms_backend::Result;
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
    let args = Args::parse();

    // Initialize common app state (includes database)
    let state = cms_backend::utils::init::init_app_state().await?;

    // If delete_post provided, attempt deletion and exit
    if let Some(id_str) = args.delete_post {
        let uuid = uuid::Uuid::parse_str(&id_str).map_err(|_| {
            cms_backend::AppError::BadRequest("Invalid UUID provided for --delete-post".to_string())
        })?;

        state.db_admin_delete_post(uuid).await?;

        if args.json {
            println!("{}", json!({"deleted": true, "post_id": uuid }));
        } else {
            println!("Deleted post id {uuid}");
        }
        return Ok(());
    }

    // Count users
    let users_count: i64 = state.db_admin_users_count().await?;

    // Check for admin user
    let admin_user = state.db_admin_find_admin_user().await?;

    // Count posts
    let posts_count: i64 = state.db_admin_posts_count().await?;

    // Recent posts via query
    let recent = state.db_admin_list_recent_posts(args.limit).await?;

    if args.json {
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
        println!("Users count: {users_count}");
        match admin_user {
            Some(u) => println!("Found admin user: {} <{}>", u.username, u.email),
            None => println!("No admin user found"),
        }
        println!("Posts count: {posts_count}");

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

    Ok(())
}
