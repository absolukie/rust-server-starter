//! Minimal production-style Axum HTTP server starter.
//!
//! Routes:
//!   GET  /health          -> {"status":"ok"}
//!   GET  /api/hello?name= -> {"message":"hello, <name>"}
//!   POST /api/echo        -> echoes the JSON body back
//!   GET  /version         -> {"version":"<crate version>"}
//!   GET  /*               -> static files from ./static/ (index.html at /)

use std::{collections::HashMap, net::SocketAddr, time::Duration};

use axum::{
    extract::{MatchedPath, Query},
    http::{header, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use tower::service_fn;
use tower_http::{
    services::ServeDir,
    trace::TraceLayer,
};
use tracing::{info, info_span};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

/// Per-request middleware: assigns a request id and logs method + path + status + latency.
async fn request_id_mw(req: Request<axum::body::Body>, next: Next) -> Response {
    let request_id = Uuid::new_v4().to_string();
    let method = req.method().clone();
    let path = req.uri().path().to_owned();
    let start = std::time::Instant::now();

    let span = info_span!("request", request_id = %request_id, method = %method, path = %path);
    let _guard = span.enter();

    let mut res = next.run(req).await;
    let elapsed = start.elapsed();
    let status = res.status();

    info!(
        status = %status.as_u16(),
        latency_ms = elapsed.as_secs_f64() * 1000.0,
        "request completed"
    );

    res.headers_mut().insert(
        "x-request-id",
        request_id.parse().expect("uuid is a valid header value"),
    );
    res
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({"status": "ok"})))
}

async fn hello(Query(params): Query<HashMap<String, String>>) -> impl IntoResponse {
    let name = params.get("name").map(String::as_str).unwrap_or("world");
    Json(json!({"message": format!("hello, {name}")}))
}

async fn echo(Json(body): Json<Value>) -> impl IntoResponse {
    (StatusCode::OK, Json(body))
}

async fn version() -> impl IntoResponse {
    Json(json!({"version": env!("CARGO_PKG_VERSION")}))
}

fn app() -> Router {
    // JSON 404 for anything the static dir doesn't have.
    let static_files = ServeDir::new("static")
        .append_index_html_on_directories(true)
        .not_found_service(service_fn(
            |_req: Request<axum::body::Body>| async move {
                Ok::<_, std::convert::Infallible>(
                    (
                        StatusCode::NOT_FOUND,
                        [(header::CONTENT_TYPE, "application/json")],
                        Json(json!({"error": "not found"})),
                    )
                        .into_response(),
                )
            },
        ));

    Router::new()
        .route("/health", get(health))
        .route("/api/hello", get(hello))
        .route("/api/echo", post(echo))
        .route("/version", get(version))
        // Static files last: anything not matched above falls through to ./static/.
        .fallback_service(static_files)
        .layer(middleware::from_fn(request_id_mw))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|req: &Request<_>| {
                    let matched = req
                        .extensions()
                        .get::<MatchedPath>()
                        .map(MatchedPath::as_str)
                        .unwrap_or("<static>");
                    info_span!("http", method = %req.method(), route = matched)
                })
                .on_response(
                    |res: &Response, latency: Duration, _span: &tracing::Span| {
                        info!(status = res.status().as_u16(), latency_ms = latency.as_secs_f64() * 1000.0, "tower response");
                    },
                ),
        )
}

/// Wait for SIGINT or SIGTERM, whichever comes first.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    info!("shutdown signal received");
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rust_server_starter=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind");
    info!(%addr, "listening");

    axum::serve(listener, app())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
}
