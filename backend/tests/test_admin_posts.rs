use poool_backend::community::models::Post;
use sqlx::postgres::PgPoolOptions;

#[tokio::test]
async fn test_query_posts() {
    let pool = PgPoolOptions::new()
        .connect("postgres://martin@localhost/poool_community")
        .await
        .unwrap();

    let posts = sqlx::query_as::<_, Post>("SELECT * FROM posts ORDER BY created_at DESC")
        .fetch_all(&pool)
        .await;
    
    match posts {
        Ok(p) => println!("Success: {} posts fetched", p.len()),
        Err(e) => panic!("SQLx returned an error: {:?}", e),
    }
}
