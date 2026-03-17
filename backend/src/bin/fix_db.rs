use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let url = "postgres://postgres:P000lPassw0rd!@127.0.0.1:5433/poool?sslmode=disable";
    let pool = PgPoolOptions::new().max_connections(5).connect(url).await?;

    sqlx::query("ALTER TABLE user_sessions ADD COLUMN IF NOT EXISTS is_2fa_verified BOOLEAN NOT NULL DEFAULT FALSE;")
        .execute(&pool)
        .await?;

    println!("Successfully added is_2fa_verified column");
    Ok(())
}
