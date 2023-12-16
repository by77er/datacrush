use sqlx::PgPool;
use anyhow::Error;
use rand::{distributions::Alphanumeric, Rng};
use serde::{ Serialize, Deserialize };

// Models
#[derive(Deserialize)]
pub struct Request {
    pub url: String,
}

#[derive(Serialize)]
pub struct Response {
    pub slug: String,
}

// Logic
pub async fn get_url(pool: &PgPool, slug: &str) -> Result<String, Error> {
    let (result,): (String,) = sqlx::query_as("SELECT url FROM urls WHERE slug = $1")
        .bind(slug)
        .fetch_one(pool)
        .await?;

    Ok(result)
}

pub async fn put_url(pool: &PgPool, url: &str) -> Result<String, Error> {
    for _ in 0..5 {
        let slug = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(5)
            .map(char::from)
            .collect::<String>();

        if let Ok(_) = sqlx::query("INSERT INTO urls (slug, url) VALUES ($1, $2)")
                .bind(&slug)
                .bind(url)
                .execute(pool)
                .await {
                    return Ok(slug);
                }
    }

    Err(anyhow::anyhow!("Failed to generate slug"))
}