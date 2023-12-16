use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::{delete, get, post, put},
    Router,
};
use filestore::FileStore;
use futures::TryStreamExt;
use sqlx::{
    postgres::{PgPoolOptions, Postgres},
    Pool,
};
use std::io;

mod file;
mod paste;
mod redirect;

#[derive(Clone)]
struct AppState {
    pub pool: Pool<Postgres>,
    pub filestore: FileStore,
}

#[tokio::main]
async fn main() {
    let state = AppState {
        pool: PgPoolOptions::new()
            .max_connections(5)
            .connect("postgres://postgres:postgres@localhost/datacrush")
            .await
            .unwrap(),
        filestore: FileStore::new("objects".to_string()),
    };

    sqlx::migrate!("./migrations")
        .run(&state.pool)
        .await
        .unwrap();

    let unauthenticated = Router::new()
        .route("/", get(handle))
        .route("/", post(handle))
        .route("/f/*file", get(get_file))
        .route("/p/:slug", get(get_paste))
        .route("/r/:slug", get(get_redirect));

    let authenticated = Router::new()
        .route("/f/*file", put(put_file))
        .route("/f/*file", delete(delete_file))
        .route("/p", post(put_paste))
        .route("/r", post(put_redirect));

    let app = Router::new()
        .nest("/", unauthenticated)
        .nest("/", authenticated)
        .fallback(get(not_found))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn handle() -> &'static str {
    "Hello, World!"
}

async fn get_file(State(state): State<AppState>, Path(file): Path<String>) -> Response {
    if let Ok(stream) = state.filestore.get_file(&file).await {
        axum::body::Body::from_stream(stream).into_response()
    } else {
        (StatusCode::NOT_FOUND, "Not Found").into_response()
    }
}

async fn put_file(
    State(mut state): State<AppState>,
    Path(file): Path<String>,
    body: axum::body::Body,
) -> Response {
    let stream = body.into_data_stream();
    if let Ok(size) = state
        .filestore
        .create_file(
            &file,
            stream.map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "Broken pipe")),
        )
        .await
    {
        (StatusCode::CREATED, format!("Created ({} bytes)", size)).into_response()
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, "Couldn't create file").into_response()
    }
}

async fn delete_file(State(mut state): State<AppState>, Path(file): Path<String>) -> Response {
    if let Ok(_) = state.filestore.delete_file(&file).await {
        (StatusCode::OK, "OK").into_response()
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, "Couldn't delete file").into_response()
    }
}

async fn get_paste(State(state): State<AppState>, Path(slug): Path<String>) -> Response {
    if let Ok(paste) = paste::get_paste(&state.pool, &slug).await {
        paste.into_response()
    } else {
        (StatusCode::NOT_FOUND, "Not Found").into_response()
    }
}

async fn put_paste(State(state): State<AppState>, Json(payload): Json<paste::Request>) -> Response {
    if let Ok(slug) = paste::put_paste(&state.pool, &payload.data).await {
        axum::Json(paste::Response { slug }).into_response()
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Couldn't allocate a slug",
        )
            .into_response()
    }
}

async fn get_redirect(State(state): State<AppState>, Path(slug): Path<String>) -> Response {
    if let Ok(url) = redirect::get_url(&state.pool, &slug).await {
        Redirect::to(&url).into_response()
    } else {
        (StatusCode::NOT_FOUND, "Not Found").into_response()
    }
}

async fn put_redirect(
    State(state): State<AppState>,
    Json(payload): Json<redirect::Request>,
) -> Response {
    if let Ok(slug) = redirect::put_url(&state.pool, &payload.url).await {
        axum::Json(redirect::Response { slug }).into_response()
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Couldn't allocate a slug",
        )
            .into_response()
    }
}

async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "Not Found")
}
