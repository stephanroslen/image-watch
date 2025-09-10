mod axum_util;
mod config;
mod error;
mod file_change_data;
mod file_change_tracker_actor;
mod file_tracker_actor;
mod serve_frontend;
mod tokio_util;
mod web_socket_actor;

use axum::{
    Router,
    extract::{State, ws::WebSocketUpgrade},
    middleware,
    response::IntoResponse,
    routing::get,
};
use error::Result;
use file_change_tracker_actor::FileChangeTrackerActorHandler;
use file_tracker_actor::FileTrackerActorHandler;
use serve_frontend::serve_frontend;
use std::{
    panic, process,
    sync::{Arc, Weak},
};
use tokio::task::JoinSet;
use tower_http::{compression::CompressionLayer, services::fs::ServeDir, trace, trace::TraceLayer};
use tracing::{Level, instrument};
use tracing_subscriber::{EnvFilter, filter::LevelFilter};

#[derive(Debug)]
struct WsState {
    file_tracker_actor_handler: Weak<FileTrackerActorHandler>,
}

#[instrument]
async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<WsState>>) -> impl IntoResponse {
    ws.on_upgrade(async move |socket| {
        tracing::debug!("on_upgrade");
        let file_tracker_actor_handler = state.file_tracker_actor_handler.upgrade();
        if let Some(file_tracker_actor_handler) = file_tracker_actor_handler {
            tracing::debug!("got file tracker actor handler");
            file_tracker_actor_handler
                .add_web_socket(socket)
                .await
                .expect("Expected to be able to add web socket");
        }
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut join_set = JoinSet::new();
    let result = image_watch(&mut join_set).await;

    if let Err(e) = &result {
        tracing::error!("Error: {}", e);
    }

    join_set.join_all().await;

    Ok(())
}

async fn image_watch(join_set: &mut JoinSet<()>) -> Result<()> {
    panic::set_hook(Box::new(|info| {
        tracing::error!("Task panic: {}", info);
        process::exit(1);
    }));

    let dotenvy_result = dotenvy::dotenv();

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    tracing_subscriber::fmt().with_env_filter(filter).init();

    let _ = dotenvy_result.inspect_err(|e| tracing::warn!("Couldn't load .env: {}", e));

    let config = config::Config::from_env()?;

    let listener = tokio::net::TcpListener::bind(&config.listen_address).await?;

    let file_tracker_actor_handler = FileTrackerActorHandler::new(
        join_set,
        config.file_add_chunk_size,
        config.file_add_chunk_delay,
    )?;

    let serve_dir_service = ServeDir::new(&config.serve_dir).fallback(get(axum_util::not_found));

    let mut app = Router::new()
        .route("/", get(serve_frontend))
        .route("/{*path}", get(serve_frontend))
        .route("/ws", get(ws_handler))
        .nest_service("/data", serve_dir_service)
        .fallback(get(axum_util::not_found))
        .with_state(Arc::new(WsState {
            file_tracker_actor_handler: Arc::downgrade(&file_tracker_actor_handler),
        }));

    if !config.auth_disabled {
        app = app.layer(middleware::from_fn(move |req, next| {
            axum_util::basic_auth(
                req,
                next,
                config.auth_user.clone(),
                config.auth_pass_argon2.clone(),
            )
        }));
    }

    // tracing must happen after adding auth layer so failed auth gets logged
    app = app
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(
            CompressionLayer::new()
                .br(true)
                .deflate(true)
                .gzip(true)
                .zstd(true),
        );

    let _file_change_tracker_actor_handler = FileChangeTrackerActorHandler::new(
        join_set,
        file_tracker_actor_handler,
        config.rescrape_interval,
        config.serve_dir.clone(),
        config.file_extensions,
    )?;

    tracing::info!("Starting server");

    axum::serve(listener, app)
        .with_graceful_shutdown(tokio_util::shutdown_signal())
        .await?;

    tracing::info!("Server stopped");

    Ok(())
}
