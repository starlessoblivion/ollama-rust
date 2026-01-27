use axum::{
    extract::State,
    response::{sse::{Event, Sse}, Html},
    routing::{get, post},
    Json, Router,
};
use futures::stream::{StreamExt, BoxStream};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, process::Command, sync::Arc, time::Duration};
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;
use tower_http::services::ServeDir;
use tower_http::cors::CorsLayer;

#[derive(Deserialize)]
struct PromptRequest {
    model: String,
    prompt: String,
}

#[derive(Serialize)]
struct StatusResponse {
    running: bool,
    models: Vec<String>,
}

struct AppState {
    client: Client,
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        client: Client::new(),
    });

    let app = Router::new()
        // Serves the index.html at the root URL
        .route("/", get(|| async { 
            Html(include_str!("../static/index.html")) 
        }))
        // Correctly maps "/static/..." requests to the local "static" folder
        .nest_service("/static", ServeDir::new("static"))
        .route("/status", get(get_status))
        .route("/toggle-ollama", post(toggle_ollama))
        .route("/stream-run", post(stream_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "0.0.0.0:5000";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("🚀 Arch-Ollama server running on http://{}", addr);
    axum::serve(listener, app).await.unwrap();
}

// --- Logic Functions ---

fn check_process() -> bool {
    // Checks if the 'ollama' process is currently running
    Command::new("pgrep")
        .arg("-x")
        .arg("ollama")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn get_models() -> Vec<String> {
    // Queries the local ollama instance for available models
    let output = Command::new("ollama")
        .arg("list")
        .output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        stdout.lines()
            .skip(1)
            .filter_map(|line| line.split_whitespace().next())
            .map(|s| s.to_string())
            .collect()
    } else {
        vec![]
    }
}

// --- Handlers ---

async fn get_status() -> Json<StatusResponse> {
    Json(StatusResponse {
        running: check_process(),
        models: get_models(),
    })
}

async fn toggle_ollama() -> Json<StatusResponse> {
    if check_process() {
        // Stops the ollama serve process if it's running
        let _ = Command::new("pkill").arg("-x").arg("ollama").output();
    } else {
        // Starts the ollama serve process
        let _ = Command::new("ollama")
            .arg("serve")
            .spawn();
    }
    
    tokio::time::sleep(Duration::from_millis(800)).await;
    get_status().await
}

async fn stream_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PromptRequest>,
) -> Sse<BoxStream<'static, Result<Event, Infallible>>> {
    
    // Connects to the local Ollama API for generation
    let res = state.client
        .post("http://localhost:11434/api/generate")
        .json(&serde_json::json!({
            "model": payload.model,
            "prompt": payload.prompt,
            "stream": true
        }))
        .send()
        .await;

    match res {
        Ok(response) => {
            let body_with_io_error = response.bytes_stream().map(|res| {
                res.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            });
            let reader = StreamReader::new(body_with_io_error);
            let mut lines = FramedRead::new(reader, LinesCodec::new());

            let stream = async_stream::stream! {
                while let Some(Ok(line)) = lines.next().await {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                        if let Some(text) = json["response"].as_str() {
                            yield Ok(Event::default().data(text));
                        }
                        if json["done"].as_bool().unwrap_or(false) {
                            yield Ok(Event::default().data("__END__"));
                        }
                    }
                }
            };
            // Boxes the stream to match the expected return type
            Sse::new(stream.boxed())
        }
        Err(_) => {
            let error_stream = futures::stream::once(async { 
                Ok(Event::default().data("[Error: Ollama not reachable]")) 
            });
            Sse::new(error_stream.boxed())
        }
    }
}