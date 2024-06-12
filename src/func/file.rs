use std::path::Path;

use sqlx::prelude::FromRow;

// Models
#[derive(FromRow, Debug)]
pub struct FileData {
    pub location: String,
    pub size_bytes: i64,
    pub content_type: String,
}

// Logic
fn path_to_string(path: &Path) -> Result<String, anyhow::Error> {
    Ok(path
        .as_os_str()
        .to_str()
        .ok_or(anyhow::anyhow!("Invalid file path"))?
        .to_string())
}

pub async fn get_file_data(pool: &sqlx::PgPool, file: &Path) -> Result<FileData, anyhow::Error> {
    let tx = pool.begin().await?;
    let result: FileData =
        sqlx::query_as("SELECT location, size_bytes, content_type FROM files WHERE location = $1")
            .bind(path_to_string(file)?)
            .fetch_one(pool)
            .await?;
    sqlx::query("UPDATE files SET downloads = downloads + 1 WHERE location = $1")
        .bind(path_to_string(file)?)
        .execute(pool)
        .await?;
    tx.commit().await?;

    Ok(result)
}

pub async fn put_file_data(
    pool: &sqlx::PgPool,
    size_bytes: i64,
    content_type: &str,
    file: &Path,
) -> Result<(), anyhow::Error> {
    let tx = pool.begin().await?;
    sqlx::query("INSERT INTO files (size_bytes, content_type, location) VALUES ($1, $2, $3)")
        .bind(size_bytes)
        .bind(content_type)
        .bind(path_to_string(file)?)
        .execute(pool)
        .await?;
    tx.commit().await?;

    Ok(())
}

pub async fn delete_file_data(pool: &sqlx::PgPool, file: &Path) -> Result<(), anyhow::Error> {
    let tx = pool.begin().await?;
    sqlx::query("DELETE FROM files WHERE location = $1")
        .bind(path_to_string(file)?)
        .execute(pool)
        .await?;
    tx.commit().await?;

    Ok(())
}
