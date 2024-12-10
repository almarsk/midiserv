use axum::extract::ws::WebSocket;
use axum::extract::State;
use axum::http::{StatusCode, Uri};
use axum::routing::{any_service, MethodRouter};
use axum::Json;
use axum::{extract::WebSocketUpgrade, response::IntoResponse, routing::get, Router};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

struct AppState {
    socket: Mutex<Option<WebSocket>>,
}

#[tokio::main]
async fn main() {
    let shared_state = Arc::new(AppState {
        socket: Mutex::new(None),
    });

    let app = Router::new()
        .fallback(fallback)
        .nest_service("/", serve_dir("build".to_string()))
        .nest_service("/assets", serve_dir("build/assets/".to_string()))
        .route("/ws", get(user_ws_handler))
        .route("/login", get(login))
        .route("/ws_loc", get(local_ws_handler))
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

async fn login() -> impl IntoResponse {
    println!("login attempt");
    let response = json!({
        "success": true,
    });
    Json(response).into_response()
}

async fn local_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    println!("local here");
    let state = Arc::clone(&state);
    ws.on_upgrade(move |socket| async move {
        println!("local business");
        let mut guard = state.socket.lock().await;
        *guard = Some(socket);
    })
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
                println!("{:?}", m.clone().into_data());
                if let Some(ref mut local_socket) = *guard {
                    println!("got guard; sendin message");
                    let _ = local_socket.send(m).await;
                }
            }
        });
    })
}
