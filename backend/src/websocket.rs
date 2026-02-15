use axum::{
    extract::{State, ws::{Message, WebSocket, WebSocketUpgrade}},
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde_json::Value;
use tokio::sync::mpsc;
use uuid::Uuid;
use serde::Serialize;
use rand::RngExt;
use std::pin::pin;

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

    // Channel to signal auth success with OK message
    let (auth_done_tx, mut auth_done_rx) = mpsc::channel::<OneTimeAuthOK>(1);

    // Create a task to forward internal channel messages and handle auth completion
    let mut send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(auth_data) = rx.recv() => {
                    let json = serde_json::to_string(&auth_data).unwrap();
                    if sender.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
                Some(ok_msg) = auth_done_rx.recv() => {
                    let json = serde_json::to_string(&ok_msg).unwrap();
                    let _ = sender.send(Message::Text(json.into())).await;
                    break;
                }
            }
        }
    });
    
    // Monitor client messages and validate OTP
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match serde_json::from_slice::<Value>(&msg.into_data()) {
                Ok(msg_json) => {
                    if msg_json["type"] == "AUTH_OTP" && msg_json["payload"]["otp"] == pin {
                        let ok_response = OneTimeAuthOK { r#type: "AUTH_OK".to_string()};
                        let ok_response_await = auth_done_tx.send(ok_response).await.is_ok();
                        
                        if ok_response_await { break }
                    }
                }
                Err(_) => continue,
            }
        }
    });

    // Define the timeout future
    let mut timeout_future = pin!(tokio::time::sleep(tokio::time::Duration::from_secs(60)));

    // Wait for both tasks to complete or timeout
    loop {
        tokio::select! {
            _ = &mut send_task => {
                // send_task completed, wait for recv_task
                let _ = (&mut recv_task).await;
                break;
            }
            _ = &mut recv_task => {
                // recv_task completed, wait for send_task
                let _ = (&mut send_task).await;
                break;
            }
            _ = &mut timeout_future => {
                println!("[WebSocket] Connection timed out");
                break;
            }
        }
    }

    send_task.abort();
    recv_task.abort();

    // Cleanup
    state.waiting_clients.remove(&uuid);
    println!("[WebSocket] Disconnected: {}", &uuid);
}
