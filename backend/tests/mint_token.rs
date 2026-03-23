use sqlx::postgres::PgPoolOptions;
use poool_backend::auth::service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = PgPoolOptions::new()
        .connect("postgres://martin@localhost/poool")
        .await
        .unwrap();

    let user_id = uuid::Uuid::parse_str("93e0934f-e77b-4596-8bac-c07e8d3df5c1").unwrap();
    let token = service::create_session(&pool, user_id, true, true, None, None).await?;
    println!("SESSION_TOKEN={}", token);
    Ok(())
}
