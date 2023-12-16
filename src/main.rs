use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use sqlx::{
    postgres::{PgPoolOptions, Postgres},
    Pool,
};

mod file;
mod paste;
mod redirect;

#[derive(Clone)]
struct AppState {
    pub pool: Pool<Postgres>,
}

#[tokio::main]
async fn main() {
    let state = AppState {
        pool: PgPoolOptions::new()
            .max_connections(5)
            .connect("postgres://postgres:postgres@localhost/datacrush")
            .await
            .unwrap(),
    };

    sqlx::migrate!("./migrations")
        .run(&state.pool)
        .await.unwrap();

    let unauthenticated = Router::new()
        .route("/", get(handle))
        .route("/", post(handle))
        .route("/f/:file", get(get_file))
        .route("/p/:paste", get(get_paste))
        .route("/r/:slug", get(get_redirect));

    let authenticated = Router::new()
        .route("/upload", post(handle))
        .route("/paste", post(handle))
        .route("/redirect", post(put_redirect));

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

async fn get_file(State(state): State<AppState>, Path(file): Path<String>) -> String {
    // let (result,): (String,) = sqlx::query_as("SELECT $1")
    //     .bind(file)
    //     .fetch_one(&state.pool)
    //     .await
    //     .unwrap();

    // format!("Got file \"{}\".", result)

    // redirect::put_url(&state.pool, "https://google.com").await.unwrap();
    "Hello, World!".to_string()
}

async fn get_paste(State(state): State<AppState>, Path(paste): Path<String>) -> String {
    let (result,): (String,) = sqlx::query_as("SELECT $1")
        .bind(paste)
        .fetch_one(&state.pool)
        .await
        .unwrap();

    format!("Got file \"{}\".", result)
}

async fn get_redirect(State(state): State<AppState>, Path(slug): Path<String>) -> Response {
    if let Ok(url) = redirect::get_url(&state.pool, &slug).await {
        Redirect::to(&url).into_response()
    } else {
        (StatusCode::NOT_FOUND, "Not Found").into_response()
    }
}

async fn put_redirect(State(state): State<AppState>, Json(payload): Json<redirect::Request>) -> Response {
    if let Ok(slug) = redirect::put_url(&state.pool, &payload.url).await {
        axum::Json(redirect::Response {
            slug
        }).into_response()
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, "Couldn't allocate a slug").into_response()
    }
}

async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "Not Found")
}
