use axum::extract::{ws::WebSocket, ConnectInfo};
use axum::http::{StatusCode, Uri};
use axum::{extract::WebSocketUpgrade, response::IntoResponse, routing::get, Router};
use std::net::SocketAddr;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .fallback(fallback)
        .nest_service("/", ServeDir::new("build"))
        .nest_service("/assets", ServeDir::new("build/assets"))
        .route("/ws", get(ws_handler));

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

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, addr))
}

async fn handle_socket(socket: WebSocket, addr: SocketAddr) {
    // todo setup thread to send messages
    let _ = socket;
    println!("{}", addr);
}
