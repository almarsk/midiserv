use axum::extract::ws::Message;
use axum::extract::{Query, State};
use axum::http::{StatusCode, Uri};
use axum::routing::{any_service, MethodRouter};
use axum::Json;
use axum::{
    extract::WebSocketUpgrade,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use dotenv::dotenv;
use flume::{bounded, Receiver, Sender};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;
use util::{Device, DeviceUpdate};

struct AppState {
    connected: Mutex<bool>,
    exposed_devices: Mutex<HashMap<u8, Device>>,
    password: String,
    server_name: String,
    bridge_tx: Sender<Message>,
    bridge_rx: Receiver<Message>,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let (bridge_tx, bridge_rx): (Sender<Message>, Receiver<Message>) = bounded(10);

    let shared_state = Arc::new(AppState {
        connected: Mutex::new(false),
        exposed_devices: Mutex::new(HashMap::new()),
        password: env::var("WS_PASSWORD").expect("WS_PASSWORD must be set"),
        server_name: env::var("SERVER_NAME").expect("SERVER_NAME must be set"),
        bridge_tx,
        bridge_rx,
    });

    let app = Router::new()
        .fallback(fallback)
        .route("/login", get(local_ws_handler))
        .route("/devices", post(update_devices))
        .nest_service("/", serve_dir("build".to_string()))
        .route("/ws", get(user_ws_handler))
        .nest_service("/assets", serve_dir("build/assets/".to_string()))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

async fn fallback(uri: Uri) -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, format!("not found: {uri}"))
}

pub fn serve_dir(web_folder: String) -> MethodRouter {
    any_service(ServeDir::new(format!("{}/{}", "server", web_folder)))
}

#[derive(Deserialize)]
struct ConnectQuery {
    password: String,
}

async fn local_ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<ConnectQuery>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if query.password != state.password {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "success": false,
                "error": "Invalid password",
            })),
        )
            .into_response();
    }
    {
        let already_connected = state.connected.lock().await;
        if *already_connected {
            return (
                StatusCode::CONFLICT,
                Json(json!({
                    "success": false,
                    "error": "Already connected",
                })),
            )
                .into_response();
        }
    }

    let state = Arc::clone(&state);
    ws.on_upgrade(move |mut socket| async move {
        let _ = socket.send(Message::Text(state.server_name.clone())).await;

        *state.connected.lock().await = true;
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    message = state.bridge_rx.recv_async() => {
                            if let Ok(message) = message {
                                let _ = socket.send(message).await;
                            }
                    }
                    m = socket.recv() => {
                        if let Some(m) = m {
                            match m {
                                Ok(m) => {
                                    match m {
                                        Message::Close(_) => {
                                            *state.connected.lock().await = false;
                                            state.exposed_devices.lock().await.clear();
                                            break;
                                        },
                                        _ => {}
                                    }
                                },
                                Err(_) => {
                                    *state.connected.lock().await = false;
                                    break;
                                }
                            }

                        }
                    }
                }
            }
        });
    })
}

async fn update_devices(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ConnectQuery>,
    Json(update): Json<DeviceUpdate>,
) -> impl IntoResponse {
    if query.password != state.password {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "success": false,
                "error": "Invalid password",
            })),
        );
    }
    let mut exposed_devices = state.exposed_devices.lock().await;

    match update {
        DeviceUpdate::Add(additions) => {
            additions.into_iter().for_each(|a| {
                let _ = exposed_devices.insert(a.cc, a);
            });
        }
        DeviceUpdate::Remove(removals) => {
            removals.into_iter().for_each(|r| {
                let _ = exposed_devices.remove(&(r as u8));
            });
        }
        DeviceUpdate::Clear => exposed_devices.clear(),
    };

    let mut exp_dev = exposed_devices
        .values()
        .cloned()
        .collect::<Vec<Device>>()
        .clone();
    exp_dev.sort_by_key(|d| d.cc);

    println!("{:?}", exp_dev);

    (StatusCode::OK, Json(json!(exp_dev)))
}

async fn user_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let state = Arc::clone(&state);
    ws.on_upgrade(move |mut user_socket| async {
        tokio::spawn(async move {
            while let Some(Ok(m)) = user_socket.recv().await {
                if *state.connected.lock().await {
                    let _ = state.bridge_tx.send(m);
                }
            }
        });
    })
}
