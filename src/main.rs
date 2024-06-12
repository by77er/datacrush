use axum::{
    error_handling::HandleErrorLayer,
    extract::{Json, Path, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{delete, get, post, put},
    Router,
};
use futures::TryStreamExt;
use std::{
    io,
    path::{Path as StdPath, PathBuf},
};
use tower::{BoxError, ServiceBuilder};
use tower_http::{limit::RequestBodyLimitLayer, normalize_path::NormalizePathLayer};
use tower_sessions::{
    cookie::time::Duration, Expiry, Session, SessionManagerLayer,
};

mod func;
use func::{file, paste, redirect};
mod config;
use config::{get_session_store, get_appstate, AppState};

#[tokio::main]
async fn main() {
    let state = get_appstate().await.unwrap();
    let session_store = get_session_store(&state.pool).await.unwrap();

    let session_service = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|_: BoxError| async {
            StatusCode::BAD_REQUEST
        }))
        .layer(
            SessionManagerLayer::new(session_store)
                .with_name("datacrush.session")
                .with_domain("localhost".to_string())
                .with_secure(false)
                .with_expiry(Expiry::OnInactivity(Duration::days(1))),
        );

    let unauthenticated = Router::new()
        .route("/", get(handle))
        .route("/", post(handle))
        .route("/f/*file", get(get_file))
        .route("/p/:slug", get(get_paste))
        .route("/r/:slug", get(get_redirect))
        .layer(RequestBodyLimitLayer::new(1024 * 1024 * 2));

    let authenticated = Router::new()
        .route("/f/*file", put(put_file))
        .route("/f/*file", delete(delete_file))
        .route("/p", post(put_paste))
        .route("/r", post(put_redirect))
        .layer(RequestBodyLimitLayer::new(1024 * 1024 * 1024 * 10));

    let app = Router::new()
        .nest("/", unauthenticated)
        .nest("/", authenticated)
        .fallback(get(not_found))
        .with_state(state)
        .layer(NormalizePathLayer::trim_trailing_slash())
        .layer(session_service);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn handle(session: Session) -> String {
    let ctr = session
        .get::<u32>("ctr")
        .unwrap_or(Some(0u32))
        .unwrap_or_default();
    session.insert("ctr", ctr + 1).unwrap();
    format!("Hello, World! {}", ctr)
}

async fn get_file(State(state): State<AppState>, Path(file): Path<PathBuf>) -> Response {
    if !valid_path(&file) {
        return (StatusCode::BAD_REQUEST, "Invalid path").into_response();
    }
    if let Ok(file_data) = file::get_file_data(&state.pool, &file).await {
        if let Ok(stream) = state.filestore.get_file(&file).await {
            let mut res = axum::body::Body::from_stream(stream).into_response();
            let content_type = HeaderValue::from_str(&file_data.content_type)
                .unwrap_or(HeaderValue::from_static("application/octet-stream"));
            res.headers_mut().insert("Content-Type", content_type);
            if let Ok(size) = HeaderValue::from_str(&file_data.size_bytes.to_string()) {
                res.headers_mut().insert("Content-Length", size);
            }
            res
        } else {
            (StatusCode::INTERNAL_SERVER_ERROR, "Error retrieving file").into_response()
        }
    } else {
        (StatusCode::NOT_FOUND, "Not Found").into_response()
    }
}

async fn put_file(
    headers: HeaderMap,
    State(mut state): State<AppState>,
    Path(file): Path<PathBuf>,
    body: axum::body::Body,
) -> Response {
    if !valid_path(&file) {
        return (StatusCode::BAD_REQUEST, "Invalid path").into_response();
    }
    let stream = body.into_data_stream();
    if let Ok(size) = state
        .filestore
        .create_file(
            &file,
            stream.map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "Broken pipe")),
        )
        .await
    {
        let content_type = headers
            .get("Content-Type")
            .map(|v| v.to_str().unwrap_or("application/octet-stream"))
            .unwrap_or("application/octet-stream");

        match file::put_file_data(&state.pool, size as i64, content_type, &file).await {
            Ok(_) => (
                StatusCode::CREATED,
                format!("Created ({} bytes)", size as i64),
            )
                .into_response(),
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create file metadata",
            )
                .into_response(),
        }
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, "Couldn't create file").into_response()
    }
}

async fn delete_file(State(mut state): State<AppState>, Path(file): Path<PathBuf>) -> Response {
    if !valid_path(&file) {
        return (StatusCode::BAD_REQUEST, "Invalid path").into_response();
    }
    if state.filestore.delete_file(&file).await.is_ok() {
        match file::delete_file_data(&state.pool, &file).await {
            Ok(_) => (StatusCode::OK, "OK").into_response(),
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to delete file metadata",
            )
                .into_response(),
        }
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
    redirect::get_url(&state.pool, &slug)
        .await
        .map(|url| Redirect::to(&url).into_response())
        .unwrap_or((StatusCode::NOT_FOUND, "Not Found").into_response())
}

async fn put_redirect(
    State(state): State<AppState>,
    Json(payload): Json<redirect::Request>,
) -> Response {
    redirect::put_url(&state.pool, &payload.url)
        .await
        .map(|slug| axum::Json(redirect::Response { slug }).into_response())
        .unwrap_or(
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Couldn't allocate a slug",
            )
                .into_response(),
        )
}

// stop directory traversal
fn valid_path(path: &StdPath) -> bool {
    path.is_relative()
        && path
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
}

async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "Not Found")
}
