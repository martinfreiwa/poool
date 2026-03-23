use sqlx::postgres::PgPoolOptions;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = PgPoolOptions::new()
        .connect("postgres://martin@localhost/poool_community")
        .await
        .unwrap();

    let posts: Result<Vec<poool_backend::community::models::Post>, _> = sqlx::query_as("SELECT * FROM posts ORDER BY created_at DESC")
        .fetch_all(&pool)
        .await;
        
    println!("{:?}", posts.is_ok());
    Ok(())
}
