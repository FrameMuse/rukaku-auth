use std::{sync::Arc};
use std::net::{SocketAddr, IpAddr};


use serde::Serialize;
use dashmap::DashMap;
use tokio::sync::mpsc;

use governor::{RateLimiter, state::keyed::DefaultKeyedStateStore, clock::DefaultClock};

use axum::{
    extract::{ConnectInfo, State, Request},
    middleware::{Next},
    response::{Response},
    http::StatusCode,
};

// ---------------------------------------------------------------------------
// Types & State
// ---------------------------------------------------------------------------

// The message we send back to the frontend via WebSocket
#[derive(Debug, Serialize, Clone)]
pub struct AuthSuccess {
    pub status: String,
    pub user_id: u64,
    pub username: Option<String>,
    pub mock_jwt: String, // In production, this would be a real JWT
}

pub struct ClientValue {
    pub client: mpsc::Sender<AuthSuccess>,
    pub pin: u32,
    // pub id: str
}

// We map a UUID (String) to a Sender. 
// When the bot gets a message, it finds the sender and pipes the data to the WS.
type ClientMap = DashMap<String, ClientValue>;
// 1. Define a Type Alias for cleaner code
// We key by IpAddr using the default thread-safe state store
type ClientRateLimiter = RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>;

#[derive(Clone)]
pub struct AppState {
    // Shared map between Axum (WS) and Teloxide (Bot)
    pub waiting_clients: Arc<ClientMap>,
    pub rate_limiter: Arc<ClientRateLimiter>,
}


// 3. Custom Middleware Function
// This extracts the IP, checks the limiter, and either errors or calls next()
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Check the limiter for this specific IP
    match state.rate_limiter.check_key(&addr.ip()) {
        Ok(_) => Ok(next.run(req).await),
        Err(_) => Err(StatusCode::TOO_MANY_REQUESTS),
    }
}