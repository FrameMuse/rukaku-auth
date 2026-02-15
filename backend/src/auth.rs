use axum::{
    extract::{Path, State, ws::{Message, WebSocket, WebSocketUpgrade}},
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use tokio::sync::mpsc;
use uuid::Uuid;
use serde::Serialize;
use rand::RngExt;

use crate::app;

// ---------------------------------------------------------------------------
// Axum WebSocket Logic
// ---------------------------------------------------------------------------


#[derive(Serialize)]
struct OneTimeAuthInit {
    r#type: String,
    token: String,
}


pub async fn start_handler(
    State(state): State<app::AppState>,
) -> impl IntoResponse {

}

pub async fn confirm_handler(
    Path(pin): Path<String>,
    State(state): State<app::AppState>,
) -> impl IntoResponse {

}

async fn handle_socket(socket: WebSocket, state: app::AppState) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::channel::<app::AuthSuccess>(1);
    
    let pin = rand::rng().random_range(100000..1000000);
    let uuid = Uuid::new_v4().to_string();
    
    // Register this client in the global map
    state.waiting_clients.insert(uuid.clone(), app::ClientValue { client: tx, pin });
    let init_payload = OneTimeAuthInit {
        r#type: "ONE_TIME_AUTH".to_string(),
        token: uuid.clone()
    };
    let _ = sender.send(Message::Text(serde_json::to_string(&init_payload).unwrap().into())).await;
    println!("[WebSocket] Connected: Waiting for auth on UUID: {}", uuid);

    // Create a task to forward internal channel messages to the WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Some(auth_data) = rx.recv().await {
            let json = serde_json::to_string(&auth_data).unwrap();
            if sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
            // Once auth is sent, we can technically close, but usually we let the frontend close
        }
    });

    // Keep the socket alive until client disconnects
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(_msg)) = receiver.next().await {
            // We generally don't expect messages from the client in this flow, 
            // but we need to keep the loop running to detect disconnects.
        }
    });

    // Define the timeout future
    let timeout_duration = tokio::time::sleep(tokio::time::Duration::from_secs(60));

    // Wait for either to finish
    tokio::select! {
        _ = (&mut send_task) => {},
        _ = (&mut recv_task) => {},
        _ = timeout_duration => { println!("[WebSocket] Connection timed out") },
    }

    send_task.abort();
    recv_task.abort();

    // Cleanup
    state.waiting_clients.remove(&uuid);
    println!("[WebSocket] Disconnected: {}", &uuid);
}
