use axum::{
    extract::{DefaultBodyLimit, Multipart, State},
    response::Html,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tower_http::services::ServeDir;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RemoteCommand {
    x: i32,
    y: i32,
    action: String,
}

type SharedState = Arc<Mutex<Option<RemoteCommand>>>;

#[tokio::main]
async fn main() {
    let state = SharedState::default();
    
    // 保存用ディレクトリ作成
    let _ = std::fs::create_dir_all("uploads");

    let app = Router::new()
        .route("/control", get(control_panel))
        .route("/send_command", post(receive_command))
        .route("/get_command", get(get_command))
        .route("/upload", post(accept_file))
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024)) // 50MB制限
        .nest_service("/view", ServeDir::new("uploads"))
        .with_state(state);

    println!("Server listening on http://127.0.0.1:3000");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn control_panel() -> Html<String> {
    Html(r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Rust Remote Control</title>
            <style>
                body { font-family: sans-serif; text-align: center; background: #222; color: white; }
                #screen { border: 2px solid #555; max-width: 90%; cursor: crosshair; margin-top: 20px; }
            </style>
        </head>
        <body>
            <h1>Remote View</h1>
            <img id="screen" src="/view/screenshot.png" onclick="sendClick(event)">
            <p>Click on the image to control the remote PC.</p>
            <script>
                function sendClick(e) {
                    const rect = e.target.getBoundingClientRect();
                    // 座標を元画像の解像度にスケーリング
                    const x = Math.round((e.clientX - rect.left) * (e.target.naturalWidth / rect.width));
                    const y = Math.round((e.clientY - rect.top) * (e.target.naturalHeight / rect.height));
                    
                    fetch('/send_command', {
                        method: 'POST',
                        headers: {'Content-Type': 'application/json'},
                        body: JSON.stringify({ x: x, y: y, action: "click" })
                    });
                    console.log(`Clicked: ${x}, ${y}`);
                }
                // 画像を2秒ごとに更新
                setInterval(() => {
                    const img = document.getElementById('screen');
                    img.src = "/view/screenshot.png?t=" + Date.now();
                }, 2000);
            </script>
        </body>
        </html>
    "#.to_string())
}

async fn accept_file(mut multipart: Multipart) {
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            if let Ok(data) = field.bytes().await {
                let _ = std::fs::write("uploads/screenshot.png", data);
            }
        }
    }
}

async fn receive_command(State(state): State<SharedState>, Json(cmd): Json<RemoteCommand>) {
    let mut lock = state.lock().unwrap();
    *lock = Some(cmd);
}

async fn get_command(State(state): State<SharedState>) -> Json<Option<RemoteCommand>> {
    let mut lock = state.lock().unwrap();
    Json(lock.take())
}