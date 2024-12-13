use axum::extract::ws::WebSocket;
use axum::extract::{Query, State};
use axum::http::{StatusCode, Uri};
use axum::routing::{any_service, MethodRouter};
use axum::Json;
use axum::{extract::WebSocketUpgrade, response::IntoResponse, routing::get, Router};
use dotenv::dotenv;
use serde::Deserialize;
use serde_json::json;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;
use util::Device;

struct AppState {
    connected: Mutex<bool>,
    socket: Mutex<Option<WebSocket>>,
    _exposed_devices: Mutex<Vec<Device>>,
    password: String,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let shared_state = Arc::new(AppState {
        connected: Mutex::new(false),
        socket: Mutex::new(None),
        _exposed_devices: Mutex::new(Vec::new()),
        password: env::var("WS_PASSWORD").expect("WS_PASSWORD must be set"),
    });

    let app = Router::new()
        .fallback(fallback)
        .route("/login", get(local_ws_handler))
        .route("/logout", get(disconnect_local_ws_handle))
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
        return Json(json!({
            "success": false,
            "error": "Invalid password",
        }))
        .into_response();
    }

    let already_connected = state.connected.lock().await;
    if *already_connected {
        return Json(json!({
            "success": false,
            "error": "Already connected",
        }))
        .into_response();
    }

    let state = Arc::clone(&state);
    ws.on_upgrade(move |socket| async move {
        let mut guard = state.socket.lock().await;
        *guard = Some(socket);
        *state.connected.lock().await = true;
    })
}

async fn disconnect_local_ws_handle(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    *Arc::clone(&state).socket.lock().await = None;
    *Arc::clone(&state).connected.lock().await = false;
}

async fn user_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let state = Arc::clone(&state);
    ws.on_upgrade(move |mut user_socket| async {
        tokio::spawn(async move {
            let mut guard = state.socket.lock().await;
            while let Some(Ok(m)) = user_socket.recv().await {
                if let Some(ref mut local_socket) = *guard {
                    let _ = local_socket.send(m).await;
                }
            }
        });
    })
}
