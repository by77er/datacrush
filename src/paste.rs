use sqlx::PgPool;
use anyhow::Error;
use rand::{distributions::Alphanumeric, Rng};
use serde::{ Serialize, Deserialize };

// Models
#[derive(Deserialize)]
pub struct Request {
    pub data: String,
}

#[derive(Serialize)]
pub struct Response {
    pub slug: String,
}

// Logic
pub async fn get_paste(pool: &PgPool, slug: &str) -> Result<String, Error> {
    let (result,): (String,) = sqlx::query_as("SELECT data FROM pastes WHERE slug = $1")
        .bind(slug)
        .fetch_one(pool)
        .await?;

    Ok(result)
}

pub async fn put_paste(pool: &PgPool, data: &str) -> Result<String, Error> {
    for _ in 0..5 {
        let slug = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(5)
            .map(char::from)
            .collect::<String>();

        if let Ok(_) = sqlx::query("INSERT INTO pastes (slug, data) VALUES ($1, $2)")
                .bind(&slug)
                .bind(data)
                .execute(pool)
                .await {
                    return Ok(slug);
                }
    }

    Err(anyhow::anyhow!("Failed to generate slug"))
}