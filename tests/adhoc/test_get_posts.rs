use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = PgPoolOptions::new().connect("postgres://martin@localhost/poool_community").await?;
    
    match sqlx::query_as::<_, poool::community::models::Post>("SELECT * FROM posts ORDER BY created_at DESC").fetch_all(&pool).await {
        Ok(posts) => println!("Posts: {:?}", posts.len()),
        Err(e) => println!("Error: {}", e),
    }

    Ok(())
}
