use axum::{
    routing::get,
    Router,
    middleware::{self},
};
use dashmap::DashMap;

use governor::{RateLimiter, Quota};
use std::net::SocketAddr;
use std::sync::Arc;

use nonzero_ext::nonzero;

mod app;
mod websocket;
mod telegram_bot;

// ---------------------------------------------------------------------------
// Main Entry Point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    // --- Initialize Rate Limiter ---
    let quota = Quota::per_hour(nonzero!(2u32)).allow_burst(nonzero!(15u32));
    let rate_limiter = Arc::new(RateLimiter::keyed(quota));
    
    // 1. Initialize State
    let state = app::AppState {
        waiting_clients: Arc::new(DashMap::new()),
        rate_limiter
    };

    let bot_task = telegram_bot::spawn_bot_task(&state);

    // 3. Setup Axum Server
    let app = Router::new()
        .route("/new", get(websocket::ws_handler))

        // Apply the middleware, passing the state
        .route_layer(middleware::from_fn_with_state(state.clone(), app::rate_limit_middleware))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("🚀 Server listening on http://{}", addr);

    let server_task = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
    });

    // Run both
    let _ = tokio::join!(bot_task, server_task);
}