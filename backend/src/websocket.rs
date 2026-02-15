use std::sync::Arc;

use axum::{
    extract::{State, ws::{Message, WebSocket, WebSocketUpgrade}},
    response::IntoResponse,
};
use futures::{lock::Mutex, sink::SinkExt, stream::StreamExt};
use serde_json::Value;
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

#[derive(Serialize)]
struct OneTimeAuthOK {
    r#type: String,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<app::AppState>,
) -> impl IntoResponse {
    ws
    .on_upgrade(move |socket| handle_socket(socket, state))
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

    // Wrap sender in Arc<Mutex<>> to share between tasks
    let sender = Arc::new(Mutex::new(sender));

    // Create a task to forward internal channel messages to the WebSocket
    let sender_clone = sender.clone();
    let mut send_task = tokio::spawn(async move  {
        while let Some(auth_data) = rx.recv().await {
            let json = serde_json::to_string(&auth_data).unwrap();
            let mut s =sender_clone.lock().await;
            if s.send(Message::Text(json.into())).await.is_err() {
                break;
            }
            // Once auth is sent, we can technically close, but usually we let the frontend close
        }
    });
    
    // Keep the socket alive until client disconnects
    let sender_clone = sender.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            let msg_json: Value = serde_json::from_slice(&msg.into_data()).unwrap();
            if msg_json["type"] == "AUTH_OTP" && msg_json["payload"]["otp"] == pin {
                let mut s =sender_clone.lock().await;
                let _ = s.send(Message::Text(serde_json::to_string(&OneTimeAuthOK { r#type: "AUTH_OK".to_string() }).unwrap().into())).await;
                break;
            }

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
