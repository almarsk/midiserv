use axum::extract::{ws::WebSocket, ConnectInfo};
use axum::http::{StatusCode, Uri};
use axum::routing::{any_service, MethodRouter};
use axum::{extract::WebSocketUpgrade, response::IntoResponse, routing::get, Router};
use std::net::SocketAddr;
use tokio_tungstenite::WebSocketStream;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .fallback(fallback)
        .nest_service("/", serve_dir("build".to_string()))
        .nest_service("/assets", serve_dir("build/assets/".to_string()))
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

pub fn serve_dir(web_folder: String) -> MethodRouter {
    any_service(ServeDir::new(format!(
        "{}/{}",
        "crate_midiserv", web_folder
    )))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, addr))
}

async fn handle_socket(mut socket: WebSocket, addr: SocketAddr) {
    println!("{}", addr);
    tokio::spawn(async move {
        println!("lesgo");
        while let Some(Ok(m)) = socket.recv().await {
            println!("isin");
            if let Ok(text) = m.into_text() {
                println!("yay");
                println!("{}", text);
            } else {
                println!("whoops");
            }
        }
        println!("were done here")
    });
}
