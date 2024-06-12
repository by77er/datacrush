use anyhow::Result;
use filestore::FileStore;
use sqlx::PgPool;
use tower_sessions::{PostgresStore, ExpiredDeletion};

// TODO: Read from config file

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub filestore: FileStore,
}

pub async fn get_appstate() -> Result<AppState> {
    let pool = PgPool::connect("postgres://postgres:postgres@localhost/datacrush")
        .await
        .unwrap();
    let filestore = FileStore::new("objects".to_string());

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .unwrap();

    Ok(AppState {
        pool,
        filestore,
    })
}

pub async fn get_session_store(pool: &PgPool) -> Result<PostgresStore> {
    let session_store = PostgresStore::new(pool.clone());
    session_store.migrate().await?;
    session_store.delete_expired().await?;

    Ok(session_store)
}